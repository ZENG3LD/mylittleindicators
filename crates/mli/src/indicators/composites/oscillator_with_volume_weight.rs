//! Universal oscillator-with-volume-weight wrapper.
//!
//! Compares price movement with volume to detect:
//! - Volume confirmation (price and volume move in same direction)
//! - Volume divergence (price moves but volume is below baseline — weak move)
//! - Volume spikes (unusually high volume)
//!
//! Outputs: `inner_value`, `type_signal`, `strength`.
//! - `inner_value` — the config-chosen oscillator's scalar (the primary line).
//! - `type_signal` — the volume-event classification (-3 .. +3):
//!   `+3` bullish spike, `+2` confirmation up, `+1` divergence up (weak), `0` none,
//!   `-1` divergence down, `-2` confirmation down, `-3` bearish spike.
//! - `strength` — normalised volume strength in `[0, 1]` (0 at baseline, 1 at spike).

use std::collections::VecDeque;

use crate::engine::contract_engine::OscillatorSlot;

/// Wraps a config-chosen scalar oscillator with volume-event classification.
///
/// The wrapper observes the last `baseline_period` bars to build a rolling volume
/// baseline (mean). Each new bar is classified by comparing the bar's volume against
/// the baseline and the price direction.
#[derive(Clone)]
pub struct OscillatorWithVolumeWeight {
    /// Config-chosen scalar oscillator (RSI / CMO / …), fed the close. Box-free typed slot.
    inner: OscillatorSlot,
    baseline_period: usize,
    spike_threshold: f64,

    volume_history: VecDeque<f64>,
    prev_close: f64,
    has_prev: bool,

    last_inner: f64,
    last_type: i8,
    last_strength: f64,
}

impl std::fmt::Debug for OscillatorWithVolumeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OscillatorWithVolumeWeight")
            .field("baseline_period", &self.baseline_period)
            .field("spike_threshold", &self.spike_threshold)
            .field("history_len", &self.volume_history.len())
            .field("has_prev", &self.has_prev)
            .field("last_type", &self.last_type)
            .finish()
    }
}

impl OscillatorWithVolumeWeight {
    /// Create a new wrapper.
    ///
    /// - `inner`            — the config-chosen scalar oscillator slot, fed the close.
    /// - `baseline_period`  — rolling window for volume mean (minimum 2).
    /// - `spike_threshold`  — multiplier above which volume is a spike (e.g. `2.5`).
    pub fn new(inner: OscillatorSlot, baseline_period: usize, spike_threshold: f64) -> Self {
        Self {
            inner,
            baseline_period: baseline_period.max(2),
            spike_threshold: spike_threshold.max(1.01),
            volume_history: VecDeque::with_capacity(baseline_period.max(2) + 1),
            prev_close: 0.0,
            has_prev: false,
            last_inner: 0.0,
            last_type: 0,
            last_strength: 0.0,
        }
    }

    /// Feed the resolved input lanes — `[open, high, low, close, volume]`. The oscillator
    /// slot is fed the close; the volume event is classified against the rolling baseline.
    pub fn feed(&mut self, lanes: &[f64]) {
        let (_open, _high, _low, close, volume) =
            (lanes[0], lanes[1], lanes[2], lanes[3], lanes[4]);

        self.last_inner = self.inner.feed(close);

        // Accumulate volume history (includes current bar).
        self.volume_history.push_back(volume);
        if self.volume_history.len() > self.baseline_period {
            self.volume_history.pop_front();
        }

        // Warm-up: not enough history yet.
        if self.volume_history.len() < self.baseline_period {
            self.last_type = 0;
            self.last_strength = 0.0;
            return;
        }

        // Baseline = mean of all bars in history except the current one.
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

        // First bar after warm-up: set prev_close, no classification yet.
        if !self.has_prev {
            self.prev_close = close;
            self.has_prev = true;
            self.last_type = 0;
            self.last_strength = 0.0;
            return;
        }

        let price_dir = if close > self.prev_close + EPS {
            1i8
        } else if close < self.prev_close - EPS {
            -1i8
        } else {
            0i8
        };

        let type_signal: i8 = if price_dir == 0 {
            0
        } else if volume_ratio >= self.spike_threshold {
            price_dir * 3
        } else if volume_ratio > 1.0 {
            price_dir * 2
        } else {
            price_dir
        };

        // Strength: 0.0 at baseline, 1.0 at/above the spike threshold.
        let strength = ((volume_ratio - 1.0) / (self.spike_threshold - 1.0)).clamp(0.0, 1.0);

        self.last_type = type_signal;
        self.last_strength = strength;
        self.prev_close = close;
    }

    /// Named output: brace `signal` (volume-event classification -3..+3).
    pub fn signal(&self) -> f64 { self.last_type as f64 }
    /// Named output: brace `strength`.
    pub fn strength(&self) -> f64 { self.last_strength }


    /// Named output: brace `line` (the inner oscillator value).
    pub fn line(&self) -> f64 { self.last_inner }

    /// Returns `true` once the inner oscillator has warmed up.
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    /// Clears all internal state.
    pub fn reset(&mut self) {
        self.inner.reset();
        self.volume_history.clear();
        self.prev_close = 0.0;
        self.has_prev = false;
        self.last_inner = 0.0;
        self.last_type = 0;
        self.last_strength = 0.0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, OscillatorSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, Slot, SourceAxis, Store, StoreKind,
    UpdateComplexity, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`OscillatorWithVolumeWeight`] — the inner oscillator (a config-chosen
/// scalar oscillator SLOT), the rolling baseline period, and the spike threshold.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct OscVolWeightConfig {
    #[slot]
    pub oscillator: Param<OscillatorSlotOrder>,
    pub baseline_period: Param<usize>,
    pub spike_threshold: Param<f64>,
}

impl Indicator for OscillatorWithVolumeWeight {
    const ID: IndicatorId = IndicatorId::OscVolWeight;
    /// No family — a composite detector, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full O/H/L/C/V — the oscillator slot is fed close; volume drives the baseline.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// Rolling VecDeque capped at `baseline_period`; linear in window size.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    const SLOTS: &'static [Slot] = OscVolWeightConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::OscVolWeightLine),
        Output::ordinal(IndicatorOutputId::OscVolWeightSignal),
        Output::percent(IndicatorOutputId::OscVolWeightStrength),
    ];
    const NEEDS_VOLUME: bool = true;
    type Config = OscVolWeightConfig;
    type Runtime = OscillatorWithVolumeWeight;

    fn create(cfg: OscVolWeightConfig) -> OscillatorWithVolumeWeight {
        OscillatorWithVolumeWeight::new(
            cfg.oscillator.resolved().into_slot(),
            cfg.baseline_period.resolved(),
            cfg.spike_threshold.resolved(),
        )
    }

    fn slot_members(cfg: &OscVolWeightConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for OscVolWeightConfig {
    fn defaults() -> Self {
        OscVolWeightConfig {
            oscillator: Param::Solo(OscillatorSlotOrder::Rsi(PeriodConfig { period: 14 })),
            baseline_period: Param::Solo(14),
            spike_threshold: Param::Solo(2.5),
        }
    }
    fn machine_defaults() -> Self {
        // oscillator (#[slot] Param<OscillatorSlotOrder>): structural slot — left Solo.
        //   Slot/smoother sweep is a deferred wave. FLAG: oscillator is an OscillatorSlotOrder slot.
        // baseline_period (usize): Class A period — auto gives range(2,4048,1) ✓
        // spike_threshold (f64): Class C multiplier — volume spike detection threshold
        //   (ratio above baseline); sweep_f64(0.1,10.0,0.1)
        let mut s = Self::machine_defaults_auto();
        s.spike_threshold = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for OscillatorWithVolumeWeight {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::OscVolWeightLine, "Oscillator", Color::hex(0x00BCD4))
            .line_output(IndicatorOutputId::OscVolWeightSignal, "Vol Event", Color::hex(0xFF9800))
            .precision(2)
            .build()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    fn osc_inner(period: usize) -> OscillatorSlot {
        OscillatorSlotOrder::Rsi(PeriodConfig { period }).into_slot()
    }

    /// Feed via the contract `feed(&[..])` interface.
    fn feed(ind: &mut OscillatorWithVolumeWeight, close: f64, volume: f64) {
        ind.feed(&[close, close + 0.5, close - 0.5, close, volume])
    }

    fn feed_n(ind: &mut OscillatorWithVolumeWeight, price: f64, vol: f64, n: usize) {
        for _ in 0..n {
            feed(ind, price, vol);
        }
    }

    #[test]
    fn warmup_returns_zero_signal() {
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), 10, 2.5);
        for i in 0..9u32 {
            feed(&mut ind, 100.0 + i as f64, 100.0);
            assert_eq!(ind.signal(), 0.0, "expected 0 classification during warmup at bar {i}");
        }
    }

    #[test]
    fn volume_spike_up_signals_plus_three() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0); // set prev_close
        feed(&mut ind, 101.0, 300.0); // price up, 3x volume -> spike
        assert_eq!(ind.signal(), 3.0, "expected +3 for spike-up");
    }

    #[test]
    fn confirmation_up_signals_plus_two() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);
        feed(&mut ind, 101.0, 150.0); // 1.5x baseline
        assert_eq!(ind.signal(), 2.0, "expected +2 for confirmation-up");
    }

    #[test]
    fn divergence_up_weak_signals_plus_one() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);
        feed(&mut ind, 101.0, 50.0); // 0.5x baseline
        assert_eq!(ind.signal(), 1.0, "expected +1 for divergence-up (weak)");
    }

    #[test]
    fn confirmation_down_signals_minus_two() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 101.0, 100.0, baseline);
        feed(&mut ind, 101.0, 100.0);
        feed(&mut ind, 100.0, 150.0);
        assert_eq!(ind.signal(), -2.0, "expected -2 for confirmation-down");
    }

    #[test]
    fn flat_price_zero_signal() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);
        feed(&mut ind, 100.0, 300.0); // flat price
        assert_eq!(ind.signal(), 0.0, "expected 0 for flat price");
    }

    #[test]
    fn bearish_spike_signals_minus_three() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 101.0, 100.0, baseline);
        feed(&mut ind, 101.0, 100.0);
        feed(&mut ind, 100.0, 300.0);
        assert_eq!(ind.signal(), -3.0, "expected -3 for bearish spike");
    }

    #[test]
    fn reset_clears_state() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 100.0, 100.0, baseline + 2);
        ind.reset();
        feed(&mut ind, 100.0, 100.0);
        assert_eq!(ind.signal(), 0.0, "expected 0 classification after reset (warmup again)");
    }

    #[test]
    fn value_reflects_last_classification() {
        let baseline = 10;
        let mut ind = OscillatorWithVolumeWeight::new(osc_inner(5), baseline, 2.5);
        feed_n(&mut ind, 100.0, 100.0, baseline);
        feed(&mut ind, 100.0, 100.0);
        feed(&mut ind, 101.0, 300.0); // spike up
        assert_eq!(ind.signal(), 3.0, "signal() must reflect the last classification");
    }

    #[test]
    fn factory_feeds_resolved_osc_vol_weight() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::OscVolWeight(<<OscillatorWithVolumeWeight as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..60 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0;
            let vol = 500.0 + (i as f64 * 0.7).cos() * 200.0;
            f.feed(0, MarketSample::Bar { open: p, high: p + 0.5, low: p - 0.5, close: p, volume: vol });
        }
        assert!(f.is_ready(), "osc_vol_weight must be ready after warmup through the factory");
        assert!(f.primary().is_finite());
    }
}
