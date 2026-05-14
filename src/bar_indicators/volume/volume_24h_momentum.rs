//! Volume24hMomentum — rolling slope of 24-hour trading volume.
//!
//! Tracks how quickly 24h volume is accelerating or decelerating.
//! Slope = (latest - oldest) / (N - 1), normalised by the mean to give
//! a dimensionless rate-of-change that is comparable across symbols.
//!
//! Output: `Single(slope)` — positive = volume trending up, negative = down.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ticker_consumer::TickerConsumer;
use crate::core::types::Ticker;

/// Rolling slope of 24-hour traded volume.
#[derive(Clone)]
pub struct Volume24hMomentum {
    /// Window length.
    period: usize,
    /// Ring buffer of historical volume_24h values.
    history: Vec<f64>,
    /// Write cursor in the ring buffer.
    cursor: usize,
    /// Number of values received (capped at period).
    count: usize,
    /// Last computed slope.
    last_slope: f64,
}

impl Volume24hMomentum {
    /// Create with the given rolling window size.
    pub fn new(period: usize) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            history: vec![0.0; p],
            cursor: 0,
            count: 0,
            last_slope: 0.0,
        }
    }

    fn compute_slope(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let n = self.count.min(self.period);
        // oldest entry in the ring buffer
        let oldest_idx = if self.count < self.period {
            0
        } else {
            self.cursor // next write position = oldest slot
        };
        let oldest = self.history[oldest_idx];
        // newest entry is the one just before cursor
        let newest_idx = if self.cursor == 0 {
            self.period - 1
        } else {
            self.cursor - 1
        };
        let newest = self.history[newest_idx];
        let steps = (n - 1) as f64;
        (newest - oldest) / steps
    }
}

impl TickerConsumer for Volume24hMomentum {
    fn update_ticker(&mut self, ticker: &Ticker) -> IndicatorValue {
        let vol = ticker.volume_24h.unwrap_or(0.0);
        self.history[self.cursor] = vol;
        self.cursor = (self.cursor + 1) % self.period;
        if self.count < self.period {
            self.count += 1;
        }
        self.last_slope = self.compute_slope();
        IndicatorValue::Single(self.last_slope)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_slope)
    }

    fn reset(&mut self) {
        self.history.fill(0.0);
        self.cursor = 0;
        self.count = 0;
        self.last_slope = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(vol: f64) -> Ticker {
        Ticker {
            symbol: "BTCUSDT".to_string(),
            last_price: 50000.0,
            bid_price: None,
            ask_price: None,
            high_24h: None,
            low_24h: None,
            volume_24h: Some(vol),
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = Volume24hMomentum::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn zero_slope_on_constant_volume() {
        let mut ind = Volume24hMomentum::new(3);
        for _ in 0..10 {
            ind.update_ticker(&ticker(100.0));
        }
        assert!(ind.is_ready());
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s.abs() < 1e-12, "constant volume → slope = 0, got {}", s);
        }
    }

    #[test]
    fn positive_slope_on_rising_volume() {
        let mut ind = Volume24hMomentum::new(3);
        for i in 0..5 {
            ind.update_ticker(&ticker(i as f64 * 100.0));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "rising volume → positive slope, got {}", s);
        }
    }

    #[test]
    fn negative_slope_on_falling_volume() {
        let mut ind = Volume24hMomentum::new(3);
        for i in (0..5).rev() {
            ind.update_ticker(&ticker(i as f64 * 100.0));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "falling volume → negative slope, got {}", s);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = Volume24hMomentum::new(3);
        for i in 0..5 {
            ind.update_ticker(&ticker(i as f64 * 100.0));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(s) = ind.value() {
            assert_eq!(s, 0.0);
        }
    }
}
