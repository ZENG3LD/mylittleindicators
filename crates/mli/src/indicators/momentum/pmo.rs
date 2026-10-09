// PMO (Price Momentum Oscillator) - double-smoothed ROC

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

#[derive(Debug, Clone)]
pub struct Pmo {
    roc_period: usize,
    smooth1: usize,
    smooth2: usize,
    signal_period: usize,
    ema1: SmootherSlot,
    ema2: SmootherSlot,
    signal_ema: SmootherSlot,
    prev_close: f64,
    initialized: bool,
    pmo_value: f64,
    signal_value: f64,
}

impl Pmo {
    pub fn new(roc_period: usize, smooth1: usize, smooth2: usize) -> Self {
        Self::from_smoothers(roc_period, smooth1, smooth2, 10, SmootherId::Ema)
    }

    pub fn from_smoothers(
        roc_period: usize,
        smooth1: usize,
        smooth2: usize,
        signal_period: usize,
        id: SmootherId,
    ) -> Self {
        let s1 = smooth1.max(1);
        let s2 = smooth2.max(1);
        let sig = signal_period.max(1);
        Self {
            roc_period: roc_period.max(1),
            smooth1: s1,
            smooth2: s2,
            signal_period: sig,
            ema1: SmootherSlot::new(id, s1),
            ema2: SmootherSlot::new(id, s2),
            signal_ema: SmootherSlot::new(id, sig),
            prev_close: 0.0,
            initialized: false,
            pmo_value: 0.0,
            signal_value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema1 = SmootherSlot::new(SmootherId::Ema, self.smooth1);
        self.ema2 = SmootherSlot::new(SmootherId::Ema, self.smooth2);
        self.signal_ema = SmootherSlot::new(SmootherId::Ema, self.signal_period);
        self.prev_close = 0.0;
        self.initialized = false;
        self.pmo_value = 0.0;
        self.signal_value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized && self.ema2.is_ready()
    }


    /// Feed close price scalar.
    pub fn feed(&mut self, c: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.pmo_value;
        }
        let roc = if self.prev_close.abs() > 1e-12 {
            (c - self.prev_close) / self.prev_close * 100.0
        } else {
            0.0
        };
        self.prev_close = c;
        let s1 = self.ema1.feed(roc);
        self.pmo_value = self.ema2.feed(s1);
        self.signal_value = self.signal_ema.feed(self.pmo_value);
        self.pmo_value
    }

    pub fn roc_period(&self) -> usize {
        self.roc_period
    }

    pub fn smooth1_period(&self) -> usize {
        self.smooth1
    }

    pub fn smooth2_period(&self) -> usize {
        self.smooth2
    }

    /// Brace-named getter: `pmo` output (PMO oscillator line).
    #[inline]
    pub fn pmo(&self) -> f64 {
        self.pmo_value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal_value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, Slot};

/// Typed contract config for [`Pmo`] — three smoother slots (double-smooth + signal).
///
/// Dual-mode: every field is a `Param`. Each `#[slot]` smoother is a `Param<SmootherChoice>`
/// that follows its lane's period field.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct PmoConfig {
    pub roc_period: Param<usize>,
    pub smooth1_period: Param<usize>,
    pub smooth2_period: Param<usize>,
    pub signal_period: Param<usize>,
    #[slot]
    pub smooth1: Param<SmootherChoice>,
    #[slot]
    pub smooth2: Param<SmootherChoice>,
    #[slot]
    pub signal: Param<SmootherChoice>,
}

impl Indicator for Pmo {
    const ID: IndicatorId = IndicatorId::Pmo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    // PMO only reads close; hardcoded single-field source.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = PmoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::PmoPmo),
        Output::centered(IndicatorOutputId::PmoSignal),
    ];
    type Config = PmoConfig;
    type Runtime = Pmo;

    fn create(cfg: PmoConfig) -> Pmo {
        let s1 = cfg.smooth1_period.resolved().max(1);
        let s2 = cfg.smooth2_period.resolved().max(1);
        let sig = cfg.signal_period.resolved().max(1);
        Pmo {
            roc_period: cfg.roc_period.resolved().max(1),
            smooth1: s1,
            smooth2: s2,
            signal_period: sig,
            ema1: cfg.smooth1.resolved().build(s1),
            ema2: cfg.smooth2.resolved().build(s2),
            signal_ema: cfg.signal.resolved().build(sig),
            prev_close: 0.0,
            initialized: false,
            pmo_value: 0.0,
            signal_value: 0.0,
        }
    }

    fn source_fields(_cfg: &PmoConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }

    fn slot_members(cfg: &PmoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for PmoConfig {
    fn defaults() -> Self {
        PmoConfig {
            roc_period: Param::Solo(1),
            smooth1_period: Param::Solo(35),
            smooth2_period: Param::Solo(20),
            signal_period: Param::Solo(10),
            smooth1: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            smooth2: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            signal: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // roc_period/smooth1_period/smooth2_period/signal_period: Class A → auto range(2,4048,1).
        // smooth1/smooth2/signal: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Pmo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::PmoPmo, "PMO", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::PmoSignal, "Signal", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmo_creation() {
        let pmo = Pmo::new(1, 35, 20);
        assert!(!pmo.is_ready());
        assert_eq!(pmo.pmo(), 0.0);
        assert_eq!(pmo.roc_period(), 1);
        assert_eq!(pmo.smooth1_period(), 35);
        assert_eq!(pmo.smooth2_period(), 20);
    }

    #[test]
    fn test_pmo_basic_calculation() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            pmo.feed(price);
        }
        assert!(pmo.is_ready());
        assert!(pmo.pmo() > 0.0, "PMO should be positive in uptrend");
    }

    #[test]
    fn test_pmo_downtrend() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 200.0 - i as f64;
            pmo.feed(price);
        }
        assert!(pmo.is_ready());
        assert!(pmo.pmo() < 0.0, "PMO should be negative in downtrend");
    }

    #[test]
    fn test_pmo_reset() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            pmo.feed(price);
        }
        assert!(pmo.is_ready());
        pmo.reset();
        assert!(!pmo.is_ready());
        assert_eq!(pmo.pmo(), 0.0);
    }

    #[test]
    fn test_pmo_finite_values() {
        let mut pmo = Pmo::new(1, 10, 5);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = pmo.feed(price);
            assert!(value.is_finite(), "PMO should always be finite");
        }
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut cfg = <<Pmo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        // Use shorter periods for faster warmup
        cfg.smooth1 = Param::Solo(SmootherChoice::own(SmootherId::Ema, 5));
        cfg.smooth2 = Param::Solo(SmootherChoice::own(SmootherId::Ema, 3));
        cfg.signal = Param::Solo(SmootherChoice::own(SmootherId::Ema, 3));
        let mut f = IndicatorOrder::Pmo(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            // high=9999.0 proves only close is resolved
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() > 0.0);
    }
}

impl Default for Pmo {
    fn default() -> Self {
        Self::new(10, 10, 10)
    }
}
