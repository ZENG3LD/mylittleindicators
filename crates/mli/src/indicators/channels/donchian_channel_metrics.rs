use crate::indicators::channels::donchian_channel::DonchianChannel;

/// Lightweight metrics over Donchian Channel: width and price position in channel
#[derive(Debug, Clone)]
pub struct DonchianMetrics {
    dc: DonchianChannel,
    width: f64,
    position: f64,
}

impl Default for DonchianMetrics {
    fn default() -> Self {
        Self::new(14)
    }
}

impl DonchianMetrics {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period),
            width: 0.0,
            position: 0.5,
        }
    }
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn position(&self) -> f64 {
        self.position
    }


    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dc.is_ready()
    }

    pub fn reset(&mut self) {
        self.dc.reset();
        self.width = 0.0;
        self.position = 0.5;
    }

    /// Feed resolved [High, Low, Close] lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let (upper, lower, _mid) = self.dc.feed(&[high, low]);
        self.width = upper - lower;
        self.position = if self.width > 0.0 {
            (close - lower) / self.width
        } else {
            0.5
        };
        (self.width, self.position)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`DonchianMetrics`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DonchianMetricsConfig {
    pub period: Param<usize>,
}

impl Indicator for DonchianMetrics {
    const ID: IndicatorId = IndicatorId::Dcmetrics;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Dc, &[
            IndicatorOutputId::DcUpper,
            IndicatorOutputId::DcMiddle,
            IndicatorOutputId::DcLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::DcmetricsWidth),
        Output::percent(IndicatorOutputId::DcmetricsPosition),
    ];
    type Config = DonchianMetricsConfig;
    type Runtime = DonchianMetrics;

    fn create(cfg: DonchianMetricsConfig) -> DonchianMetrics {
        DonchianMetrics::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DonchianMetricsConfig {
    fn defaults() -> Self {
        DonchianMetricsConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1) — only axis
    }
}


impl Render for DonchianMetrics {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DcmetricsWidth, "DC Width", Color::hex(0x9C27B0))
            .line_output(IndicatorOutputId::DcmetricsPosition, "DC Position", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_dcmetrics() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DonchianMetrics as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Dcmetrics(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "dcmetrics main should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_metrics_creation() {
        let dm = DonchianMetrics::new(20);
        assert!(!dm.is_ready());
        assert_eq!(dm.width(), 0.0);
        assert_eq!(dm.position(), 0.5);
    }

    #[test]
    fn test_donchian_metrics_warmup() {
        let mut dm = DonchianMetrics::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dm.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(dm.is_ready());
    }

    #[test]
    fn test_donchian_metrics_values() {
        let mut dm = DonchianMetrics::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (width, position) = dm.feed(&[price + 1.0, price - 1.0, price]);
            assert!(width >= 0.0, "Width should be non-negative");
            assert!(position.is_finite(), "Position should be finite");
        }
    }

    #[test]
    fn test_donchian_metrics_reset() {
        let mut dm = DonchianMetrics::new(20);
        for i in 0..25 {
            dm.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        dm.reset();
        assert!(!dm.is_ready());
        assert_eq!(dm.position(), 0.5);
    }
}
