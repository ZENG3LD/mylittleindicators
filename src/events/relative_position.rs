//! RelativePosition primitive — emits trend state (+1/-1) based on subject
//! vs reference comparison. Holds last non-zero state across bars (does not
//! reset to 0 between transitions, unlike Crossover).
//!
//! This is the "MaCross-style" output: persistent ±1 trend label rather
//! than a one-shot event. Crossover and RelativePosition are complementary:
//! - Crossover emits ±1 only on the crossing bar (event semantics)
//! - RelativePosition emits ±1 continuously while the relation holds (state semantics)
//!
//! Replaces hardcoded MaCross (fast vs slow MA), SSL channel direction, and
//! any "is X above Y right now" question.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;
use crate::core::events::direction::Direction;
use crate::core::events::kind::{SignalKind, TrendSub};

#[derive(Clone)]
pub struct RelativePosition {
    subject: Box<IndicatorInstance>,
    reference: Box<IndicatorInstance>,
    /// Last non-zero state (sticky — holds across flat bars where subject == reference).
    last_trend: i8,
    ready: bool,
}

impl RelativePosition {
    pub fn new(subject: IndicatorInstance, reference: IndicatorInstance) -> Self {
        Self {
            subject: Box::new(subject),
            reference: Box::new(reference),
            last_trend: 0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> i8 {
        let s = self
            .subject
            .update_bar(open, high, low, close, volume, None)
            .main();
        let r = self
            .reference
            .update_bar(open, high, low, close, volume, None)
            .main();

        if self.subject.is_ready() && self.reference.is_ready() {
            let new_trend = if s > r { 1 } else if s < r { -1 } else { 0 };
            // Sticky: only overwrite when new_trend is decisive (±1) AND differs.
            if new_trend != 0 && new_trend != self.last_trend {
                self.last_trend = new_trend;
            }
            self.ready = true;
        }
        self.last_trend
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_trend)
    }

    /// Feed one bar and return a typed signal reflecting the persistent trend state.
    ///
    /// Maps to `SignalKind::Trend(TrendSub::MaCross)` — subject line position relative
    /// to reference line, maintained as sticky trend direction.
    /// Returns `None` until both inner indicators are ready.
    pub fn detect(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.update_bar(open, high, low, close, volume);
        if !self.ready {
            return None;
        }
        match self.last_trend {
            1 => Some((SignalKind::Trend(TrendSub::MaCross), Direction::Up)),
            -1 => Some((SignalKind::Trend(TrendSub::MaCross), Direction::Down)),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.subject.reset();
        self.reference.reset();
        self.last_trend = 0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    fn sma(period: usize) -> IndicatorInstance {
        let cfg = IndicatorConfig::new(BarIndicatorId::Sma, "Sma".into(), vec![period]);
        IndicatorInstance::create(&cfg).unwrap()
    }

    fn ema(period: usize) -> IndicatorInstance {
        let cfg = IndicatorConfig::new(BarIndicatorId::Ema, "Ema".into(), vec![period]);
        IndicatorInstance::create(&cfg).unwrap()
    }

    #[test]
    fn uptrend_state_holds_plus_one() {
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=50 {
            let p = 100.0 + i as f64 * 2.0;
            let _ = rp.update_bar(p, p, p, p, 0.0);
        }
        assert!(rp.is_ready());
        assert_eq!(rp.value(), IndicatorValue::Signal(1));
    }

    #[test]
    fn downtrend_state_holds_minus_one() {
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=50 {
            let p = 200.0 - i as f64 * 2.0;
            let _ = rp.update_bar(p, p, p, p, 0.0);
        }
        assert!(rp.is_ready());
        assert_eq!(rp.value(), IndicatorValue::Signal(-1));
    }

    #[test]
    fn parity_with_legacy_macross_uptrend() {
        // Legacy MaCross::test_ma_cross_uptrend behaviour: 40 bars of price 100+2i.
        let mut rp = RelativePosition::new(ema(9), ema(21));
        let cfg_fast = IndicatorConfig::new(BarIndicatorId::Ema, "EmaFast".into(), vec![9]);
        let cfg_slow = IndicatorConfig::new(BarIndicatorId::Ema, "EmaSlow".into(), vec![21]);
        let _ = (cfg_fast, cfg_slow);

        for i in 1..=40 {
            let p = 100.0 + i as f64 * 2.0;
            let _ = rp.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
        }
        assert!(rp.is_ready());
        assert_eq!(rp.value(), IndicatorValue::Signal(1), "uptrend → +1 like legacy MaCross");
    }

    #[test]
    fn parity_with_legacy_macross_downtrend() {
        let mut rp = RelativePosition::new(ema(9), ema(21));
        for i in 1..=40 {
            let p = 200.0 - i as f64 * 2.0;
            let _ = rp.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
        }
        assert_eq!(rp.value(), IndicatorValue::Signal(-1));
    }

    #[test]
    fn state_sticks_across_oscillation() {
        // Oscillating price — once trend established, sticky on small flips.
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=80 {
            let p = 100.0 + (i as f64 * 0.5).sin() * 8.0;
            let _ = rp.update_bar(p, p, p, p, 0.0);
        }
        assert!(rp.is_ready());
        let v = rp.value();
        match v {
            IndicatorValue::Signal(s) => assert!(s == 1 || s == -1, "sticky sign after oscillation"),
            _ => panic!("expected Signal"),
        }
    }

    #[test]
    fn reset_clears_trend_state() {
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=30 {
            let p = 100.0 + i as f64;
            let _ = rp.update_bar(p, p, p, p, 0.0);
        }
        rp.reset();
        assert!(!rp.is_ready());
        assert_eq!(rp.value(), IndicatorValue::Signal(0));
    }
}
