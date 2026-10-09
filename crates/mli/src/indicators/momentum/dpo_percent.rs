// DPO%: detrended price oscillator as percent of price

use crate::indicators::momentum::dpo::DetrendedPriceOscillator;


#[derive(Debug, Clone)]
pub struct DpoPercent {
    dpo: DetrendedPriceOscillator,
    value: f64,
}

impl DpoPercent {
    pub fn new(period: usize) -> Self {
        Self {
            dpo: DetrendedPriceOscillator::with_period(period.max(2)),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.dpo.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dpo.is_ready()
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed close price scalar — DPO% = DPO / price.
    pub fn feed(&mut self, c: f64) -> f64 {
        let d = self.dpo.feed(c);
        self.value = if c.abs() > 1e-12 { d / c } else { 0.0 };
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`DpoPercent`] — period only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DpoPctConfig {
    pub period: Param<usize>,
}

impl Indicator for DpoPercent {
    const ID: IndicatorId = IndicatorId::DpoPct;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Dpo, &[IndicatorOutputId::Dpo])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::DpoPct)];
    type Config = DpoPctConfig;
    type Runtime = DpoPercent;

    fn create(cfg: DpoPctConfig) -> DpoPercent {
        DpoPercent::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &DpoPctConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for DpoPctConfig {
    fn defaults() -> Self {
        DpoPctConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for DpoPercent {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DpoPct, "DPO %", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpo_percent_creation() {
        let dpo = DpoPercent::new(14);
        assert!(!dpo.is_ready());
        assert_eq!(dpo.value(), 0.0);
    }

    #[test]
    fn test_dpo_percent_basic() {
        let mut dpo = DpoPercent::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dpo.feed(price);
        }
        assert!(dpo.is_ready());
        assert!(dpo.value().is_finite());
    }

    #[test]
    fn test_dpo_percent_reset() {
        let mut dpo = DpoPercent::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dpo.feed(price);
        }
        assert!(dpo.is_ready());
        dpo.reset();
        assert!(!dpo.is_ready());
        assert_eq!(dpo.value(), 0.0);
    }

    #[test]
    fn test_dpo_percent_finite_values() {
        let mut dpo = DpoPercent::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = dpo.feed(price);
            assert!(value.is_finite(), "DPO% should always be finite");
        }
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DpoPercent as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 14);
        let mut f = IndicatorOrder::DpoPct(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            // high=9999.0 proves only close is resolved
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }
}

impl Default for DpoPercent {
    fn default() -> Self {
        Self::new(14)
    }
}
