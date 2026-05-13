//! Universal oscillator-with-divergence wrapper.
//!
//! Adds divergence detection (and optionally strength) to **any** oscillator
//! backed by `IndicatorInstance`. Works as layer 2 or layer 3 on top of any
//! oscillator that produces a scalar value.
//!
//! Output layers:
//! - `with_strength = false`: `IndicatorValue::Double(osc_value, type_signal)`
//! - `with_strength = true`:  `IndicatorValue::Triple(osc_value, type_signal, strength)`
//!
//! `type_signal` encoding:
//! - `+2.0` Bullish Regular
//! - `+1.0` Bullish Hidden
//! -  `0.0` no divergence
//! - `-1.0` Bearish Hidden
//! - `-2.0` Bearish Regular

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;

/// A detected swing point: `(absolute_bar_index, price_value, oscillator_value)`.
type SwingPoint = (usize, f64, f64);

/// Computes divergence strength from two swing points.
///
/// Returned value is in `[0.0, 1.0]`.
fn compute_strength(
    s0: &SwingPoint,
    s1: &SwingPoint,
    osc_range: f64,
    price_range: f64,
    atr_value: Option<f64>,
    price_mean_in_window: f64,
) -> f64 {
    const EPS: f64 = 1e-9;

    let delta_osc = (s1.2 - s0.2).abs();
    let delta_price = (s1.1 - s0.1).abs();

    let osc_norm = if osc_range > EPS {
        delta_osc / osc_range
    } else {
        0.0
    };
    let price_norm = if price_range > EPS {
        delta_price / price_range
    } else {
        0.0
    };
    let angle_score = if price_norm > EPS {
        (osc_norm / price_norm).min(1.0)
    } else {
        0.0
    };

    let swing_quality = match atr_value {
        Some(atr) if atr > EPS => {
            ((s1.1 - price_mean_in_window).abs() / atr).min(1.0)
        }
        _ => 0.0,
    };

    let distance_bars = (s1.0.saturating_sub(s0.0)) as f64;
    let distance_score = (-((distance_bars - 10.0) / 5.0).powi(2)).exp();

    (0.4 * angle_score + 0.3 * swing_quality + 0.3 * distance_score).clamp(0.0, 1.0)
}

/// Wraps any oscillator indicator with swing-point based divergence detection
/// and optional strength scoring.
///
/// Divergence types emitted via `type_signal`:
/// - `+2.0` Bullish Regular — price lower low, oscillator higher low
/// - `+1.0` Bullish Hidden  — price higher low, oscillator lower low
/// - `-1.0` Bearish Hidden  — price lower high, oscillator higher high
/// - `-2.0` Bearish Regular — price higher high, oscillator lower high
///
/// Signal is edge-detection: non-zero only on the bar where the confirming
/// swing point is detected. All other bars emit `0.0`.
#[derive(Clone)]
pub struct OscillatorWithDivergence {
    inner: Box<IndicatorInstance>,
    swing_lookback: usize,
    detect_regular: bool,
    detect_hidden: bool,
    with_strength: bool,

    /// Rolling close-price buffer (capped at `BUF_CAP`).
    price_buf: Vec<f64>,
    /// Rolling oscillator-value buffer (capped at `BUF_CAP`).
    osc_buf: Vec<f64>,

    /// Last 4 swing-high points `(abs_bar_idx, price, osc)`.
    swing_highs: Vec<SwingPoint>,
    /// Last 4 swing-low points `(abs_bar_idx, price, osc)`.
    swing_lows: Vec<SwingPoint>,

    /// Total bars fed (used to map buf positions to absolute indices).
    bar_counter: usize,

    /// Optional ATR for strength normalisation (layer 3 only).
    atr: Option<Box<IndicatorInstance>>,
}

impl std::fmt::Debug for OscillatorWithDivergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OscillatorWithDivergence")
            .field("swing_lookback", &self.swing_lookback)
            .field("detect_regular", &self.detect_regular)
            .field("detect_hidden", &self.detect_hidden)
            .field("with_strength", &self.with_strength)
            .field("price_buf_len", &self.price_buf.len())
            .field("osc_buf_len", &self.osc_buf.len())
            .field("bar_counter", &self.bar_counter)
            .finish()
    }
}

impl OscillatorWithDivergence {
    const BUF_CAP: usize = 512;
    const MAX_SWINGS: usize = 4;

    /// Creates a new wrapper around `inner`.
    ///
    /// - `swing_lookback` — bars left/right of candidate for swing comparison (min 2).
    /// - `detect_regular` — enable regular divergence signals (`±2`).
    /// - `detect_hidden`  — enable hidden divergence signals (`±1`).
    /// - `with_strength`  — emit layer 3 `Triple` output; if `false` emits layer 2 `Double`.
    /// - `atr`            — ATR instance for strength normalisation (layer 3 only).
    pub fn new(
        inner: Box<IndicatorInstance>,
        swing_lookback: usize,
        detect_regular: bool,
        detect_hidden: bool,
        with_strength: bool,
        atr: Option<Box<IndicatorInstance>>,
    ) -> Self {
        Self {
            inner,
            swing_lookback: swing_lookback.max(2),
            detect_regular,
            detect_hidden,
            with_strength,
            price_buf: Vec::with_capacity(Self::BUF_CAP),
            osc_buf: Vec::with_capacity(Self::BUF_CAP),
            swing_highs: Vec::with_capacity(Self::MAX_SWINGS + 1),
            swing_lows: Vec::with_capacity(Self::MAX_SWINGS + 1),
            bar_counter: 0,
            atr,
        }
    }

    /// Feed one OHLCV bar. Returns `Double(osc, signal)` or `Triple(osc, signal, strength)`.
    ///
    /// `signal` is non-zero only on the bar where a new confirming swing is detected.
    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> IndicatorValue {
        let osc_val = self
            .inner
            .update_bar(open, high, low, close, volume, None)
            .main();

        // Advance ATR (needed whether or not strength is computed this bar).
        let atr_now = if let Some(atr_ind) = &mut self.atr {
            atr_ind.update_bar(open, high, low, close, volume, None).main()
        } else {
            0.0
        };

        // Rolling buffer — cap at BUF_CAP.
        if self.price_buf.len() >= Self::BUF_CAP {
            self.price_buf.remove(0);
            self.osc_buf.remove(0);
        }
        self.price_buf.push(close);
        self.osc_buf.push(osc_val);
        self.bar_counter += 1;

        let mut new_signal: f64 = 0.0;
        let mut new_strength: f64 = 0.0;

        // Need at least 2*lookback+1 bars in the buffer to test the centre bar.
        let min_len = 2 * self.swing_lookback + 1;
        if self.price_buf.len() >= min_len {
            let buf_len = self.price_buf.len();
            // Index within price_buf of the candidate swing bar.
            let check_idx = buf_len - self.swing_lookback - 1;
            // Corresponding absolute bar index.
            // bar_counter was just incremented, so the newest bar is at absolute index
            // (bar_counter - 1). The check_idx bar is swing_lookback bars older.
            let abs_idx = self.bar_counter - 1 - self.swing_lookback;

            let center_price = self.price_buf[check_idx];
            let center_osc = self.osc_buf[check_idx];

            let mut is_high = true;
            let mut is_low = true;
            let lo = check_idx.saturating_sub(self.swing_lookback);
            let hi = (check_idx + self.swing_lookback).min(buf_len - 1);
            for i in lo..=hi {
                if i == check_idx {
                    continue;
                }
                if self.price_buf[i] >= center_price {
                    is_high = false;
                }
                if self.price_buf[i] <= center_price {
                    is_low = false;
                }
            }

            // --- Swing high → bearish divergence ---
            if is_high {
                self.swing_highs.push((abs_idx, center_price, center_osc));
                if self.swing_highs.len() > Self::MAX_SWINGS {
                    self.swing_highs.remove(0);
                }
                if self.swing_highs.len() >= 2 {
                    let n = self.swing_highs.len();
                    let s0 = self.swing_highs[n - 2];
                    let s1 = self.swing_highs[n - 1];

                    // Bearish Regular: price[1] > price[0] AND osc[1] < osc[0]
                    let bearish_regular =
                        self.detect_regular && s1.1 > s0.1 && s1.2 < s0.2;
                    // Bearish Hidden: price[1] < price[0] AND osc[1] > osc[0]
                    let bearish_hidden =
                        self.detect_hidden && s1.1 < s0.1 && s1.2 > s0.2;

                    if bearish_regular {
                        new_signal = -2.0;
                    } else if bearish_hidden {
                        new_signal = -1.0;
                    }

                    if new_signal != 0.0 && self.with_strength {
                        new_strength = self.calc_strength(&s0, &s1, atr_now);
                    }
                }
            }

            // --- Swing low → bullish divergence ---
            // Regular overrides hidden, so only update signal if not already -2/-1.
            if is_low {
                self.swing_lows.push((abs_idx, center_price, center_osc));
                if self.swing_lows.len() > Self::MAX_SWINGS {
                    self.swing_lows.remove(0);
                }
                if self.swing_lows.len() >= 2 {
                    let n = self.swing_lows.len();
                    let s0 = self.swing_lows[n - 2];
                    let s1 = self.swing_lows[n - 1];

                    // Bullish Regular: price[1] < price[0] AND osc[1] > osc[0]
                    let bull_regular =
                        self.detect_regular && s1.1 < s0.1 && s1.2 > s0.2;
                    // Bullish Hidden: price[1] > price[0] AND osc[1] < osc[0]
                    let bull_hidden =
                        self.detect_hidden && s1.1 > s0.1 && s1.2 < s0.2;

                    // Prioritise Regular > Hidden; bullish only if no bearish signal yet.
                    if new_signal == 0.0 {
                        if bull_regular {
                            new_signal = 2.0;
                        } else if bull_hidden {
                            new_signal = 1.0;
                        }

                        if new_signal != 0.0 && self.with_strength {
                            new_strength = self.calc_strength(&s0, &s1, atr_now);
                        }
                    }
                }
            }
        }

        if self.with_strength {
            IndicatorValue::Triple(osc_val, new_signal, new_strength)
        } else {
            IndicatorValue::Double(osc_val, new_signal)
        }
    }

    /// Compute strength for a pair of swing points, using the buf window between them.
    fn calc_strength(&self, s0: &SwingPoint, s1: &SwingPoint, atr_val: f64) -> f64 {
        // Oldest absolute index stored in price_buf.
        let oldest_abs = self.bar_counter.saturating_sub(self.price_buf.len());

        // Check both swings are still inside the rolling buffer.
        if s0.0 < oldest_abs || s1.0 < oldest_abs {
            return 0.0;
        }

        let i0 = s0.0 - oldest_abs;
        let i1 = s1.0 - oldest_abs;

        if i0 >= self.price_buf.len() || i1 >= self.price_buf.len() || i0 > i1 {
            return 0.0;
        }

        let price_slice = &self.price_buf[i0..=i1];
        let osc_slice = &self.osc_buf[i0..=i1];

        let price_max = price_slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let price_min = price_slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let osc_max = osc_slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let osc_min = osc_slice.iter().cloned().fold(f64::INFINITY, f64::min);

        let price_range = price_max - price_min;
        let osc_range = osc_max - osc_min;

        let price_mean = if price_slice.is_empty() {
            0.0
        } else {
            price_slice.iter().sum::<f64>() / price_slice.len() as f64
        };

        let atr_opt = if self.atr.is_some() && atr_val > 1e-9 {
            Some(atr_val)
        } else {
            None
        };

        compute_strength(s0, s1, osc_range, price_range, atr_opt, price_mean)
    }

    /// Returns the last computed `IndicatorValue` without advancing state.
    pub fn value(&self) -> IndicatorValue {
        let osc_val = self.osc_buf.last().copied().unwrap_or(0.0);
        if self.with_strength {
            IndicatorValue::Triple(osc_val, 0.0, 0.0)
        } else {
            IndicatorValue::Double(osc_val, 0.0)
        }
    }

    /// Returns `true` once the inner oscillator has warmed up.
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    /// Clears all internal state.
    pub fn reset(&mut self) {
        self.inner.reset();
        self.price_buf.clear();
        self.osc_buf.clear();
        self.swing_highs.clear();
        self.swing_lows.clear();
        self.bar_counter = 0;
        if let Some(atr) = &mut self.atr {
            atr.reset();
        }
    }

    /// Returns `swing_lookback`.
    pub fn swing_lookback(&self) -> usize {
        self.swing_lookback
    }

    /// `true` if regular divergence detection is enabled.
    pub fn detects_regular(&self) -> bool {
        self.detect_regular
    }

    /// `true` if hidden divergence detection is enabled.
    pub fn detects_hidden(&self) -> bool {
        self.detect_hidden
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    /// Build an RSI(7) wrapped in IndicatorInstance.
    fn rsi_inner(period: usize) -> Box<IndicatorInstance> {
        Box::new(
            IndicatorInstance::create(&IndicatorConfig::new(
                BarIndicatorId::Rsi,
                "Rsi".into(),
                vec![period],
            ))
            .expect("RSI creation must succeed"),
        )
    }

    /// Build an ATR(14) wrapped in IndicatorInstance.
    fn atr_inner() -> Box<IndicatorInstance> {
        Box::new(
            IndicatorInstance::create(&IndicatorConfig::new(
                BarIndicatorId::Atr,
                "Atr".into(),
                vec![14],
            ))
            .expect("ATR creation must succeed"),
        )
    }

    /// Feed a bar with identical OHLC (flat candle).
    fn feed(ind: &mut OscillatorWithDivergence, close: f64, volume: f64) -> IndicatorValue {
        ind.update_bar(close, close + 0.5, close - 0.5, close, volume)
    }

    // ── Smoke tests (preserved from phase 1) ──────────────────────────────────

    #[test]
    fn smoke_layer_2_returns_double() {
        let mut ind =
            OscillatorWithDivergence::new(rsi_inner(14), 3, true, true, false, None);
        let mut last = IndicatorValue::Single(0.0);
        for i in 0..20u32 {
            let p = 100.0 + i as f64;
            last = ind.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
        }
        match last {
            IndicatorValue::Double(_, _) => {}
            other => panic!("expected Double, got {:?}", other),
        }
    }

    #[test]
    fn smoke_layer_3_returns_triple() {
        let mut ind =
            OscillatorWithDivergence::new(rsi_inner(14), 3, true, true, true, None);
        let mut last = IndicatorValue::Single(0.0);
        for i in 0..20u32 {
            let p = 100.0 + i as f64;
            last = ind.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
        }
        match last {
            IndicatorValue::Triple(_, _, _) => {}
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_buffers() {
        let mut ind = OscillatorWithDivergence::new(rsi_inner(14), 3, true, true, false, None);
        for i in 0..20u32 {
            let p = 100.0 + i as f64;
            ind.update_bar(p, p, p, p, 1000.0);
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
    }

    // ── Divergence detection tests ─────────────────────────────────────────────
    //
    // Strategy: SMA(5) as inner oscillator with swing_lookback=3.
    //
    // SMA(5) at bar i = average of close[i-4 .. i].
    //
    // To engineer exact oscillator values at swing centres we surround the
    // swing bar with known constant prices.  For a swing-low at position C:
    //   bars C-4 .. C-1 = SURROUND_PRICE   (these set SMA baseline)
    //   bar  C          = DIP_PRICE         (this is the swing centre)
    //   SMA at C        = (4 * SURROUND + DIP) / 5
    //
    // With lookback=3, detection fires on bar C+3 (3 bars after the centre).
    // We add 3 bars of SURROUND_PRICE after DIP to satisfy the swing condition
    // (all neighbours strictly larger).
    //
    // Flat warm-up (8 bars) primes the SMA without forming any swing points.

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

    fn collect_signals(vals: &[IndicatorValue]) -> Vec<f64> {
        vals.iter()
            .map(|v| match v {
                IndicatorValue::Double(_, s) => *s,
                IndicatorValue::Triple(_, s, _) => *s,
                _ => 0.0,
            })
            .collect()
    }

    /// Feed `n` flat bars at `price`.
    fn feed_flat(
        ind: &mut OscillatorWithDivergence,
        price: f64,
        n: usize,
    ) -> Vec<IndicatorValue> {
        (0..n).map(|_| feed(ind, price, 1000.0)).collect()
    }

    /// Build one swing-low segment:
    ///   4 bars at `surround`, 1 bar at `dip`, 3 bars at `surround`.
    /// Returns 8 IndicatorValues (detection fires on the last one).
    fn swing_low_segment(
        ind: &mut OscillatorWithDivergence,
        dip: f64,
        surround: f64,
    ) -> Vec<IndicatorValue> {
        let mut out = feed_flat(ind, surround, 4);
        out.push(feed(ind, dip, 1000.0));
        out.extend(feed_flat(ind, surround, 3));
        out
    }

    /// Build one swing-high segment:
    ///   4 bars at `surround`, 1 bar at `peak`, 3 bars at `surround`.
    fn swing_high_segment(
        ind: &mut OscillatorWithDivergence,
        peak: f64,
        surround: f64,
    ) -> Vec<IndicatorValue> {
        let mut out = feed_flat(ind, surround, 4);
        out.push(feed(ind, peak, 1000.0));
        out.extend(feed_flat(ind, surround, 3));
        out
    }

    // ── Swing-low SMA formula helper (SMA period=5):
    //   sma = (4 * surround + dip) / 5
    //   We use this in comments to verify the maths.

    #[test]
    fn synthetic_bullish_regular_divergence() {
        // Bullish Regular: price lower low, SMA higher low  → signal +2
        //
        // Swing low 1: dip=80, surround=120  → SMA = (480+80)/5 = 112
        // Swing low 2: dip=70, surround=150  → SMA = (600+70)/5 = 134  (134>112, 70<80) ✓
        let mut ind =
            OscillatorWithDivergence::new(sma_inner(5), 3, true, true, false, None);
        let mut all: Vec<IndicatorValue> = Vec::new();

        // Warm-up: 8 flat bars so SMA stabilises without creating swing points.
        all.extend(feed_flat(&mut ind, 100.0, 8));

        // Inter-segment separator: 4 bars at a neutral level so the next swing's
        // 4-bar pre-surround is clean.
        all.extend(feed_flat(&mut ind, 110.0, 4));

        // Swing low 1
        all.extend(swing_low_segment(&mut ind, 80.0, 120.0));

        // Separator between swings (must be > both dip values to avoid false lows)
        all.extend(feed_flat(&mut ind, 140.0, 4));

        // Swing low 2
        all.extend(swing_low_segment(&mut ind, 70.0, 150.0));

        let signals = collect_signals(&all);
        assert!(
            signals.iter().any(|&s| s == 2.0),
            "expected +2.0 (bullish regular); signals: {:?}",
            signals
        );
        assert!(
            signals.iter().all(|&s| s >= 0.0),
            "unexpected bearish signal; signals: {:?}",
            signals
        );
    }

    #[test]
    fn synthetic_bearish_regular_divergence() {
        // Bearish Regular: price higher high, SMA lower high  → signal -2
        //
        // Swing high 1: peak=120, surround=80   → SMA = (320+120)/5 = 88
        // Swing high 2: peak=130, surround=60   → SMA = (240+130)/5 = 74  (74<88, 130>120) ✓
        let mut ind =
            OscillatorWithDivergence::new(sma_inner(5), 3, true, true, false, None);
        let mut all: Vec<IndicatorValue> = Vec::new();

        all.extend(feed_flat(&mut ind, 100.0, 8));
        all.extend(feed_flat(&mut ind, 90.0, 4));

        // Swing high 1
        all.extend(swing_high_segment(&mut ind, 120.0, 80.0));

        // Separator (must be < both peak values to avoid false highs)
        all.extend(feed_flat(&mut ind, 55.0, 4));

        // Swing high 2
        all.extend(swing_high_segment(&mut ind, 130.0, 60.0));

        let signals = collect_signals(&all);
        assert!(
            signals.iter().any(|&s| s == -2.0),
            "expected -2.0 (bearish regular); signals: {:?}",
            signals
        );
        assert!(
            signals.iter().all(|&s| s <= 0.0),
            "unexpected bullish signal; signals: {:?}",
            signals
        );
    }

    #[test]
    fn synthetic_bullish_hidden_divergence() {
        // Bullish Hidden: price higher low, SMA lower low  → signal +1
        //
        // surround > dip is required for a valid swing low.
        //
        // Swing low 1: dip=80, surround=150  → SMA = (4*150+80)/5  = 136.0
        // Swing low 2: dip=90, surround=120  → SMA = (4*120+90)/5  = 114.0
        //   (114 < 136, 90 > 80) ✓  → Bullish Hidden
        //
        // Separator=130 (between surrounds): 130 > 120 (right neighbour) so 130
        // cannot be a local minimum; 130 < 150 (left neighbour) so it cannot be
        // a local maximum either.
        let mut ind =
            OscillatorWithDivergence::new(sma_inner(5), 3, true, true, false, None);
        let mut all: Vec<IndicatorValue> = Vec::new();

        all.extend(feed_flat(&mut ind, 100.0, 8));
        all.extend(feed_flat(&mut ind, 145.0, 4));

        // Swing low 1
        all.extend(swing_low_segment(&mut ind, 80.0, 150.0));

        // Separator: between 120 and 150, prevents spurious swing points.
        all.extend(feed_flat(&mut ind, 130.0, 4));

        // Swing low 2
        all.extend(swing_low_segment(&mut ind, 90.0, 120.0));

        let signals = collect_signals(&all);
        assert!(
            signals.iter().any(|&s| s == 1.0),
            "expected +1.0 (bullish hidden); signals: {:?}",
            signals
        );
        assert!(
            signals.iter().all(|&s| s >= 0.0),
            "unexpected bearish signal; signals: {:?}",
            signals
        );
    }

    #[test]
    fn synthetic_bearish_hidden_divergence() {
        // Bearish Hidden: price lower high (downtrend rally), SMA higher high  → signal -1
        //
        // Constraint: surround < peak for a valid swing-high.
        // Swing high 1: peak=120, surround=80  → SMA = (320+120)/5 = 88
        // Swing high 2: peak=110, surround=90  → SMA = (360+110)/5 = 94  (94>88, 110<120) ✓
        //
        // surround=90 < peak=110 ✓  (110 is a true local maximum)
        let mut ind =
            OscillatorWithDivergence::new(sma_inner(5), 3, true, true, false, None);
        let mut all: Vec<IndicatorValue> = Vec::new();

        all.extend(feed_flat(&mut ind, 100.0, 8));
        all.extend(feed_flat(&mut ind, 75.0, 4));

        // Swing high 1
        all.extend(swing_high_segment(&mut ind, 120.0, 80.0));

        // Separator must be < both peaks (< 110): use 85.
        all.extend(feed_flat(&mut ind, 85.0, 4));

        // Swing high 2
        all.extend(swing_high_segment(&mut ind, 110.0, 90.0));

        let signals = collect_signals(&all);
        assert!(
            signals.iter().any(|&s| s == -1.0),
            "expected -1.0 (bearish hidden); signals: {:?}",
            signals
        );
        assert!(
            signals.iter().all(|&s| s <= 0.0),
            "unexpected bullish signal; signals: {:?}",
            signals
        );
    }

    #[test]
    fn no_divergence_emits_zero_on_monotonic_up() {
        // Strict monotonic uptrend: no swing lows, no divergence.
        let mut ind =
            OscillatorWithDivergence::new(sma_inner(5), 3, true, true, false, None);

        let signals: Vec<f64> = (0..60u32)
            .map(|i| {
                let p = 100.0 + i as f64 * 1.5;
                match feed(&mut ind, p, 1000.0) {
                    IndicatorValue::Double(_, s) => s,
                    other => panic!("unexpected {:?}", other),
                }
            })
            .collect();

        assert!(
            signals.iter().all(|&s| s == 0.0),
            "monotonic uptrend should emit no divergence signals; got: {:?}",
            signals
        );
    }

    #[test]
    fn layer_3_returns_triple_with_positive_strength() {
        // Bullish-regular pattern with with_strength=true + ATR.
        // Signal bar must have Triple output and strength > 0.
        let mut ind = OscillatorWithDivergence::new(
            sma_inner(5),
            3,
            true,
            true,
            true, // layer 3
            Some(atr_inner()),
        );
        let mut all: Vec<IndicatorValue> = Vec::new();

        all.extend(feed_flat(&mut ind, 100.0, 8));
        all.extend(feed_flat(&mut ind, 110.0, 4));
        all.extend(swing_low_segment(&mut ind, 80.0, 120.0));
        all.extend(feed_flat(&mut ind, 140.0, 4));
        all.extend(swing_low_segment(&mut ind, 70.0, 150.0));

        let signal_bars: Vec<(f64, f64)> = all
            .iter()
            .filter_map(|v| match v {
                IndicatorValue::Triple(_, s, str_) if *s != 0.0 => Some((*s, *str_)),
                _ => None,
            })
            .collect();

        assert!(
            !signal_bars.is_empty(),
            "expected at least one non-zero signal in layer-3 mode"
        );
        assert!(
            signal_bars.iter().any(|(_, str_)| *str_ > 0.0),
            "expected strength > 0.0 on signal bar; got: {:?}",
            signal_bars
        );

        for v in &all {
            assert!(
                matches!(v, IndicatorValue::Triple(_, _, _)),
                "expected Triple for all bars, got {:?}",
                v
            );
        }
    }

    #[test]
    fn detect_regular_false_suppresses_regular_signal() {
        // Same bullish-regular pattern but detect_regular=false → no +2 signal.
        let mut ind =
            OscillatorWithDivergence::new(sma_inner(5), 3, false, true, false, None);
        let mut all: Vec<IndicatorValue> = Vec::new();

        all.extend(feed_flat(&mut ind, 100.0, 8));
        all.extend(feed_flat(&mut ind, 110.0, 4));
        all.extend(swing_low_segment(&mut ind, 80.0, 120.0));
        all.extend(feed_flat(&mut ind, 140.0, 4));
        all.extend(swing_low_segment(&mut ind, 70.0, 150.0));

        let signals = collect_signals(&all);
        assert!(
            signals.iter().all(|&s| s != 2.0),
            "detect_regular=false must suppress +2.0 signals; got: {:?}",
            signals
        );
    }
}
