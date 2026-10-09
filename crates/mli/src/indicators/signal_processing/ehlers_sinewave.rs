// Ehlers Sinewave - simplified streaming proxy
#[derive(Debug, Clone)]
pub struct EhlersSinewave {
    alpha: f64,
    prev: f64,
    phase: f64,
    value: f64,
}

impl EhlersSinewave {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            prev: 0.0,
            phase: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev = 0.0;
        self.phase = 0.0;
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
        let diff = c - self.prev;
        self.prev = c;
        // crude phase increment proportional to normalized diff
        self.phase += (diff.tanh()) * self.alpha;
        self.value = self.phase.sin();
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

}

impl Default for EhlersSinewave {
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
use crate::contract::{Color, Render, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`EhlersSinewave`]: smoothing alpha.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EsineConfig {
    pub alpha: Param<f64>,
}

impl Indicator for EhlersSinewave {
    const ID: IndicatorId = IndicatorId::Esine;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[crate::contract::Output::centered(IndicatorOutputId::Esine)];
    type Config = EsineConfig;
    type Runtime = EhlersSinewave;

    fn create(cfg: EsineConfig) -> EhlersSinewave {
        EhlersSinewave::new(cfg.alpha.resolved())
    }

    fn source_fields(cfg: &EsineConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for EsineConfig {
    fn defaults() -> Self {
        EsineConfig { alpha: Param::Solo(0.2) }
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


impl Render for EhlersSinewave {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Esine, "Ehlers Sinewave", Color::hex(0x2196F3))
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehlers_sinewave_creation() {
        let sw = EhlersSinewave::new(0.5);
        assert!(sw.is_ready());
        assert_eq!(sw.value(), 0.0);
        assert!((sw.alpha() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_ehlers_sinewave_range() {
        let mut sw = EhlersSinewave::new(0.5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = sw.feed(price);
            assert!(value >= -1.0 && value <= 1.0, "Sinewave should be in [-1, 1], got {}", value);
        }
    }

    #[test]
    fn factory_feeds_resolved_esine() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<EhlersSinewave as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Esine(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v >= -1.0 && v <= 1.0, "sinewave out of [-1,1]: {v}");
    }

    #[test]
    fn test_ehlers_sinewave_reset() {
        let mut sw = EhlersSinewave::new(0.5);
        for i in 1..=20 {
            sw.feed(100.0 + i as f64);
        }
        sw.reset();
        assert_eq!(sw.value(), 0.0);
    }
}
