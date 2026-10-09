// Intraday Intensity Ratio (IIR)

#[derive(Debug, Clone)]
pub struct IntradayIntensityRatio {
    window: usize,
    sum_iip: f64,
    count: usize,
    value: f64,
}

impl Default for IntradayIntensityRatio {
    /// Factory default: window = 14.
    fn default() -> Self {
        Self::new(14)
    }
}

impl IntradayIntensityRatio {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            sum_iip: 0.0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.sum_iip = 0.0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved input lanes `[high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let denom = (h - l).abs().max(1e-9);
        let iip = ((2.0 * c - h - l) / denom) * v;
        let alpha = 1.0 / (self.window as f64);
        self.sum_iip = (1.0 - alpha) * self.sum_iip + alpha * iip;
        self.count += 1;
        self.value = self.sum_iip.clamp(-1e9, 1e9);
        self.value
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
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`IntradayIntensityRatio`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IirConfig {
    pub period: Param<usize>,
}

impl Indicator for IntradayIntensityRatio {
    const ID: IndicatorId = IndicatorId::Iir;
    /// No family — exponential-decay II accumulator PRODUCER.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close, volume.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(1) pure scalar decay — no store.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 2)]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Iir)];
    type Config = IirConfig;
    type Runtime = IntradayIntensityRatio;

    fn create(cfg: IirConfig) -> IntradayIntensityRatio {
        IntradayIntensityRatio::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for IirConfig {
    fn defaults() -> Self {
        IirConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (EMA decay window) → auto range(2,4048,1). No other axes.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IntradayIntensityRatio {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Iir, "II Ratio", Color::hex(0x009688))
            .reference_line(ReferenceLine::new(1.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_intensity_ratio_creation() {
        let iir = IntradayIntensityRatio::new(21);
        assert!(!iir.is_ready());
        assert_eq!(iir.value(), 0.0);
    }

    #[test]
    fn test_intraday_intensity_ratio_warmup() {
        let mut iir = IntradayIntensityRatio::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            iir.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(iir.is_ready());
    }

    #[test]
    fn test_intraday_intensity_ratio_values_finite() {
        let mut iir = IntradayIntensityRatio::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = iir.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_intraday_intensity_ratio_reset() {
        let mut iir = IntradayIntensityRatio::new(14);
        for i in 0..20 {
            iir.feed(&[105.0, 95.0, 100.0 + i as f64, 1000.0]);
        }
        iir.reset();
        assert!(!iir.is_ready());
        assert_eq!(iir.value(), 0.0);
    }

    /// Factory resolves H/L/C/Volume; open ignored (9999 as proof).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Iir(<<IntradayIntensityRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..20 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 105.0, low: 95.0, close: 102.0, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.read(IndicatorOutputId::Iir).is_finite());
    }
}
