//! Universal oscillator-with-volume-weight wrapper.
//!
//! Compares price movement with volume to detect:
//! - Volume confirmation (price and volume move in same direction)
//! - Volume divergence (price moves but volume is below baseline — weak move)
//! - Volume spikes (unusually high volume)
//!
//! Output layers:
//! - Layer 1 (default): `Single(inner_value)` — passthrough
//! - Layer 2 (`with_volume_event = true`): `Double(inner_value, vw_signal as f64)`
//! - Layer 3 (`with_strength = true`): `Triple(inner_value, vw_signal, volume_strength)`
//!
//! `type_signal` encoding (`i8` as `f64`):
//! - `+3` Bullish Volume Spike (price up, volume > spike threshold)
//! - `+2` Volume Confirmation Up (price up, volume above baseline)
//! - `+1` Volume Divergence Up (price up, volume below baseline — weak)
//! -  `0` No significant event
//! - `-1` Volume Divergence Down (price down, volume below baseline)
//! - `-2` Volume Confirmation Down (price down, volume above baseline)
//! - `-3` Bearish Volume Spike (price down, volume > spike threshold)

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;

/// Wraps any indicator with volume event classification.
///
/// The wrapper observes the last `baseline_period` bars to build a rolling
/// volume baseline (mean). Each new bar is classified by comparing the
/// bar's volume against the baseline and the price direction.
#[derive(Clone)]
pub struct OscillatorWithVolumeWeight {
    inner: Box<IndicatorInstance>,
    baseline_period: usize,
    spike_threshold: f64,
    with_volume_event: bool,
    with_strength: bool,

    volume_history: VecDeque<f64>,
    prev_close: f64,
    has_prev: bool,
}

impl std::fmt::Debug for OscillatorWithVolumeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OscillatorWithVolumeWeight")
            .field("baseline_period", &self.baseline_period)
            .field("spike_threshold", &self.spike_threshold)
            .field("with_volume_event", &self.with_volume_event)
            .field("with_strength", &self.with_strength)
            .field("history_len", &self.volume_history.len())
            .field("has_prev", &self.has_prev)
            .finish()
    }
}

impl OscillatorWithVolumeWeight {
    /// Create a new wrapper.
    ///
    /// - `inner`            — any indicator instance; its scalar output is passed through.
    /// - `baseline_period`  — rolling window for volume mean (minimum 2).
    /// - `spike_threshold`  — multiplier above which volume is a spike (e.g. `2.5`).
    /// - `with_volume_event`— emit `Double` layer; must be `true` for any signal output.
    /// - `with_strength`    — emit `Triple` layer with normalised volume strength.
    pub fn new(
        inner: Box<IndicatorInstance>,
        baseline_period: usize,
        spike_threshold: f64,
        with_volume_event: bool,
        with_strength: bool,
    ) -> Self {
        Self {
            inner,
            baseline_period: baseline_period.max(2),
            spike_threshold: spike_threshold.max(1.01),
            with_volume_event,
            with_strength,
            volume_history: VecDeque::with_capacity(baseline_period.max(2) + 1),
            prev_close: 0.0,
            has_prev: false,
        }
    }

    /// Feed one OHLCV bar. Returns the appropriate `IndicatorValue` layer.
    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> IndicatorValue {
        let inner_value = self
            .inner
            .update_bar(open, high, low, close, volume, None)
            .main();

        // Accumulate volume history (includes current bar).
        self.volume_history.push_back(volume);
        if self.volume_history.len() > self.baseline_period {
            self.volume_history.pop_front();
        }

        // Warm-up: not enough history yet.
        if self.volume_history.len() < self.baseline_period {
            return IndicatorValue::Single(inner_value);
        }

        // Baseline = mean of all bars in history except the current one.
        // history is full (len == baseline_period); exclude the last element (current bar).
        let n = self.volume_history.len();
        let baseline_sum: f64 = self.volume_history.iter().take(n.saturating_sub(1)).sum();
        let baseline_count = n.saturating_sub(1);
        let baseline_volume = if baseline_count > 0 {
            baseline_sum / baseline_count as f64
        } else {
            volume
        };

        const EPS: f64 = 1e-9;
        let volume_ratio = if baseline_volume > EPS {
            volume / baseline_volume
        } else {
            1.0
        };

        // First bar after warm-up: set prev_close, emit Single.
        if !self.has_prev {
            self.prev_close = close;
            self.has_prev = true;
            return IndicatorValue::Single(inner_value);
        }

        let price_dir = if close > self.prev_close + EPS {
            1i8
        } else if close < self.prev_close - EPS {
            -1i8
        } else {
            0i8
        };

        let type_signal: f64 = if price_dir == 0 {
            0.0
        } else if volume_ratio >= self.spike_threshold {
            (price_dir as f64) * 3.0
        } else if volume_ratio > 1.0 {
            (price_dir as f64) * 2.0
        } else {
            (price_dir as f64) * 1.0
        };

        // strength: 0.0 at baseline, 1.0 at spike_threshold.
        let strength = ((volume_ratio - 1.0) / (self.spike_threshold - 1.0)).clamp(0.0, 1.0);

        self.prev_close = close;

        if self.with_strength {
            IndicatorValue::Triple(inner_value, type_signal, strength)
        } else if self.with_volume_event {
            IndicatorValue::Double(inner_value, type_signal)
        } else {
            IndicatorValue::Single(inner_value)
        }
    }

    /// Returns the last computed value without advancing state.
    pub fn value(&self) -> IndicatorValue {
        // Use a placeholder; actual last value tracking would require storing it.
        // Return sensible default matching the configured output layer.
        if self.with_strength {
            IndicatorValue::Triple(0.0, 0.0, 0.0)
        } else if self.with_volume_event {
            IndicatorValue::Double(0.0, 0.0)
        } else {
            IndicatorValue::Single(0.0)
        }
    }

    /// Returns `true` once the inner indicator has warmed up.
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    /// Clears all internal state.
    pub fn reset(&mut self) {
        self.inner.reset();
        self.volume_history.clear();
        self.prev_close = 0.0;
        self.has_prev = false;
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    fn sma_inner(period: usize) -> Box<IndicatorInstance> {
        Box::new(
            IndicatorInstance::create(&IndicatorConfig::new(
                BarIndicatorId::Sma,
                "Sma".into(),
                vec![period],
            ))
            .expect("SMA creation must succeed"),
        )
    }

    /// Feed a flat bar (same O/H/L/C).
    fn feed(
        ind: &mut OscillatorWithVolumeWeight,
        close: f64,
        volume: f64,
    ) -> IndicatorValue {
        ind.update_bar(close, close + 0.5, close - 0.5, close, volume)
    }

    /// Feed `n` flat bars at `price` and `vol`.
    fn feed_n(
        ind: &mut OscillatorWithVolumeWeight,
        price: f64,
        vol: f64,
        n: usize,
    ) -> Vec<IndicatorValue> {
        (0..n).map(|_| feed(ind, price, vol)).collect()
    }

    /// Extract the signal component from a Double or Triple value.
    fn signal_of(v: &IndicatorValue) -> f64 {
        match v {
            IndicatorValue::Double(_, s) => *s,
            IndicatorValue::Triple(_, s, _) => *s,
            IndicatorValue::Single(_) => 0.0,
            _ => 0.0,
        }
    }

    // ── Warm-up returns Single ────────────────────────────────────────────────

    #[test]
    fn warmup_returns_single() {
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), 10, 2.5, true, false);
        // Feed fewer bars than baseline_period → must stay Single.
        for i in 0..9u32 {
            let v = feed(&mut ind, 100.0 + i as f64, 100.0);
            assert!(
                matches!(v, IndicatorValue::Single(_)),
                "expected Single during warmup at bar {i}, got {:?}",
                v
            );
        }
    }

    // ── Volume spike up → +3 ─────────────────────────────────────────────────

    #[test]
    fn volume_spike_up_signals_plus_three() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        // Fill baseline_period bars at baseline volume=100 so warmup completes.
        feed_n(&mut ind, 100.0, 100.0, baseline);

        // One bar that sets prev_close (first post-warmup bar).
        feed(&mut ind, 100.0, 100.0);

        // Now: price up, volume = 300 (3× baseline → spike).
        let v = feed(&mut ind, 101.0, 300.0);
        assert_eq!(
            signal_of(&v),
            3.0,
            "expected +3 for spike-up; got {:?}",
            v
        );
    }

    // ── Confirmation up → +2 ─────────────────────────────────────────────────

    #[test]
    fn confirmation_up_signals_plus_two() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0); // set prev_close

        // volume = 150 (1.5× baseline, above 1.0 but < 2.5 spike).
        let v = feed(&mut ind, 101.0, 150.0);
        assert_eq!(
            signal_of(&v),
            2.0,
            "expected +2 for confirmation-up; got {:?}",
            v
        );
    }

    // ── Divergence up (weak) → +1 ─────────────────────────────────────────────

    #[test]
    fn divergence_up_weak_signals_plus_one() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);

        // volume = 50 (0.5× baseline → below baseline).
        let v = feed(&mut ind, 101.0, 50.0);
        assert_eq!(
            signal_of(&v),
            1.0,
            "expected +1 for divergence-up (weak); got {:?}",
            v
        );
    }

    // ── Confirmation down → -2 ────────────────────────────────────────────────

    #[test]
    fn confirmation_down_signals_minus_two() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        feed_n(&mut ind, 101.0, 100.0, baseline);
        feed(&mut ind, 101.0, 100.0);

        // Price down, volume 1.5× baseline.
        let v = feed(&mut ind, 100.0, 150.0);
        assert_eq!(
            signal_of(&v),
            -2.0,
            "expected -2 for confirmation-down; got {:?}",
            v
        );
    }

    // ── Flat price → 0 ───────────────────────────────────────────────────────

    #[test]
    fn flat_price_zero_signal() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);

        // Same price — flat, no matter the volume.
        let v = feed(&mut ind, 100.0, 300.0);
        assert_eq!(
            signal_of(&v),
            0.0,
            "expected 0 for flat price; got {:?}",
            v
        );
    }

    // ── Layer 3 returns Triple with strength ──────────────────────────────────

    #[test]
    fn layer_3_returns_triple_with_strength() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, true);

        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);

        // Spike: volume = 350 (> 2.5× baseline of 100).
        let v = feed(&mut ind, 101.0, 350.0);
        match v {
            IndicatorValue::Triple(_, sig, strength) => {
                assert_eq!(sig, 3.0, "expected +3 signal");
                assert!(strength > 0.0, "expected positive strength; got {strength}");
            }
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    // ── Bearish spike → -3 ───────────────────────────────────────────────────

    #[test]
    fn bearish_spike_signals_minus_three() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        feed_n(&mut ind, 101.0, 100.0, baseline);
        feed(&mut ind, 101.0, 100.0);

        // Price down + volume spike.
        let v = feed(&mut ind, 100.0, 300.0);
        assert_eq!(
            signal_of(&v),
            -3.0,
            "expected -3 for bearish spike; got {:?}",
            v
        );
    }

    // ── with_volume_event=false → Single ─────────────────────────────────────

    #[test]
    fn no_event_flag_returns_single_passthrough() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, false, false);

        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);

        // Even with a spike, with_volume_event=false → Single.
        let v = feed(&mut ind, 101.0, 300.0);
        assert!(
            matches!(v, IndicatorValue::Single(_)),
            "expected Single when with_volume_event=false; got {:?}",
            v
        );
    }

    // ── reset clears state ────────────────────────────────────────────────────

    #[test]
    fn reset_clears_state() {
        let baseline = 10;
        let mut ind =
            OscillatorWithVolumeWeight::new(sma_inner(5), baseline, 2.5, true, false);

        feed_n(&mut ind, 100.0, 100.0, baseline + 2);
        ind.reset();

        // After reset, next bar must be Single (warmup again).
        let v = feed(&mut ind, 100.0, 100.0);
        assert!(
            matches!(v, IndicatorValue::Single(_)),
            "expected Single after reset; got {:?}",
            v
        );
    }
}
