use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Intraday Intensity (II) and IIP21 variant
#[derive(Debug, Clone)]
pub struct IntradayIntensity {
    ii_ma: SmootherSlot,
    vol_ma: SmootherSlot,
    value: f64,
}

impl Default for IntradayIntensity {
    /// Factory default: period = 14.
    fn default() -> Self {
        Self::new(14)
    }
}

impl IntradayIntensity {
    pub fn new(period: usize) -> Self {
        Self {
            ii_ma: SmootherSlot::new(SmootherId::Sma, period.max(1)),
            vol_ma: SmootherSlot::new(SmootherId::Sma, period.max(1)),
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
        let ii = ((2.0 * c - h - l) / hl) * v;
        let num = self.ii_ma.feed(ii);
        let den = self.vol_ma.feed(v);
        self.value = if den.abs() < 1e-12 {
            0.0
        } else {
            100.0 * num / den
        };
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.ii_ma.is_ready() && self.vol_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.ii_ma.reset();
        self.vol_ma.reset();
        self.value = 0.0;
    }

}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`IntradayIntensity`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IiConfig {
    pub period: Param<usize>,
}

impl Indicator for IntradayIntensity {
    const ID: IndicatorId = IndicatorId::Ii;
    /// No family — II is a volume-weighted money-flow PRODUCER.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close, volume.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// Two SMA slots (ii_ma, vol_ma) — each a period-deep Vec internally.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Ii)];
    type Config = IiConfig;
    type Runtime = IntradayIntensity;

    fn create(cfg: IiConfig) -> IntradayIntensity {
        IntradayIntensity::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for IiConfig {
    fn defaults() -> Self {
        IiConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (SMA window for ii_ma and vol_ma) → auto range(2,4048,1). No other axes.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IntradayIntensity {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ii, "Intraday Intensity", Color::hex(0xFF9800))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_intensity_creation() {
        let ii = IntradayIntensity::new(21);
        assert!(!ii.is_ready());
        assert_eq!(ii.value(), 0.0);
    }

    #[test]
    fn test_intraday_intensity_warmup() {
        let mut ii = IntradayIntensity::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ii.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(ii.is_ready());
    }

    #[test]
    fn test_intraday_intensity_values_finite() {
        let mut ii = IntradayIntensity::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ii.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_intraday_intensity_reset() {
        let mut ii = IntradayIntensity::new(14);
        for i in 0..20 {
            ii.feed(&[105.0, 95.0, 100.0 + i as f64, 1000.0]);
        }
        ii.reset();
        assert!(!ii.is_ready());
        assert_eq!(ii.value(), 0.0);
    }

    /// Factory resolves H/L/C/Volume; open ignored (9999 as proof).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Ii(<<IntradayIntensity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..20 {
            // close at midpoint → II numerator = 0 → value = 0
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 105.0, low: 95.0, close: 100.0, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert_eq!(f.read(IndicatorOutputId::Ii), 0.0);
    }
}
