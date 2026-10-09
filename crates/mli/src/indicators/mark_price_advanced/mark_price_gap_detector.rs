//! MarkPriceGapDetector — detects abnormal price jumps in the mark price feed.
//!
//! A jump is detected when `|current - prev| > sigma_threshold × rolling_std`.
//!
//! Output: `Triple(jump_signal_as_f64, jump_size, sigma_ratio)`.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;

/// Detects statistically abnormal jumps in the mark price.
///
/// `jump_signal_as_f64 = +1.0 / -1.0 / 0.0`
/// `jump_size = |current - prev|`
/// `sigma_ratio = jump_size / rolling_std`
#[derive(Debug, Clone)]
pub struct MarkPriceGapDetector {
    window: usize,
    sigma_threshold: f64,
    history: VecDeque<f64>,
    prev_mark: f64,
    last_jump_signal: f64,
    last_jump_size: f64,
    last_sigma_ratio: f64,
}

impl MarkPriceGapDetector {
    /// Create a new indicator.
    ///
    /// - `window`: rolling std lookback (min 2).
    /// - `sigma_threshold`: jump threshold in standard deviations (default 3.0).
    pub fn new(window: usize, sigma_threshold: f64) -> Self {
        Self {
            window: window.max(2),
            sigma_threshold,
            history: VecDeque::new(),
            prev_mark: f64::NAN,
            last_jump_signal: 0.0,
            last_jump_size: 0.0,
            last_sigma_ratio: 0.0,
        }
    }

    /// Jump signal: `+1.0` (upward gap), `-1.0` (downward gap), or `0.0`.
    pub fn signal(&self) -> f64 {
        self.last_jump_signal
    }

    /// Absolute jump size: `|current - prev|`.
    pub fn jump_size(&self) -> f64 {
        self.last_jump_size
    }

    /// Jump size in standard deviations of the rolling window.
    pub fn sigma_ratio(&self) -> f64 {
        self.last_sigma_ratio
    }

    fn rolling_std(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        variance.sqrt()
    }
}

impl Default for MarkPriceGapDetector {
    fn default() -> Self {
        Self::new(20, 3.0)
    }
}

/// Typed configuration for [`MarkPriceGapDetector`]. Consumes `MarkPrice` stream.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MarkPriceGapDetectorConfig {
    pub window: crate::contract::Param<usize>,
    pub sigma_threshold: crate::contract::Param<f64>,
}

impl Indicator for MarkPriceGapDetector {
    const ID: IndicatorId = IndicatorId::MarkPriceGapDetector;
    /// Mark-price indicator — not a pluggable family.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarkPrice];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::MarkPriceGapDetectorSignal),
        Output::magnitude(IndicatorOutputId::MarkPriceGapDetectorJumpSize),
        Output::magnitude(IndicatorOutputId::MarkPriceGapDetectorSigmaRatio),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = MarkPriceGapDetectorConfig;
    type Runtime = MarkPriceGapDetector;

    fn create(cfg: MarkPriceGapDetectorConfig) -> MarkPriceGapDetector {
        MarkPriceGapDetector::new(cfg.window.resolved(), cfg.sigma_threshold.resolved())
    }
}

impl crate::contract::Config for MarkPriceGapDetectorConfig {
    fn defaults() -> Self {
        MarkPriceGapDetectorConfig {
            window: crate::contract::Param::Solo(20),
            sigma_threshold: crate::contract::Param::Solo(3.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: Class A (rolling std lookback) → auto range(2,4048,1) is correct.
        // sigma_threshold: Class F (jump threshold in standard deviations) →
        //   sweep_f64(0.1,5.0,0.1).
        s.sigma_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MarkPriceGapDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MarkPriceGapDetectorSignal, "Gap Signal", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::MarkPriceGapDetectorJumpSize, "Jump Size", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::MarkPriceGapDetectorSigmaRatio, "Sigma Ratio", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl MarkPriceConsumer for MarkPriceGapDetector {
    fn update_mark(&mut self, mp: &MarkPrice) {
        let current = mp.mark_price;

        // Compute jump vs previous observation
        let jump_size = if self.prev_mark.is_finite() {
            (current - self.prev_mark).abs()
        } else {
            0.0
        };
        let jump_direction = if self.prev_mark.is_finite() && current != self.prev_mark {
            if current > self.prev_mark { 1.0 } else { -1.0 }
        } else {
            0.0
        };

        // Update rolling history
        self.history.push_back(current);
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        let std = self.rolling_std();
        let sigma_ratio = if std > 1e-15 { jump_size / std } else { 0.0 };

        self.last_jump_signal = if sigma_ratio > self.sigma_threshold { jump_direction } else { 0.0 };
        self.last_jump_size = jump_size;
        self.last_sigma_ratio = sigma_ratio;
        self.prev_mark = current;

    }


    fn reset(&mut self) {
        self.history.clear();
        self.prev_mark = f64::NAN;
        self.last_jump_signal = 0.0;
        self.last_jump_size = 0.0;
        self.last_sigma_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window && self.prev_mark.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_mark_price() {
        let mut f = IndicatorOrder::MarkPriceGapDetector(
            <<MarkPriceGapDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let mp = MarkPrice {
            mark_price: 50_000.0,
            index_price: None,
            funding_rate: None,
            timestamp: 1,
            ..Default::default()
        };
        f.feed(0, MarketSample::MarkPrice(&mp));
        // First observation: no prev mark yet so no jump; f.value() = first output (signal)
        assert_eq!(f.primary(), 0.0);
    }

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice {
            mark_price,
            index_price: None,
            funding_rate: None,
            timestamp: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn no_jump_for_stable_prices() {
        let mut ind = MarkPriceGapDetector::new(5, 3.0);
        for p in [50000.0, 50001.0, 50002.0, 50001.5, 50002.5] {
            ind.update_mark(&make_mp(p));
        }
        assert_eq!(ind.signal(), 0.0, "small moves should not trigger jump signal");
    }

    #[test]
    fn large_positive_jump_fires_plus_one() {
        let mut ind = MarkPriceGapDetector::new(5, 2.0);
        // Build history with slight variation so std > 0
        for p in [50000.0_f64, 50001.0, 49999.0, 50002.0, 49998.0] {
            ind.update_mark(&make_mp(p));
        }
        // Massive upward jump — many sigma above the tiny std
        ind.update_mark(&make_mp(51000.0));
        let sig = ind.signal();
        let size = ind.jump_size();
        let ratio = ind.sigma_ratio();
        assert_eq!(sig, 1.0, "upward jump should give +1, ratio={ratio}");
        assert!(size > 0.0, "jump size should be positive");
        assert!(ratio > 2.0, "sigma ratio should exceed threshold");
    }

    #[test]
    fn large_negative_jump_fires_minus_one() {
        let mut ind = MarkPriceGapDetector::new(5, 2.0);
        for p in [50000.0_f64, 50001.0, 49999.0, 50002.0, 49998.0] {
            ind.update_mark(&make_mp(p));
        }
        // Massive downward jump
        ind.update_mark(&make_mp(49000.0));
        assert_eq!(ind.signal(), -1.0, "downward jump should give -1");
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = MarkPriceGapDetector::new(5, 3.0);
        for i in 0..4 {
            ind.update_mark(&make_mp(50000.0 + i as f64));
        }
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceGapDetector::new(5, 3.0);
        for p in [50000.0, 50100.0, 50200.0, 50300.0, 50400.0] {
            ind.update_mark(&make_mp(p));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.signal(), 0.0);
        assert_eq!(ind.jump_size(), 0.0);
        assert_eq!(ind.sigma_ratio(), 0.0);
    }
}
