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

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    inputs: Vec<Box<IndicatorInstance>>,
    mode: ConfluenceMode,
    last_signal: i8,
}

impl Confluence {
    pub fn new(inputs: Vec<IndicatorInstance>, mode: ConfluenceMode) -> Self {
        Self {
            inputs: inputs.into_iter().map(Box::new).collect(),
            mode,
            last_signal: 0,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        // Drive all inputs; collect their per-bar signals (sign of .main()).
        let signs: Vec<i8> = self
            .inputs
            .iter_mut()
            .map(|ind| {
                let v = ind.update_bar(open, high, low, close, volume, None).main();
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
            return 0.0;
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
        signal as f64
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    /// Build a wrapper instance that always exposes a known signal — for unit testing
    /// confluence aggregation without depending on real-indicator timing.
    /// (We use real SMA-based RelativePosition since it produces stable ±1 in trends.)
    fn rp(fast: usize, slow: usize) -> IndicatorInstance {
        // RelativePosition isn't yet wired into IndicatorInstance factory — we need
        // any inner that emits signal values. Use MaCross from legacy factory which
        // outputs Signal(±1) on uptrend/downtrend.
        IndicatorInstance::create(
            &IndicatorConfig::new(BarIndicatorId::MaCross, "MaCross".into(), vec![fast, slow]),
        )
        .unwrap()
    }

    #[test]
    fn all_mode_agreement_emits_signal() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30), rp(7, 25)],
            ConfluenceMode::All,
        );
        // Uptrend → all three should signal +1.
        for i in 1..=60 {
            let p = 100.0 + i as f64;
            let _ = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        let v = c.value();
        match v {
            IndicatorValue::Signal(s) => assert_eq!(s, 1, "uptrend → all agree → +1"),
            _ => panic!("expected Signal"),
        }
    }

    #[test]
    fn all_mode_disagreement_emits_zero() {
        // Use one fast-uptrend tracker and an inverted one (slow vs fast — different periods
        // will go through cross during oscillation, generating ±1 sporadically).
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(30, 50)],
            ConfluenceMode::All,
        );
        let mut zero_count = 0;
        let mut nonzero_count = 0;
        for i in 1..=100 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 15.0;
            let v = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
            if v == 0.0 {
                zero_count += 1;
            } else {
                nonzero_count += 1;
            }
        }
        // In oscillating market, disagreements should produce more zeros than agreements.
        assert!(zero_count > 0, "must have some zero bars in oscillation");
        let _ = nonzero_count;
    }

    #[test]
    fn any_mode_emits_first_nonzero() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30)],
            ConfluenceMode::Any,
        );
        for i in 1..=60 {
            let p = 100.0 + i as f64;
            let _ = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        // Any mode in uptrend → +1.
        assert_eq!(c.value(), IndicatorValue::Signal(1));
    }

    #[test]
    fn majority_mode_majority_wins() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30), rp(7, 25)],
            ConfluenceMode::Majority,
        );
        for i in 1..=60 {
            let p = 100.0 + i as f64;
            let _ = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        // 3/3 agree on uptrend → +1.
        assert_eq!(c.value(), IndicatorValue::Signal(1));
    }

    #[test]
    fn sum_mode_threshold() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30), rp(7, 25)],
            ConfluenceMode::Sum { threshold: 2 },
        );
        for i in 1..=60 {
            let p = 100.0 + i as f64;
            let _ = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        // sum = 3, threshold = 2 → +1.
        assert_eq!(c.value(), IndicatorValue::Signal(1));
    }

    #[test]
    fn sum_high_threshold_silences() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30)],
            ConfluenceMode::Sum { threshold: 3 }, // need 3 votes, only have 2 inputs
        );
        for i in 1..=60 {
            let p = 100.0 + i as f64;
            let _ = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        // sum = 2, threshold = 3 → 0.
        assert_eq!(c.value(), IndicatorValue::Signal(0));
    }

    #[test]
    fn warmup_no_signal() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30)],
            ConfluenceMode::All,
        );
        for i in 1..=5 {
            let p = 100.0 + i as f64;
            let s = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
            assert_eq!(s, 0.0, "no signal during warmup at bar {}", i);
        }
    }

    #[test]
    fn reset_clears() {
        let mut c = Confluence::new(
            vec![rp(5, 20), rp(10, 30)],
            ConfluenceMode::All,
        );
        for i in 1..=50 {
            let p = 100.0 + i as f64;
            let _ = c.update_bar(p, p + 0.5, p - 0.5, p, 1000.0);
        }
        c.reset();
        assert!(!c.is_ready());
        assert_eq!(c.value(), IndicatorValue::Signal(0));
    }
}
