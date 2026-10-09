// HL Value Area Proxy: rolling middle and bandwidth percentile from High/Low

#[derive(Clone, Debug)]
pub struct HlValueArea {
    window: usize,
    mids: Vec<f64>,
    bands: Vec<f64>,
    idx: usize,
    filled: bool,
    mid: f64,
    band: f64,
    band_percentile: f64,
}

impl HlValueArea {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            mids: vec![0.0; window.max(1)],
            bands: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            mid: 0.0,
            band: 0.0,
            band_percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.mids.fill(0.0);
        self.bands.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.mid = 0.0;
        self.band = 0.0;
        self.band_percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    fn recompute(&mut self, high: f64, low: f64) -> (f64, f64, f64) {
        let mid_now = 0.5 * (high + low);
        let band_now = (high - low).max(0.0);
        self.mid = mid_now;
        self.band = band_now;
        self.mids[self.idx] = mid_now;
        self.bands[self.idx] = band_now;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        // percentile of band within window
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.bands[i] <= band_now {
                    count_le += 1;
                }
            }
            self.band_percentile = count_le as f64 / len as f64;
        } else {
            self.band_percentile = 0.0;
        }
        (self.mid, self.band, self.band_percentile)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.mid
    }

}

impl Default for HlValueArea {
    /// Factory default: `new(50)` — window = `unwrap_or(50)`.
    fn default() -> Self {
        Self::new(50)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, SourceAxis,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`HlValueArea`] — window period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HlvaConfig {
    pub period: Param<usize>,
}

impl Indicator for HlValueArea {
    const ID: IndicatorId = IndicatorId::Hlva;
    /// No family — HL mid-price rolling tracker, a structural level producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L fields — mid = (H + L) / 2.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(window) per bar — band percentile rescans window on every update. Two period-deep Vecs.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Hlva)];
    type Config = HlvaConfig;
    type Runtime = HlValueArea;

    fn create(cfg: HlvaConfig) -> HlValueArea {
        HlValueArea::new(cfg.period.resolved())
    }
}

impl HlValueArea {
    /// Feed resolved `[high, low]` lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let (mid, _band, _pct) = self.recompute(high, low);
        mid
    }
}

impl crate::contract::Config for HlvaConfig {
    fn defaults() -> Self {
        HlvaConfig { period: Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for HlValueArea {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::Hlva,
                "HL Vol Avg",
                Color::hex(0x2196F3),
                1.0,
            ))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_hl() {
        let mut f = IndicatorOrder::Hlva(<<HlValueArea as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=55 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 2.0,
                low: base - 2.0,
                close: 9999.0,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v > 100.0 && v < 160.0, "hlva out of range: {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hl_value_area_creation() {
        let hlva = HlValueArea::new(20);
        assert!(!hlva.is_ready());
        assert_eq!(hlva.value(), 0.0);
    }

    #[test]
    fn test_hl_value_area_warmup() {
        let mut hlva = HlValueArea::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            hlva.recompute(price + 1.0, price - 1.0);
        }
        assert!(hlva.is_ready());
    }

    #[test]
    fn test_hl_value_area_values() {
        let mut hlva = HlValueArea::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (mid, band, pct) = hlva.recompute(price + 2.0, price - 2.0);
            assert!(mid > 0.0, "Mid should be positive");
            assert!(band >= 0.0, "Band should be non-negative");
            assert!(pct >= 0.0 && pct <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_hl_value_area_reset() {
        let mut hlva = HlValueArea::new(20);
        for _i in 0..25 {
            hlva.recompute(101.0, 99.0);
        }
        hlva.reset();
        assert!(!hlva.is_ready());
        assert_eq!(hlva.value(), 0.0);
    }
}
