//! MarkPriceGapDetector — detects abnormal price jumps in the mark price feed.
//!
//! A jump is detected when `|current - prev| > sigma_threshold × rolling_std`.
//!
//! Output: `Triple(jump_signal_as_f64, jump_size, sigma_ratio)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::MarkPrice;

/// Detects statistically abnormal jumps in the mark price.
///
/// `jump_signal_as_f64 = +1.0 / -1.0 / 0.0`
/// `jump_size = |current - prev|`
/// `sigma_ratio = jump_size / rolling_std`
#[derive(Clone)]
pub struct MarkPriceGapDetector {
    window: usize,
    sigma_threshold: f64,
    history: VecDeque<f64>,
    prev_mark: f64,
    last_jump_signal: f64,
    last_jump_size: f64,
    last_sigma_ratio: f64,
}

impl MarkPriceGapDetector {
    /// Create a new indicator.
    ///
    /// - `window`: rolling std lookback (min 2).
    /// - `sigma_threshold`: jump threshold in standard deviations (default 3.0).
    pub fn new(window: usize, sigma_threshold: f64) -> Self {
        Self {
            window: window.max(2),
            sigma_threshold,
            history: VecDeque::new(),
            prev_mark: f64::NAN,
            last_jump_signal: 0.0,
            last_jump_size: 0.0,
            last_sigma_ratio: 0.0,
        }
    }

    fn rolling_std(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        variance.sqrt()
    }
}

impl Default for MarkPriceGapDetector {
    fn default() -> Self {
        Self::new(20, 3.0)
    }
}

impl MarkPriceConsumer for MarkPriceGapDetector {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        let current = mp.mark_price;

        // Compute jump vs previous observation
        let jump_size = if self.prev_mark.is_finite() {
            (current - self.prev_mark).abs()
        } else {
            0.0
        };
        let jump_direction = if self.prev_mark.is_finite() && current != self.prev_mark {
            if current > self.prev_mark { 1.0 } else { -1.0 }
        } else {
            0.0
        };

        // Update rolling history
        self.history.push_back(current);
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        let std = self.rolling_std();
        let sigma_ratio = if std > 1e-15 { jump_size / std } else { 0.0 };

        self.last_jump_signal = if sigma_ratio > self.sigma_threshold { jump_direction } else { 0.0 };
        self.last_jump_size = jump_size;
        self.last_sigma_ratio = sigma_ratio;
        self.prev_mark = current;

        IndicatorValue::Triple(self.last_jump_signal, self.last_jump_size, self.last_sigma_ratio)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_jump_signal, self.last_jump_size, self.last_sigma_ratio)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.prev_mark = f64::NAN;
        self.last_jump_signal = 0.0;
        self.last_jump_size = 0.0;
        self.last_sigma_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window && self.prev_mark.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice {
            mark_price,
            index_price: None,
            funding_rate: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn no_jump_for_stable_prices() {
        let mut ind = MarkPriceGapDetector::new(5, 3.0);
        for p in [50000.0, 50001.0, 50002.0, 50001.5, 50002.5] {
            ind.update_mark(&make_mp(p));
        }
        if let IndicatorValue::Triple(sig, _, _) = ind.value() {
            assert_eq!(sig, 0.0, "small moves should not trigger jump signal");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn large_positive_jump_fires_plus_one() {
        let mut ind = MarkPriceGapDetector::new(5, 2.0);
        // Build history with slight variation so std > 0
        for (i, p) in [50000.0_f64, 50001.0, 49999.0, 50002.0, 49998.0].iter().enumerate() {
            let _ = i;
            ind.update_mark(&make_mp(*p));
        }
        // Massive upward jump — many sigma above the tiny std
        let v = ind.update_mark(&make_mp(51000.0));
        if let IndicatorValue::Triple(sig, size, ratio) = v {
            assert_eq!(sig, 1.0, "upward jump should give +1, ratio={ratio}");
            assert!(size > 0.0, "jump size should be positive");
            assert!(ratio > 2.0, "sigma ratio should exceed threshold");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn large_negative_jump_fires_minus_one() {
        let mut ind = MarkPriceGapDetector::new(5, 2.0);
        for p in [50000.0_f64, 50001.0, 49999.0, 50002.0, 49998.0] {
            ind.update_mark(&make_mp(p));
        }
        // Massive downward jump
        let v = ind.update_mark(&make_mp(49000.0));
        if let IndicatorValue::Triple(sig, _, _) = v {
            assert_eq!(sig, -1.0, "downward jump should give -1");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = MarkPriceGapDetector::new(5, 3.0);
        for i in 0..4 {
            ind.update_mark(&make_mp(50000.0 + i as f64));
        }
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceGapDetector::new(5, 3.0);
        for p in [50000.0, 50100.0, 50200.0, 50300.0, 50400.0] {
            ind.update_mark(&make_mp(p));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Triple(s, j, r) = ind.value() {
            assert_eq!(s, 0.0);
            assert_eq!(j, 0.0);
            assert_eq!(r, 0.0);
        }
    }
}
