// Williams Accumulation/Distribution (WAD)

#[derive(Debug, Clone)]
pub struct WilliamsAd {
    prev_close: f64,
    initialized: bool,
    value: f64,
}

impl Default for WilliamsAd {
    fn default() -> Self {
        Self::new()
    }
}

impl WilliamsAd {
    pub fn new() -> Self {
        Self {
            prev_close: 0.0,
            initialized: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev_close = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved input lanes `[high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.value;
        }
        let trh = h.max(self.prev_close);
        let trl = l.min(self.prev_close);
        let ad = if c > self.prev_close {
            c - trl
        } else if c < self.prev_close {
            c - trh
        } else {
            0.0
        };
        self.prev_close = c;
        self.value += ad;
        self.value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Unit config — WAD has no configurable parameters.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct WadConfig;

impl Indicator for WilliamsAd {
    const ID: IndicatorId = IndicatorId::Wad;
    /// No family — Williams A/D is a cumulative accumulation PRODUCER.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close (open/volume unused by the formula).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1) cumulative update — pure scalar running state.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 2)]);
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Wad)];
    type Config = WadConfig;
    type Runtime = WilliamsAd;

    fn create(_cfg: WadConfig) -> WilliamsAd {
        WilliamsAd::new()
    }
}

impl crate::contract::Config for WadConfig {
    fn defaults() -> Self {
        WadConfig
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for WilliamsAd {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Wad, "Williams A/D", Color::hex(0x9C27B0))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_williams_ad_creation() {
        let wad = WilliamsAd::new();
        assert!(!wad.is_ready());
        assert_eq!(wad.value(), 0.0);
    }

    #[test]
    fn test_williams_ad_warmup() {
        let mut wad = WilliamsAd::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wad.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(wad.is_ready());
    }

    #[test]
    fn test_williams_ad_values_finite() {
        let mut wad = WilliamsAd::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = wad.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_williams_ad_reset() {
        let mut wad = WilliamsAd::new();
        for i in 0..10 {
            wad.feed(&[105.0, 95.0, 100.0 + i as f64]);
        }
        wad.reset();
        assert!(!wad.is_ready());
        assert_eq!(wad.value(), 0.0);
    }

    /// Factory resolves H/L/C lanes; open+volume ignored (9999 as proof).
    /// Rising close → positive WAD.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Wad(<<WilliamsAd as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Uptrend: each close higher than last
        let closes = [100.0_f64, 101.0, 102.0, 103.0, 104.0, 105.0];
        for c in closes {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: c + 1.0, low: c - 1.0, close: c, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.read(IndicatorOutputId::Wad) > 0.0, "rising close should yield positive WAD");
    }
}
