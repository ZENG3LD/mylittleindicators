// Volume Flow Indicator (VFI) - improved money flow proxy

#[derive(Debug, Clone)]
pub struct Vfi {
    window: usize,
    value: f64,
    sum_flow: f64,
    sum_vol: f64,
}

impl Vfi {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            value: 0.0,
            sum_flow: 0.0,
            sum_vol: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.sum_flow = 0.0;
        self.sum_vol = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved lanes `[high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let tp = (h + l + c) / 3.0;
        let flow = tp * v;
        self.sum_flow += flow - self.sum_flow / self.window as f64;
        self.sum_vol += v - self.sum_vol / self.window as f64;
        self.value = if self.sum_vol.abs() > 1e-12 {
            self.sum_flow / self.sum_vol
        } else {
            0.0
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfi_creation() {
        let vfi = Vfi::new(20);
        assert!(vfi.is_ready()); // Always ready
        assert_eq!(vfi.value(), 0.0);
    }

    #[test]
    fn test_vfi_update() {
        let mut vfi = Vfi::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let value = vfi.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vfi_values_finite() {
        let mut vfi = Vfi::new(20);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = vfi.feed(&[price + 2.0, price - 2.0, price, 500.0 + i as f64 * 10.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vfi_reset() {
        let mut vfi = Vfi::new(20);
        for _i in 0..30 {
            vfi.feed(&[105.0, 95.0, 101.0, 1000.0]);
        }
        vfi.reset();
        assert_eq!(vfi.value(), 0.0);
    }
}

impl Default for Vfi {
    fn default() -> Self {
        Self::new(130)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`Vfi`] — window (EMA-style decay) size for volume-flow smoothing.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VfiConfig {
    pub window: Param<usize>,
}

impl Indicator for Vfi {
    const ID: IndicatorId = IndicatorId::Vfi;
    /// Standalone volume-flow indicator — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C + volume — typical-price × volume flow calculation.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(1) per bar — running EMA-style sums, no window buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Vfi)];
    type Config = VfiConfig;
    type Runtime = Vfi;

    fn create(cfg: VfiConfig) -> Vfi {
        Vfi::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for VfiConfig {
    fn defaults() -> Self {
        VfiConfig { window: Param::Solo(130) }
    }
    fn machine_defaults() -> Self {
        // window: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Vfi {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vfi, "VFI", Color::hex(0x00BCD4))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Vfi(<<Vfi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Positive-flow bar: high volume at higher prices
        f.feed(0, MarketSample::Bar { open: 9999.0, high: 102.0, low: 98.0, close: 101.0, volume: 1000.0 });
        let v = f.read(IndicatorOutputId::Vfi);
        assert!(v.is_finite(), "VFI must be finite, got {v}");
        assert!(v > 0.0, "positive-flow bar should give VFI > 0, got {v}");
    }
}
