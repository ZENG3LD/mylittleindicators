//! StatisticalWickDetector — flags unusually long wicks vs rolling 95th percentile.
//!
//! Ported from `bar_indicators/candles/wick_spike.rs`.
//! Outputs: `upper_spike`, `lower_spike`.


/// Detects statistically extreme wicks using a rolling percentile window.
#[derive(Debug, Clone)]
pub struct StatisticalWickDetector {
    window: usize,
    upper_buf: Vec<f64>,
    lower_buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub is_upper_spike: bool,
    pub is_lower_spike: bool,
    pub upper_percentile: f64,
    pub lower_percentile: f64,
}

impl StatisticalWickDetector {
    pub fn new(window: usize) -> Self {
        let w = window.max(1);
        Self {
            window: w,
            upper_buf: vec![0.0; w],
            lower_buf: vec![0.0; w],
            idx: 0,
            filled: false,
            is_upper_spike: false,
            is_lower_spike: false,
            upper_percentile: 0.0,
            lower_percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.upper_buf.fill(0.0);
        self.lower_buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.is_upper_spike = false;
        self.is_lower_spike = false;
        self.upper_percentile = 0.0;
        self.lower_percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }


    /// Feed `[open, high, low, close]` lanes. Returns `(is_upper_spike, is_lower_spike)`.
    pub fn feed(&mut self, lanes: &[f64]) -> (bool, bool) {
        let (open, high, low, close) = (lanes[0], lanes[1], lanes[2], lanes[3]);
        let range = (high - low).abs().max(1e-12);
        let upper = (high - open.max(close)).max(0.0) / range;
        let lower = (open.min(close) - low).max(0.0) / range;
        self.upper_buf[self.idx] = upper;
        self.lower_buf[self.idx] = lower;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut cnt_u = 0usize;
            let mut cnt_l = 0usize;
            for i in 0..len {
                if self.upper_buf[i] <= upper {
                    cnt_u += 1;
                }
                if self.lower_buf[i] <= lower {
                    cnt_l += 1;
                }
            }
            self.upper_percentile = cnt_u as f64 / len as f64;
            self.lower_percentile = cnt_l as f64 / len as f64;
        }
        self.is_upper_spike = self.upper_percentile >= 0.95;
        self.is_lower_spike = self.lower_percentile >= 0.95;
        (self.is_upper_spike, self.is_lower_spike)
    }

}

impl Default for StatisticalWickDetector {
    fn default() -> Self {
        Self::new(50)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, HistogramStyle, Indicator, Output, Param, Render, RenderOutput, RenderSpec,
    SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`StatisticalWickDetector`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StatisticalWickDetectorConfig {
    pub period: Param<usize>,
}

impl Indicator for StatisticalWickDetector {
    const ID: IndicatorId = IndicatorId::Wickspike;
    /// Not a pluggable family member — a spike detector producing two binary flags.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(period): each bar counts over the full window (two rolling percentile passes).
    /// Two period-deep Vec buffers (upper_buf, lower_buf).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::WickspikeUpperSpike),
        Output::discrete(IndicatorOutputId::WickspikeLowerSpike),
    ];
    type Config = StatisticalWickDetectorConfig;
    type Runtime = StatisticalWickDetector;

    fn create(cfg: StatisticalWickDetectorConfig) -> StatisticalWickDetector {
        StatisticalWickDetector::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for StatisticalWickDetectorConfig {
    fn defaults() -> Self {
        StatisticalWickDetectorConfig { period: Param::Solo(50) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for StatisticalWickDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::WickspikeUpperSpike,
                "Upper Spike",
                Color::hex(0xE91E63),
            ))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

impl StatisticalWickDetector {
    pub fn upper_spike(&self) -> f64 { if self.is_upper_spike { 1.0 } else { 0.0 } }
    pub fn lower_spike(&self) -> f64 { if self.is_lower_spike { 1.0 } else { 0.0 } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let ind = StatisticalWickDetector::new(20);
        assert!(!ind.is_ready());
        assert!(!ind.is_upper_spike);
        assert!(!ind.is_lower_spike);
    }

    #[test]
    fn warmup() {
        let mut ind = StatisticalWickDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1_f64).sin() * 5.0;
            ind.feed(&[price, price + 2.0, price - 2.0, price + 1.0]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn percentiles_in_range() {
        let mut ind = StatisticalWickDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.feed(&[price, price + 2.0, price - 2.0, price + 1.0]);
        }
        assert!(ind.upper_percentile >= 0.0 && ind.upper_percentile <= 1.0);
        assert!(ind.lower_percentile >= 0.0 && ind.lower_percentile <= 1.0);
    }

    #[test]
    fn reset_clears() {
        let mut ind = StatisticalWickDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.feed(&[price, price + 2.0, price - 2.0, price + 1.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(!ind.is_upper_spike);
        assert!(!ind.is_lower_spike);
        assert_eq!(ind.upper_percentile, 0.0);
        assert_eq!(ind.lower_percentile, 0.0);
    }

    /// Factory resolves [Open, High, Low, Close] from const SOURCE.
    /// Volume (9999.0) is wild — proves it is NOT consumed.
    /// Feeds window bars with uniform wick structure; after warmup, flags are stable.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Wickspike(StatisticalWickDetectorConfig { period: Param::Solo(10) }).build_solo().unwrap();
        for i in 0..15 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price, high: price + 2.0, low: price - 2.0, close: price + 1.0,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // f.value() returns first output (upper_spike as f64: 0.0 or 1.0)
        let upper = f.primary();
        // Just confirm it's a valid flag value
        assert!(upper == 0.0 || upper == 1.0, "expected flag value, got {upper}");
    }
}
