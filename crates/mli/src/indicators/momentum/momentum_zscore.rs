// Momentum Z-Score: zscore of close change over lookback with rolling window


#[derive(Debug, Clone)]
pub struct MomentumZscore {
    diff_period: usize,
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    // rolling stats for diffs
    diffs: Vec<f64>,
    didx: usize,
    dfilled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl MomentumZscore {
    pub fn new(diff_period: usize, window: usize) -> Self {
        Self {
            diff_period: diff_period.max(1),
            window: window.max(2),
            closes: vec![0.0; diff_period.max(1) + window.max(2)],
            idx: 0,
            filled: false,
            diffs: vec![0.0; window.max(2)],
            didx: 0,
            dfilled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.diffs.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.didx = 0;
        self.dfilled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dfilled
    }

    /// Feed ONE pre-extracted scalar (close) — contract input (SOURCE = Field{Close}).
    pub fn feed(&mut self, close: f64) -> f64 {
        let cbuf = self.closes.len();
        self.closes[self.idx % cbuf] = close;

        if self.idx >= self.diff_period {
            let prev = self.closes[(self.idx - self.diff_period) % cbuf];
            let diff = close - prev;

            let old = self.diffs[self.didx];
            self.diffs[self.didx] = diff;
            self.didx = (self.didx + 1) % self.window;
            if self.didx == 0 {
                self.dfilled = true;
            }

            self.sum += diff - old;
            self.sumsq += diff * diff - old * old;

            let n = if self.dfilled {
                self.window as f64
            } else {
                self.didx as f64
            };
            if n >= 2.0 {
                let mean = self.sum / n;
                let var = (self.sumsq / n) - mean * mean;
                let std = if var > 0.0 { var.sqrt() } else { 0.0 };
                self.value = if std > 1e-12 {
                    (diff - mean) / std
                } else {
                    0.0
                };
            } else {
                self.value = 0.0;
            }
        }

        self.idx += 1;
        if self.idx >= cbuf {
            self.filled = true;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn diff_period(&self) -> usize {
        self.diff_period
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_momentum_zscore_creation() {
        let mz = MomentumZscore::new(10, 20);
        assert!(!mz.is_ready());
        assert_eq!(mz.value(), 0.0);
        assert_eq!(mz.diff_period(), 10);
        assert_eq!(mz.window(), 20);
    }

    #[test]
    fn test_momentum_zscore_basic() {
        let mut mz = MomentumZscore::new(10, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            mz.feed(price);
        }
        assert!(mz.is_ready());
        assert!(mz.value().is_finite());
    }

    #[test]
    fn test_momentum_zscore_reset() {
        let mut mz = MomentumZscore::new(10, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            mz.feed(price);
        }
        assert!(mz.is_ready());
        mz.reset();
        assert!(!mz.is_ready());
        assert_eq!(mz.value(), 0.0);
    }

    #[test]
    fn test_momentum_zscore_finite_values() {
        let mut mz = MomentumZscore::new(10, 20);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = mz.feed(price);
            assert!(value.is_finite(), "Momentum Zscore should always be finite");
        }
    }
}

impl Default for MomentumZscore {
    fn default() -> Self {
        Self::new(1, 100)
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

/// Typed config for MomentumZscore: momentum period + z-score window.
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MomentumZscoreConfig {
    /// Period for computing momentum diff (close - close[diff_period]).
    pub diff_period: Param<usize>,
    /// Rolling window over which to compute the z-score of those diffs.
    pub window: Param<usize>,
}

impl Indicator for MomentumZscore {
    const ID: IndicatorId = IndicatorId::MomZscore;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::MomZscore)];
    /// O(1): rolling ring-buffer sums — no full window rescan.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Vec), // closes ring
            Store::window(StoreKind::Vec), // diffs ring
        ],
    );

    type Config = MomentumZscoreConfig;
    type Runtime = MomentumZscore;

    fn create(cfg: MomentumZscoreConfig) -> MomentumZscore {
        MomentumZscore::new(cfg.diff_period.resolved(), cfg.window.resolved())
    }
}

impl crate::contract::Config for MomentumZscoreConfig {
    fn defaults() -> Self {
        MomentumZscoreConfig {
            diff_period: Param::Solo(1),
            window: Param::Solo(100),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // diff_period/window: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for MomentumZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::MomZscore,
                "Mom Z-Score",
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
        let cfg = <<MomentumZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.diff_period.resolved(), 1);
        assert_eq!(cfg.window.resolved(), 100);
        let mut f = IndicatorOrder::MomZscore(cfg)
            .build_solo()
            .unwrap();
        for i in 1..=50 {
            let close = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close,
                volume: 0.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
