//! Dynamic Volatility Regime - Advanced volatility regime detection
//!
//! This indicator detects and classifies different volatility regimes using
//! multiple methodologies: GARCH-like analysis, regime switching models,
//! and adaptive thresholds. It provides early warning of volatility shifts.
//!
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::volatility::atr::Atr;
use crate::indicators::utils::math::percentile::quickselect_nth;

/// Тип режима волатильности
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolatilityRegime {
    VeryLow,      // Очень низкая волатильность
    Low,          // Низкая волатильность
    Normal,       // Нормальная волатильность
    High,         // Высокая волатильность
    VeryHigh,     // Очень высокая волатильность
    Extreme,      // Экстремальная волатильность
}

impl VolatilityRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            VolatilityRegime::VeryLow => "Очень низкая",
            VolatilityRegime::Low => "Низкая",
            VolatilityRegime::Normal => "Нормальная",
            VolatilityRegime::High => "Высокая",
            VolatilityRegime::VeryHigh => "Очень высокая",
            VolatilityRegime::Extreme => "Экстремальная",
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            VolatilityRegime::VeryLow => 1.0,
            VolatilityRegime::Low => 2.0,
            VolatilityRegime::Normal => 3.0,
            VolatilityRegime::High => 4.0,
            VolatilityRegime::VeryHigh => 5.0,
            VolatilityRegime::Extreme => 6.0,
        }
    }
}

/// Результат Dynamic Volatility Regime
#[derive(Debug, Clone, Copy)]
pub struct DynamicVolatilityRegimeResult {
    pub current_regime: VolatilityRegime,    // Текущий режим
    pub regime_probability: f64,             // Вероятность текущего режима (0.0-1.0)
    pub volatility_score: f64,               // Оценка волатильности (0.0-10.0)
    pub regime_persistence: f64,             // Устойчивость режима (0.0-1.0)
    pub transition_probability: f64,         // Вероятность смены режима (0.0-1.0)
    pub volatility_trend: f64,               // Тренд волатильности (-1.0 до 1.0)
    pub adaptive_threshold_low: f64,         // Адаптивный нижний порог
    pub adaptive_threshold_high: f64,        // Адаптивный верхний порог
    pub garch_volatility: f64,               // GARCH-подобная волатильность
    pub regime_signal: i8,                   // Сигнал: 1 (рост волат.), -1 (падение), 0 (стабильно)
}

impl DynamicVolatilityRegimeResult {
    pub fn empty() -> Self {
        Self {
            current_regime: VolatilityRegime::Normal,
            regime_probability: 0.0,
            volatility_score: 3.0,
            regime_persistence: 0.0,
            transition_probability: 0.0,
            volatility_trend: 0.0,
            adaptive_threshold_low: 0.0,
            adaptive_threshold_high: 0.0,
            garch_volatility: 0.0,
            regime_signal: 0,
        }
    }
}

/// Dynamic Volatility Regime индикатор
#[derive(Debug, Clone)]
pub struct DynamicVolatilityRegime {
    // Переиспользуем существующие компоненты
    atr: Atr,                               // ATR — smoothing controlled by #[slot]
    volatility_ma: SmootherSlot,            // MA для сглаживания волатильности (EMA/10 baked)
    long_term_vol: SmootherSlot,            // Долгосрочная волатильность (SMA/50 baked)
    regime_smoother: SmootherSlot,          // Сглаживание режима (EMA/5 baked)

    // Буферы для расчетов
    returns: Vec<f64>,
    volatilities: Vec<f64>,
    regime_scores: Vec<f64>,
    regime_history: Vec<VolatilityRegime>,

    // GARCH-подобные параметры
    garch_alpha: f64,
    garch_beta: f64,   // derived: garch_persistence * (1 - garch_alpha)
    garch_omega: f64,
    conditional_variance: f64,

    // Адаптивные пороги
    threshold_adaptation_speed: f64,

    // Результат
    current_result: DynamicVolatilityRegimeResult,

    // Состояние
    prev_price: Option<f64>,
    is_ready: bool,
    update_count: usize,
}

impl DynamicVolatilityRegime {
    /// Создать новый Dynamic Volatility Regime с параметрами по умолчанию.
    /// ATR smoothed with SMA (original default).
    pub fn new() -> Self {
        // persistence = old_beta / (1 - old_alpha) = 0.85 / 0.9 ≈ 0.9444
        Self::from_smoothers(0.1, 0.85 / 0.9, 0.01, 0.05, SmootherId::Sma)
    }

    /// Создать с настраиваемыми параметрами.
    ///
    /// `garch_persistence` — the share of prior variance retained, ∈ (0, 1).
    /// GARCH beta is derived internally: `beta = garch_persistence * (1 - garch_alpha)`,
    /// which guarantees `alpha + beta < 1` (stationarity) BY CONSTRUCTION for any
    /// `alpha, garch_persistence ∈ (0, 1)`.
    ///
    /// `atr_smoother` — smoothing applied to the internal ATR(14).
    /// Original default: `SmootherId::Sma`.
    pub fn from_smoothers(
        garch_alpha: f64,
        garch_persistence: f64,
        garch_omega: f64,
        threshold_adaptation_speed: f64,
        atr_smoother: SmootherId,
    ) -> Self {
        assert!(garch_alpha > 0.0 && garch_alpha < 1.0, "GARCH alpha must be between 0.0 and 1.0");
        assert!(garch_persistence > 0.0 && garch_persistence < 1.0, "GARCH persistence must be between 0.0 and 1.0");
        // Derive beta: alpha + beta = alpha + persistence*(1-alpha) < 1 for any alpha,persistence in (0,1)
        let garch_beta = garch_persistence * (1.0 - garch_alpha);
        assert!(garch_beta > 0.0 && garch_beta < 1.0, "GARCH beta must be between 0.0 and 1.0");
        assert!(garch_omega > 0.0, "GARCH omega must be positive");
        assert!(threshold_adaptation_speed > 0.0 && threshold_adaptation_speed <= 1.0,
                "Threshold adaptation speed must be between 0.0 and 1.0");

        Self {
            atr: Atr::from_smoother(14, atr_smoother),
            volatility_ma: SmootherSlot::new(SmootherId::Ema, 10),
            long_term_vol: SmootherSlot::new(SmootherId::Sma, 50),
            regime_smoother: SmootherSlot::new(SmootherId::Ema, 5),

            returns: Vec::with_capacity(64),
            volatilities: Vec::with_capacity(32),
            regime_scores: Vec::with_capacity(16),
            regime_history: Vec::with_capacity(16),

            garch_alpha,
            garch_beta,
            garch_omega,
            conditional_variance: 0.0,

            threshold_adaptation_speed,

            current_result: DynamicVolatilityRegimeResult::empty(),

            prev_price: None,
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed resolved input lanes — `[open, high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> DynamicVolatilityRegimeResult {
        let high  = lanes[1];
        let low   = lanes[2];
        let close = lanes[3];

        let atr_value = self.atr.feed(&[high, low, close]);

        if let Some(prev_price) = self.prev_price {
            let log_return = (close / prev_price).ln();

            if self.returns.len() >= 64 {
                self.returns.remove(0);
            }
            self.returns.push(log_return);

            self.update_garch_volatility(log_return);

            let volatility = self.calculate_composite_volatility(atr_value);

            self.update_adaptive_thresholds(volatility);

            let regime = self.classify_volatility_regime(volatility);

            self.analyze_regime_persistence(regime);

            self.calculate_volatility_trend();

            self.generate_regime_signals();

            self.current_result.current_regime = regime;
            self.current_result.garch_volatility = self.conditional_variance.sqrt();

            if self.returns.len() >= 20 && self.volatilities.len() >= 10 {
                self.is_ready = true;
            }
        }

        self.prev_price = Some(close);
        self.update_count += 1;
        self.current_result
    }


    /// Обновить GARCH-подобную условную дисперсию
    fn update_garch_volatility(&mut self, log_return: f64) {
        if self.update_count == 1 {
            self.conditional_variance = log_return * log_return;
        } else {
            let squared_return = log_return * log_return;
            self.conditional_variance = self.garch_omega +
                                       self.garch_alpha * squared_return +
                                       self.garch_beta * self.conditional_variance;
        }
    }

    /// Рассчитать комплексную волатильность
    fn calculate_composite_volatility(&mut self, atr_value: f64) -> f64 {
        if self.returns.len() < 2 {
            return atr_value;
        }

        let realized_vol = self.calculate_realized_volatility();
        let garch_vol = self.conditional_variance.sqrt();
        let atr_vol = atr_value;

        let composite_vol = realized_vol * 0.4 + garch_vol * 0.4 + atr_vol * 0.2;

        let smoothed_vol = self.volatility_ma.feed(composite_vol);

        if self.volatilities.len() >= 32 {
            self.volatilities.remove(0);
        }
        self.volatilities.push(smoothed_vol);

        smoothed_vol
    }

    /// Рассчитать реализованную волатильность
    fn calculate_realized_volatility(&self) -> f64 {
        if self.returns.len() < 10 {
            return 0.0;
        }

        let window_size = 20.min(self.returns.len());
        let start = self.returns.len() - window_size;

        let mean: f64 = self.returns[start..].iter().sum::<f64>() / window_size as f64;

        let variance: f64 = self.returns[start..].iter()
            .map(|&r| (r - mean).powi(2))
            .sum::<f64>() / (window_size - 1) as f64;

        variance.sqrt()
    }

    /// Обновить адаптивные пороги
    fn update_adaptive_thresholds(&mut self, current_volatility: f64) {
        if self.volatilities.len() < 10 {
            self.current_result.adaptive_threshold_low = current_volatility * 0.7;
            self.current_result.adaptive_threshold_high = current_volatility * 1.3;
            return;
        }

        let _long_term_vol = self.long_term_vol.feed(current_volatility);

        let vol_percentiles = self.calculate_volatility_percentiles();

        let new_low_threshold = vol_percentiles.0;
        let new_high_threshold = vol_percentiles.1;

        let speed = self.threshold_adaptation_speed;
        self.current_result.adaptive_threshold_low =
            speed * new_low_threshold + (1.0 - speed) * self.current_result.adaptive_threshold_low;
        self.current_result.adaptive_threshold_high =
            speed * new_high_threshold + (1.0 - speed) * self.current_result.adaptive_threshold_high;
    }

    /// Рассчитать перцентили волатильности
    fn calculate_volatility_percentiles(&self) -> (f64, f64) {
        if self.volatilities.len() < 10 {
            return (0.0, 0.0);
        }

        let mut sorted_vols: Vec<f64> = self.volatilities.iter().cloned().collect();

        let len = sorted_vols.len();
        let percentile_25 = quickselect_nth(&mut sorted_vols, len / 4);
        let percentile_75 = quickselect_nth(&mut sorted_vols, 3 * len / 4);

        (percentile_25, percentile_75)
    }

    /// Классифицировать режим волатильности
    fn classify_volatility_regime(&mut self, volatility: f64) -> VolatilityRegime {
        let low_threshold = self.current_result.adaptive_threshold_low;
        let high_threshold = self.current_result.adaptive_threshold_high;

        let very_low_threshold = low_threshold * 0.7;
        let very_high_threshold = high_threshold * 1.3;
        let extreme_threshold = high_threshold * 1.8;

        let regime = if volatility <= very_low_threshold {
            VolatilityRegime::VeryLow
        } else if volatility <= low_threshold {
            VolatilityRegime::Low
        } else if volatility <= high_threshold {
            VolatilityRegime::Normal
        } else if volatility <= very_high_threshold {
            VolatilityRegime::High
        } else if volatility <= extreme_threshold {
            VolatilityRegime::VeryHigh
        } else {
            VolatilityRegime::Extreme
        };

        self.current_result.regime_probability = self.calculate_regime_probability(volatility, regime);

        self.current_result.volatility_score = self.regime_smoother.feed(regime.as_f64());

        if self.regime_history.len() >= 16 {
            self.regime_history.remove(0);
        }
        self.regime_history.push(regime);

        regime
    }

    /// Рассчитать вероятность режима
    fn calculate_regime_probability(&self, volatility: f64, regime: VolatilityRegime) -> f64 {
        let low_threshold = self.current_result.adaptive_threshold_low;
        let high_threshold = self.current_result.adaptive_threshold_high;

        match regime {
            VolatilityRegime::VeryLow => {
                let threshold = low_threshold * 0.7;
                if volatility <= threshold {
                    1.0 - (volatility / threshold).min(1.0)
                } else {
                    0.0
                }
            },
            VolatilityRegime::Low => {
                let center = (low_threshold * 0.7 + low_threshold) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (low_threshold - low_threshold * 0.7) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::Normal => {
                let center = (low_threshold + high_threshold) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (high_threshold - low_threshold) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::High => {
                let center = (high_threshold + high_threshold * 1.3) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (high_threshold * 1.3 - high_threshold) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::VeryHigh => {
                let center = (high_threshold * 1.3 + high_threshold * 1.8) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (high_threshold * 1.8 - high_threshold * 1.3) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::Extreme => {
                let threshold = high_threshold * 1.8;
                if volatility >= threshold {
                    (volatility / threshold - 1.0).min(1.0)
                } else {
                    0.0
                }
            },
        }
    }

    /// Анализировать устойчивость режима
    fn analyze_regime_persistence(&mut self, current_regime: VolatilityRegime) {
        if self.regime_history.len() < 5 {
            self.current_result.regime_persistence = 0.0;
            self.current_result.transition_probability = 1.0;
            return;
        }

        let mut persistence_count = 0;
        for &regime in self.regime_history.iter().rev() {
            if regime == current_regime {
                persistence_count += 1;
            } else {
                break;
            }
        }

        self.current_result.regime_persistence = (persistence_count as f64 / self.regime_history.len() as f64).min(1.0);
        self.current_result.transition_probability = 1.0 - self.current_result.regime_persistence;
    }

    /// Рассчитать тренд волатильности
    fn calculate_volatility_trend(&mut self) {
        if self.volatilities.len() < 10 {
            self.current_result.volatility_trend = 0.0;
            return;
        }

        let len = self.volatilities.len();
        let recent_window = 5.min(len);
        let older_window = 10.min(len);

        let recent_avg: f64 = self.volatilities[len - recent_window..].iter().sum::<f64>() / recent_window as f64;
        let older_avg: f64 = self.volatilities[len - older_window..len - recent_window].iter().sum::<f64>() / (older_window - recent_window) as f64;

        if older_avg > 0.0 {
            self.current_result.volatility_trend = ((recent_avg - older_avg) / older_avg).clamp(-1.0, 1.0);
        } else {
            self.current_result.volatility_trend = 0.0;
        }
    }

    /// Генерировать сигналы смены режима
    fn generate_regime_signals(&mut self) {
        if !self.is_ready || self.regime_history.len() < 2 {
            self.current_result.regime_signal = 0;
            return;
        }

        let current_regime = self.current_result.current_regime;
        let prev_regime = self.regime_history[self.regime_history.len() - 2];

        match (prev_regime, current_regime) {
            (VolatilityRegime::Low | VolatilityRegime::Normal, VolatilityRegime::High | VolatilityRegime::VeryHigh | VolatilityRegime::Extreme) => {
                self.current_result.regime_signal = 1;
            },
            (VolatilityRegime::High | VolatilityRegime::VeryHigh | VolatilityRegime::Extreme, VolatilityRegime::Low | VolatilityRegime::VeryLow) => {
                self.current_result.regime_signal = -1;
            },
            _ => {
                self.current_result.regime_signal = 0;
            }
        }
    }

    /// Получить текущий режим
    pub fn current_regime(&self) -> VolatilityRegime {
        self.current_result.current_regime
    }

    /// Получить полный результат
    pub fn result(&self) -> DynamicVolatilityRegimeResult {
        self.current_result
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.volatility_ma.reset();
        self.long_term_vol.reset();
        self.regime_smoother.reset();

        self.returns.clear();
        self.volatilities.clear();
        self.regime_scores.clear();
        self.regime_history.clear();

        self.conditional_variance = 0.0;
        self.current_result = DynamicVolatilityRegimeResult::empty();

        self.prev_price = None;
        self.is_ready = false;
        self.update_count = 0;
    }

    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }

        self.current_result.regime_signal
    }

    pub fn value(&self) -> f64 {
        self.current_result.volatility_score
    }

    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;

        format!(
            "Volatility Regime: {} (prob: {:.2}), Score: {:.1}, Persistence: {:.2}, Trend: {:.2}",
            result.current_regime.as_str(),
            result.regime_probability,
            result.volatility_score,
            result.regime_persistence,
            result.volatility_trend
        )
    }

    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Получить GARCH параметры
    pub fn garch_parameters(&self) -> (f64, f64, f64) {
        (self.garch_alpha, self.garch_beta, self.garch_omega)
    }
}

impl Default for DynamicVolatilityRegime {
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

/// Typed contract config for [`DynamicVolatilityRegime`].
/// Four period fields + four follow slots: ATR(14/Sma), volatility-MA(10/Ema),
/// long-term-vol(50/Sma), regime-smoother(5/Ema). GARCH scalars are `Param<f64>`.
///
/// `garch_persistence` replaces the former `garch_beta` axis. Beta is derived
/// internally as `garch_persistence * (1 - garch_alpha)`, so `alpha + beta < 1`
/// holds BY CONSTRUCTION for any sweep values in (0, 1).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct DynamicVolatilityRegimeConfig {
    pub garch_alpha: Param<f64>,
    /// Persistence ratio ∈ (0, 1). Derived beta = persistence * (1 − alpha).
    pub garch_persistence: Param<f64>,
    pub garch_omega: Param<f64>,
    pub threshold_adaptation_speed: Param<f64>,
    pub atr_period: Param<usize>,
    pub volatility_period: Param<usize>,
    pub long_term_vol_period: Param<usize>,
    pub regime_smoother_period: Param<usize>,
    #[slot]
    pub atr_ma: Param<SmootherChoice>,
    #[slot]
    pub volatility_ma: Param<SmootherChoice>,
    #[slot]
    pub long_term_vol_ma: Param<SmootherChoice>,
    #[slot]
    pub regime_smoother_ma: Param<SmootherChoice>,
}

impl Indicator for DynamicVolatilityRegime {
    const ID: IndicatorId = IndicatorId::Dvr;
    /// Volatility regime detector — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Open/High/Low/Close — open+high+low drive the internal Atr; close drives log-returns.
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
    const SLOTS: &'static [Slot] = DynamicVolatilityRegimeConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::Dvr)];
    type Config = DynamicVolatilityRegimeConfig;
    type Runtime = DynamicVolatilityRegime;

    fn create(cfg: DynamicVolatilityRegimeConfig) -> DynamicVolatilityRegime {
        let alpha = cfg.garch_alpha.resolved();
        let persistence = cfg.garch_persistence.resolved();
        // Stationarity by construction: alpha + beta = alpha + persistence*(1-alpha) < 1
        let beta = persistence * (1.0 - alpha);
        DynamicVolatilityRegime {
            atr: Atr::from_smoother(cfg.atr_period.resolved(), cfg.atr_ma.resolved().kind),
            volatility_ma: cfg.volatility_ma.resolved().build(cfg.volatility_period.resolved()),
            long_term_vol: cfg.long_term_vol_ma.resolved().build(cfg.long_term_vol_period.resolved()),
            regime_smoother: cfg.regime_smoother_ma.resolved().build(cfg.regime_smoother_period.resolved()),

            returns: Vec::with_capacity(64),
            volatilities: Vec::with_capacity(32),
            regime_scores: Vec::with_capacity(16),
            regime_history: Vec::with_capacity(16),

            garch_alpha: alpha,
            garch_beta: beta,
            garch_omega: cfg.garch_omega.resolved(),
            conditional_variance: 0.0,
            threshold_adaptation_speed: cfg.threshold_adaptation_speed.resolved(),

            current_result: DynamicVolatilityRegimeResult::empty(),
            prev_price: None,
            is_ready: false,
            update_count: 0,
        }
    }

    fn slot_members(cfg: &DynamicVolatilityRegimeConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for DynamicVolatilityRegimeConfig {
    fn defaults() -> Self {
        DynamicVolatilityRegimeConfig {
            garch_alpha: Param::Solo(0.1),
            // persistence = old_beta / (1 - old_alpha) = 0.85 / 0.9
            // derived beta = (0.85/0.9) * 0.9 = 0.85 — exact reproduction
            garch_persistence: Param::Solo(0.85 / 0.9),
            garch_omega: Param::Solo(0.01),
            threshold_adaptation_speed: Param::Solo(0.05),
            atr_period: Param::Solo(14),
            volatility_period: Param::Solo(10),
            long_term_vol_period: Param::Solo(50),
            regime_smoother_period: Param::Solo(5),
            atr_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            volatility_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            long_term_vol_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            regime_smoother_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // garch_alpha: strictly inside (0,1) — >0 assert holds.
        s.garch_alpha = Param::many(sweep_f64(0.05, 0.95, 0.05));
        // garch_persistence: strictly inside (0,1) — >0 assert holds.
        // alpha+beta = alpha + persistence*(1-alpha) < 1 BY CONSTRUCTION for any
        // alpha,persistence in (0,1) — no filter needed on the sweep cube.
        s.garch_persistence = Param::many(sweep_f64(0.05, 0.95, 0.05));
        s.garch_omega = Param::many(sweep_f64(0.0, 1.0, 0.05));
        // threshold_adaptation_speed: Class E alpha/decay — sweep_f64(0.01, 0.99, 0.01).
        s.threshold_adaptation_speed = Param::many(sweep_f64(0.01, 0.99, 0.01));
        // atr_period / volatility_period / long_term_vol_period / regime_smoother_period:
        //   Class A usize — auto range(2,4048,1) covers all.
        // atr_ma / volatility_ma / long_term_vol_ma / regime_smoother_ma:
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


impl Render for DynamicVolatilityRegime {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dvr, "Dyn Vol Regime", Color::hex(0x9C27B0))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_dynamic_volatility_regime_creation() {
        let dvr = DynamicVolatilityRegime::new();
        assert!(!dvr.is_ready());
        let (a, b, o) = dvr.garch_parameters();
        assert!((a - 0.1).abs() < 1e-12);
        // beta is derived: persistence*(1-alpha) = (0.85/0.9)*0.9 = 0.85
        assert!((b - 0.85).abs() < 1e-12);
        assert!((o - 0.01).abs() < 1e-12);
    }

    #[test]
    fn test_dynamic_volatility_regime_with_parameters() {
        // persistence = 0.8 / (1 - 0.15) so derived beta = 0.8, reproducing the original model.
        let dvr = DynamicVolatilityRegime::from_smoothers(0.15, 0.8 / 0.85, 0.02, 0.1, SmootherId::Sma);
        let (a, b, o) = dvr.garch_parameters();
        assert!((a - 0.15).abs() < 1e-12);
        assert!((b - 0.8).abs() < 1e-12);
        assert!((o - 0.02).abs() < 1e-12);
    }

    #[test]
    fn test_dvr_with_atr_ma_type_ema() {
        // persistence = 0.85 / (1 - 0.1) — reproduces original alpha=0.1, beta=0.85 model.
        let mut dvr = DynamicVolatilityRegime::from_smoothers(0.1, 0.85 / 0.9, 0.01, 0.05, SmootherId::Ema);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 2.0;
            let result = dvr.feed(&[price, price + 1.0, price - 1.0, price]);
            assert!(result.volatility_score.is_finite());
        }
        assert!(dvr.is_ready());
    }

    #[test]
    fn test_dynamic_volatility_regime_update() {
        let mut dvr = DynamicVolatilityRegime::new();

        for i in 0..30 {
            let base_price = 100.0;
            let volatility_factor = if i < 15 { 0.5 } else { 2.0 };
            let price = base_price + (i as f64 * 0.1).sin() * volatility_factor;

            let result = dvr.feed(&[price, price + 1.0, price - 1.0, price]);

            if i > 25 {
                assert!(dvr.is_ready());
                assert!(result.volatility_score >= 1.0 && result.volatility_score <= 6.0);
                assert!(result.regime_probability >= 0.0 && result.regime_probability <= 1.0);
            }
        }
    }

    #[test]
    fn test_volatility_regime_classification() {
        let mut dvr = DynamicVolatilityRegime::new();

        for i in 0..50 {
            let price = if i < 25 {
                100.0 + (i as f64 * 0.01).sin() * 0.1
            } else {
                100.0 + (i as f64 * 0.1).sin() * 5.0
            };

            dvr.feed(&[price, price + 0.1, price - 0.1, price]);
        }

        assert!(dvr.is_ready());
        let regime = dvr.current_regime();
        assert!(matches!(regime, VolatilityRegime::High | VolatilityRegime::VeryHigh | VolatilityRegime::Extreme));
    }

    #[test]
    fn test_dynamic_volatility_regime_reset() {
        let mut dvr = DynamicVolatilityRegime::new();

        for i in 0..25 {
            let price = 100.0 + i as f64;
            dvr.feed(&[price, price + 1.0, price - 1.0, price]);
        }

        dvr.reset();
        assert!(!dvr.is_ready());
        assert_eq!(dvr.update_count(), 0);
    }

    #[test]
    fn factory_feeds_resolved_dvr() {
        let mut f = IndicatorOrder::Dvr(<<DynamicVolatilityRegime as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v >= 1.0 && v <= 6.0, "DVR score should be in [1,6], got {v}");
    }
}
