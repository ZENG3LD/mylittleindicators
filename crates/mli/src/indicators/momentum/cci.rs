//! Commodity Channel Index (CCI) indicator.


/// Commodity Channel Index (CCI) - measures price deviation from statistical mean.
///
/// CCI = (Typical Price - MA) / (scalar × Mean Deviation)
///
/// where Typical Price = (High + Low + Close) / 3
/// and Mean Deviation = average of |TP - MA| over the period.
///
/// CCI typically oscillates between -100 and +100:
/// - Above +100: Overbought / strong uptrend
/// - Below -100: Oversold / strong downtrend
/// - Zero line crossings: Potential trend changes
///
/// # Parameters
/// - `period`: Lookback period (typically 20)
/// - `scalar`: Lambert's constant (typically 0.015)
/// - `ma_type`: Moving average type for smoothing
///
/// # Implementation
///
/// Uses configurable MA and ring buffer for mean deviation. O(period) per update.
/// Maximum period is 512.
#[derive(Debug, Clone)]
pub struct Cci {
    period: usize,
    scalar: f64,
    ma: SmootherSlot,
    typical_buf: Vec<f64>,
    idx: usize,
    count: usize,
    value: f64,
    filled: bool,
}

impl Cci {
    /// Creates a new CCI with the specified parameters.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    /// * `scalar` - Lambert's constant (typically 0.015)
    /// * `ma_type` - Optional MA type (defaults to SMA)
    /// Default ctor — mean line smoothed with SMA.
    pub fn new(period: usize, scalar: f64) -> Self {
        Self::from_smoother(period, scalar, SmootherId::Sma)
    }

    /// Feed the resolved input lanes — `[high, low, close]` (the factory resolves the fixed
    /// HLC slice). Computes the typical price internally; the core knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let typical = (lanes[0] + lanes[1] + lanes[2]) / 3.0;

        if self.count < self.period {
            self.typical_buf.push(typical);
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            self.typical_buf[self.idx] = typical;
            self.idx = (self.idx + 1) % self.period;
        }

        self.ma.feed(typical);

        if self.count >= self.period {
            let mean = self.ma.value();
            let mad = self.typical_buf.iter().map(|&v| (v - mean).abs()).sum::<f64>() / self.period as f64;
            if mad.abs() < 1e-12 {
                self.value = 0.0;
            } else {
                self.value = (typical - mean) / (self.scalar * mad);
            }
            self.filled = true;
        } else {
            self.value = 0.0;
            self.filled = false;
        }
        self.value
    }

    /// Returns the current CCI value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the CCI has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the CCI to its initial state.
    pub fn reset(&mut self) {
        self.typical_buf.clear();
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
        self.filled = false;
        self.ma.reset();
    }

    /// Returns the period of this CCI.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{
    Cost, Family, Slot, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

impl Cci {
    /// Build a CCI from a smoother CHOICE (kind + follow/own period) at the host `period`.
    /// The mean-line smoother uses the choice; `Follow` resolves to `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize, scalar: f64) -> Self {
        Self {
            period,
            scalar,
            ma: choice.build(period),
            typical_buf: Vec::with_capacity(period),
            idx: 0,
            count: 0,
            value: 0.0,
            filled: false,
        }
    }

    /// Build a CCI whose mean-line smoother is the given [`SmootherId`].
    pub fn from_smoother(period: usize, scalar: f64, smoother: SmootherId) -> Self {
        Self {
            period,
            scalar,
            ma: SmootherSlot::new(smoother, period),
            typical_buf: Vec::with_capacity(period),
            idx: 0,
            count: 0,
            value: 0.0,
            filled: false,
        }
    }
}

/// Typed config for [`Cci`]: period + Lambert scalar + the mean-line smoother CHOICE.
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set. The
/// `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`. The `#[slot]` smoother is a
/// `Param<SmootherChoice>`: which kind + follow/own period.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct CciConfig {
    pub period: Param<usize>,
    pub scalar: Param<f64>,
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for Cci {
    const ID: IndicatorId = IndicatorId::Cci;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices — its typical price is (h+l+c)/3.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// CCI's OWN state is a period-deep typical-price `Vec`, rescanned for the mean
    /// deviation each bar -> O(period). Its mean-line MA smoother is a MovingAverage
    /// slot, charged recursively by the barometer.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = CciConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cci)];

    type Config = CciConfig;
    type Runtime = Cci;

    fn create(cfg: CciConfig) -> Cci {
        let period = cfg.period.resolved();
        Cci::from_choice(cfg.smoother.resolved(), period, cfg.scalar.resolved())
    }

    // Input fields: the factory derives the fixed [High, Low, Close] lanes straight from
    // `const SOURCE = KlineSlice([..])` (the default `source_fields`) — no re-declaration.

    fn slot_members(cfg: &CciConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for CciConfig {
    fn defaults() -> Self {
        CciConfig {
            period: Param::Solo(20),
            scalar: Param::Solo(0.015),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // scalar: Class K (CCI scaling constant, classic 0.015) — sweep_f64(0.005,0.10,0.005).
        // smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.scalar = Param::many(sweep_f64(0.005, 0.10, 0.005));
        s
    }
}


impl Render for Cci {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cci, "CCI", Color::hex(0x9C27B0))
            .zero_baseline()
            .reference_line(ReferenceLine::new(100.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(-100.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests — pure three-lane `feed(&[high, low, close])` core
    // =========================================================================

    #[test]
    fn test_cci_basic_calculation() {
        let mut cci = Cci::new(20, 0.015);

        // Feed uptrend data
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            cci.feed(&[base + 2.0, base - 2.0, base]);
        }

        assert!(cci.is_ready());
        // In strong uptrend, CCI should be positive
        assert!(cci.value() > 0.0, "CCI in uptrend should be positive");
    }

    #[test]
    fn test_cci_downtrend() {
        let mut cci = Cci::new(20, 0.015);

        // Feed downtrend data
        for i in 1..=30 {
            let base = 200.0 - i as f64;
            cci.feed(&[base + 2.0, base - 2.0, base]);
        }

        assert!(cci.is_ready());
        // In strong downtrend, CCI should be negative
        assert!(cci.value() < 0.0, "CCI in downtrend should be negative");
    }

    #[test]
    fn test_cci_constant_price() {
        let mut cci = Cci::new(20, 0.015);

        // Feed constant price data
        for _ in 1..=30 {
            cci.feed(&[102.0, 98.0, 100.0]);
        }

        assert!(cci.is_ready());
        // With constant typical price, CCI should be ~0
        assert!(cci.value().abs() < 1.0, "CCI with constant price should be ~0");
    }

    #[test]
    fn test_cci_with_ma_type() {
        let mut cci = Cci::from_smoother(20, 0.015, SmootherId::Ema);

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            cci.feed(&[base + 2.0, base - 2.0, base]);
        }

        assert!(cci.is_ready());
    }

    #[test]
    fn test_cci_reset() {
        let mut cci = Cci::new(20, 0.015);

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            cci.feed(&[base + 2.0, base - 2.0, base]);
        }
        assert!(cci.is_ready());

        cci.reset();
        assert!(!cci.is_ready());
        assert!((cci.value()).abs() < 1e-10);
    }

    #[test]
    fn test_cci_period_getter() {
        let cci = Cci::new(20, 0.015);
        assert_eq!(cci.period(), 20);
    }

    #[test]
    fn test_cci_not_ready_before_period() {
        let mut cci = Cci::new(20, 0.015);

        for i in 1..=15 {
            let base = 100.0 + i as f64;
            cci.feed(&[base + 2.0, base - 2.0, base]);
        }

        assert!(!cci.is_ready());
    }

    /// The factory resolves the fixed HLC lanes and feeds the three scalars; CCI computes
    /// the typical price end-to-end (not from a raw bar).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Cci(<<Cci as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 0.0, high: base + 2.0, low: base - 2.0, close: base, volume: 0.0,
            });
        }
        assert!(f.primary() > 0.0, "factory CCI in uptrend should be > 0, got {}", f.primary());
    }
}
