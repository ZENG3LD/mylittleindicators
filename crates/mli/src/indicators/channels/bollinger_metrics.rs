use crate::indicators::channels::bollinger_bands::BollingerBands;

/// Lightweight metrics over Bollinger Bands: %B and Bandwidth
#[derive(Debug, Clone)]
pub struct BollingerMetrics {
    bb: BollingerBands,
    percent_b: f64,
    bandwidth: f64,
}

impl Default for BollingerMetrics {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

impl BollingerMetrics {
    pub fn new(period: usize, k: f64) -> Self {
        use crate::engine::contract_engine::SmootherId;
        Self {
            bb: BollingerBands::from_smoother(SmootherId::Sma, period.max(1), k),
            percent_b: 0.5,
            bandwidth: 0.0,
        }
    }

    /// Feed one close price scalar; returns (%B, bandwidth).
    pub fn feed(&mut self, c: f64) -> (f64, f64) {
        let (upper, middle, lower) = self.bb.feed(c);
        let width = (upper - lower).max(0.0);
        self.percent_b = if width > 0.0 {
            (c - lower) / width
        } else {
            0.5
        };
        self.bandwidth = if middle.abs() > 1e-12 {
            width / middle.abs()
        } else {
            0.0
        };
        (self.percent_b, self.bandwidth)
    }

    /// Update with OHLCV; returns (%B, bandwidth) — legacy bridge.

    pub fn percent_b(&self) -> f64 {
        self.percent_b
    }
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }


    #[inline]
    pub fn is_ready(&self) -> bool {
        self.bb.is_ready()
    }

    pub fn reset(&mut self) {
        self.bb.reset();
        self.percent_b = 0.5;
        self.bandwidth = 0.0;
    }

}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`BollingerMetrics`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BollingerMetricsConfig {
    pub period: Param<usize>,
    pub k: Param<f64>,
}

impl Indicator for BollingerMetrics {
    const ID: IndicatorId = IndicatorId::Bbmetrics;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close is the sole price source; fed as scalar to the inner BB.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Bb, &[
            IndicatorOutputId::BbUpper,
            IndicatorOutputId::BbMiddle,
            IndicatorOutputId::BbLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::BbmetricsPercentB),
        Output::magnitude(IndicatorOutputId::BbmetricsBandwidth),
    ];
    type Config = BollingerMetricsConfig;
    type Runtime = BollingerMetrics;

    fn create(cfg: BollingerMetricsConfig) -> BollingerMetrics {
        BollingerMetrics::new(cfg.period.resolved(), cfg.k.resolved())
    }

    fn source_fields(cfg: &BollingerMetricsConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for BollingerMetricsConfig {
    fn defaults() -> Self {
        BollingerMetricsConfig {
            period: Param::Solo(14),
            k: Param::Solo(2.0),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for BollingerMetrics {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BbmetricsPercentB, "%B", Color::hex(0x9C27B0))
            .line_output(IndicatorOutputId::BbmetricsBandwidth, "Bandwidth", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_bbmetrics() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<BollingerMetrics as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Bbmetrics(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,  // SOURCE = Close
                volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "bbmetrics %B should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bollinger_metrics_creation() {
        let bm = BollingerMetrics::new(20, 2.0);
        assert!(!bm.is_ready());
        assert_eq!(bm.percent_b(), 0.5);
        assert_eq!(bm.bandwidth(), 0.0);
    }

    #[test]
    fn test_bollinger_metrics_warmup() {
        let mut bm = BollingerMetrics::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bm.feed(price);
        }
        assert!(bm.is_ready());
    }

    #[test]
    fn test_bollinger_metrics_values() {
        let mut bm = BollingerMetrics::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (pct_b, bw) = bm.feed(price);
            assert!(pct_b.is_finite(), "%B should be finite");
            assert!(bw >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_bollinger_metrics_reset() {
        let mut bm = BollingerMetrics::new(20, 2.0);
        for i in 0..25 {
            bm.feed(100.0 + i as f64);
        }
        bm.reset();
        assert!(!bm.is_ready());
        assert_eq!(bm.percent_b(), 0.5);
    }
}
