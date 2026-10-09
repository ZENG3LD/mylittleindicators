//! Confluence primitive — combines multiple inner signal-emitting indicators
//! into one composite signal using a configurable aggregation mode.
//!
//! Owns N inner indicators (each must emit a signal value in its `.main()`).
//! Aggregation:
//! - `All` — all must be non-zero AND share sign; emits common sign or 0
//! - `Any` — any non-zero; emits sign of first non-zero (or 0)
//! - `Majority` — sign of the majority (ties → 0)
//! - `Sum` — sums signs, threshold the result: if abs ≥ `threshold`, emit sign
//!
//! Replaces "MultiDivergence" (3 oscillator divergence votes), "MarketCipher"
//! (WT + RSI + MF + VWAP confluence), "NeuralMomentumNetwork" (NN over multiple
//! MA features). Any composite that asks "do N detectors agree?".

use crate::engine::contract_engine::OscillatorSlot;
use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{CompositeSub, SignalKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum ConfluenceMode {
    /// All inputs must agree on sign and be non-zero. Strictest.
    All,
    /// Any non-zero input contributes; emit sign of first non-zero. Loosest.
    Any,
    /// Majority sign wins; ties → 0.
    Majority,
    /// Sum of signs (each input contributes -1/0/+1); emit sign if |sum| ≥ threshold.
    Sum { threshold: i32 },
}

impl Default for ConfluenceMode {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Clone)]
pub struct Confluence {
    /// Config-chosen scalar oscillator slots (RSI / CMO / …), each fed the close. Box-free.
    inputs: Vec<OscillatorSlot>,
    mode: ConfluenceMode,
    last_signal: i8,
}

impl std::fmt::Debug for Confluence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Confluence")
            .field("mode", &self.mode)
            .field("input_count", &self.inputs.len())
            .field("last_signal", &self.last_signal)
            .finish()
    }
}

impl Confluence {
    pub fn new(inputs: Vec<OscillatorSlot>, mode: ConfluenceMode) -> Self {
        Self {
            inputs,
            mode,
            last_signal: 0,
        }
    }

    /// Feed the resolved input lanes — `[open, high, low, close, volume]`.
    /// Drives all inner oscillator slots (fed the close) and aggregates their signs.
    /// Returns the current confluence signal.
    pub fn feed(&mut self, lanes: &[f64]) {
        let close = lanes[3];

        // Drive all oscillator slots with the close; collect the sign of each value.
        let signs: Vec<i8> = self
            .inputs
            .iter_mut()
            .map(|ind| {
                let v = ind.feed(close);
                if v > 0.0 {
                    1i8
                } else if v < 0.0 {
                    -1i8
                } else {
                    0i8
                }
            })
            .collect();

        let all_ready = self.inputs.iter().all(|i| i.is_ready());
        if !all_ready {
            self.last_signal = 0;
            return;
        }

        let signal = match self.mode {
            ConfluenceMode::All => {
                if signs.iter().all(|&s| s > 0) {
                    1
                } else if signs.iter().all(|&s| s < 0) {
                    -1
                } else {
                    0
                }
            }
            ConfluenceMode::Any => signs.iter().find(|&&s| s != 0).copied().unwrap_or(0),
            ConfluenceMode::Majority => {
                let pos = signs.iter().filter(|&&s| s > 0).count() as i32;
                let neg = signs.iter().filter(|&&s| s < 0).count() as i32;
                if pos > neg {
                    1
                } else if neg > pos {
                    -1
                } else {
                    0
                }
            }
            ConfluenceMode::Sum { threshold } => {
                let sum: i32 = signs.iter().map(|&s| s as i32).sum();
                if sum >= threshold {
                    1
                } else if sum <= -threshold {
                    -1
                } else {
                    0
                }
            }
        };

        self.last_signal = signal;
    }

    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Feed one bar and return a typed signal when the configured aggregation mode fires.
    ///
    /// Multiple inputs agreeing maps to `SignalKind::Composite(CompositeSub::Strong)`.
    /// Returns `None` when inputs disagree or are not ready.
    pub fn detect(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.feed(&[open, high, low, close, volume]);
        match self.last_signal {
            1 => Some((SignalKind::Composite(CompositeSub::Strong), Direction::Up)),
            -1 => Some((SignalKind::Composite(CompositeSub::Strong), Direction::Down)),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        !self.inputs.is_empty() && self.inputs.iter().all(|i| i.is_ready())
    }

    pub fn reset(&mut self) {
        for ind in self.inputs.iter_mut() {
            ind.reset();
        }
        self.last_signal = 0;
    }

    /// Detect confluence from pre-computed values (slice-based hot loop).
    ///
    /// `values` is a slice of pre-computed indicator outputs — one per logical input.
    /// Does NOT touch the inner slot(s).
    /// Returns `None` if slice is empty.
    pub fn detect_from_values(&mut self, values: &[f64]) -> Option<(SignalKind, Direction)> {
        if values.is_empty() {
            return None;
        }
        let signs: Vec<i8> = values
            .iter()
            .map(|&v| {
                if v > 0.0 {
                    1i8
                } else if v < 0.0 {
                    -1
                } else {
                    0
                }
            })
            .collect();

        let signal: i8 = match self.mode {
            ConfluenceMode::All => {
                if signs.iter().all(|&s| s > 0) {
                    1
                } else if signs.iter().all(|&s| s < 0) {
                    -1
                } else {
                    0
                }
            }
            ConfluenceMode::Any => signs.iter().find(|&&s| s != 0).copied().unwrap_or(0),
            ConfluenceMode::Majority => {
                let pos = signs.iter().filter(|&&s| s > 0).count() as i32;
                let neg = signs.iter().filter(|&&s| s < 0).count() as i32;
                if pos > neg {
                    1
                } else if neg > pos {
                    -1
                } else {
                    0
                }
            }
            ConfluenceMode::Sum { threshold } => {
                let sum: i32 = signs.iter().map(|&s| s as i32).sum();
                if sum >= threshold {
                    1
                } else if sum <= -threshold {
                    -1
                } else {
                    0
                }
            }
        };

        self.last_signal = signal;
        match signal {
            1 => Some((SignalKind::Composite(CompositeSub::Strong), Direction::Up)),
            -1 => Some((SignalKind::Composite(CompositeSub::Strong), Direction::Down)),
            _ => None,
        }
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, OscillatorSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Render, RenderSpec, SourceAxis, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Confluence`] — a list of config-chosen oscillator SLOT orders and
/// the aggregation mode. The N inputs are a `Vec<OscillatorSlotOrder>` (the count is
/// config-chosen); each admits only `+oscillator`-flagged members by the compiler. Box-free.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ConfluenceConfig {
    /// The inner oscillator set — a whole swept axis (one cube point = one input set).
    pub inputs: crate::contract::Param<Vec<OscillatorSlotOrder>>,
    pub mode: crate::contract::Param<ConfluenceMode>,
}

impl Indicator for Confluence {
    const ID: IndicatorId = IndicatorId::Confluence;
    /// No family — a composite DETECTOR over N inner indicators, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full O/H/L/C/V slice — drives each inner indicator with the full bar.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(N) over the number of inputs per bar — config-chosen count, charged at
    /// Constant since no rolling buffers are maintained by the outer struct itself.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Confluence)];
    type Config = ConfluenceConfig;
    type Runtime = Confluence;

    fn create(cfg: ConfluenceConfig) -> Confluence {
        let inputs = cfg.inputs.resolved().into_iter().map(|o| o.into_slot()).collect::<Vec<_>>();
        Confluence::new(inputs, cfg.mode.resolved())
    }
}

impl crate::contract::Config for ConfluenceConfig {
    fn defaults() -> Self {
        ConfluenceConfig {
            inputs: crate::contract::Param::Solo(vec![
                OscillatorSlotOrder::Rsi(PeriodConfig { period: 14 }),
                OscillatorSlotOrder::Cmo(PeriodConfig { period: 14 }),
            ]),
            mode: crate::contract::Param::Solo(ConfluenceMode::Majority),
        }
    }
    fn machine_defaults() -> Self {
        // inputs (Param<Vec<OscillatorSlotOrder>>): structural slot set — left Solo.
        //   Slot/smoother sweep is a deferred wave (§3.5 / protocol §1).
        //   FLAG: inputs is a Vec<OscillatorSlotOrder> — nested oscillator slot, swept later.
        // mode (Param<ConfluenceMode>): Class Q enum — but ConfluenceMode::Sum { threshold: i32 }
        //   carries embedded state, making it non-trivially enumerable without choosing a threshold.
        //   Left Solo (deferred). FLAG: mode is a Q enum with embedded i32; needs generator support
        //   for Sum variant with swept threshold.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Confluence {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Confluence, "Confluence", Color::hex(0x9C27B0))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::{IndicatorOrder, OscillatorSlotOrder};
    use crate::indicators::average::moving_average::PeriodConfig;

    fn rsi_slot() -> OscillatorSlot {
        OscillatorSlotOrder::Rsi(PeriodConfig { period: 14 }).into_slot()
    }

    #[test]
    fn warmup_no_signal() {
        let mut c = Confluence::new(vec![rsi_slot()], ConfluenceMode::All);
        for i in 0..5 {
            c.feed(&[100.0, 100.0, 100.0, 100.0 + i as f64, 0.0]);
            let s = c.value();
            assert_eq!(s, 0.0, "no signal during warmup at bar {}", i);
        }
    }

    #[test]
    fn detect_calls_feed() {
        let mut c = Confluence::new(vec![rsi_slot()], ConfluenceMode::Any);
        for i in 0..30 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 5.0;
            c.detect(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        assert!(c.is_ready());
    }

    #[test]
    fn reset_clears() {
        let mut c = Confluence::new(vec![rsi_slot()], ConfluenceMode::All);
        for i in 0..30 {
            let p = 100.0 + i as f64;
            c.feed(&[p, p, p, p, 0.0]);
        }
        c.reset();
        assert!(!c.is_ready());
        assert_eq!(c.value(), 0.0);
    }

    #[test]
    fn majority_mode_single_input() {
        let mut c = Confluence::new(vec![rsi_slot()], ConfluenceMode::Majority);
        for i in 0..40 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 8.0;
            c.feed(&[p, p + 0.5, p - 0.5, p, 1000.0]);
        }
        assert!(c.is_ready());
    }

    #[test]
    fn detect_from_values_all_positive() {
        let mut c = Confluence::new(vec![], ConfluenceMode::All);
        let result = c.detect_from_values(&[1.0, 2.0, 0.5]);
        assert!(matches!(result, Some((SignalKind::Composite(CompositeSub::Strong), Direction::Up))));
    }

    #[test]
    fn detect_from_values_mixed_gives_none() {
        let mut c = Confluence::new(vec![], ConfluenceMode::All);
        let result = c.detect_from_values(&[1.0, -1.0]);
        assert!(result.is_none());
    }

    #[test]
    fn factory_feeds_resolved_confluence() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Confluence(<<Confluence as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..60 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0 + i as f64 * 0.1;
            f.feed(0, MarketSample::Bar { open: p, high: p + 0.5, low: p - 0.5, close: p, volume: 1000.0 });
        }
        assert!(f.is_ready(), "confluence must be ready after warmup through the factory");
        assert!(f.primary().is_finite());
    }
}
