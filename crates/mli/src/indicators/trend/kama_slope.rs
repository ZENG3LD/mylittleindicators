// KAMA Slope: slope of Kaufman Adaptive Moving Average

use crate::indicators::average::kaufman_adaptive_ma::KaufmanAdaptiveMA;

#[derive(Debug, Clone)]
pub struct KamaSlope {
    kama: KaufmanAdaptiveMA,
    prev: f64,
    value: f64,
}

impl KamaSlope {
    pub fn new(period: usize) -> Self {
        Self {
            kama: KaufmanAdaptiveMA::new(period.max(2), 2, 30),
            prev: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kama.reset();
        self.prev = 0.0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kama.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed ONE resolved scalar (close). Slope of the inner KAMA over the close series.
    pub fn feed(&mut self, c: f64) -> f64 {
        let k = self.kama.feed(c);
        self.value = k - self.prev;
        self.prev = k;
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Port, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Own config for [`KamaSlope`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KamaSlopeConfig {
    pub period: Param<usize>,
}

impl Indicator for KamaSlope {
    const ID: IndicatorId = IndicatorId::KamaSlope;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Slope of KAMA(close) -- reads close intrinsically, not a swept field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// A NODE: its own state is two scalars (prev/value), the slope is O(1). Its
    /// real cost is the inner EDGE -- it consumes the `value` port of `Kama`. The
    /// barometer charges this node's base + `Kama`'s base + the value-port marginal
    /// (FREE) through the edge, instead of the dependency hiding in a held struct +
    /// undeclared method calls. This is the internal contract replacing the
    /// reach-around.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Kama, &[IndicatorOutputId::KamaLine])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::KamaSlope)];
    type Config = KamaSlopeConfig;
    type Runtime = KamaSlope;

    fn create(cfg: KamaSlopeConfig) -> KamaSlope {
        KamaSlope::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for KamaSlopeConfig {
    /// Slope period 10 (factory default), floored at 2 (the KAMA ER minimum).
    fn defaults() -> Self {
        KamaSlopeConfig { period: Param::Solo(10) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for KamaSlope {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::KamaSlope, "KAMA Slope", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kama_slope_creation() {
        let kama = KamaSlope::new(10);
        assert!(!kama.is_ready());
        assert_eq!(kama.value(), 0.0);
    }

    #[test]
    fn test_kama_slope_warmup() {
        let mut kama = KamaSlope::new(10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kama.feed(price);
        }
        assert!(kama.is_ready());
    }

    #[test]
    fn test_kama_slope_values_finite() {
        let mut kama = KamaSlope::new(10);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = kama.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kama_slope_reset() {
        let mut kama = KamaSlope::new(10);
        for _i in 0..20 {
            kama.feed(101.0);
        }
        kama.reset();
        assert!(!kama.is_ready());
        assert_eq!(kama.value(), 0.0);
    }
}
