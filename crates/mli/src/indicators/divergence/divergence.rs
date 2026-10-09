//! Price/oscillator divergence — the canonical `Divergence` detector.
//!
//! Swing-pivot divergence (and optional ATR-normalised strength) over a box-free
//! inner oscillator slot (RSI / CMO / … by config). Detects regular AND hidden,
//! bullish AND bearish, via multi-pivot linear regression over detected swing
//! points. (Replaced the naive bar[now]-vs-bar[now-N] lookback variant 2026-06-23.)
//!
//! Contract output: a signal (i8) where the signal is the last
//! computed `type_signal` rounded to i8 (+2/+1/-1/-2/0).
//!
//! Internal `type_signal` encoding:
//! - `+2` Bullish Regular
//! - `+1` Bullish Hidden
//! -  `0` no divergence
//! - `-1` Bearish Hidden
//! - `-2` Bearish Regular

use crate::engine::contract_engine::OscillatorSlot;
use crate::indicators::volatility::atr::Atr;

/// A detected swing point: `(absolute_bar_index, price_value, oscillator_value)`.
type SwingPoint = (usize, f64, f64);

/// Compute the least-squares slope of a series of scalar values.
///
/// Returns the slope coefficient `b` in `y = a + b·x` where x = 0, 1, 2, …
/// Returns `0.0` for series with fewer than 2 elements.
fn compute_slope(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let n_f = n as f64;
    let sum_x: f64 = (0..n).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
    let sum_x2: f64 = (0..n).map(|i| (i as f64).powi(2)).sum();
    let denom = n_f * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-12 {
        return 0.0;
    }
    (n_f * sum_xy - sum_x * sum_y) / denom
}

/// Multi-pivot divergence via linear regression over N swing points.
///
/// Returns `Some((signal_value, direction))` where `signal_value` encodes:
/// - `+2.0` Bullish Regular (price slope < 0, osc slope > 0)
/// - `-2.0` Bearish Regular (price slope > 0, osc slope < 0)
/// - `None` otherwise.
fn detect_multi_pivot_divergence(swings: &[SwingPoint], n: usize) -> Option<f64> {
    if swings.len() < n {
        return None;
    }
    let last_n = &swings[swings.len() - n..];
    let prices: Vec<f64> = last_n.iter().map(|s| s.1).collect();
    let oscs: Vec<f64> = last_n.iter().map(|s| s.2).collect();

    let price_slope = compute_slope(&prices);
    let osc_slope = compute_slope(&oscs);

    if price_slope < 0.0 && osc_slope > 0.0 {
        Some(2.0) // Bullish Regular
    } else if price_slope > 0.0 && osc_slope < 0.0 {
        Some(-2.0) // Bearish Regular
    } else {
        None
    }
}

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
/// - `+2` Bullish Regular — price lower low, oscillator higher low
/// - `+1` Bullish Hidden  — price higher low, oscillator lower low
/// - `-1` Bearish Hidden  — price lower high, oscillator higher high
/// - `-2` Bearish Regular — price higher high, oscillator lower high
///
/// Signal is edge-detection: non-zero only on the bar where the confirming
/// swing point is detected. All other bars emit `0`.
///
/// The contract output (`value()`) returns the last signal.
#[derive(Clone)]
pub struct Divergence {
    /// Config-chosen scalar oscillator (RSI / CMO / …), fed the close. Box-free typed slot.
    inner: OscillatorSlot,
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

    /// Optional concrete ATR for strength normalisation (layer 3 only).
    atr: Option<Atr>,

    /// How many swing points to compare for divergence detection.
    /// 2 = compare only last 2 (original behaviour, pairwise comparison).
    /// 3-4 = compare last N via linear-regression slope analysis.
    compare_swings: usize,

    /// Last `(osc_value, signal, strength)` emitted — stored for `value()`.
    last_osc: f64,
    last_signal: i8,
    last_strength: f64,
}

impl std::fmt::Debug for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Divergence")
            .field("swing_lookback", &self.swing_lookback)
            .field("detect_regular", &self.detect_regular)
            .field("detect_hidden", &self.detect_hidden)
            .field("with_strength", &self.with_strength)
            .field("price_buf_len", &self.price_buf.len())
            .field("osc_buf_len", &self.osc_buf.len())
            .field("bar_counter", &self.bar_counter)
            .field("last_signal", &self.last_signal)
            .finish()
    }
}

impl Divergence {
    const BUF_CAP: usize = 512;
    const MAX_SWINGS: usize = 4;

    /// Creates a new wrapper around `inner`.
    ///
    /// - `swing_lookback`  — bars left/right of candidate for swing comparison (min 2).
    /// - `detect_regular`  — enable regular divergence signals (`±2`).
    /// - `detect_hidden`   — enable hidden divergence signals (`±1`).
    /// - `with_strength`   — emit layer 3 `Triple` output; if `false` emits layer 2 `Double`.
    /// - `atr`             — ATR instance for strength normalisation (layer 3 only).
    pub fn new(
        inner: OscillatorSlot,
        swing_lookback: usize,
        detect_regular: bool,
        detect_hidden: bool,
        with_strength: bool,
        atr: Option<Atr>,
    ) -> Self {
        Self::with_compare_swings(inner, swing_lookback, detect_regular, detect_hidden, with_strength, atr, 2)
    }

    /// Like `new` but with an explicit `compare_swings` parameter.
    pub fn with_compare_swings(
        inner: OscillatorSlot,
        swing_lookback: usize,
        detect_regular: bool,
        detect_hidden: bool,
        with_strength: bool,
        atr: Option<Atr>,
        compare_swings: usize,
    ) -> Self {
        let cs = compare_swings.clamp(2, Self::MAX_SWINGS);
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
            compare_swings: cs,
            last_osc: 0.0,
            last_signal: 0,
            last_strength: 0.0,
        }
    }

    /// Feed one OHLCV bar via the resolved lane slice `[open, high, low, close, volume]`.
    ///
    /// Stores the current divergence signal in `self.last_signal` and returns
    /// `self.value()` (a signal).
    pub fn feed(&mut self, lanes: &[f64]) {
        let (_open, high, low, close, _volume) = (lanes[0], lanes[1], lanes[2], lanes[3], lanes[4]);

        // Drive the config-chosen scalar oscillator slot with the close.
        let osc_val = self.inner.feed(close);

        // Advance the concrete ATR (needed whether or not strength is computed this bar).
        let atr_now = if let Some(atr_ind) = &mut self.atr {
            atr_ind.feed(&[high, low, close])
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
                if self.compare_swings <= 2 {
                    // Classic pairwise comparison.
                    if self.swing_highs.len() >= 2 {
                        let n = self.swing_highs.len();
                        let s0 = self.swing_highs[n - 2];
                        let s1 = self.swing_highs[n - 1];

                        let bearish_regular =
                            self.detect_regular && s1.1 > s0.1 && s1.2 < s0.2;
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
                } else if self.detect_regular && self.swing_highs.len() >= self.compare_swings {
                    // Multi-pivot: linear regression over last N swing-high points.
                    if let Some(sig) = detect_multi_pivot_divergence(&self.swing_highs, self.compare_swings) {
                        new_signal = sig;
                        if self.with_strength && self.swing_highs.len() >= 2 {
                            let n = self.swing_highs.len();
                            let s0 = self.swing_highs[n - 2];
                            let s1 = self.swing_highs[n - 1];
                            new_strength = self.calc_strength(&s0, &s1, atr_now);
                        }
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
                if self.compare_swings <= 2 {
                    // Classic pairwise comparison.
                    if self.swing_lows.len() >= 2 {
                        let n = self.swing_lows.len();
                        let s0 = self.swing_lows[n - 2];
                        let s1 = self.swing_lows[n - 1];

                        let bull_regular =
                            self.detect_regular && s1.1 < s0.1 && s1.2 > s0.2;
                        let bull_hidden =
                            self.detect_hidden && s1.1 > s0.1 && s1.2 < s0.2;

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
                } else if self.detect_regular && new_signal == 0.0
                    && self.swing_lows.len() >= self.compare_swings
                {
                    // Multi-pivot: linear regression over last N swing-low points.
                    if let Some(sig) = detect_multi_pivot_divergence(&self.swing_lows, self.compare_swings) {
                        new_signal = sig;
                        if self.with_strength && self.swing_lows.len() >= 2 {
                            let n = self.swing_lows.len();
                            let s0 = self.swing_lows[n - 2];
                            let s1 = self.swing_lows[n - 1];
                            new_strength = self.calc_strength(&s0, &s1, atr_now);
                        }
                    }
                }
            }
        }

        // Store the (osc_value, signal, strength) triple for the contract output.
        self.last_osc = osc_val;
        self.last_signal = new_signal as i8;
        self.last_strength = new_strength;

    }


    /// Named output: brace `line` (the inner oscillator value).
    pub fn line(&self) -> f64 {
        self.last_osc
    }

    /// Named getter for the `signal` brace output (divergence signal: +2/+1/-1/-2/0).
    pub fn signal(&self) -> f64 {
        self.last_signal as f64
    }

    /// Named getter for the `strength` brace output (normalised strength in [0, 1]).
    pub fn strength(&self) -> f64 {
        self.last_strength
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
        self.last_osc = 0.0;
        self.last_signal = 0;
        self.last_strength = 0.0;
        if let Some(atr) = &mut self.atr {
            atr.reset();
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

    /// Number of swing points used for divergence analysis.
    pub fn compare_swings(&self) -> usize {
        self.compare_swings
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, OscillatorSlotOrder, SmootherId, SmootherChoice};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, Slot, SourceAxis, Store, StoreKind,
    UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Divergence`] — the inner oscillator (a config-chosen
/// scalar oscillator SLOT), swing detection parameters, and OPTIONAL ATR parameters
/// for layer-3 strength normalisation (`use_atr=false` → no ATR built).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct DivergenceConfig {
    /// The inner oscillator — a config-chosen slot over the `+oscillator` family; carries
    /// its OWN period (no host to follow), so it stays an `OscillatorSlotOrder` (not a
    /// follow/own smoother choice).
    #[slot]
    pub oscillator: Param<OscillatorSlotOrder>,
    pub swing_lookback: Param<usize>,
    pub detect_regular: Param<bool>,
    pub detect_hidden: Param<bool>,
    pub with_strength: Param<bool>,
    /// Whether to build a concrete ATR for strength normalisation (layer 3).
    pub use_atr: Param<bool>,
    /// ATR period (used when `use_atr=true`).
    pub atr_period: Param<usize>,
    /// ATR smoother (used when `use_atr=true`; default `follow(Rma)`).
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for Divergence {
    const ID: IndicatorId = IndicatorId::Divergence;
    /// No family — a composite DETECTOR over an inner oscillator, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full O/H/L/C/V — the oscillator slot is fed close; the optional ATR uses H/L/C.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// Two rolling price/osc buffers (capped at BUF_CAP); the oscillator is priced
    /// through the slot.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const SLOTS: &'static [Slot] = DivergenceConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::DivergenceLine),
        Output::ordinal(IndicatorOutputId::DivergenceSignal),
        Output::percent(IndicatorOutputId::DivergenceStrength),
    ];
    type Config = DivergenceConfig;
    type Runtime = Divergence;

    fn create(cfg: DivergenceConfig) -> Divergence {
        let atr = if cfg.use_atr.resolved() {
            let choice = cfg.atr_smoother.resolved();
            let period = choice.period.resolve(cfg.atr_period.resolved());
            Some(Atr::from_smoother(period, choice.kind))
        } else {
            None
        };
        Divergence::new(
            cfg.oscillator.resolved().into_slot(),
            cfg.swing_lookback.resolved(),
            cfg.detect_regular.resolved(),
            cfg.detect_hidden.resolved(),
            cfg.with_strength.resolved(),
            atr,
        )
    }

    fn slot_members(cfg: &DivergenceConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for DivergenceConfig {
    fn defaults() -> Self {
        DivergenceConfig {
            oscillator: Param::Solo(OscillatorSlotOrder::Rsi(PeriodConfig { period: 14 })),
            swing_lookback: Param::Solo(14),
            detect_regular: Param::Solo(true),
            detect_hidden: Param::Solo(true),
            with_strength: Param::Solo(false),
            use_atr: Param::Solo(false),
            atr_period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // detect_regular / detect_hidden / with_strength / use_atr: Class R bool — auto [false,true].
        // swing_lookback / atr_period: Class A period — auto range(2,4048,1).
        // atr_smoother: Param<SmootherChoice> — deferred wave (smoother sweep). [FLAG: smoother slot, deferred wave]
        // oscillator: Param<OscillatorSlotOrder> — #[slot], nested oscillator + inner period (deferred wave). [FLAG: oscillator slot order, deferred wave]
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Divergence {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DivergenceLine, "Oscillator", Color::hex(0xE91E63))
            .line_output(IndicatorOutputId::DivergenceSignal, "Divergence", Color::hex(0xFF9800))
            .precision(2)
            .build()
    }
}


// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::{IndicatorOrder, OscillatorSlotOrder, SmootherId};
    use crate::indicators::average::moving_average::PeriodConfig;
    use crate::indicators::volatility::atr::Atr;

    /// Build the inner oscillator slot directly from its narrow order.
    fn osc_inner(period: usize) -> OscillatorSlot {
        OscillatorSlotOrder::Rsi(PeriodConfig { period }).into_slot()
    }

    fn atr_concrete() -> Atr {
        Atr::from_smoother(14, SmootherId::Sma)
    }

    #[test]
    fn smoke_triple_output() {
        let mut ind = Divergence::new(osc_inner(14), 3, true, true, false, None);
        for i in 0..40u32 {
            let p = 100.0 + (i as f64 * 0.5).sin() * 8.0;
            ind.feed(&[p, p + 1.0, p - 1.0, p, 1000.0]);
        }
        assert!(ind.line().is_finite());
        assert!(ind.signal().is_finite());
        assert!(ind.strength().is_finite());
    }

    #[test]
    fn layer_3_runs_with_atr() {
        // with_strength=true + concrete ATR — runs without panic.
        let mut ind =
            Divergence::new(osc_inner(14), 3, true, true, true, Some(atr_concrete()));
        for i in 0..60u32 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0;
            ind.feed(&[p, p + 1.0, p - 1.0, p, 1000.0]);
            assert!(ind.signal().is_finite());
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_buffers() {
        let mut ind = Divergence::new(osc_inner(14), 3, true, true, false, None);
        for i in 0..20u32 {
            let p = 100.0 + i as f64;
            ind.feed(&[p, p, p, p, 1000.0]);
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.line(), 0.0);
        assert_eq!(ind.signal(), 0.0);
        assert_eq!(ind.strength(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_osc_divergence() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Divergence(<<Divergence as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..120 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0;
            f.feed(0, MarketSample::Bar { open: p, high: p + 0.5, low: p - 0.5, close: p, volume: 1000.0 });
        }
        // factory value() returns the primary output (line = inner oscillator value)
        assert!(f.primary().is_finite());
    }
}
