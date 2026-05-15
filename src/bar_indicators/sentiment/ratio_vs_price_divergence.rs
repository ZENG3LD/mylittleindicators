//! RatioVsPriceDivergence — detects when long_ratio direction opposes price direction.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::LongShortRatioConsumer;
use crate::core::types::LongShortRatio;

/// Measures divergence between the rolling trend of `long_ratio` and price.
///
/// Divergence is present when their directional changes have opposite signs.
/// The score is normalised to [0, 1] based on relative move sizes.
///
/// Output: `IndicatorValue::Double(score, side)`.
/// - `score` ∈ [0, 1]: strength of the divergence.
/// - `side`: `1.0` = bullish divergence (ratio ↑ while price ↓),
///           `-1.0` = bearish divergence (ratio ↓ while price ↑),
///           `0.0` = no divergence.
#[derive(Clone)]
pub struct RatioVsPriceDivergence {
    period: usize,
    ratio_history: VecDeque<f64>,
    price_history: VecDeque<f64>,
    last_score: f64,
    last_side: f64,
}

impl RatioVsPriceDivergence {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            ratio_history: VecDeque::with_capacity(period),
            price_history: VecDeque::with_capacity(period),
            last_score: 0.0,
            last_side: 0.0,
        }
    }

    fn compute(&mut self) {
        if self.ratio_history.len() < 2 || self.price_history.len() < 2 {
            return;
        }

        let r_first = self.ratio_history[0];
        let r_last = self.ratio_history[self.ratio_history.len() - 1];
        let p_first = self.price_history[0];
        let p_last = self.price_history[self.price_history.len() - 1];

        let dr = r_last - r_first;
        let dp = p_last - p_first;

        if dr.signum() != dp.signum() && dr.abs() > 1e-9 && dp.abs() > 1e-9 {
            let r_max = self
                .ratio_history
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let r_min = self
                .ratio_history
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let p_max = self
                .price_history
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let p_min = self
                .price_history
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);

            let r_range = r_max - r_min;
            let p_range = p_max - p_min;

            let r_norm = if r_range > 1e-9 { dr.abs() / r_range } else { 0.0 };
            let p_norm = if p_range > 1e-9 { dp.abs() / p_range } else { 0.0 };

            self.last_score = (r_norm * p_norm).min(1.0);
            // ratio ↑ + price ↓ → crowd overextended long → bullish reversal expected
            // ratio ↓ + price ↑ → crowd missing the move → bearish reversal expected
            self.last_side = if dr > 0.0 { 1.0 } else { -1.0 };
        } else {
            self.last_score = 0.0;
            self.last_side = 0.0;
        }
    }

    /// Update with close price from a bar event.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.price_history.push_back(c);
        while self.price_history.len() > self.period {
            self.price_history.pop_front();
        }
        self.compute();
        IndicatorValue::Double(self.last_score, self.last_side)
    }
}

impl LongShortRatioConsumer for RatioVsPriceDivergence {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) -> IndicatorValue {
        self.ratio_history.push_back(lsr.long_ratio);
        while self.ratio_history.len() > self.period {
            self.ratio_history.pop_front();
        }
        self.compute();
        IndicatorValue::Double(self.last_score, self.last_side)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_score, self.last_side)
    }

    fn reset(&mut self) {
        self.ratio_history.clear();
        self.price_history.clear();
        self.last_score = 0.0;
        self.last_side = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.ratio_history.len() >= 2 && self.price_history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: None,
            timestamp: 0,
        }
    }

    #[test]
    fn divergence_detected_when_opposite_directions() {
        let mut ind = RatioVsPriceDivergence::new(4);

        // ratio rises, price falls → divergence
        for (r, p) in [(0.4f64, 100.0f64), (0.5, 95.0), (0.6, 90.0), (0.7, 85.0)] {
            ind.update_long_short_ratio(&make_lsr(r));
            ind.update_bar(p, p, p, p, 1.0);
        }

        if let IndicatorValue::Double(score, side) = ind.value() {
            assert!(score > 0.0, "expected positive divergence score, got {score}");
            assert_eq!(side, 1.0, "expected bullish side (ratio up, price down)");
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn no_divergence_same_direction() {
        let mut ind = RatioVsPriceDivergence::new(4);

        // both rise — no divergence
        for (r, p) in [(0.4f64, 85.0f64), (0.5, 90.0), (0.6, 95.0), (0.7, 100.0)] {
            ind.update_long_short_ratio(&make_lsr(r));
            ind.update_bar(p, p, p, p, 1.0);
        }

        if let IndicatorValue::Double(score, side) = ind.value() {
            assert_eq!(score, 0.0, "expected no divergence, score={score}");
            assert_eq!(side, 0.0, "expected neutral side");
        } else {
            panic!("expected Double");
        }
    }
}
