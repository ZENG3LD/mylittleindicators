//! Know Sure Thing (KST) indicator.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;

/// Know Sure Thing (KST) - composite momentum indicator by Martin Pring.
///
/// KST = Weighted average of four smoothed ROC values:
/// KST = (ROC1×SMA1×1 + ROC2×SMA2×2 + ROC3×SMA3×3 + ROC4×SMA4×4) / 10
///
/// Combines multiple Rate of Change indicators at different timeframes
/// to create a smoother, more reliable momentum oscillator.
///
/// # Parameters
/// - `roc_periods`: ROC lookback periods [10, 15, 20, 30]
/// - `sma_periods`: SMA smoothing periods [10, 10, 10, 15]
/// - `signal_period`: Signal line period (typically 9)
/// - `source`: OHLCV field to use for ROC calculation
#[derive(Clone)]
pub struct KnowSureThing {
    roc_periods: [usize; 4],
    sma_periods: [usize; 4],
    signal_period: usize,
    source: OhlcvField,

    source_prices: Vec<f64>,
    roc_smas: [SmootherSlot; 4],
    signal_sma: SmootherSlot,
    kst_values: Vec<f64>,

    kst_value: f64,
    signal_value: f64,

    bars_count: usize,
    is_ready: bool,
}

impl std::fmt::Debug for KnowSureThing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowSureThing")
            .field("roc_periods", &self.roc_periods)
            .field("sma_periods", &self.sma_periods)
            .field("signal_period", &self.signal_period)
            .field("kst_value", &self.kst_value)
            .field("is_ready", &self.is_ready)
            .finish()
    }
}

impl KnowSureThing {
    /// Creates a new KST with default parameters (SMA smoothers).
    pub fn new() -> Self {
        Self::with_params([10, 15, 20, 30], [10, 10, 10, 15], 9)
    }

    /// Creates a new KST with custom parameters.
    ///
    /// # Arguments
    /// * `roc_periods` - ROC lookback periods (typically [10, 15, 20, 30])
    /// * `sma_periods` - SMA smoothing periods (typically [10, 10, 10, 15])
    /// * `signal_period` - Signal line period (typically 9)
    pub fn with_params(roc_periods: [usize; 4], sma_periods: [usize; 4], signal_period: usize) -> Self {
        Self::from_smoothers(
            roc_periods, sma_periods, signal_period,
            [SmootherId::Sma; 4], SmootherId::Sma,
        )
    }

    /// Build from narrow SmootherId arrays for each slot.
    pub fn from_smoothers(
        roc_periods: [usize; 4],
        sma_periods: [usize; 4],
        signal_period: usize,
        roc_ids: [SmootherId; 4],
        signal_id: SmootherId,
    ) -> Self {
        assert!(roc_periods.iter().all(|&p| p > 0), "All ROC periods must be greater than 0");
        assert!(sma_periods.iter().all(|&p| p > 0), "All SMA periods must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            roc_periods,
            sma_periods,
            signal_period,
            source: OhlcvField::Close,
            source_prices: Vec::with_capacity(512),
            roc_smas: [
                SmootherSlot::new(roc_ids[0], sma_periods[0]),
                SmootherSlot::new(roc_ids[1], sma_periods[1]),
                SmootherSlot::new(roc_ids[2], sma_periods[2]),
                SmootherSlot::new(roc_ids[3], sma_periods[3]),
            ],
            signal_sma: SmootherSlot::new(signal_id, signal_period),
            kst_values: Vec::with_capacity(512),
            kst_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new KST with custom source field.
    pub fn with_source(roc_periods: [usize; 4], sma_periods: [usize; 4], signal_period: usize, source: OhlcvField) -> Self {
        let mut kst = Self::with_params(roc_periods, sma_periods, signal_period);
        kst.source = source;
        kst
    }

    /// Feed a pre-extracted scalar and update KST.
    pub fn feed(&mut self, source_value: f64) -> f64 {
        self.bars_count += 1;

        if self.source_prices.len() >= 512 {
            self.source_prices.remove(0);
        }
        self.source_prices.push(source_value);

        let max_roc_period = *self.roc_periods.iter().max().unwrap();
        if self.source_prices.len() < max_roc_period + 1 {
            return self.kst_value;
        }

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for i in 0..4 {
            let roc_period = self.roc_periods[i];

            if self.source_prices.len() > roc_period {
                let current_price = source_value;
                let past_price = self.source_prices[self.source_prices.len() - roc_period - 1];

                let roc = if past_price.abs() < 1e-12 {
                    0.0
                } else {
                    ((current_price - past_price) / past_price) * 100.0
                };

                let smoothed_roc = self.roc_smas[i].feed(roc);

                let weight = match i {
                    0 => 1.0,
                    1 => 2.0,
                    2 => 3.0,
                    3 => 4.0,
                    _ => 1.0,
                };

                weighted_sum += smoothed_roc * weight;
                total_weight += weight;
            }
        }

        self.kst_value = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        if self.kst_values.len() >= 512 {
            self.kst_values.remove(0);
        }
        self.kst_values.push(self.kst_value);

        self.signal_value = self.signal_sma.feed(self.kst_value);

        let min_bars = max_roc_period + self.sma_periods.iter().max().unwrap() + self.signal_period;
        if self.bars_count >= min_bars {
            self.is_ready = true;
        }

        self.kst_value
    }


    /// Returns the signal line value.
    #[inline]
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }

    /// Returns the histogram (KST - Signal).
    #[inline]
    pub fn histogram(&self) -> f64 {
        self.kst_value - self.signal_value
    }

    /// Brace-named getter: `kst` output (KST oscillator line).
    #[inline]
    pub fn kst(&self) -> f64 {
        self.kst_value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal_value
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the indicator parameters (roc_periods, sma_periods, signal_period).
    #[inline]
    pub fn parameters(&self) -> ([usize; 4], [usize; 4], usize) {
        (self.roc_periods, self.sma_periods, self.signal_period)
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.source_prices.clear();
        self.roc_smas = [
            SmootherSlot::new(SmootherId::Sma, self.sma_periods[0]),
            SmootherSlot::new(SmootherId::Sma, self.sma_periods[1]),
            SmootherSlot::new(SmootherId::Sma, self.sma_periods[2]),
            SmootherSlot::new(SmootherId::Sma, self.sma_periods[3]),
        ];
        self.signal_sma = SmootherSlot::new(SmootherId::Sma, self.signal_period);
        self.kst_values.clear();
        self.kst_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Sets a new source field and resets the indicator.
    pub fn set_source(&mut self, source: OhlcvField) {
        self.source = source;
        self.reset();
    }

    /// Returns the current market condition.
    pub fn market_condition(&self) -> &'static str {
        match self.kst_value {
            v if v > 0.0 && self.kst_value > self.signal_value => "Strong Bullish",
            v if v > 0.0 => "Bullish",
            v if v < 0.0 && self.kst_value < self.signal_value => "Strong Bearish",
            v if v < 0.0 => "Bearish",
            _ => "Neutral"
        }
    }

    /// Returns trading signal (1 = buy, -1 = sell, 0 = neutral).
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        let histogram = self.histogram();
        if self.kst_value > self.signal_value && histogram > 0.1 {
            1
        } else if self.kst_value < self.signal_value && histogram < -0.1 {
            -1
        } else {
            0
        }
    }

    /// Returns momentum strength (absolute KST value).
    #[inline]
    pub fn momentum_strength(&self) -> f64 {
        self.kst_value.abs()
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, Slot};

/// Typed contract config for [`KnowSureThing`] — 4 ROC-smoother slots + 1 signal slot.
///
/// Dual-mode: every field is a `Param`. Each `#[slot]` smoother is a `Param<SmootherChoice>`
/// that follows its lane's period: the four ROC smoothers follow `sma_periods[i]` and the
/// signal follows `signal_period`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KstConfig {
    pub source: Param<OhlcvField>,
    pub roc_periods: Param<[usize; 4]>,
    pub sma_periods: Param<[usize; 4]>,
    pub signal_period: Param<usize>,
    #[slot]
    pub roc1_ma: Param<SmootherChoice>,
    #[slot]
    pub roc2_ma: Param<SmootherChoice>,
    #[slot]
    pub roc3_ma: Param<SmootherChoice>,
    #[slot]
    pub roc4_ma: Param<SmootherChoice>,
    #[slot]
    pub signal: Param<SmootherChoice>,
}

impl Indicator for KnowSureThing {
    const ID: IndicatorId = IndicatorId::Kst;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = KstConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::KstKst),
        Output::centered(IndicatorOutputId::KstSignal),
    ];
    type Config = KstConfig;
    type Runtime = KnowSureThing;

    fn create(cfg: KstConfig) -> KnowSureThing {
        let sma_periods = cfg.sma_periods.resolved().map(|p| p.max(1));
        let signal_period = cfg.signal_period.resolved().max(1);
        KnowSureThing {
            roc_periods: cfg.roc_periods.resolved(),
            sma_periods,
            signal_period,
            source: cfg.source.resolved(),
            source_prices: Vec::with_capacity(512),
            roc_smas: [
                cfg.roc1_ma.resolved().build(sma_periods[0]),
                cfg.roc2_ma.resolved().build(sma_periods[1]),
                cfg.roc3_ma.resolved().build(sma_periods[2]),
                cfg.roc4_ma.resolved().build(sma_periods[3]),
            ],
            signal_sma: cfg.signal.resolved().build(signal_period),
            kst_values: Vec::with_capacity(512),
            kst_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    fn source_fields(cfg: &KstConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &KstConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for KstConfig {
    fn defaults() -> Self {
        KstConfig {
            source: Param::Solo(OhlcvField::Close),
            roc_periods: Param::Solo([10, 15, 20, 30]),
            sma_periods: Param::Solo([10, 10, 10, 15]),
            signal_period: Param::Solo(9),
            roc1_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            roc2_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            roc3_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            roc4_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            signal: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // source: Class O → auto all-8; signal_period: Class A → auto range(2,4048,1).
        // roc_periods/sma_periods: Class S [usize;4] — 4 canonical presets from KST literature.
        // roc1_ma..roc4_ma/signal: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.roc_periods = Param::many(vec![
            [10, 15, 20, 30],
            [10, 13, 14, 15],
            [6, 10, 14, 18],
            [20, 30, 40, 50],
        ]);
        s.sma_periods = Param::many(vec![
            [10, 10, 10, 15],
            [10, 13, 14, 15],
            [4,  5,  6,  8],
            [10, 13, 14, 15],
        ]);
        s
    }
}


impl Render for KnowSureThing {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::KstKst, "KST", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::KstSignal, "Signal", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kst_basic_calculation() {
        let mut kst = KnowSureThing::new();
        for i in 1..=80 {
            kst.feed(100.0 + i as f64);
        }
        assert!(kst.is_ready());
        let (kst_val, signal_val) = (kst.kst(), kst.signal());
        assert!(kst_val.is_finite());
        assert!(signal_val.is_finite());
    }

    #[test]
    fn test_kst_uptrend() {
        let mut kst = KnowSureThing::new();
        for i in 1..=100 {
            kst.feed(100.0 + i as f64 * 2.0);
        }
        assert!(kst.is_ready());
        let kst_val = kst.kst();
        assert!(kst_val > 0.0, "KST should be positive in uptrend");
    }

    #[test]
    fn test_kst_downtrend() {
        let mut kst = KnowSureThing::new();
        for i in 1..=100 {
            kst.feed(300.0 - i as f64 * 2.0);
        }
        assert!(kst.is_ready());
        let kst_val = kst.kst();
        assert!(kst_val < 0.0, "KST should be negative in downtrend");
    }

    #[test]
    fn test_kst_reset() {
        let mut kst = KnowSureThing::new();
        for i in 1..=100 {
            kst.feed(100.0 + i as f64);
        }
        assert!(kst.is_ready());
        kst.reset();
        assert!(!kst.is_ready());
    }

    #[test]
    fn test_kst_parameters() {
        let kst = KnowSureThing::with_params([5, 10, 15, 20], [5, 5, 5, 10], 7);
        let (roc, sma, sig) = kst.parameters();
        assert_eq!(roc, [5, 10, 15, 20]);
        assert_eq!(sma, [5, 5, 5, 10]);
        assert_eq!(sig, 7);
    }

    #[test]
    fn test_kst_trading_signal() {
        let mut kst = KnowSureThing::new();
        for i in 1..=100 {
            kst.feed(100.0 + i as f64);
        }
        assert!(kst.is_ready());
        let signal = kst.trading_signal();
        assert!(signal >= -1 && signal <= 1);
    }

    #[test]
    fn test_kst_momentum_strength() {
        let mut kst = KnowSureThing::new();
        for i in 1..=100 {
            kst.feed(100.0 + i as f64);
        }
        assert!(kst.is_ready());
        assert!(kst.momentum_strength() >= 0.0);
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<KnowSureThing as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Kst(cfg).build_solo().unwrap();
        for i in 1..=100 {
            let price = 100.0 + i as f64 * 2.0;
            // high=9999.0 proves only close is resolved
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        let kst_val = f.primary();
        assert!(kst_val > 0.0);
    }
}

impl Default for KnowSureThing {
    fn default() -> Self {
        Self::new()
    }
}
