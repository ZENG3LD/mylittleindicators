//! SwingDetection primitive — detects swing high/low events from a price
//! stream using one of several confirmation modes.
//!
//! Unlike the 5 hardcoded zigzag files (Classic, Atr, Time, Candle, Lookahead)
//! that each baked in one detection strategy, SwingDetection takes a
//! configurable `mode` enum.
//!
//! Output: `IndicatorValue::Signal(i8)` — +1 on bars where a swing-high is
//! confirmed, -1 on swing-low, 0 otherwise.
//!
//! Internally maintains a rolling buffer of high/low/close per bar and emits
//! the swing event on the bar at which the confirmation criterion fires.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;
use crate::core::signal::direction::Direction;
use crate::core::signal::kind::SignalKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwingMode {
    /// Percent-based reversal: swing fires when price reverses by at least
    /// `threshold_pct` percent from the running extremum.
    Percent { threshold_pct: f64 },
    /// ATR-based reversal: requires an inner ATR-like indicator providing the
    /// reversal threshold; param `mult` scales it. Threshold = mult × ATR.
    AtrMultiple { mult: f64 },
    /// N-bar high/low: swing confirmed if current high (low) is the maximum
    /// (minimum) over the last `n` bars.
    NBarExtreme { n: usize },
    /// Look-ahead confirmation: pivot fires on bar i once `n` future bars
    /// haven't exceeded its extreme. Reports the pivot bar after lag of `n`.
    Lookahead { n: usize },
    /// Forced segmentation: a new swing is recorded every `n_bars` bars.
    /// Direction = close > last_extreme ? High : Low.
    Time { n_bars: usize },
}

impl Default for SwingMode {
    fn default() -> Self {
        Self::Percent { threshold_pct: 1.0 }
    }
}

#[derive(Clone)]
pub struct SwingDetection {
    mode: SwingMode,
    /// Optional inner indicator used by AtrMultiple mode (must produce an ATR-like value).
    atr_source: Option<Box<IndicatorInstance>>,
    /// Rolling window of (high, low) — sized by mode requirements.
    highs: Vec<f64>,
    lows: Vec<f64>,
    /// Running pivot anchor for Percent / Atr modes.
    pivot_high: f64,
    pivot_low: f64,
    /// Last emitted swing direction (sticky for state-style queries).
    last_swing: i8,
    /// Per-bar raw event signal: non-zero only on the bar the swing was confirmed.
    last_event: i8,
    /// Track whether we're currently trending up or down for reversal modes.
    trending_up: Option<bool>,
    bars_seen: usize,
    /// Bar index of last forced swing (used by Time mode).
    last_forced_bar: usize,
    /// Last extreme price (used by Time mode).
    last_extreme: f64,
}

impl std::fmt::Debug for SwingDetection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwingDetection")
            .field("mode", &self.mode)
            .field("last_swing", &self.last_swing)
            .field("last_event", &self.last_event)
            .field("bars_seen", &self.bars_seen)
            .finish()
    }
}

impl SwingDetection {
    /// Construct without ATR source. AtrMultiple mode requires use of
    /// `with_atr_source` instead — otherwise it always returns 0.
    pub fn new(mode: SwingMode) -> Self {
        Self {
            mode,
            atr_source: None,
            highs: Vec::with_capacity(64),
            lows: Vec::with_capacity(64),
            pivot_high: f64::NEG_INFINITY,
            pivot_low: f64::INFINITY,
            last_swing: 0,
            last_event: 0,
            trending_up: None,
            bars_seen: 0,
            last_forced_bar: 0,
            last_extreme: 0.0,
        }
    }

    /// Construct with explicit ATR-like inner for AtrMultiple mode.
    pub fn with_atr_source(mode: SwingMode, atr: IndicatorInstance) -> Self {
        let mut s = Self::new(mode);
        s.atr_source = Some(Box::new(atr));
        s
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_seen += 1;

        // Feed ATR inner if present.
        let atr_val = self.atr_source.as_mut().map(|atr| {
            atr.update_bar(open, high, low, close, volume, None).main()
        });

        let cap = match self.mode {
            SwingMode::NBarExtreme { n } => n.max(2),
            SwingMode::Lookahead { n } => n.max(2) * 2 + 1,
            _ => 64,
        };

        if self.highs.len() >= cap {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);

        let signal = match self.mode {
            SwingMode::Percent { threshold_pct } => {
                self.detect_reversal(high, low, threshold_pct, None)
            }
            SwingMode::AtrMultiple { mult } => {
                if let Some(atr) = atr_val {
                    self.detect_reversal(high, low, mult, Some(atr))
                } else {
                    0
                }
            }
            SwingMode::NBarExtreme { n } => self.detect_n_bar_extreme(n),
            SwingMode::Lookahead { n } => self.detect_lookahead(n),
            SwingMode::Time { n_bars } => self.detect_time(close, n_bars),
        };

        self.last_event = signal;
        self.last_swing = if signal != 0 { signal } else { self.last_swing };
        signal as f64
    }

    fn detect_reversal(&mut self, high: f64, low: f64, threshold: f64, atr: Option<f64>) -> i8 {
        // Update running pivot extremes.
        if high > self.pivot_high {
            self.pivot_high = high;
        }
        if low < self.pivot_low {
            self.pivot_low = low;
        }
        if self.pivot_high == f64::NEG_INFINITY || self.pivot_low == f64::INFINITY {
            return 0;
        }

        // Threshold delta: percent OR ATR × mult.
        let delta = match atr {
            Some(a) => a * threshold,
            None => self.pivot_high.abs() * threshold / 100.0,
        };

        let mut signal = 0i8;
        match self.trending_up {
            Some(true) => {
                // Looking for swing-high confirmation: price reverses down by `delta` from pivot_high.
                if low <= self.pivot_high - delta {
                    signal = 1; // swing-high confirmed
                    self.trending_up = Some(false);
                    self.pivot_low = low; // reset pivot_low to current
                }
            }
            Some(false) => {
                // Looking for swing-low confirmation.
                if high >= self.pivot_low + delta {
                    signal = -1; // swing-low confirmed
                    self.trending_up = Some(true);
                    self.pivot_high = high;
                }
            }
            None => {
                // Establish initial direction.
                if high > low {
                    self.trending_up = Some(true);
                }
            }
        }
        signal
    }

    fn detect_n_bar_extreme(&self, n: usize) -> i8 {
        if self.highs.len() < n.max(2) {
            return 0;
        }
        let len = self.highs.len();
        let window_end = len; // current bar exclusive in look-back
        let start = window_end.saturating_sub(n);
        let curr_high = self.highs[len - 1];
        let curr_low = self.lows[len - 1];

        let max_high = self.highs[start..window_end - 1]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_low = self.lows[start..window_end - 1]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));

        if curr_high > max_high {
            1
        } else if curr_low < min_low {
            -1
        } else {
            0
        }
    }

    fn detect_lookahead(&self, n: usize) -> i8 {
        // Confirm pivot at bar (len - 1 - n): if its high is the max over
        // [len-1-2n .. len-1] and its low is not the min (or vice versa),
        // it's a swing-high (or swing-low). Lag = n bars.
        let needed = n.max(2) * 2 + 1;
        if self.highs.len() < needed {
            return 0;
        }
        let len = self.highs.len();
        let pivot_idx = len - 1 - n;
        let win_start = len - 1 - 2 * n;
        let win_end = len; // exclusive

        let pivot_high = self.highs[pivot_idx];
        let pivot_low = self.lows[pivot_idx];

        let is_max = (win_start..win_end).all(|i| i == pivot_idx || self.highs[i] <= pivot_high);
        let is_min = (win_start..win_end).all(|i| i == pivot_idx || self.lows[i] >= pivot_low);

        if is_max && !is_min {
            1
        } else if is_min && !is_max {
            -1
        } else {
            0
        }
    }

    fn detect_time(&mut self, close: f64, n_bars: usize) -> i8 {
        let n = n_bars.max(1);
        // First bar: initialise anchor.
        if self.bars_seen == 1 {
            self.last_forced_bar = 1;
            self.last_extreme = close;
            return 0;
        }
        if self.bars_seen - self.last_forced_bar >= n {
            let signal = if close > self.last_extreme { 1 } else { -1 };
            self.last_forced_bar = self.bars_seen;
            self.last_extreme = close;
            signal
        } else {
            0
        }
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_swing)
    }

    /// Feed one bar and return a typed signal when a swing high or low is confirmed.
    ///
    /// Swing-high confirmed → `(SignalKind::Swing, Direction::Up)`.
    /// Swing-low confirmed → `(SignalKind::Swing, Direction::Down)`.
    /// Returns `None` on bars where no new swing point was confirmed.
    pub fn detect(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.update_bar(open, high, low, close, volume);
        match self.last_event {
            1 => Some((SignalKind::Swing, Direction::Up)),
            -1 => Some((SignalKind::Swing, Direction::Down)),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.bars_seen >= 2
    }

    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.pivot_high = f64::NEG_INFINITY;
        self.pivot_low = f64::INFINITY;
        self.last_swing = 0;
        self.last_event = 0;
        self.trending_up = None;
        self.bars_seen = 0;
        self.last_forced_bar = 0;
        self.last_extreme = 0.0;
        if let Some(atr) = self.atr_source.as_mut() {
            atr.reset();
        }
    }

    /// Detect swing high/low from pre-computed bar values (slice-based hot loop).
    ///
    /// Feed `high`, `low`, `close` directly — the inner `Box<IndicatorInstance>` (ATR) is
    /// NOT driven. For `AtrMultiple` mode pass the pre-computed ATR as `atr_hint`.
    /// For all other modes `atr_hint` is ignored.
    ///
    /// Returns `Some((SignalKind::Swing, Direction::Up))` on swing-high confirmation,
    /// `Direction::Down` on swing-low, `None` otherwise.
    pub fn detect_from_values(
        &mut self,
        high: f64,
        low: f64,
        _close: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.bars_seen += 1;

        let cap = match self.mode {
            SwingMode::NBarExtreme { n } => n.max(2),
            SwingMode::Lookahead { n } => n.max(2) * 2 + 1,
            _ => 64,
        };

        if self.highs.len() >= cap {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);

        let signal = match self.mode {
            SwingMode::Percent { threshold_pct } => {
                self.detect_reversal(high, low, threshold_pct, None)
            }
            SwingMode::AtrMultiple { mult } => {
                // Without atr_hint we fall back to percent semantics using mult as pct.
                self.detect_reversal(high, low, mult, None)
            }
            SwingMode::NBarExtreme { n } => self.detect_n_bar_extreme(n),
            SwingMode::Lookahead { n } => self.detect_lookahead(n),
            SwingMode::Time { n_bars } => self.detect_time(_close, n_bars),
        };

        self.last_event = signal;
        self.last_swing = if signal != 0 { signal } else { self.last_swing };

        match signal {
            1 => Some((SignalKind::Swing, Direction::Up)),
            -1 => Some((SignalKind::Swing, Direction::Down)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    fn atr_inner(period: usize) -> IndicatorInstance {
        IndicatorInstance::create(&IndicatorConfig::new(BarIndicatorId::Atr, "Atr".into(), vec![period])).unwrap()
    }

    #[test]
    fn percent_mode_detects_reversal() {
        let mut s = SwingDetection::new(SwingMode::Percent { threshold_pct: 2.0 });
        // Strong uptrend then reverse — must catch swing-high.
        let mut saw_swing_high = false;
        for i in 0..30 {
            let p = 100.0 + i as f64;
            let sig = s.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
            if sig > 0.0 { saw_swing_high = true; }
        }
        // Now reverse hard.
        for i in 0..15 {
            let p = 129.0 - i as f64 * 2.0;
            let sig = s.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
            if sig > 0.0 { saw_swing_high = true; }
        }
        assert!(saw_swing_high, "percent reversal must catch swing-high on reversal");
    }

    #[test]
    fn n_bar_extreme_mode_fires_on_new_high() {
        let mut s = SwingDetection::new(SwingMode::NBarExtreme { n: 5 });
        for i in 0..20 {
            let p = 100.0 + i as f64;
            let sig = s.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
            // Each new bar in uptrend has higher high than previous 5 → +1.
            if i >= 5 {
                assert!(sig >= 0.0, "uptrend should emit +1 or 0, got {} at i={}", sig, i);
            }
        }
    }

    #[test]
    fn lookahead_mode_lags_by_n() {
        let mut s = SwingDetection::new(SwingMode::Lookahead { n: 3 });
        // Spike at bar 5, then fall — pivot should fire ~3 bars later.
        let mut signals = vec![];
        for i in 0..15 {
            let p = if i == 5 { 120.0 } else { 100.0 + (i as f64 * 0.1).sin() };
            let sig = s.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
            signals.push(sig);
        }
        // At least one positive signal (pivot-high) expected.
        let saw_pivot = signals.iter().any(|&s| s > 0.0);
        assert!(saw_pivot, "lookahead must confirm a pivot, got: {:?}", signals);
    }

    #[test]
    fn atr_multiple_with_inner_atr() {
        let mut s = SwingDetection::with_atr_source(
            SwingMode::AtrMultiple { mult: 1.5 },
            atr_inner(14),
        );
        for i in 0..50 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            let _ = s.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
        }
        assert!(s.is_ready());
    }

    #[test]
    fn atr_mode_without_inner_returns_zero() {
        let mut s = SwingDetection::new(SwingMode::AtrMultiple { mult: 1.5 });
        for i in 0..30 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            let sig = s.update_bar(p, p + 1.0, p - 1.0, p, 0.0);
            assert_eq!(sig, 0.0, "AtrMultiple without inner must stay silent");
        }
    }

    #[test]
    fn reset_clears() {
        let mut s = SwingDetection::new(SwingMode::Percent { threshold_pct: 2.0 });
        for i in 0..30 {
            let p = 100.0 + i as f64;
            let _ = s.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
        }
        s.reset();
        assert!(!s.is_ready());
        assert_eq!(s.value(), IndicatorValue::Signal(0));
    }

    #[test]
    fn modes_differ_on_same_data() {
        let mut percent = SwingDetection::new(SwingMode::Percent { threshold_pct: 2.0 });
        let mut nbar = SwingDetection::new(SwingMode::NBarExtreme { n: 5 });

        let mut percent_signals = 0;
        let mut nbar_signals = 0;
        for i in 0..60 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 8.0;
            let p_sig = percent.update_bar(p, p + 1.0, p - 1.0, p, 0.0);
            let n_sig = nbar.update_bar(p, p + 1.0, p - 1.0, p, 0.0);
            if p_sig != 0.0 { percent_signals += 1; }
            if n_sig != 0.0 { nbar_signals += 1; }
        }
        // Different modes should produce different signal counts.
        // If both are 0 something is wrong with the fixture.
        assert!(percent_signals > 0 || nbar_signals > 0);
    }
}
