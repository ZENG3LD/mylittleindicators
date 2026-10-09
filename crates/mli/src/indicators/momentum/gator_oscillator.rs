// Gator Oscillator: difference of two smoothed MAs

use crate::engine::contract_engine::SmootherSlot;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::SmootherId;

/// Gator Oscillator - difference between fast and slow moving averages.
///
/// Value = Fast MA - Slow MA
///
/// The Gator Oscillator measures the divergence between two moving averages.
/// Positive values indicate the fast MA is above the slow MA (bullish momentum),
/// while negative values indicate bearish momentum.
///
/// # Parameters
/// - `fast_period`: Fast moving average period
/// - `slow_period`: Slow moving average period
/// - `source`: OHLCV field to use as input (default Close)
///
/// # Implementation
///
/// Uses configurable moving average types. O(1) per update.
#[derive(Debug, Clone)]
pub struct GatorOscillator {
    fast_period: usize,
    slow_period: usize,
    source: OhlcvField,
    fast: SmootherSlot,
    slow: SmootherSlot,
    value: f64,
}

impl GatorOscillator {
    /// Creates a new Gator Oscillator with default MA type (EMA) and Close source.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, SmootherId::Ema, slow_period)
    }

    /// Build from narrow SmootherId for fast and slow MA.
    pub fn from_smoothers(fast_id: SmootherId, fast_period: usize, slow_id: SmootherId, slow_period: usize) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(2);
        Self {
            fast_period: fast,
            slow_period: slow,
            source: OhlcvField::Close,
            fast: SmootherSlot::new(fast_id, fast),
            slow: SmootherSlot::new(slow_id, slow),
            value: 0.0,
        }
    }

    /// Creates a new Gator Oscillator with custom source field.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `source` - OHLCV field to use as input
    pub fn with_source(fast_period: usize, slow_period: usize, source: OhlcvField) -> Self {
        let mut gator = Self::new(fast_period, slow_period);
        gator.source = source;
        gator
    }

    /// Resets the Gator Oscillator to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.fast = SmootherSlot::new(SmootherId::Ema, self.fast_period);
        self.slow = SmootherSlot::new(SmootherId::Ema, self.slow_period);
        self.value = 0.0;
    }

    /// Returns `true` if the Gator Oscillator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }

    /// Returns the Gator Oscillator value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed a pre-extracted scalar (Source flavor).
    pub fn feed(&mut self, price: f64) -> f64 {
        let f = self.fast.feed(price);
        let s = self.slow.feed(price);
        self.value = f - s;
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, Slot};

/// Typed contract config for [`GatorOscillator`] — two independent smoother slots.
///
/// Dual-mode: every field is a `Param`. The two `#[slot]` smoothers are `Param<SmootherChoice>`
/// that follow their respective period fields (fast=5, slow=8 by default).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct GatorConfig {
    pub source: Param<OhlcvField>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherChoice>,
    #[slot]
    pub slow: Param<SmootherChoice>,
}

impl Indicator for GatorOscillator {
    const ID: IndicatorId = IndicatorId::Gator;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = GatorConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Gator)];
    type Config = GatorConfig;
    type Runtime = GatorOscillator;

    fn create(cfg: GatorConfig) -> GatorOscillator {
        let fast_period = cfg.fast_period.resolved().max(1);
        let slow_period = cfg.slow_period.resolved().max(2);
        GatorOscillator {
            fast_period,
            slow_period,
            source: cfg.source.resolved(),
            fast: cfg.fast.resolved().build(fast_period),
            slow: cfg.slow.resolved().build(slow_period),
            value: 0.0,
        }
    }

    fn source_fields(cfg: &GatorConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &GatorConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for GatorConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        GatorConfig {
            source: Param::Solo(OhlcvField::Close),
            fast_period: Param::Solo(5),
            slow_period: Param::Solo(8),
            fast: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fast_period/slow_period: Class A → auto range(2,4048,1) EACH, but both then resolve
        // to the same min (2), failing this config's OWN `valid_params` (fast < slow) at the
        // min corner (2026-07-03 fix). Split into disjoint ranges so `resolved()` stays ordered.
        // source: Class O → auto all-8.
        // fast/slow: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
}


impl Render for GatorOscillator {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Gator, "Gator", Color::hex(0x4CAF50))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gator_creation() {
        let gator = GatorOscillator::new(5, 13);
        assert!(!gator.is_ready());
        assert_eq!(gator.value(), 0.0);
    }

    #[test]
    fn test_gator_uptrend() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            gator.feed(price);
        }
        assert!(gator.is_ready());
        // In uptrend, fast EMA > slow EMA
        assert!(gator.value() > 0.0, "Gator should be positive in uptrend, got {}", gator.value());
    }

    #[test]
    fn test_gator_downtrend() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            gator.feed(price);
        }
        assert!(gator.is_ready());
        // In downtrend, fast EMA < slow EMA
        assert!(gator.value() < 0.0, "Gator should be negative in downtrend, got {}", gator.value());
    }

    #[test]
    fn test_gator_reset() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            gator.feed(price);
        }
        assert!(gator.is_ready());
        gator.reset();
        assert!(!gator.is_ready());
        assert_eq!(gator.value(), 0.0);
    }

    #[test]
    fn test_gator_finite_values() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = gator.feed(price);
            assert!(value.is_finite(), "Gator should always be finite");
        }
    }

    #[test]
    fn test_gator_with_sma() {
        let mut gator = GatorOscillator::from_smoothers(SmootherId::Sma, 5, SmootherId::Sma, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            gator.feed(price);
        }
        assert!(gator.is_ready());
        assert!(gator.value() > 0.0);
    }

    #[test]
    fn test_gator_with_high_source() {
        let gator = GatorOscillator::with_source(5, 13, OhlcvField::High);
        assert_eq!(gator.source, OhlcvField::High);
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<GatorOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Gator(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            // high=9999.0 is NOT in const SOURCE (only Close), proving field resolution
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() > 0.0);
    }
}

impl Default for GatorOscillator {
    fn default() -> Self {
        Self::new(5, 8)
    }
}
