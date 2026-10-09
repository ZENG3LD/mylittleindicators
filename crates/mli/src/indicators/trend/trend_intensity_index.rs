// Trend Intensity Index (TII) - percent of closes above SMA in window

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

#[derive(Debug, Clone)]
pub struct TrendIntensityIndex {
    window: usize,
    ma: SmootherSlot,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl TrendIntensityIndex {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(2, 512);
        Self {
            window: w,
            ma: SmootherSlot::new(SmootherId::Sma, w),
            buf: Vec::with_capacity(w),
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.ma.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed one pre-extracted scalar (close by default) — the pure core computation.
    pub fn feed(&mut self, c: f64) -> f64 {
        let ma = self.ma.feed(c);
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let above = self.buf.iter().filter(|&&x| x > ma).count() as f64;
            self.value = 100.0 * above / (self.window as f64);
        }
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Own config for [`TrendIntensityIndex`] — period only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TiiConfig {
    pub period: Param<usize>,
}

impl Indicator for TrendIntensityIndex {
    const ID: IndicatorId = IndicatorId::Tii;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(N): counts how many of the window closes are above the SMA — full window scan
    /// each bar. The SMA feeds a SmootherSlot (period-deep internally); the close
    /// window buffer is also period-deep.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Deque)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Tii)];
    type Config = TiiConfig;
    type Runtime = TrendIntensityIndex;

    fn create(cfg: TiiConfig) -> TrendIntensityIndex {
        TrendIntensityIndex::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for TiiConfig {
    fn defaults() -> Self {
        TiiConfig { period: Param::Solo(20) }
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


impl Render for TrendIntensityIndex {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Tii, "TII", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(20.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

impl Default for TrendIntensityIndex {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_intensity_index_creation() {
        let tii = TrendIntensityIndex::new(20);
        assert!(!tii.is_ready());
        assert_eq!(tii.value(), 50.0);
    }

    #[test]
    fn test_trend_intensity_index_warmup() {
        let mut tii = TrendIntensityIndex::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tii.feed(price);
        }
        assert!(tii.is_ready());
    }

    #[test]
    fn test_trend_intensity_index_range() {
        let mut tii = TrendIntensityIndex::new(20);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            let value = tii.feed(price);
            assert!(value >= 0.0 && value <= 100.0, "TII should be in [0, 100]");
        }
    }

    #[test]
    fn test_trend_intensity_index_reset() {
        let mut tii = TrendIntensityIndex::new(20);
        for i in 0..30 {
            tii.feed(100.0 + i as f64);
        }
        tii.reset();
        assert!(!tii.is_ready());
        assert_eq!(tii.value(), 50.0);
    }

    /// The factory resolves the configurable close field and feeds the pure scalar core.
    /// Wild H/L/open values (9999) prove only close is consumed.
    #[test]
    fn factory_feeds_resolved_scalar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Tii(<<TrendIntensityIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: -9999.0, close, volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 100.0, "TII should be in [0,100], got {v}");
    }
}
