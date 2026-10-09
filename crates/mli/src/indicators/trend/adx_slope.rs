// ADX slope and persistence score

use crate::engine::contract_engine::SmootherId;
use crate::indicators::trend::adx::Adx;

#[derive(Debug, Clone)]
pub struct AdxSlope {
    adx: Adx,
    prev: f64,
    slope: f64,
}

impl AdxSlope {
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Build with a specific ATR smoothing type threaded into the inner Adx.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            adx: Adx::from_smoother(period.max(2), smoother),
            prev: 0.0,
            slope: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.adx.reset();
        self.prev = 0.0;
        self.slope = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.adx.is_ready()
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.slope
    }

    /// Feed resolved input lanes `[high, low, close]` — the pure core computation.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let val = self.adx.feed(lanes);
        if self.adx.is_ready() {
            self.slope = val - self.prev;
            self.prev = val;
        }
        self.slope
    }
}

impl Default for AdxSlope {
    fn default() -> Self {
        Self::from_smoother(14, SmootherId::Rma)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

impl Indicator for AdxSlope {
    const ID: IndicatorId = IndicatorId::AdxSlope;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::fixed(StoreKind::Scalar, 2)],
        inner: &[Port::new(IndicatorId::Adx, &[IndicatorOutputId::Adx])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::AdxSlope)];
    type Config = AdxSlopeConfig;
    type Runtime = AdxSlope;

    fn create(cfg: AdxSlopeConfig) -> AdxSlope {
        AdxSlope::from_smoother(cfg.period.resolved(), SmootherId::Rma)
    }
}

/// Typed config for [`AdxSlope`] — its own period axis (not shared with Adx).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AdxSlopeConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for AdxSlopeConfig {
    fn defaults() -> Self {
        AdxSlopeConfig { period: Param::Solo(14) }
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


impl Render for AdxSlope {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AdxSlope, "ADX Slope", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adx_slope_creation() {
        let adx = AdxSlope::new(14);
        assert!(!adx.is_ready());
        assert_eq!(adx.value(), 0.0);
    }

    #[test]
    fn test_adx_slope_warmup() {
        let mut adx = AdxSlope::new(14);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            adx.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(adx.is_ready());
    }

    #[test]
    fn test_adx_slope_values_finite() {
        let mut adx = AdxSlope::new(14);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = adx.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_adx_slope_reset() {
        let mut adx = AdxSlope::new(14);
        for i in 0..50 {
            adx.feed(&[100.0 + i as f64 + 5.0, 95.0 + i as f64, 101.0 + i as f64]);
        }
        adx.reset();
        assert!(!adx.is_ready());
        assert_eq!(adx.value(), 0.0);
    }

    #[test]
    fn test_adx_slope_ema_warmup_finite() {
        let mut adx = AdxSlope::from_smoother(14, SmootherId::Ema);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 4.0;
            let v = adx.feed(&[price + 1.5, price - 1.5, price]);
            assert!(v.is_finite());
        }
        assert!(adx.is_ready());
    }

    #[test]
    fn test_factory_feeds_resolved_adx_slope() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::AdxSlope(<<AdxSlope as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
