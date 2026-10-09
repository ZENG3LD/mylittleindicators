//! QuoteStuffingDetector — rolling L2 orderbook delta event rate anomaly.
//!
//! Counts the number of orderbook delta events within a rolling `window_ms`
//! window and computes the rate (events per second). If the rate exceeds
//! `rate_threshold`, a quote-stuffing signal is emitted.
//!
//! Outputs: `rate_per_sec`, `is_stuffing`.
//! - `rate_per_sec`: rolling event rate.
//! - `is_stuffing`:  `1.0` when rate > threshold, `0.0` otherwise.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Param, sweep_f64};
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, RenderSpec, SourceAxis};
use crate::core::types::OrderbookDelta;
use crate::engine::stream_kind::StreamKind;

/// Rolling L2 orderbook delta event rate / quote-stuffing detector.
///
/// Parameters:
/// - `window_ms`      — rolling time window in milliseconds (clamped ≥ 1).
/// - `rate_threshold` — events-per-second threshold above which stuffing is
///                      signalled (default 100 eps).
#[derive(Debug, Clone)]
pub struct QuoteStuffingDetector {
    window_ms: i64,
    rate_threshold: f64,
    /// Circular buffer of event timestamps (milliseconds).
    timestamps: VecDeque<i64>,
    last_rate: f64,
    last_signal: f64,
}

impl QuoteStuffingDetector {
    /// Create a new detector.
    ///
    /// - `window_ms`      — rolling window in milliseconds.
    /// - `rate_threshold` — events/sec threshold (clamped ≥ 0).
    pub fn new(window_ms: i64, rate_threshold: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            rate_threshold: rate_threshold.max(0.0),
            timestamps: VecDeque::with_capacity(1024),
            last_rate: 0.0,
            last_signal: 0.0,
        }
    }

    /// Convenience constructor with 100 eps default threshold.
    pub fn with_window(window_ms: i64) -> Self {
        Self::new(window_ms, 100.0)
    }
}

impl Default for QuoteStuffingDetector {
    fn default() -> Self {
        Self::new(1_000, 100.0)
    }
}

/// Typed dual-mode config for [`QuoteStuffingDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct QuoteStuffingDetectorConfig {
    pub window: crate::contract::Param<TimeWindow>,
    pub rate_threshold: crate::contract::Param<f64>,
}

impl Indicator for QuoteStuffingDetector {
    const ID: IndicatorId = IndicatorId::QuoteStuffingDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookDelta];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::QuoteStuffingDetectorRate),
        Output::discrete(IndicatorOutputId::QuoteStuffingDetectorSignal),
    ];
    type Config = QuoteStuffingDetectorConfig;
    type Runtime = QuoteStuffingDetector;

    fn create(cfg: QuoteStuffingDetectorConfig) -> QuoteStuffingDetector {
        QuoteStuffingDetector::new(cfg.window.resolved().as_millis(), cfg.rate_threshold.resolved())
    }
}

impl crate::contract::Config for QuoteStuffingDetectorConfig {
    fn defaults() -> Self {
        QuoteStuffingDetectorConfig {
            window: crate::contract::Param::Solo(TimeWindow::Seconds(1)),
            rate_threshold: crate::contract::Param::Solo(100.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // Class N — TimeWindow discrete set spanning 1s..24h
        s.window = Param::many(vec![
            TimeWindow::Seconds(1),
            TimeWindow::Seconds(5),
            TimeWindow::Seconds(15),
            TimeWindow::Seconds(30),
            TimeWindow::Minutes(1),
            TimeWindow::Minutes(5),
            TimeWindow::Minutes(15),
            TimeWindow::Minutes(30),
            TimeWindow::Hours(1),
            TimeWindow::Hours(4),
            TimeWindow::Hours(24),
        ]);
        // rate_threshold: Param<f64> — Class F per taxonomy (sweep_f64(0.1,5.0,0.1))
        s.rate_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for QuoteStuffingDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::QuoteStuffingDetectorRate, "Rate (eps)", Color::hex(0xFF7043))
            .line_output(IndicatorOutputId::QuoteStuffingDetectorSignal, "Signal", Color::hex(0xEF5350))
            .precision(1)
            .build()
    }
}

impl QuoteStuffingDetector {
    /// Named output: brace `rate`.
    pub fn rate(&self) -> f64 { self.last_rate }
    /// Named output: brace `signal`.
    pub fn signal(&self) -> f64 { self.last_signal }
}

impl OrderbookDeltaConsumer for QuoteStuffingDetector {
    fn update_delta(&mut self, delta: &OrderbookDelta) {
        let now = delta.timestamp;
        self.timestamps.push_back(now);

        // Evict events outside the rolling window.
        while let Some(&ts) = self.timestamps.front() {
            if now - ts > self.window_ms {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        let count = self.timestamps.len() as f64;
        // Window in seconds (avoid divide-by-zero; window_ms ≥ 1 so safe).
        let window_sec = self.window_ms as f64 / 1_000.0;
        self.last_rate = count / window_sec;
        self.last_signal = if self.last_rate > self.rate_threshold {
            1.0
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.timestamps.clear();
        self.last_rate = 0.0;
        self.last_signal = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.timestamps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(timestamp: i64) -> OrderbookDelta {
        OrderbookDelta {
            bids: vec![],
            asks: vec![],
            timestamp,
            first_update_id: None,
            last_update_id: None,
            prev_update_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn low_rate_no_signal() {
        // 5 events over 1 second → rate = 5 eps < 100 threshold.
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        for i in 0..5 {
            det.update_delta(&delta(i * 200)); // 200 ms apart → 5 eps
        }
        let rate = det.rate();
        let signal = det.signal();
        assert!(rate < 100.0, "rate {rate} should be < 100");
        assert!((signal).abs() < 1e-9, "signal should be 0 at low rate");
    }

    #[test]
    fn high_rate_triggers_signal() {
        // 200 events in 1 second → rate = 200 eps > 100 threshold.
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        for i in 0..200 {
            det.update_delta(&delta(i as i64 * 5)); // 5 ms apart → 200 eps
        }
        let rate = det.rate();
        let signal = det.signal();
        assert!(rate > 100.0, "rate {rate} should be > 100");
        assert!((signal - 1.0).abs() < 1e-9, "signal should be 1.0 at high rate");
    }

    #[test]
    fn stale_events_evicted() {
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        // Insert 200 events at t=0..999 ms.
        for i in 0..200 {
            det.update_delta(&delta(i * 5));
        }
        // New event 2 s later — all old ones evicted.
        det.update_delta(&delta(2_000));
        // Only 1 event in window → rate = 1/1s = 1 eps.
        let rate = det.rate();
        let signal = det.signal();
        assert!(rate < 100.0, "rate after eviction should be low, got {rate}");
        assert!((signal).abs() < 1e-9, "signal should be 0 after eviction");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        det.update_delta(&delta(0));
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.rate(), 0.0);
        assert_eq!(det.signal(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_quote_stuffing_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::QuoteStuffingDetector(<<QuoteStuffingDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        f.feed(0, MarketSample::OrderbookDelta(&delta(0)));
        // f.value() returns first output (rate)
        assert!(f.primary() > 0.0, "rate={}", f.primary());
    }
}
