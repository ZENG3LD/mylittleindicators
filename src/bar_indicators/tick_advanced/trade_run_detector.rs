//! TradeRunDetector — consecutive same-side tick run counter.
//!
//! Increments a run counter while successive ticks are on the same side
//! (buy/sell). Resets the counter to 1 when the side flips.
//!
//! Output: `IndicatorValue::Double(side_f64, run_length_f64)`
//!   side: +1.0 = buy run, -1.0 = sell run
//!   run_length: how many consecutive same-side ticks (≥ 1)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Consecutive same-side tick run detector.
///
/// `min_run_length` controls `is_ready`: the indicator signals as ready only
/// after at least one run of that length has been observed.
#[derive(Debug, Clone)]
pub struct TradeRunDetector {
    min_run: usize,
    current_run: usize,
    /// +1 = buy side, -1 = sell side, 0 = not yet started.
    current_side: i8,
    max_run_seen: usize,
}

impl TradeRunDetector {
    /// Create detector. `min_run_length` default is 3.
    pub fn new(min_run_length: usize) -> Self {
        Self {
            min_run: min_run_length.max(1),
            current_run: 0,
            current_side: 0,
            max_run_seen: 0,
        }
    }
}

impl TickConsumer for TradeRunDetector {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        let side: i8 = if tick.is_buy { 1 } else { -1 };

        if self.current_side == side {
            self.current_run += 1;
        } else {
            self.current_side = side;
            self.current_run = 1;
        }

        if self.current_run > self.max_run_seen {
            self.max_run_seen = self.current_run;
        }

        IndicatorValue::Double(side as f64, self.current_run as f64)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.current_side as f64, self.current_run as f64)
    }

    fn reset(&mut self) {
        self.current_run = 0;
        self.current_side = 0;
        self.max_run_seen = 0;
    }

    fn is_ready(&self) -> bool {
        self.max_run_seen >= self.min_run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_tick() -> Tick {
        Tick::new(0, 100.0, 1.0, true)
    }

    fn sell_tick() -> Tick {
        Tick::new(0, 100.0, 1.0, false)
    }

    #[test]
    fn run_increments_on_same_side() {
        let mut det = TradeRunDetector::new(3);
        for expected_run in 1..=5usize {
            let v = det.update_tick(&buy_tick());
            match v {
                IndicatorValue::Double(side, run) => {
                    assert!((side - 1.0).abs() < 1e-9);
                    assert!((run - expected_run as f64).abs() < 1e-9);
                }
                other => panic!("expected Double, got {:?}", other),
            }
        }
    }

    #[test]
    fn run_resets_on_side_change() {
        let mut det = TradeRunDetector::new(1);
        det.update_tick(&buy_tick());
        det.update_tick(&buy_tick());
        let v = det.update_tick(&sell_tick());
        match v {
            IndicatorValue::Double(side, run) => {
                assert!((side - (-1.0)).abs() < 1e-9, "side should be sell");
                assert!((run - 1.0).abs() < 1e-9, "run resets to 1");
            }
            other => panic!("expected Double, got {:?}", other),
        }
    }

    #[test]
    fn is_ready_after_min_run_reached() {
        let mut det = TradeRunDetector::new(3);
        assert!(!det.is_ready());
        det.update_tick(&buy_tick());
        det.update_tick(&buy_tick());
        assert!(!det.is_ready());
        det.update_tick(&buy_tick());
        assert!(det.is_ready(), "min_run=3 reached");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = TradeRunDetector::new(3);
        for _ in 0..5 {
            det.update_tick(&buy_tick());
        }
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
