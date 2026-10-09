// PercentB - wrapper over BollingerBands percent_b metric

use crate::indicators::channels::bollinger_bands::BollingerBands;

#[derive(Debug, Clone)]
pub struct PercentB {
    bb: BollingerBands,
    value: f64,
}

impl Default for PercentB {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

impl PercentB {
    pub fn new(period: usize, std_mult: f64) -> Self {
        use crate::engine::contract_engine::SmootherId;
        Self {
            bb: BollingerBands::from_smoother(SmootherId::Sma, period.max(2), std_mult.max(0.1)),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.bb.reset();
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.bb.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed one close price scalar.
    pub fn feed(&mut self, c: f64) -> f64 {
        let _ = self.bb.feed(c);
        self.value = self.bb.percent_b();
        self.value
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`PercentB`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PercentBConfig {
    pub period: Param<usize>,
    pub std_mult: Param<f64>,
}

impl Indicator for PercentB {
    const ID: IndicatorId = IndicatorId::Percentb;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close is the sole price source.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Bb, &[
            IndicatorOutputId::BbPercentB,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Percentb)];
    type Config = PercentBConfig;
    type Runtime = PercentB;

    fn create(cfg: PercentBConfig) -> PercentB {
        PercentB::new(cfg.period.resolved(), cfg.std_mult.resolved())
    }

    fn source_fields(cfg: &PercentBConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for PercentBConfig {
    fn defaults() -> Self {
        PercentBConfig {
            period: Param::Solo(14),
            std_mult: Param::Solo(2.0),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.std_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for PercentB {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Percentb, "%B", Color::hex(0x9C27B0))
            .bounds(0.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_percentb() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<PercentB as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Percentb(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,  // SOURCE = Close
                volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "percentb should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_b_creation() {
        let pb = PercentB::new(20, 2.0);
        assert!(!pb.is_ready());
        assert_eq!(pb.value(), 0.5);
    }

    #[test]
    fn test_percent_b_warmup() {
        let mut pb = PercentB::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pb.feed(price);
        }
        assert!(pb.is_ready());
    }

    #[test]
    fn test_percent_b_values() {
        let mut pb = PercentB::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pb.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_percent_b_reset() {
        let mut pb = PercentB::new(20, 2.0);
        for i in 0..25 {
            pb.feed(100.0 + i as f64);
        }
        pb.reset();
        assert!(!pb.is_ready());
        assert_eq!(pb.value(), 0.5);
    }
}
