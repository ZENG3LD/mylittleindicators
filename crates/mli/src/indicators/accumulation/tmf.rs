use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Twiggs Money Flow (TMF) = EMA( (Close - Low) - (High - Close) / (High - Low), n ) * Volume smoothed / Volume smoothed
#[derive(Debug, Clone)]
pub struct Tmf {
    cmf_num_ma: SmootherSlot,
    vol_ma: SmootherSlot,
    value: f64,
}

impl Default for Tmf {
    /// Factory default: period = 21.
    fn default() -> Self {
        Self::new(21)
    }
}

impl Tmf {
    pub fn new(period: usize) -> Self {
        Self {
            cmf_num_ma: SmootherSlot::new(SmootherId::Ema, period.max(1)),
            vol_ma: SmootherSlot::new(SmootherId::Ema, period.max(1)),
            value: 0.0,
        }
    }
    /// Feed resolved input lanes `[high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let hl = (h - l).abs().max(1e-12);
        let mf = ((c - l) - (h - c)) / hl;
        let num = self.cmf_num_ma.feed(mf * v);
        let den = self.vol_ma.feed(v);
        self.value = if den.abs() < 1e-12 { 0.0 } else { num / den };
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.cmf_num_ma.is_ready() && self.vol_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.cmf_num_ma.reset();
        self.vol_ma.reset();
        self.value = 0.0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`Tmf`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TmfConfig {
    pub period: Param<usize>,
}

impl Indicator for Tmf {
    const ID: IndicatorId = IndicatorId::Tmf;
    /// No family — Twiggs Money Flow is a money-flow PRODUCER (EMA-smoothed variant of CMF).
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close, volume.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// Two EMA SmootherSlots — each an O(1) scalar state, no window store.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Tmf)];
    type Config = TmfConfig;
    type Runtime = Tmf;

    fn create(cfg: TmfConfig) -> Tmf {
        Tmf::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for TmfConfig {
    fn defaults() -> Self {
        TmfConfig { period: Param::Solo(21) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (EMA window for cmf_num_ma and vol_ma) → auto range(2,4048,1). No other axes.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Tmf {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Tmf, "Twiggs MF", Color::hex(0x9C27B0))
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
    fn test_tmf_creation() {
        let tmf = Tmf::new(21);
        assert!(!tmf.is_ready());
        assert_eq!(tmf.value(), 0.0);
    }

    #[test]
    fn test_tmf_warmup() {
        let mut tmf = Tmf::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tmf.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(tmf.is_ready());
    }

    #[test]
    fn test_tmf_values_finite() {
        let mut tmf = Tmf::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = tmf.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_tmf_values_range() {
        let mut tmf = Tmf::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = tmf.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value >= -1.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_tmf_reset() {
        let mut tmf = Tmf::new(14);
        for i in 0..20 {
            tmf.feed(&[105.0, 95.0, 100.0 + i as f64, 1000.0]);
        }
        tmf.reset();
        assert!(!tmf.is_ready());
        assert_eq!(tmf.value(), 0.0);
    }

    /// Factory resolves H/L/C/Volume; open ignored (9999 as proof).
    /// Close at midpoint → MFM = 0 → TMF = 0.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Tmf(<<Tmf as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..30 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 110.0, low: 90.0, close: 100.0, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert_eq!(f.read(IndicatorOutputId::Tmf), 0.0);
    }
}
