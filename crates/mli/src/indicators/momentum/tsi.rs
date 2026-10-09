//! True Strength Index (TSI) indicator.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;

/// True Strength Index (TSI) - double-smoothed momentum indicator by William Blau.
///
/// TSI = 100 × (Smoothed(Smoothed(PC)) / Smoothed(Smoothed(|PC|)))
///
/// where PC = Price Change = Price - Price_prev
///
/// Double exponential smoothing reduces noise while maintaining responsiveness.
///
/// Oscillates between -100 and +100:
/// - Above +25: Strong bullish momentum
/// - Below -25: Strong bearish momentum
/// - Zero line crossings: Trend changes
///
/// # Parameters
/// - `first_smoothing`: Long-term smoothing (typically 25)
/// - `second_smoothing`: Short-term smoothing (typically 13)
/// - `signal_period`: Signal line smoothing (typically 13)
///
/// # Implementation
///
/// PURE core: five smoother slots for the double smoothing + signal line, O(1) update.
/// It owns NO source field and NO `update_bar` — the factory resolves the configured
/// price field (`TsiConfig::source`) and feeds the scalar via [`TrueStrengthIndex::feed`].
#[derive(Debug, Clone)]
pub struct TrueStrengthIndex {
    first_smoothing: usize,
    second_smoothing: usize,

    prev_price: f64,

    first_pc_ema: SmootherSlot,
    second_pc_ema: SmootherSlot,

    first_apc_ema: SmootherSlot,
    second_apc_ema: SmootherSlot,

    signal_line_period: usize,
    signal_ema: SmootherSlot,

    tsi_value: f64,
    signal_value: f64,

    bars_count: usize,
    is_ready: bool,
}

impl TrueStrengthIndex {
    /// Создать новый TSI с параметрами по умолчанию (25, 13, 13), все сглаживатели EMA.
    pub fn new() -> Self {
        Self::from_smoother(SmootherId::Ema, 25, 13, 13)
    }

    /// Build a TSI whose five smoother slots all use one narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `TsiConfig` (three independent
    /// smoother orders).
    pub fn from_smoother(
        ma: SmootherId,
        first_smoothing: usize,
        second_smoothing: usize,
        signal_period: usize,
    ) -> Self {
        assert!(first_smoothing > 0, "First smoothing period must be greater than 0");
        assert!(second_smoothing > 0, "Second smoothing period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            first_smoothing,
            second_smoothing,
            prev_price: 0.0,
            first_pc_ema: SmootherSlot::new(ma, first_smoothing),
            second_pc_ema: SmootherSlot::new(ma, second_smoothing),
            first_apc_ema: SmootherSlot::new(ma, first_smoothing),
            second_apc_ema: SmootherSlot::new(ma, second_smoothing),
            signal_line_period: signal_period,
            signal_ema: SmootherSlot::new(ma, signal_period),
            tsi_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar price — the pure core computation. The caller (the
    /// factory's source-resolving feed, or a host composite) supplies the price; the core
    /// knows nothing about OHLCV fields.
    pub fn feed(&mut self, current_price: f64) -> f64 {
        self.bars_count += 1;

        if self.bars_count == 1 {
            self.prev_price = current_price;
            return self.tsi_value;
        }

        let price_change = current_price - self.prev_price;
        let abs_price_change = price_change.abs();

        let first_pc_smooth = self.first_pc_ema.feed(price_change);
        let first_apc_smooth = self.first_apc_ema.feed(abs_price_change);

        let second_pc_smooth = self.second_pc_ema.feed(first_pc_smooth);
        let second_apc_smooth = self.second_apc_ema.feed(first_apc_smooth);

        self.tsi_value = if second_apc_smooth.abs() < 1e-12 {
            0.0
        } else {
            100.0 * (second_pc_smooth / second_apc_smooth)
        };

        self.signal_value = self.signal_ema.feed(self.tsi_value);

        self.prev_price = current_price;

        let min_bars = self.first_smoothing + self.second_smoothing + self.signal_line_period;
        if self.bars_count >= min_bars {
            self.is_ready = true;
        }

        self.tsi_value
    }


    /// Named output: brace `line` (the TSI oscillator line).
    pub fn line(&self) -> f64 {
        self.tsi_value
    }

    /// Получить значение сигнальной линии
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }

    /// Получить гистограмму (TSI - Signal)
    pub fn histogram(&self) -> f64 {
        self.tsi_value - self.signal_value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal_value
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Получить периоды сглаживания
    pub fn periods(&self) -> (usize, usize, usize) {
        (self.first_smoothing, self.second_smoothing, self.signal_line_period)
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.prev_price = 0.0;
        self.first_pc_ema.reset();
        self.second_pc_ema.reset();
        self.first_apc_ema.reset();
        self.second_apc_ema.reset();
        self.signal_ema.reset();
        self.tsi_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.tsi_value {
            v if v > 25.0 => "Strong Bullish",
            v if v > 0.0 => "Bullish",
            v if v < -25.0 => "Strong Bearish",
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

        if self.tsi_value > self.signal_value && histogram > 0.5 {
            1
        } else if self.tsi_value < self.signal_value && histogram < -0.5 {
            -1
        } else {
            0
        }
    }

    /// Получить силу momentum (абсолютное значение TSI)
    pub fn momentum_strength(&self) -> f64 {
        self.tsi_value.abs()
    }

    /// Получить уровни перекупленности/перепроданности
    pub fn overbought_oversold_levels(&self) -> (&'static str, f64, f64) {
        let overbought = 25.0;
        let oversold = -25.0;

        let condition = match self.tsi_value {
            v if v >= overbought => "Overbought",
            v if v <= oversold => "Oversold",
            _ => "Normal"
        };

        (condition, overbought, oversold)
    }

    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let (condition, ob_level, os_level) = self.overbought_oversold_levels();
        format!(
            "TSI: {:.2}, Signal: {:.2}, Histogram: {:.2}, Condition: {} (OB: {:.1}, OS: {:.1}), Strength: {:.2}",
            self.tsi_value,
            self.signal_value,
            self.histogram(),
            condition,
            ob_level,
            os_level,
            self.momentum_strength()
        )
    }
}

impl Default for TrueStrengthIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Param, Slot, Indicator, Output, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, ReferenceLine, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::engine::contract_engine::SmootherChoice;

/// Typed config for [`TrueStrengthIndex`]: the two smoothing periods + signal period
/// (host scalars), and THREE smoother choices — `first` drives both first-stage EMAs (PC
/// + |PC|) at the first period, `second` both second-stage EMAs, `signal` the signal
/// line. Each slot follows its own host period by default.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoothers are `Param<SmootherChoice>`
/// following their respective host period axes.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct TsiConfig {
    pub first_smoothing: Param<usize>,
    pub second_smoothing: Param<usize>,
    pub signal_period: Param<usize>,
    #[slot]
    pub first: Param<SmootherChoice>,
    #[slot]
    pub second: Param<SmootherChoice>,
    #[slot]
    pub signal: Param<SmootherChoice>,
    pub source: Param<OhlcvField>,
}

impl Indicator for TrueStrengthIndex {
    const ID: IndicatorId = IndicatorId::Tsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Three outputs: TSI value, its signal line, and the histogram (value - signal).
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::TsiLine),
        Output::centered(IndicatorOutputId::TsiSignal),
        Output::centered(IndicatorOutputId::TsiHistogram),
    ];
    /// Own state is O(1) scalars (no history buffer). Three smoother choices — `first`
    /// + `second` (the two double-smoothing stages, each its own period) + `signal` —
    /// so three `#[slot]` fields / cost entries.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = TsiConfig::SLOTS;

    type Config = TsiConfig;
    type Runtime = TrueStrengthIndex;

    fn create(cfg: TsiConfig) -> TrueStrengthIndex {
        let first_period = cfg.first_smoothing.resolved();
        let second_period = cfg.second_smoothing.resolved();
        let sig_period = cfg.signal_period.resolved();
        // `first` follows first_smoothing, `second` follows second_smoothing,
        // `signal` follows signal_period. `Follow` resolves to the respective host.
        TrueStrengthIndex {
            first_smoothing: first_period,
            second_smoothing: second_period,
            prev_price: 0.0,
            first_pc_ema: cfg.first.resolved().build(first_period),
            second_pc_ema: cfg.second.resolved().build(second_period),
            first_apc_ema: cfg.first.resolved().build(first_period),
            second_apc_ema: cfg.second.resolved().build(second_period),
            signal_line_period: sig_period,
            signal_ema: cfg.signal.resolved().build(sig_period),
            tsi_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Single-source core: the factory variant holds the resolved `Source` and feeds the
    /// scalar — so this `TrueStrengthIndex` is pure (no `source` field, no `update_bar`).
    fn source_fields(cfg: &TsiConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &TsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for TsiConfig {
    fn defaults() -> Self {
        TsiConfig {
            first_smoothing: Param::Solo(25),
            second_smoothing: Param::Solo(13),
            signal_period: Param::Solo(13),
            first: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            second: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
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
        // first_smoothing/second_smoothing/signal_period: Class A → auto range(2,4048,1).
        // source: Class O → auto all-8.
        // first/second/signal: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for TrueStrengthIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::TsiLine, "TSI", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::TsiSignal, "Signal", Color::hex(0xFF9800), 1.0))
            .output(RenderOutput::histogram(IndicatorOutputId::TsiHistogram, "Histogram", Color::hex(0x4CAF50)))
            .histogram_style(HistogramStyle::Centered)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(25.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(-25.0, Color::hex(0x4CAF50)))
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(2)
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
    fn test_tsi_basic_calculation() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            tsi.feed(100.0 + i as f64);
        }
        assert!(tsi.is_ready());
        assert!(tsi.line() > 0.0, "TSI in uptrend should be positive, got {}", tsi.line());
    }

    #[test]
    fn test_tsi_downtrend() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            tsi.feed(200.0 - i as f64);
        }
        assert!(tsi.is_ready());
        assert!(tsi.line() < 0.0, "TSI in downtrend should be negative, got {}", tsi.line());
    }

    #[test]
    fn test_tsi_range() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            let price = if i % 2 == 0 { 100.0 + i as f64 } else { 100.0 };
            tsi.feed(price);
        }
        if tsi.is_ready() {
            let val = tsi.line();
            assert!(val >= -100.0 && val <= 100.0, "TSI should be in [-100, 100], got {}", val);
        }
    }

    #[test]
    fn test_tsi_with_custom_params() {
        let mut tsi = TrueStrengthIndex::from_smoother(SmootherId::Ema, 25, 13, 7);
        for i in 1..=60 {
            tsi.feed(100.0 + i as f64);
        }
        assert!(tsi.is_ready());
        let (first, second, signal) = tsi.periods();
        assert_eq!(first, 25);
        assert_eq!(second, 13);
        assert_eq!(signal, 7);
    }

    #[test]
    fn test_tsi_signal_line() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            tsi.feed(100.0 + i as f64);
        }
        assert!(tsi.is_ready());
        let signal = tsi.signal_value();
        assert!(signal.is_finite());
    }

    #[test]
    fn test_tsi_histogram() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            tsi.feed(100.0 + i as f64);
        }
        assert!(tsi.is_ready());
        let hist = tsi.histogram();
        let expected = tsi.line() - tsi.signal_value();
        assert!((hist - expected).abs() < 1e-10);
    }

    #[test]
    fn test_tsi_reset() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            tsi.feed(100.0 + i as f64);
        }
        assert!(tsi.is_ready());

        tsi.reset();
        assert!(!tsi.is_ready());
        assert!((tsi.line()).abs() < 1e-10);
    }

    #[test]
    fn test_tsi_momentum_strength() {
        let mut tsi = TrueStrengthIndex::new();
        for i in 1..=60 {
            tsi.feed(100.0 + i as f64);
        }
        if tsi.is_ready() {
            let strength = tsi.momentum_strength();
            assert!(strength >= 0.0);
            assert_eq!(strength, tsi.line().abs());
        }
    }

    /// The `ContractFactory` variant holds the resolved source (close) and the `Source`
    /// feed runs TSI end-to-end over the resolved scalar (not a raw OHLCV bar). The high
    /// (999) would never produce a clean uptrend TSI — proving the close source resolves.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Tsi(<<TrueStrengthIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |close: f64| MarketSample::Bar {
            open: 0.0, high: 999.0, low: 0.0, close, volume: 0.0,
        };
        for i in 1..=60 {
            f.feed(0, bar(100.0 + i as f64));
        }
        assert!(f.primary() > 0.0, "factory TSI on CLOSE uptrend should be > 0, got {}", f.primary());
    }
}
