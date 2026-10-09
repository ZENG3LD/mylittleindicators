// Ehlers Cyber Cycle - simplified recursive filter
#[derive(Debug, Clone)]
pub struct CyberCycle {
    alpha: f64,
    prev1: f64,
    prev2: f64,
    value: f64,
}

impl CyberCycle {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            prev1: 0.0,
            prev2: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev1 = 0.0;
        self.prev2 = 0.0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn feed(&mut self, c: f64) -> f64 {
        let a = self.alpha;
        let val = (1.0 - a / 2.0) * (1.0 - a / 2.0) * (c - 2.0 * self.prev1 + self.prev2)
            + 2.0 * (1.0 - a) * self.prev1
            - (1.0 - a) * (1.0 - a) * self.prev2;
        self.prev2 = self.prev1;
        self.prev1 = c;
        self.value = val;
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

impl Default for CyberCycle {
    /// Factory default: alpha=0.2.
    fn default() -> Self {
        Self::new(0.2)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`CyberCycle`]: smoothing alpha (0..1).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CyberConfig {
    pub alpha: Param<f64>,
}

impl Indicator for CyberCycle {
    const ID: IndicatorId = IndicatorId::Cyber;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): pure scalar recursive IIR, 2 scalar delay registers.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[crate::contract::Output::centered(IndicatorOutputId::Cyber)];
    type Config = CyberConfig;
    type Runtime = CyberCycle;

    fn create(cfg: CyberConfig) -> CyberCycle {
        CyberCycle::new(cfg.alpha.resolved())
    }

    fn source_fields(cfg: &CyberConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for CyberConfig {
    fn defaults() -> Self {
        CyberConfig { alpha: Param::Solo(0.07) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        s.alpha = Param::many(sweep_f64(0.01, 0.99, 0.01)); // Class E alpha/decay
        s
    }
}


impl Render for CyberCycle {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cyber, "Cyber Cycle", Color::hex(0x00BCD4))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyber_cycle_creation() {
        let cc = CyberCycle::new(0.07);
        assert!(cc.is_ready());
        assert_eq!(cc.value(), 0.0);
        assert!((cc.alpha() - 0.07).abs() < 1e-9);
    }

    #[test]
    fn test_cyber_cycle_finite() {
        let mut cc = CyberCycle::new(0.07);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = cc.feed(price);
            assert!(value.is_finite(), "CyberCycle should always be finite");
        }
    }

    #[test]
    fn test_cyber_cycle_reset() {
        let mut cc = CyberCycle::new(0.07);
        for i in 1..=20 {
            cc.feed(100.0 + i as f64);
        }
        cc.reset();
        assert_eq!(cc.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_cyber() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<CyberCycle as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Cyber(cfg).build_solo().unwrap();
        for i in 1..=20 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }
}
