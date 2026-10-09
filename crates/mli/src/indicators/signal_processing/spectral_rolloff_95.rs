// Convenience wrapper around SpectralRolloff with target 0.95

use crate::indicators::signal_processing::spectral_rolloff::SpectralRolloff;

#[derive(Debug, Clone)]
pub struct SpectralRolloff95 {
    inner: SpectralRolloff,
}

impl SpectralRolloff95 {
    pub fn new(window: usize) -> Self {
        Self {
            inner: SpectralRolloff::new(window, 0.95),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn feed(&mut self, c: f64) -> f64 {
        self.inner.feed(c)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.inner.value()
    }
}

impl Default for SpectralRolloff95 {
    /// Factory default: window=32.
    fn default() -> Self {
        Self::new(32)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`SpectralRolloff95`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct Sroll95Config {
    pub period: Param<usize>,
}

impl Indicator for SpectralRolloff95 {
    const ID: IndicatorId = IndicatorId::Sroll95;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sroll, &[IndicatorOutputId::Sroll])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Sroll95)];
    type Config = Sroll95Config;
    type Runtime = SpectralRolloff95;

    fn create(cfg: Sroll95Config) -> SpectralRolloff95 {
        SpectralRolloff95::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &Sroll95Config) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for Sroll95Config {
    fn defaults() -> Self {
        Sroll95Config { period: Param::Solo(32) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for SpectralRolloff95 {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sroll95, "Spectral Rolloff 95%", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_rolloff_95_creation() {
        let sr = SpectralRolloff95::new(64);
        assert!(!sr.is_ready());
        assert_eq!(sr.value(), 0.0);
    }

    #[test]
    fn test_spectral_rolloff_95_warmup() {
        let mut sr = SpectralRolloff95::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sr.feed(price);
        }
        assert!(sr.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_95_finite() {
        let mut sr = SpectralRolloff95::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sr.feed(price);
            assert!(value.is_finite(), "Rolloff95 should be finite");
        }
    }

    #[test]
    fn test_spectral_rolloff_95_reset() {
        let mut sr = SpectralRolloff95::new(64);
        for i in 0..70 {
            sr.feed(100.0 + i as f64);
        }
        sr.reset();
        assert!(!sr.is_ready());
        assert_eq!(sr.value(), 0.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds rolloff-95.
    #[test]
    fn factory_feeds_resolved_sroll95() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralRolloff95 as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sroll95(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
