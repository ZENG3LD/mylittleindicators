//! Volume Price Trend (VPT) indicator.
//! VPT = Previous VPT + Volume × (Close - Previous Close) / Previous Close
//! Cumulative volume-price momentum.

use crate::engine::contract_engine::SmootherSlot;

/// Volume Price Trend indicator.
#[derive(Debug, Clone)]
pub struct VolumePriceTrend {
    signal_period: usize,
    signal_ma: SmootherSlot,
    prev_close: f64,
    vpt_value: f64,
    signal_value: f64,
    bars_count: usize,
    is_ready: bool,
}

impl VolumePriceTrend {
    pub fn new() -> Self {
        Self::with_signal_period(21)
    }

    pub fn with_signal_period(signal_period: usize) -> Self {
        use crate::engine::contract_engine::SmootherId;
        assert!(signal_period > 0, "Signal period must be greater than 0");
        Self {
            signal_period,
            signal_ma: SmootherSlot::new(SmootherId::Sma, signal_period),
            prev_close: 0.0,
            vpt_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// lanes = [close, volume]
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let close  = lanes[0];
        let volume = lanes[1];
        self.bars_count += 1;

        if self.bars_count == 1 {
            self.prev_close = close;
            self.vpt_value = 0.0;
            return self.vpt_value;
        }

        let price_change_pct = if self.prev_close.abs() > 1e-12 {
            (close - self.prev_close) / self.prev_close
        } else {
            0.0
        };

        self.vpt_value += volume * price_change_pct;
        self.signal_value = self.signal_ma.feed(self.vpt_value);
        self.prev_close = close;

        if self.bars_count >= self.signal_period + 5 {
            self.is_ready = true;
        }

        self.vpt_value
    }

    pub fn value(&self) -> f64 {
        self.vpt_value
    }

    pub fn signal_value(&self) -> f64 { self.signal_value }
    pub fn histogram(&self) -> f64 { self.vpt_value - self.signal_value }
    pub fn is_ready(&self) -> bool { self.is_ready }
    pub fn signal_period(&self) -> usize { self.signal_period }

    pub fn reset(&mut self) {
        self.signal_ma.reset();
        self.prev_close = 0.0;
        self.vpt_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
}

impl Default for VolumePriceTrend {
    fn default() -> Self {
        Self::with_signal_period(21)
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Config for [`VolumePriceTrend`] — signal line period.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VptConfig {
    pub signal_period: Param<usize>,
}

impl Indicator for VolumePriceTrend {
    const ID: IndicatorId = IndicatorId::Vpt;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed: Close then Volume.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Vpt)];
    type Config = VptConfig;
    type Runtime = VolumePriceTrend;

    fn create(cfg: VptConfig) -> VolumePriceTrend {
        VolumePriceTrend::with_signal_period(cfg.signal_period.resolved().max(1))
    }
}

impl crate::contract::Config for VptConfig {
    fn defaults() -> Self {
        VptConfig { signal_period: Param::Solo(21) }
    }
    fn machine_defaults() -> Self {
        // signal_period: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumePriceTrend {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vpt, "VPT", Color::hex(0x2196F3))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_vpt_creation() {
        let vpt = VolumePriceTrend::new();
        assert!(!vpt.is_ready());
        assert_eq!(vpt.value(), 0.0);
    }

    #[test]
    fn test_vpt_with_signal_period() {
        let vpt = VolumePriceTrend::with_signal_period(10);
        assert!(!vpt.is_ready());
        assert_eq!(vpt.signal_period(), 10);
    }

    #[test]
    fn test_vpt_warmup() {
        let mut vpt = VolumePriceTrend::with_signal_period(10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vpt.feed(&[price, 1000.0]);
        }
        assert!(vpt.is_ready());
    }

    #[test]
    fn test_vpt_values_finite() {
        let mut vpt = VolumePriceTrend::new();
        for i in 0..40 {
            let price = 100.0 + i as f64;
            let value = vpt.feed(&[price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vpt_reset() {
        let mut vpt = VolumePriceTrend::new();
        for i in 0..30 {
            vpt.feed(&[100.0 + i as f64, 1000.0]);
        }
        vpt.reset();
        assert!(!vpt.is_ready());
        assert_eq!(vpt.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vpt() {
        let mut f = IndicatorOrder::Vpt(<<VolumePriceTrend as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.15).sin() * 8.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Vpt).is_finite());
    }
}
