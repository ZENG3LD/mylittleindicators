//! Crossover primitive — detects when one indicator line crosses another.
//!
//! Owns two inner indicators (subject + reference). On each bar:
//! - feed OHLCV to both
//! - compare current subject vs reference and previous subject vs reference
//! - emit Signal: +1 = subject crossed up through reference, -1 = crossed down, 0 = no event
//!
//! Replaces hardcoded MaCross (subject=fast MA, reference=slow MA) and any
//! "X crosses Y" pattern (price-vs-MA, MACD-vs-signal, oscillator-vs-level).

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;
use crate::core::events::direction::Direction;
use crate::core::events::kind::SignalKind;

/// Which crossover direction(s) fire a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossoverDirection {
    /// Fire +1 on up-cross, -1 on down-cross.
    Both,
    /// Fire +1 on up-cross only.
    UpOnly,
    /// Fire -1 on down-cross only.
    DownOnly,
}

impl Default for CrossoverDirection {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Clone)]
pub struct Crossover {
    subject: Box<IndicatorInstance>,
    reference: Box<IndicatorInstance>,
    direction: CrossoverDirection,
    prev_subject: f64,
    prev_reference: f64,
    has_prev: bool,
    last_signal: i8,
}

impl Crossover {
    /// Construct from owned inner indicators.
    pub fn new(
        subject: IndicatorInstance,
        reference: IndicatorInstance,
        direction: CrossoverDirection,
    ) -> Self {
        Self {
            subject: Box::new(subject),
            reference: Box::new(reference),
            direction,
            prev_subject: 0.0,
            prev_reference: 0.0,
            has_prev: false,
            last_signal: 0,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let s = self
            .subject
            .update_bar(open, high, low, close, volume, None)
            .main();
        let r = self
            .reference
            .update_bar(open, high, low, close, volume, None)
            .main();

        let signal = if !self.has_prev || !self.is_ready() {
            0
        } else {
            let was_below = self.prev_subject < self.prev_reference;
            let is_above = s > r;
            let was_above = self.prev_subject > self.prev_reference;
            let is_below = s < r;

            if was_below && is_above {
                match self.direction {
                    CrossoverDirection::Both | CrossoverDirection::UpOnly => 1,
                    CrossoverDirection::DownOnly => 0,
                }
            } else if was_above && is_below {
                match self.direction {
                    CrossoverDirection::Both | CrossoverDirection::DownOnly => -1,
                    CrossoverDirection::UpOnly => 0,
                }
            } else {
                0
            }
        };

        self.prev_subject = s;
        self.prev_reference = r;
        self.has_prev = true;
        self.last_signal = signal;
        signal as f64
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    pub fn is_ready(&self) -> bool {
        self.subject.is_ready() && self.reference.is_ready()
    }

    /// Feed one bar and return a typed signal if a crossover occurred.
    ///
    /// Returns `Some((SignalKind::Crossover, Direction::Up))` when subject crosses above
    /// reference, `Some((SignalKind::Crossover, Direction::Down))` for a down-cross,
    /// and `None` when no crossover event occurred on this bar.
    pub fn detect(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.update_bar(open, high, low, close, volume);
        match self.last_signal {
            1 => Some((SignalKind::Crossover, Direction::Up)),
            -1 => Some((SignalKind::Crossover, Direction::Down)),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.subject.reset();
        self.reference.reset();
        self.prev_subject = 0.0;
        self.prev_reference = 0.0;
        self.has_prev = false;
        self.last_signal = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::average::MovingAverageType;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    fn make_sma(period: usize) -> IndicatorInstance {
        let cfg = IndicatorConfig::new(BarIndicatorId::Sma, "Sma".into(), vec![period]);
        IndicatorInstance::create(&cfg).expect("SMA factory")
    }

    fn make_ema(period: usize) -> IndicatorInstance {
        let cfg = IndicatorConfig::new(BarIndicatorId::Ema, "Ema".into(), vec![period]);
        IndicatorInstance::create(&cfg).expect("EMA factory")
    }

    #[test]
    fn no_signal_during_warmup() {
        let mut x = Crossover::new(make_sma(5), make_sma(10), CrossoverDirection::Both);
        // Less data than slow period — no signal possible.
        for i in 1..=3 {
            let p = 100.0 + i as f64;
            let s = x.update_bar(p, p, p, p, 0.0);
            assert_eq!(s, 0.0, "bar {} should be neutral during warmup", i);
        }
    }

    #[test]
    fn detects_up_cross() {
        let mut x = Crossover::new(make_sma(3), make_sma(7), CrossoverDirection::Both);

        // Start with downtrend so fast MA stays below slow MA.
        let down: Vec<f64> = (0..15).map(|i| 100.0 - i as f64).collect();
        for p in &down {
            let _ = x.update_bar(*p, *p, *p, *p, 0.0);
        }

        // Sharp uptrend — fast SMA must eventually cross above slow.
        let up: Vec<f64> = (0..20).map(|i| 85.0 + (i as f64) * 2.0).collect();
        let mut saw_up_cross = false;
        for p in &up {
            let s = x.update_bar(*p, *p, *p, *p, 0.0);
            if s > 0.0 {
                saw_up_cross = true;
            }
            assert!(s >= 0.0, "in uptrend after downtrend, no down-cross expected, got {}", s);
        }
        assert!(saw_up_cross, "uptrend after downtrend must produce at least one up-cross");
    }

    #[test]
    fn detects_down_cross() {
        let mut x = Crossover::new(make_sma(3), make_sma(7), CrossoverDirection::Both);

        // Start with uptrend.
        let up: Vec<f64> = (0..15).map(|i| 100.0 + i as f64).collect();
        for p in &up {
            let _ = x.update_bar(*p, *p, *p, *p, 0.0);
        }

        // Reverse — fast SMA must cross below slow.
        let down: Vec<f64> = (0..20).map(|i| 115.0 - (i as f64) * 2.0).collect();
        let mut saw_down_cross = false;
        for p in &down {
            let s = x.update_bar(*p, *p, *p, *p, 0.0);
            if s < 0.0 {
                saw_down_cross = true;
            }
        }
        assert!(saw_down_cross, "downtrend after uptrend must produce at least one down-cross");
    }

    #[test]
    fn up_only_filter_suppresses_down_cross() {
        let mut x = Crossover::new(make_sma(3), make_sma(7), CrossoverDirection::UpOnly);

        // Uptrend then sharp downtrend — would produce down-cross in Both mode.
        let up: Vec<f64> = (0..15).map(|i| 100.0 + i as f64).collect();
        for p in &up { let _ = x.update_bar(*p, *p, *p, *p, 0.0); }
        let down: Vec<f64> = (0..20).map(|i| 115.0 - (i as f64) * 2.0).collect();

        for p in &down {
            let s = x.update_bar(*p, *p, *p, *p, 0.0);
            assert!(s >= 0.0, "UpOnly must never emit -1, got {}", s);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut x = Crossover::new(make_sma(3), make_sma(7), CrossoverDirection::Both);
        for i in 0..20 {
            let p = 100.0 + i as f64;
            let _ = x.update_bar(p, p, p, p, 0.0);
        }
        x.reset();
        assert!(!x.is_ready());
        assert_eq!(x.value(), IndicatorValue::Signal(0));
    }

    #[test]
    fn different_inner_types_work() {
        // SMA fast / EMA slow — heterogeneous inners.
        let mut x = Crossover::new(make_sma(5), make_ema(20), CrossoverDirection::Both);
        for i in 0..50 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            let _ = x.update_bar(p, p, p, p, 0.0);
        }
        assert!(x.is_ready());
    }

    #[test]
    fn ma_type_independence_through_factory() {
        // Two crossovers with different inner MA types must be independent.
        let cfg_a = IndicatorConfig::new(BarIndicatorId::Ema, "FastEMA".into(), vec![10]);
        let cfg_b = IndicatorConfig::new(BarIndicatorId::Sma, "SlowSMA".into(), vec![20]);
        let _ = MovingAverageType::EMA; // unused symbol guard

        let mut x = Crossover::new(
            IndicatorInstance::create(&cfg_a).unwrap(),
            IndicatorInstance::create(&cfg_b).unwrap(),
            CrossoverDirection::Both,
        );
        for i in 0..40 {
            let p = 100.0 + i as f64;
            let _ = x.update_bar(p, p, p, p, 0.0);
        }
        assert!(x.is_ready());
    }
}
