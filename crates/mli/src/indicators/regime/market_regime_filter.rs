//! Market Regime Filter - фильтр рыночных режимов
//!
//! Определяет текущий режим рынка: тренд, флэт, высокая волатильность, спокойствие.
//! Использует комбинацию индикаторов для классификации рыночных условий.
//!
//! Переиспользует существующие компоненты: SmootherSlot, ATR, momentum индикаторы

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::volatility::atr::Atr;
use std::collections::HashMap;

/// Типы рыночных режимов
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketRegime {
    UpTrend,        // Восходящий тренд
    DownTrend,      // Нисходящий тренд
    SidewaysFlat,   // Боковой флэт (низкая волатильность)
    ChoppyFlat,     // Рваный флэт (средняя волатильность)
    HighVolatility, // Высокая волатильность
    Quiet,          // Спокойный рынок
    Transition,     // Переходное состояние
}

impl MarketRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            MarketRegime::UpTrend => "Восходящий тренд",
            MarketRegime::DownTrend => "Нисходящий тренд",
            MarketRegime::SidewaysFlat => "Боковой флэт",
            MarketRegime::ChoppyFlat => "Рваный флэт",
            MarketRegime::HighVolatility => "Высокая волатильность",
            MarketRegime::Quiet => "Спокойный рынок",
            MarketRegime::Transition => "Переходное состояние",
        }
    }

    pub fn as_number(&self) -> i8 {
        match self {
            MarketRegime::UpTrend => 2,
            MarketRegime::DownTrend => -2,
            MarketRegime::SidewaysFlat => 0,
            MarketRegime::ChoppyFlat => 1,
            MarketRegime::HighVolatility => 3,
            MarketRegime::Quiet => -1,
            MarketRegime::Transition => 99,
        }
    }
}

/// Результат Market Regime Filter
#[derive(Debug, Clone, Copy)]
pub struct MarketRegimeResult {
    pub regime: MarketRegime,        // Текущий режим рынка
    pub confidence: f64,             // Уверенность в определении (0.0-1.0)
    pub trend_strength: f64,         // Сила тренда (0.0-1.0)
    pub volatility_level: f64,       // Уровень волатильности (0.0-1.0+)
    pub momentum_score: f64,         // Оценка momentum (-1.0 до 1.0)
    pub stability_index: f64,        // Индекс стабильности (0.0-1.0)
    pub regime_duration: usize,      // Продолжительность текущего режима в барах
    pub next_regime_probability: f64, // Вероятность смены режима (0.0-1.0)
}

impl MarketRegimeResult {
    pub fn empty() -> Self {
        Self {
            regime: MarketRegime::Transition,
            confidence: 0.0,
            trend_strength: 0.0,
            volatility_level: 0.5,
            momentum_score: 0.0,
            stability_index: 0.5,
            regime_duration: 0,
            next_regime_probability: 0.5,
        }
    }
}

/// Market Regime Filter индикатор
#[derive(Debug, Clone)]
pub struct MarketRegimeFilter {
    // Smoother slots — configurable MA shapes
    fast_ma: SmootherSlot,           // Быстрая MA для тренда
    fast_period: usize,
    slow_ma: SmootherSlot,           // Медленная MA для тренда
    slow_period: usize,
    atr: Atr,                        // ATR для волатильности
    volatility_ma: SmootherSlot,     // MA для сглаживания волатильности
    momentum_ma: SmootherSlot,       // MA для momentum
    stability_ma: SmootherSlot,      // MA для стабильности

    // Буферы для анализа
    prices: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    ranges: Vec<f64>,
    regime_history: Vec<MarketRegime>,

    // Параметры анализа
    trend_threshold: f64,            // Порог для определения тренда
    volatility_threshold: f64,       // Порог высокой волатильности
    quiet_threshold: f64,            // Порог спокойного рынка

    // Текущее состояние
    current_regime: MarketRegime,
    regime_start_time: usize,

    // Результат
    current_result: MarketRegimeResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl MarketRegimeFilter {
    /// Создать новый Market Regime Filter с параметрами по умолчанию.
    /// Defaults: fast_ema=10, slow_ema=30, atr_wilder(14), vol_sma(20), mom_ema(8), stab_sma(15).
    pub fn new() -> Self {
        Self::from_smoothers(
            SmootherId::Ema, 10,
            SmootherId::Ema, 30,
            14,
            SmootherId::Sma, 20,
            SmootherId::Ema, 8,
            SmootherId::Sma, 15,
            0.02, 1.5, 0.3,
        )
    }

    /// Создать с настраиваемыми параметрами trend/threshold (fast_ma и slow_ma = EMA).
    pub fn with_parameters(
        fast_period: usize,
        slow_period: usize,
        trend_threshold: f64,
        volatility_threshold: f64,
        quiet_threshold: f64,
    ) -> Self {
        Self::from_smoothers(
            SmootherId::Ema, fast_period,
            SmootherId::Ema, slow_period,
            14,
            SmootherId::Sma, 20,
            SmootherId::Ema, 8,
            SmootherId::Sma, 15,
            trend_threshold,
            volatility_threshold,
            quiet_threshold,
        )
    }

    /// Full-config ctor: exposes all slot IDs + periods.
    #[allow(clippy::too_many_arguments)]
    pub fn from_smoothers(
        fast_id: SmootherId,
        fast_period: usize,
        slow_id: SmootherId,
        slow_period: usize,
        atr_period: usize,
        volatility_id: SmootherId,
        volatility_period: usize,
        momentum_id: SmootherId,
        momentum_period: usize,
        stability_id: SmootherId,
        stability_period: usize,
        trend_threshold: f64,
        volatility_threshold: f64,
        quiet_threshold: f64,
    ) -> Self {
        assert!(fast_period > 0 && slow_period > fast_period, "Invalid MA periods");
        assert!(trend_threshold > 0.0, "Trend threshold must be positive");
        assert!(volatility_threshold > 1.0, "Volatility threshold must be > 1.0");
        assert!(quiet_threshold > 0.0 && quiet_threshold < 1.0, "Invalid quiet threshold");

        Self {
            fast_ma: SmootherSlot::new(fast_id, fast_period),
            fast_period,
            slow_ma: SmootherSlot::new(slow_id, slow_period),
            slow_period,
            atr: Atr::new_wilder(atr_period),
            volatility_ma: SmootherSlot::new(volatility_id, volatility_period),
            momentum_ma: SmootherSlot::new(momentum_id, momentum_period),
            stability_ma: SmootherSlot::new(stability_id, stability_period),

            prices: Vec::with_capacity(64),
            highs: Vec::with_capacity(32),
            lows: Vec::with_capacity(32),
            ranges: Vec::with_capacity(32),
            regime_history: Vec::with_capacity(32),

            trend_threshold,
            volatility_threshold,
            quiet_threshold,

            current_regime: MarketRegime::Transition,
            regime_start_time: 0,

            current_result: MarketRegimeResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed resolved input lanes — `[open, high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> MarketRegimeResult {
        let high  = lanes[1];
        let low   = lanes[2];
        let close = lanes[3];

        // Добавляем данные в буферы
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(close);

        if self.highs.len() >= 32 {
            self.highs.remove(0);
        }
        self.highs.push(high);

        if self.lows.len() >= 32 {
            self.lows.remove(0);
        }
        self.lows.push(low);

        let range = high - low;
        if self.ranges.len() >= 32 {
            self.ranges.remove(0);
        }
        self.ranges.push(range);

        // 1. Обновляем все индикаторы
        let fast_ma_value = self.fast_ma.feed(close);
        let slow_ma_value = self.slow_ma.feed(close);
        let atr_value = self.atr.feed(&[high, low, close]);

        // 2. Анализируем тренд
        let trend_analysis = self.analyze_trend(close, fast_ma_value, slow_ma_value);

        // 3. Анализируем волатильность
        let volatility_analysis = self.analyze_volatility(atr_value);

        // 4. Анализируем momentum
        let momentum_analysis = self.analyze_momentum(close);

        // 5. Анализируем стабильность
        let stability_analysis = self.analyze_stability();

        // 6. Определяем режим рынка
        let new_regime = self.determine_regime(
            trend_analysis,
            volatility_analysis,
            momentum_analysis,
            stability_analysis
        );

        // 7. Обновляем состояние режима
        self.update_regime_state(new_regime);

        // 8. Рассчитываем уверенность и дополнительные метрики
        self.calculate_confidence_and_metrics(
            trend_analysis,
            volatility_analysis,
            momentum_analysis,
            stability_analysis
        );

        // Готов после накопления достаточных данных
        if self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.atr.is_ready() {
            self.is_ready = true;
        }

        self.update_count += 1;
        self.current_result
    }


    /// Анализировать тренд
    fn analyze_trend(&self, price: f64, fast_ma: f64, slow_ma: f64) -> (f64, i8) {
        let ma_distance = if slow_ma != 0.0 {
            (fast_ma - slow_ma).abs() / slow_ma
        } else {
            0.0
        };

        let trend_strength = (ma_distance / self.trend_threshold).min(1.0);

        let trend_direction = if fast_ma > slow_ma && price > fast_ma {
            1
        } else if fast_ma < slow_ma && price < fast_ma {
            -1
        } else {
            0
        };

        (trend_strength, trend_direction)
    }

    /// Анализировать волатильность
    fn analyze_volatility(&mut self, atr_value: f64) -> f64 {
        let smoothed_volatility = self.volatility_ma.feed(atr_value);

        if smoothed_volatility > 0.0 {
            atr_value / smoothed_volatility
        } else {
            1.0
        }
    }

    /// Анализировать momentum
    fn analyze_momentum(&mut self, price: f64) -> f64 {
        if self.prices.len() < 5 {
            return 0.0;
        }

        let len = self.prices.len();
        let momentum = price - self.prices[len - 5];

        let smoothed_momentum = self.momentum_ma.feed(momentum);

        if price != 0.0 {
            (smoothed_momentum / price).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    /// Анализировать стабильность
    fn analyze_stability(&mut self) -> f64 {
        if self.ranges.len() < 10 {
            return 0.5;
        }

        let recent_ranges = &self.ranges[self.ranges.len() - 10..];
        let mean_range: f64 = recent_ranges.iter().sum::<f64>() / recent_ranges.len() as f64;

        if mean_range > 0.0 {
            let variance: f64 = recent_ranges.iter()
                .map(|&r| (r - mean_range).powi(2))
                .sum::<f64>() / recent_ranges.len() as f64;

            let cv = variance.sqrt() / mean_range;
            let stability = (1.0 - cv.min(1.0)).max(0.0);

            self.stability_ma.feed(stability)
        } else {
            0.5
        }
    }

    /// Определить режим рынка
    fn determine_regime(
        &self,
        trend: (f64, i8),
        volatility: f64,
        _momentum: f64,
        stability: f64
    ) -> MarketRegime {
        let (trend_strength, trend_direction) = trend;

        if volatility > self.volatility_threshold {
            MarketRegime::HighVolatility
        } else if volatility < self.quiet_threshold {
            MarketRegime::Quiet
        } else if trend_strength > 0.6 && stability > 0.5 {
            match trend_direction {
                1 => MarketRegime::UpTrend,
                -1 => MarketRegime::DownTrend,
                _ => MarketRegime::Transition,
            }
        } else if trend_strength < 0.3 {
            if stability > 0.6 {
                MarketRegime::SidewaysFlat
            } else {
                MarketRegime::ChoppyFlat
            }
        } else {
            MarketRegime::Transition
        }
    }

    /// Обновить состояние режима
    fn update_regime_state(&mut self, new_regime: MarketRegime) {
        if new_regime != self.current_regime {
            if self.regime_history.len() >= 32 {
                self.regime_history.remove(0);
            }
            self.regime_history.push(self.current_regime);

            self.current_regime = new_regime;
            self.regime_start_time = self.update_count;
        }

        self.current_result.regime = self.current_regime;
        self.current_result.regime_duration = self.update_count - self.regime_start_time;
    }

    /// Рассчитать уверенность и дополнительные метрики
    fn calculate_confidence_and_metrics(
        &mut self,
        trend: (f64, i8),
        volatility: f64,
        momentum: f64,
        stability: f64
    ) {
        let (trend_strength, _trend_direction) = trend;

        let volatility_confidence = match self.current_regime {
            MarketRegime::HighVolatility => (volatility - self.volatility_threshold).min(1.0),
            MarketRegime::Quiet => (self.quiet_threshold - volatility).max(0.0) / self.quiet_threshold,
            _ => 1.0 - (volatility - 1.0).abs().min(1.0),
        };

        let trend_confidence = match self.current_regime {
            MarketRegime::UpTrend | MarketRegime::DownTrend => trend_strength,
            MarketRegime::SidewaysFlat | MarketRegime::ChoppyFlat => 1.0 - trend_strength,
            _ => 0.5,
        };

        let stability_confidence = match self.current_regime {
            MarketRegime::SidewaysFlat | MarketRegime::Quiet => stability,
            MarketRegime::ChoppyFlat | MarketRegime::HighVolatility => 1.0 - stability,
            _ => 0.7,
        };

        self.current_result.confidence = (volatility_confidence + trend_confidence + stability_confidence) / 3.0;

        self.current_result.trend_strength = trend_strength;
        self.current_result.volatility_level = volatility;
        self.current_result.momentum_score = momentum;
        self.current_result.stability_index = stability;

        self.calculate_regime_change_probability();
    }

    /// Рассчитать вероятность смены режима
    fn calculate_regime_change_probability(&mut self) {
        let base_probability = match self.current_result.regime_duration {
            0..=5 => 0.1,
            6..=15 => 0.3,
            16..=30 => 0.5,
            _ => 0.7,
        };

        let confidence_factor = 1.0 - self.current_result.confidence;
        self.current_result.next_regime_probability = (base_probability + confidence_factor * 0.3).min(1.0);
    }

    /// Получить текущий режим как MarketRegime
    pub fn regime(&self) -> MarketRegime {
        self.current_result.regime
    }

    /// Получить значение (Signal)
    pub fn value(&self) -> f64 {
        (self.current_result.regime.as_number()) as f64
    }

    /// Получить полный результат
    pub fn result(&self) -> MarketRegimeResult {
        self.current_result
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.atr.reset();
        self.volatility_ma.reset();
        self.momentum_ma.reset();
        self.stability_ma.reset();

        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.ranges.clear();
        self.regime_history.clear();

        self.current_regime = MarketRegime::Transition;
        self.regime_start_time = 0;

        self.current_result = MarketRegimeResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    /// Получить период (условный)
    pub fn period(&self) -> usize {
        self.slow_period
    }

    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;

        format!(
            "Режим: {}, Уверенность: {:.1}%, Продолжительность: {} баров, Тренд: {:.1}%, Волатильность: {:.1}%",
            result.regime.as_str(),
            result.confidence * 100.0,
            result.regime_duration,
            result.trend_strength * 100.0,
            result.volatility_level * 100.0
        )
    }

    /// Получить дополнительные значения
    pub fn additional_values(&self) -> HashMap<String, f64> {
        let mut values = HashMap::new();
        values.insert("regime_number".to_string(), self.current_result.regime.as_number() as f64);
        values.insert("confidence".to_string(), self.current_result.confidence);
        values.insert("trend_strength".to_string(), self.current_result.trend_strength);
        values.insert("volatility_level".to_string(), self.current_result.volatility_level);
        values.insert("momentum_score".to_string(), self.current_result.momentum_score);
        values.insert("stability_index".to_string(), self.current_result.stability_index);
        values.insert("regime_duration".to_string(), self.current_result.regime_duration as f64);
        values.insert("change_probability".to_string(), self.current_result.next_regime_probability);
        values
    }

    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Получить историю режимов
    pub fn regime_history(&self) -> Vec<MarketRegime> {
        self.regime_history.iter().copied().collect()
    }

    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, f64, f64, f64) {
        (
            self.fast_period,
            self.slow_period,
            self.trend_threshold,
            self.volatility_threshold,
            self.quiet_threshold
        )
    }
}

impl Default for MarketRegimeFilter {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`MarketRegimeFilter`].
/// Five independent period fields + five follow slots (trend-fast, trend-slow, volatility,
/// momentum, stability). ATR period is a separate plain field. Scalars are `Param` for
/// dual-mode iteration.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct MarketRegimeFilterConfig {
    pub atr_period: Param<usize>,
    pub trend_threshold: Param<f64>,
    pub volatility_threshold: Param<f64>,
    pub quiet_threshold: Param<f64>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    pub volatility_period: Param<usize>,
    pub momentum_period: Param<usize>,
    pub stability_period: Param<usize>,
    #[slot]
    pub fast_ma: Param<SmootherChoice>,
    #[slot]
    pub slow_ma: Param<SmootherChoice>,
    #[slot]
    pub volatility_ma: Param<SmootherChoice>,
    #[slot]
    pub momentum_ma: Param<SmootherChoice>,
    #[slot]
    pub stability_ma: Param<SmootherChoice>,
}

impl Indicator for MarketRegimeFilter {
    const ID: IndicatorId = IndicatorId::Mrf;
    /// Regime classifier — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Open/High/Low/Close — open+high+low drive the embedded Atr; close drives the MAs.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = MarketRegimeFilterConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Mrf)];
    type Config = MarketRegimeFilterConfig;
    type Runtime = MarketRegimeFilter;

    fn create(cfg: MarketRegimeFilterConfig) -> MarketRegimeFilter {
        let fast_period = cfg.fast_period.resolved();
        let slow_period = cfg.slow_period.resolved();
        let volatility_period = cfg.volatility_period.resolved();
        let momentum_period = cfg.momentum_period.resolved();
        let stability_period = cfg.stability_period.resolved();
        MarketRegimeFilter::from_smoothers(
            cfg.fast_ma.resolved().kind,
            fast_period,
            cfg.slow_ma.resolved().kind,
            slow_period,
            cfg.atr_period.resolved(),
            cfg.volatility_ma.resolved().kind,
            volatility_period,
            cfg.momentum_ma.resolved().kind,
            momentum_period,
            cfg.stability_ma.resolved().kind,
            stability_period,
            cfg.trend_threshold.resolved(),
            cfg.volatility_threshold.resolved(),
            cfg.quiet_threshold.resolved(),
        )
    }

    fn slot_members(cfg: &MarketRegimeFilterConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for MarketRegimeFilterConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast == 0 || slow <= fast {
            return Err(format!("fast_period({fast}) must be > 0 and slow_period({slow}) must be > fast_period"));
        }
        let vt = self.volatility_threshold.resolved();
        if vt <= 1.0 {
            return Err(format!("volatility_threshold({vt}) must be > 1.0"));
        }
        let qt = self.quiet_threshold.resolved();
        if qt <= 0.0 || qt >= 1.0 {
            return Err(format!("quiet_threshold({qt}) must be in (0.0, 1.0)"));
        }
        let tt = self.trend_threshold.resolved();
        if tt <= 0.0 {
            return Err(format!("trend_threshold({tt}) must be > 0.0"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        MarketRegimeFilterConfig {
            atr_period: Param::Solo(14),
            trend_threshold: Param::Solo(0.02),
            volatility_threshold: Param::Solo(1.5),
            quiet_threshold: Param::Solo(0.3),
            fast_period: Param::Solo(10),
            slow_period: Param::Solo(30),
            volatility_period: Param::Solo(20),
            momentum_period: Param::Solo(8),
            stability_period: Param::Solo(15),
            fast_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            volatility_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            momentum_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            stability_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // trend_threshold: Class F sigma/threshold — sweep_f64(0.1, 5.0, 0.1).
        // Ambiguous: default is 0.02 (normalized MA distance ratio), much smaller than the
        // Class F floor of 0.1. Choosing Class F per taxonomy; the 0.01-0.09 sub-range is
        // intentionally excluded as it is below the standard floor.
        s.trend_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        // volatility_threshold: Class F — sweep_f64(0.1, 5.0, 0.1). Must stay > 1.0 per
        // `valid_params`; 0.1 as a floor here is already excluded by that same gate, so the
        // min-corner resolve (0.1) would fail regardless of fast/slow — override to keep the
        // min inside the valid (1.0, ∞) region.
        s.volatility_threshold = Param::many(sweep_f64(1.1, 5.0, 0.1));
        // quiet_threshold: Class F, but `valid_params` requires it strictly inside (0.0, 1.0) —
        // sweep_f64(0.1, 5.0, 0.1)'s upper values would violate that gate; narrow to the
        // indicator's real bounded range.
        s.quiet_threshold = Param::many(sweep_f64(0.1, 0.9, 0.05));
        // fast_period/slow_period: Class A usize — auto range(2,4048,1) EACH, but both then
        // resolve to the same min (2), failing this config's OWN `valid_params`
        // (slow > fast) at the min corner (2026-07-03 fix). Split into disjoint ranges so
        // `resolved()` stays ordered; atr_period/volatility_period/momentum_period/
        // stability_period (independent lanes) keep the full auto range.
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        // fast_ma / slow_ma / volatility_ma / momentum_ma / stability_ma:
        //   #[slot] SmootherChoice — deferred wave, leave Solo.
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MarketRegimeFilter {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Mrf, "Mean Rev Filter", Color::hex(0x4CAF50))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_market_regime_filter_creation() {
        let mrf = MarketRegimeFilter::new();
        assert!(!mrf.is_ready());
        assert_eq!(mrf.regime(), MarketRegime::Transition);
    }

    #[test]
    fn test_market_regime_filter_with_parameters() {
        let mrf = MarketRegimeFilter::with_parameters(5, 20, 0.01, 2.0, 0.2);
        assert_eq!(mrf.parameters(), (5, 20, 0.01, 2.0, 0.2));
    }

    #[test]
    fn test_market_regime_filter_with_config() {
        let mut mrf = MarketRegimeFilter::from_smoothers(
            SmootherId::Ema, 8,
            SmootherId::Ema, 20,
            10,
            SmootherId::Ema, 15,
            SmootherId::Ema, 5,
            SmootherId::Ema, 10,
            0.02, 1.5, 0.3,
        );
        assert!(!mrf.is_ready());
        let mut price = 100.0;
        for _ in 0..40 {
            price += 0.5;
            let result = mrf.feed(&[price, price + 0.2, price - 0.2, price]);
            if mrf.is_ready() {
                assert!(result.confidence.is_finite());
                assert!(result.trend_strength.is_finite());
                assert!(result.volatility_level.is_finite());
            }
        }
        assert!(mrf.is_ready());
    }

    #[test]
    fn test_regime_detection() {
        let mut mrf = MarketRegimeFilter::new();

        for i in 0..30 {
            let price = 100.0 + i as f64 * 0.5;
            let result = mrf.feed(&[price, price + 0.2, price - 0.2, price]);

            if i > 25 && mrf.is_ready() {
                assert!(matches!(result.regime, MarketRegime::UpTrend | MarketRegime::Transition));
                assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
                assert!(result.trend_strength >= 0.0 && result.trend_strength <= 1.0);
            }
        }
    }

    #[test]
    fn test_regime_transitions() {
        let mut mrf = MarketRegimeFilter::new();
        let mut previous_regime = MarketRegime::Transition;
        let mut regime_changes = 0;

        for i in 0..50 {
            let price = match i {
                0..=15 => 100.0 + i as f64 * 0.1,
                16..=25 => 101.5 + (i as f64 * 0.1).sin(),
                26..=35 => 102.0 + i as f64 * 0.5,
                _ => 120.0 + (i as f64 * 0.5).sin() * 3.0,
            };

            let result = mrf.feed(&[price, price + 0.5, price - 0.5, price]);

            if mrf.is_ready() && result.regime != previous_regime {
                regime_changes += 1;
                previous_regime = result.regime;
            }
        }

        assert!(regime_changes > 0);
    }

    #[test]
    fn factory_feeds_resolved_mrf() {
        let mut f = IndicatorOrder::Mrf(<<MarketRegimeFilter as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..40 {
            let price = 100.0 + i as f64 * 0.5;
            f.feed(0, MarketSample::Bar {
                open: price - 0.1,
                high: price + 0.2,
                low: price - 0.2,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // Signal is an i8 cast to f64
        assert!(f.primary().is_finite());
    }
}
