// Percentile Rate-of-Change over window


#[derive(Debug, Clone)]
pub struct RocPercentile {
    period: usize,
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,      // current ROC
    percentile: f64, // percentile of ROC within window [0..1]
}

impl RocPercentile {
    pub fn new(period: usize, window: usize) -> Self {
        Self {
            period: period.max(1),
            window: window.max(1),
            closes: vec![0.0; window.max(1) + period.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
            percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.idx >= self.period
    }

    /// Feed ONE pre-extracted scalar (close) — contract input (SOURCE = Field{Close}).
    /// Returns (roc, percentile).
    pub fn feed(&mut self, close: f64) -> (f64, f64) {
        let buf_len = self.closes.len();
        self.closes[self.idx % buf_len] = close;

        if self.idx >= self.period {
            let prev = self.closes[(self.idx - self.period) % buf_len];
            self.value = if prev.abs() > 1e-12 {
                (close - prev) / prev
            } else {
                0.0
            };
        } else {
            self.value = 0.0;
        }

        self.idx += 1;
        if self.idx >= buf_len {
            self.filled = true;
        }

        let mut count = 0usize;
        let mut count_le = 0usize;
        if self.idx > self.period {
            let start = self.idx.saturating_sub(self.window);
            for j in start..self.idx {
                if j <= self.period {
                    continue;
                }
                let prev = self.closes[(j - self.period) % buf_len];
                let curr = self.closes[j % buf_len];
                let roc = if prev.abs() > 1e-12 {
                    (curr - prev) / prev
                } else {
                    0.0
                };
                count += 1;
                if roc <= self.value {
                    count_le += 1;
                }
            }
        }
        self.percentile = if count > 0 {
            count_le as f64 / count as f64
        } else {
            0.0
        };
        (self.value, self.percentile)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn percentile(&self) -> f64 {
        self.percentile
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roc_percentile_creation() {
        let rp = RocPercentile::new(10, 50);
        assert!(!rp.is_ready());
        assert_eq!(rp.value(), 0.0);
        assert_eq!(rp.period(), 10);
        assert_eq!(rp.window(), 50);
    }

    #[test]
    fn test_roc_percentile_basic() {
        let mut rp = RocPercentile::new(10, 50);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            rp.feed(price);
        }
        assert!(rp.is_ready());
        assert!(rp.value().is_finite());
        assert!(rp.percentile() >= 0.0 && rp.percentile() <= 1.0);
    }

    #[test]
    fn test_roc_percentile_reset() {
        let mut rp = RocPercentile::new(10, 50);
        for i in 1..=100 {
            rp.feed(100.0 + i as f64);
        }
        assert!(rp.is_ready());
        rp.reset();
        assert!(!rp.is_ready());
        assert_eq!(rp.value(), 0.0);
    }

    #[test]
    fn test_roc_percentile_finite_values() {
        let mut rp = RocPercentile::new(10, 50);
        for i in 1..=150 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (roc, pct) = rp.feed(price);
            assert!(roc.is_finite(), "ROC should always be finite");
            assert!(pct.is_finite(), "Percentile should always be finite");
        }
    }
}

impl Default for RocPercentile {
    fn default() -> Self {
        Self::new(10, 200)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for RocPercentile: momentum lookback + percentile window.
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RocPercentileConfig {
    /// Period for rate-of-change computation (close / close[period] - 1).
    pub period: Param<usize>,
    /// Rolling window over which to compute the percentile rank.
    pub window: Param<usize>,
}

impl Indicator for RocPercentile {
    const ID: IndicatorId = IndicatorId::RocPct;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::RocPct)];
    /// O(window): percentile pass re-scans the rolling window each bar.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    type Config = RocPercentileConfig;
    type Runtime = RocPercentile;

    fn create(cfg: RocPercentileConfig) -> RocPercentile {
        RocPercentile::new(cfg.period.resolved(), cfg.window.resolved())
    }
}

impl crate::contract::Config for RocPercentileConfig {
    fn defaults() -> Self {
        RocPercentileConfig {
            period: Param::Solo(10),
            window: Param::Solo(200),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period/window: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for RocPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::RocPct,
                "ROC %",
                Color::hex(0x2196F3),
                1.5,
            ))
            .zero_baseline()
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
    fn factory_feeds_resolved_source() {
        let cfg = <<RocPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 10);
        assert_eq!(cfg.window.resolved(), 200);
        let mut f = IndicatorOrder::RocPct(cfg)
            .build_solo()
            .unwrap();
        for i in 1..=100 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: 100.0 + i as f64,
                volume: 0.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
