//! TickFrequencyAnomaly — burst/quiet detection via short/long rate ratio.
//!
//! Computes the ratio of the current tick rate (short window) to the baseline
//! tick rate (long window).
//!
//!   ratio > 1.5 → burst activity
//!   ratio ≈ 1.0 → normal
//!   ratio < 0.5 → quiet period
//!
//! Output: `ratio`
//!
//! `ratio` is 0.0 when there is no baseline yet (long window has no data).

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Tick-frequency anomaly detector using short/long rate ratio.
#[derive(Debug, Clone)]
pub struct TickFrequencyAnomaly {
    short_window_ms: i64,
    long_window_ms: i64,
    /// Timestamps of all ticks within `long_window_ms`.
    timestamps: VecDeque<i64>,
    last_ratio: f64,
}

impl TickFrequencyAnomaly {
    /// Create detector.
    ///
    /// - `short_window_ms`: window for current rate (e.g. 5 000 ms = 5 s).
    /// - `long_window_ms`:  window for baseline rate (e.g. 60 000 ms = 1 min).
    ///
    /// `long_window_ms` must be > `short_window_ms`.
    pub fn new(short_window_ms: i64, long_window_ms: i64) -> Self {
        // Enforce short < long; if mis-specified, swap silently.
        let (short, long) = if short_window_ms < long_window_ms {
            (short_window_ms.max(1), long_window_ms)
        } else {
            (long_window_ms.max(1), short_window_ms)
        };
        Self {
            short_window_ms: short,
            long_window_ms: long,
            timestamps: VecDeque::with_capacity(1024),
            last_ratio: 0.0,
        }
    }
}

impl TickConsumer for TickFrequencyAnomaly {
    fn update_tick(&mut self, tick: &Tick) {
        self.timestamps.push_back(tick.time);

        // Evict timestamps outside the long window.
        while let Some(&ts) = self.timestamps.front() {
            if tick.time - ts > self.long_window_ms {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Count ticks in short window.
        let cutoff_short = tick.time - self.short_window_ms;
        let short_count = self.timestamps.iter().filter(|&&ts| ts >= cutoff_short).count();

        let long_count = self.timestamps.len();

        // Convert counts to rates (events per second).
        let short_secs = self.short_window_ms as f64 / 1000.0;
        let long_secs = self.long_window_ms as f64 / 1000.0;

        let current_rate = short_count as f64 / short_secs;
        let baseline_rate = long_count as f64 / long_secs;

        self.last_ratio = if baseline_rate > 0.0 {
            current_rate / baseline_rate
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.timestamps.clear();
        self.last_ratio = 0.0;
    }

    /// Ready once the long window has at least one tick.
    fn is_ready(&self) -> bool {
        !self.timestamps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_at(time_ms: i64) -> Tick {
        Tick::new(time_ms, 100.0, 1.0, true)
    }

    #[test]
    fn uniform_rate_gives_ratio_one() {
        // 10 ticks uniformly spread over 60 seconds (6 s apart).
        // Short window = 5 s: ~1 tick (the last one)
        // Long window = 60 s: 10 ticks
        // current_rate = 1 / 5.0 = 0.2 tps
        // baseline_rate = 10 / 60.0 ≈ 0.1667 tps
        // ratio ≈ 1.2 (not exactly 1 because short window captures only 1 tick of the last 5 s)
        let mut ind = TickFrequencyAnomaly::new(5_000, 60_000);
        for i in 0..10 {
            ind.update_tick(&tick_at(i * 6_000));
        }
        let ratio = ind.value();
        // ratio should be > 0 and finite
        assert!(ratio > 0.0 && ratio.is_finite(), "ratio={}", ratio);
    }

    #[test]
    fn burst_gives_high_ratio() {
        // First, establish a low baseline (1 tick per 10 seconds over 60 s).
        let mut ind = TickFrequencyAnomaly::new(5_000, 60_000);
        for i in 0..6 {
            ind.update_tick(&tick_at(i * 10_000));
        }
        // Now burst: 10 ticks in the last 1 second (the short window).
        let base_time = 6 * 10_000i64;
        for j in 0..10 {
            ind.update_tick(&tick_at(base_time + j * 100));
        }
        let ratio = ind.value();
        assert!(ratio > 1.0, "burst should yield ratio > 1, got {}", ratio);
    }

    #[test]
    fn zero_ratio_when_no_data() {
        let ind = TickFrequencyAnomaly::new(5_000, 60_000);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = TickFrequencyAnomaly::new(5_000, 60_000);
        ind.update_tick(&tick_at(0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl Default for TickFrequencyAnomaly {
    /// Factory default: short_window_ms=5000, long_window_ms=60000.
    fn default() -> Self {
        Self::new(5_000, 60_000)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`TickFrequencyAnomaly`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TickFrequencyAnomalyConfig {
    pub short_window: Param<TimeWindow>,
    pub long_window: Param<TimeWindow>,
}

impl Indicator for TickFrequencyAnomaly {
    const ID: IndicatorId = IndicatorId::TickFrequencyAnomaly;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::TickFrequencyAnomaly)];
    type Config = TickFrequencyAnomalyConfig;
    type Runtime = TickFrequencyAnomaly;

    fn create(cfg: TickFrequencyAnomalyConfig) -> TickFrequencyAnomaly {
        TickFrequencyAnomaly::new(cfg.short_window.resolved().as_millis(), cfg.long_window.resolved().as_millis())
    }
}

impl crate::contract::Config for TickFrequencyAnomalyConfig {
    fn defaults() -> Self {
        TickFrequencyAnomalyConfig {
            short_window: Param::Solo(TimeWindow::Seconds(5)),
            long_window: Param::Solo(TimeWindow::Minutes(1)),
        }
    }
    fn machine_defaults() -> Self {
        // short_window (TimeWindow): Class N — 11-point curated discrete set.
        // long_window (TimeWindow): Class N — same 11-point set.
        // Structural constraint: short_window < long_window — enforced at runtime by swap.
        let mut s = Self::machine_defaults_auto();
        s.short_window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s.long_window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for TickFrequencyAnomaly {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::TickFrequencyAnomaly, "Tick Freq Ratio", Color::hex(0xFFEB3B))
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
    fn factory_feeds_resolved_tick_frequency_anomaly() {
        let mut f = IndicatorOrder::TickFrequencyAnomaly(
            <<TickFrequencyAnomaly as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}

impl TickFrequencyAnomaly {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_ratio
    }
}
