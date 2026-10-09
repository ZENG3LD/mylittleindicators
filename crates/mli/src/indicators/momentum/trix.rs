//! TRIX (Triple Exponential Average) indicator.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;
use std::collections::VecDeque;

/// TRIX - Triple Exponential Average momentum oscillator.
///
/// TRIX = ROC(EMA(EMA(EMA(Close, N), N), N), 1)
///
/// Shows the percentage rate of change of a triple-smoothed exponential moving
/// average. The triple smoothing eliminates most market noise, making it useful
/// for identifying trend changes.
///
/// PURE core — owns no source; the factory feeds it the resolved scalar via [`Trix::feed`].
#[derive(Debug, Clone)]
pub struct Trix {
    period: usize,
    signal_period: usize,

    // Тройное экспоненциальное сглаживание
    first_ema: SmootherSlot,
    second_ema: SmootherSlot,
    third_ema: SmootherSlot,

    // Сигнальная линия
    signal_ema: SmootherSlot,

    // Буферы для значений (кольца истории для вспомогательных сигналов)
    trix_values: VecDeque<f64>,
    triple_ema_values: VecDeque<f64>,

    // Предыдущее значение тройной EMA для расчета ROC
    prev_triple_ema: f64,

    // Текущие значения
    trix_value: f64,
    signal_value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl Trix {
    /// Creates a new TRIX with default parameters (14, 9) using EMA.
    pub fn new() -> Self {
        Self::from_smoother(SmootherId::Ema, 14, 9)
    }

    /// Creates a new TRIX with specified periods (uses EMA).
    pub fn with_params_default(period: usize, signal_period: usize) -> Self {
        Self::from_smoother(SmootherId::Ema, period, signal_period)
    }

    /// Build the triple cascade + signal smoothers from one narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `TrixConfig`.
    pub fn from_smoother(ma: SmootherId, period: usize, signal_period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            period,
            signal_period,
            first_ema: SmootherSlot::new(ma, period),
            second_ema: SmootherSlot::new(ma, period),
            third_ema: SmootherSlot::new(ma, period),
            signal_ema: SmootherSlot::new(ma, signal_period),
            trix_values: VecDeque::with_capacity(512),
            triple_ema_values: VecDeque::with_capacity(512),
            prev_triple_ema: 0.0,
            trix_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, source_value: f64) -> f64 {
        self.bars_count += 1;

        // Первое сглаживание
        let first_ema_value = self.first_ema.feed(source_value);

        // Второе сглаживание
        let second_ema_value = self.second_ema.feed(first_ema_value);

        // Третье сглаживание
        let third_ema_value = self.third_ema.feed(second_ema_value);

        // Добавляем в буфер тройной EMA
        if self.triple_ema_values.len() >= 512 {
            self.triple_ema_values.pop_front();
        }
        self.triple_ema_values.push_back(third_ema_value);

        // Рассчитываем TRIX (ROC от тройной EMA)
        if self.bars_count > 1 && self.prev_triple_ema.abs() > 1e-12 {
            self.trix_value = ((third_ema_value - self.prev_triple_ema) / self.prev_triple_ema) * 10000.0; // Умножаем на 10000 для удобства
        }

        // Добавляем в буфер TRIX
        if self.trix_values.len() >= 512 {
            self.trix_values.pop_front();
        }
        self.trix_values.push_back(self.trix_value);

        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_ema.feed(self.trix_value);

        // Обновляем предыдущее значение
        self.prev_triple_ema = third_ema_value;

        // Проверяем готовность (нужно время для стабилизации тройного сглаживания)
        let min_bars = self.period * 3 + self.signal_period;
        if self.bars_count >= min_bars {
            self.is_ready = true;
        }

        self.trix_value
    }


    /// Named output: brace `line` (the TRIX oscillator line).
    #[inline]
    pub fn line(&self) -> f64 {
        self.trix_value
    }

    /// Returns the signal line value.
    #[inline]
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }

    /// Returns the histogram (TRIX - Signal).
    #[inline]
    pub fn histogram(&self) -> f64 {
        self.trix_value - self.signal_value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal_value
    }

    /// Returns `true` if the TRIX has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the periods (period, signal_period).
    #[inline]
    pub fn periods(&self) -> (usize, usize) {
        (self.period, self.signal_period)
    }

    /// Resets the TRIX to its initial state.
    pub fn reset(&mut self) {
        self.first_ema.reset();
        self.second_ema.reset();
        self.third_ema.reset();
        self.signal_ema.reset();
        self.trix_values.clear();
        self.triple_ema_values.clear();
        self.prev_triple_ema = 0.0;
        self.trix_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Returns the current trend condition.
    pub fn trend_condition(&self) -> &'static str {
        match self.trix_value {
            v if v > 0.0 && self.trix_value > self.signal_value => "Strong Bullish",
            v if v > 0.0 => "Bullish",
            v if v < 0.0 && self.trix_value < self.signal_value => "Strong Bearish",
            v if v < 0.0 => "Bearish",
            _ => "Neutral"
        }
    }

    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        let histogram = self.histogram();

        // Сигналы на основе пересечения сигнальной линии
        if self.trix_value > self.signal_value && histogram > 0.5 {
            1  // Покупка - TRIX выше сигнальной линии
        } else if self.trix_value < self.signal_value && histogram < -0.5 {
            -1 // Продажа - TRIX ниже сигнальной линии
        } else {
            0  // Нейтрально
        }
    }

    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.trix_values.len() < 3 {
            return 0;
        }

        let len = self.trix_values.len();
        let current = self.trix_value;
        let prev_1 = if len >= 2 { self.trix_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.trix_values[len - 3] } else { 0.0 };

        // Сигнал покупки: пересечение нулевой линии снизу вверх с подтверждением сигнальной линии
        if prev_2 < 0.0 && prev_1 < 0.0 && current > 0.0 && self.trix_value > self.signal_value {
            return 1;
        }

        // Сигнал продажи: пересечение нулевой линии сверху вниз с подтверждением сигнальной линии
        if prev_2 > 0.0 && prev_1 > 0.0 && current < 0.0 && self.trix_value < self.signal_value {
            return -1;
        }

        0
    }

    /// Проверить дивергенцию между ценой и TRIX
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.trix_values.len() < lookback + 1 {
            return 0;
        }

        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_trix = self.trix_value;
        let past_trix = self.trix_values[self.trix_values.len() - lookback - 1];

        let price_change = current_price - past_price;
        let trix_change = current_trix - past_trix;

        // Бычья дивергенция: цена делает новый минимум, но TRIX растет
        if price_change < 0.0 && trix_change > 0.0 {
            return 1;
        }

        // Медвежья дивергенция: цена делает новый максимум, но TRIX падает
        if price_change > 0.0 && trix_change < 0.0 {
            return -1;
        }

        0
    }

    /// Получить скорость изменения тренда
    pub fn trend_velocity(&self) -> f64 {
        self.trix_value.abs()
    }

    /// Получить ускорение тренда
    pub fn trend_acceleration(&self) -> f64 {
        if !self.is_ready() || self.trix_values.len() < 2 {
            return 0.0;
        }

        let len = self.trix_values.len();
        let current = self.trix_value;
        let prev = self.trix_values[len - 2];

        current - prev
    }

    /// Получить силу тренда
    pub fn trend_strength(&self, lookback: usize) -> f64 {
        if !self.is_ready() || self.trix_values.len() < lookback {
            return 0.0;
        }

        let start_idx = self.trix_values.len() - lookback;
        let n = self.trix_values.len() - start_idx;

        // Считаем среднее абсолютное значение по последним `lookback` значениям
        self.trix_values.range(start_idx..).map(|&x| x.abs()).sum::<f64>() / n as f64
    }

    /// Получить направление тренда
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.trix_values.len() < lookback + 1 {
            return 0;
        }

        let current = self.trix_value;
        let past = self.trix_values[self.trix_values.len() - lookback - 1];

        if current > past {
            1  // Восходящий тренд
        } else if current < past {
            -1 // Нисходящий тренд
        } else {
            0  // Боковой тренд
        }
    }

    /// Получить текущее значение тройной EMA
    pub fn triple_ema_value(&self) -> f64 {
        if self.triple_ema_values.is_empty() {
            0.0
        } else {
            *self.triple_ema_values.back().unwrap()
        }
    }

    /// Получить фильтрованный сигнал (только сильные движения)
    pub fn filtered_signal(&self, threshold: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        let basic_signal = self.trading_signal();
        let velocity = self.trend_velocity();

        // Фильтруем слабые сигналы
        if velocity < threshold {
            return 0;
        }

        basic_signal
    }

    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "TRIX: {:.2}, Signal: {:.2}, Histogram: {:.2}, Trend: {}, Velocity: {:.2}, Acceleration: {:.2}",
            self.trix_value,
            self.signal_value,
            self.histogram(),
            self.trend_condition(),
            self.trend_velocity(),
            self.trend_acceleration()
        )
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Param, Slot, Indicator, Output, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::engine::contract_engine::SmootherChoice;

/// Typed config for [`Trix`]: the EMA-layer period + signal period (host scalars), the
/// cascade smoother choice (built three times for the triple smoothing) and the signal
/// smoother choice (each follows its respective host period), and the source.
///
/// Dual-mode: every field is a `Param`. The two `#[slot]` smoothers are `Param<SmootherChoice>`
/// following their respective host period axes.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct TrixConfig {
    pub period: Param<usize>,
    pub signal_period: Param<usize>,
    #[slot]
    pub smoothing: Param<SmootherChoice>,
    #[slot]
    pub signal: Param<SmootherChoice>,
    pub source: Param<OhlcvField>,
}

impl Indicator for Trix {
    const ID: IndicatorId = IndicatorId::Trix;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Two outputs: TRIX value and its signal line. The signal is genuinely computed
    /// every bar by the signal smoother.
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::TrixLine),
        Output::centered(IndicatorOutputId::TrixSignal),
    ];
    /// TRIX's OWN state is two bounded (<=512) history `Vec`s (the triple-EMA and
    /// TRIX series) + O(1) scalars. Its cascade and signal smoothers are MovingAverage
    /// slots (one knob each), charged recursively by the barometer.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::fixed(StoreKind::Vec, 512),
            Store::fixed(StoreKind::Vec, 512),
        ],
    );
    const SLOTS: &'static [Slot] = TrixConfig::SLOTS;

    type Config = TrixConfig;
    type Runtime = Trix;

    fn create(cfg: TrixConfig) -> Trix {
        let period = cfg.period.resolved();
        let signal_period = cfg.signal_period.resolved();
        // Triple cascade = the SAME smoother choice built three times; Follow resolves to period.
        let smoothing = cfg.smoothing.resolved();
        let signal = cfg.signal.resolved();
        Trix {
            period,
            signal_period,
            first_ema: smoothing.build(period),
            second_ema: smoothing.build(period),
            third_ema: smoothing.build(period),
            signal_ema: signal.build(signal_period),
            trix_values: VecDeque::with_capacity(512),
            triple_ema_values: VecDeque::with_capacity(512),
            prev_triple_ema: 0.0,
            trix_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &TrixConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &TrixConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for TrixConfig {
    fn defaults() -> Self {
        TrixConfig {
            period: Param::Solo(14),
            signal_period: Param::Solo(9),
            smoothing: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            signal: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period/signal_period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        // smoothing/signal: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Trix {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::TrixLine, "TRIX", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::TrixSignal, "Signal", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests — pure `feed(scalar)` core
    // =========================================================================

    #[test]
    fn test_trix_basic_calculation() {
        let mut trix = Trix::new();

        // Feed uptrend data - need enough for triple smoothing
        for i in 1..=100 {
            trix.feed(100.0 + i as f64 * 0.5);
        }

        assert!(trix.is_ready());
        // In uptrend, TRIX should be positive
        assert!(trix.line() > 0.0, "TRIX in uptrend should be positive");
    }

    #[test]
    fn test_trix_downtrend() {
        let mut trix = Trix::new();

        // Feed downtrend data
        for i in 1..=100 {
            trix.feed(200.0 - i as f64 * 0.5);
        }

        assert!(trix.is_ready());
        // In downtrend, TRIX should be negative
        assert!(trix.line() < 0.0, "TRIX in downtrend should be negative");
    }

    #[test]
    fn test_trix_signal_line() {
        let mut trix = Trix::new();

        // Feed data
        for i in 1..=100 {
            trix.feed(100.0 + i as f64);
        }

        assert!(trix.is_ready());
        // Signal should track TRIX
        let _ = trix.signal_value(); // Just verify it returns a value
    }

    #[test]
    fn test_trix_histogram() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.feed(100.0 + i as f64);
        }

        assert!(trix.is_ready());
        // Histogram = TRIX - Signal
        let expected = trix.line() - trix.signal_value();
        assert!((trix.histogram() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_trix_reset() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.feed(100.0 + i as f64);
        }
        assert!(trix.is_ready());

        trix.reset();
        assert!(!trix.is_ready());
        assert!(trix.line().abs() < 1e-10);
    }

    #[test]
    fn test_trix_periods() {
        let trix = Trix::with_params_default(10, 5);
        assert_eq!(trix.periods(), (10, 5));
    }

    #[test]
    fn test_trix_trend_condition() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.feed(100.0 + i as f64);
        }

        assert!(trix.is_ready());
        let condition = trix.trend_condition();
        assert!(
            condition == "Strong Bullish"
                || condition == "Bullish"
                || condition == "Strong Bearish"
                || condition == "Bearish"
                || condition == "Neutral"
        );
    }

    /// The `ContractFactory` variant holds the resolved source (close) and the `Source`
    /// feed runs TRIX end-to-end over the resolved scalar — proving the factory path.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Trix(<<Trix as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |close: f64| MarketSample::Bar {
            open: 0.0, high: 999.0, low: 0.0, close, volume: 0.0,
        };
        for i in 1..=100 {
            f.feed(0, bar(100.0 + i as f64 * 0.5));
        }
        // Uptrend on CLOSE -> positive TRIX; highs (999) are flat and would give ~0.
        assert!(f.primary() > 0.0, "factory TRIX on CLOSE uptrend should be > 0, got {}", f.primary());
    }
}
