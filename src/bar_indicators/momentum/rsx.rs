// RSX — Jurik-style RSI with reduced lag via cascaded EMA smoothing.
//
// The Jurik RSX uses a 3-level cascaded smoothing on the RSI calculation
// with a fixed alpha of 1/period (Wilder-style) for the gain/loss averages,
// then applies 3 passes of EMA with alpha=0.0625 (period≈31) for noise reduction.
// This is the classic approximation published in technical analysis literature.
//
// Output: Single(value) in [0, 1] (not 0-100, matching RSX convention of 0-1 range).

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Rsx {
    period: usize,
    /// Wilder EMA alpha for gain/loss smoothing
    wilder_alpha: f64,
    avg_gain: f64,
    avg_loss: f64,
    prev_close: f64,
    has_prev: bool,
    /// 3-level cascaded smoother state (Jurik-style lag reduction)
    s1: f64,
    s2: f64,
    s3: f64,
    /// Smoothing alpha for the 3 cascaded passes (fixed Jurik constant)
    smooth_alpha: f64,
    count: usize,
    value: f64,
}

impl Rsx {
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            wilder_alpha: 1.0 / p as f64,
            avg_gain: 0.0,
            avg_loss: 0.0,
            prev_close: 0.0,
            has_prev: false,
            s1: 0.0,
            s2: 0.0,
            s3: 0.0,
            // Jurik RSX uses alpha ≈ 0.0625 for cascaded smoothing (published approximation)
            smooth_alpha: 0.0625_f64,
            count: 0,
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.prev_close = 0.0;
        self.has_prev = false;
        self.s1 = 0.0;
        self.s2 = 0.0;
        self.s3 = 0.0;
        self.count = 0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.period + 3
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        self.count += 1;

        if !self.has_prev {
            self.prev_close = close;
            self.has_prev = true;
            // Initialise cascade with RSI=0.5 (neutral)
            self.s1 = 0.5;
            self.s2 = 0.5;
            self.s3 = 0.5;
            return self.value;
        }

        let diff = close - self.prev_close;
        self.prev_close = close;

        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };

        // Wilder EMA for gain/loss (equivalent to RMA)
        if self.count <= self.period {
            // Seed with SMA
            self.avg_gain += gain;
            self.avg_loss += loss;
            if self.count == self.period {
                self.avg_gain /= self.period as f64;
                self.avg_loss /= self.period as f64;
            }
        } else {
            self.avg_gain = self.avg_gain + self.wilder_alpha * (gain - self.avg_gain);
            self.avg_loss = self.avg_loss + self.wilder_alpha * (loss - self.avg_loss);
        }

        let rsi = if self.avg_loss < 1e-12 {
            1.0 // No losses = fully overbought
        } else {
            let rs = self.avg_gain / self.avg_loss;
            1.0 - 1.0 / (1.0 + rs)
        };

        // 3-level cascaded EMA (Jurik-style lag reduction)
        self.s1 += self.smooth_alpha * (rsi - self.s1);
        self.s2 += self.smooth_alpha * (self.s1 - self.s2);
        self.s3 += self.smooth_alpha * (self.s2 - self.s3);

        self.value = self.s3;
        self.value
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsx_creation() {
        let rsx = Rsx::new(14);
        assert!(!rsx.is_ready());
        assert_eq!(rsx.value().main(), 0.0);
        assert_eq!(rsx.period(), 14);
    }

    #[test]
    fn test_rsx_uptrend() {
        let mut rsx = Rsx::new(10);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsx.is_ready(), "RSX should be ready after 50 bars with period 10");
        assert!(rsx.value().main() > 0.5, "RSX should be > 0.5 in uptrend, got {}", rsx.value().main());
    }

    #[test]
    fn test_rsx_downtrend() {
        let mut rsx = Rsx::new(10);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsx.is_ready(), "RSX should be ready after 50 bars with period 10");
        assert!(rsx.value().main() < 0.5, "RSX should be < 0.5 in downtrend, got {}", rsx.value().main());
    }

    #[test]
    fn test_rsx_range() {
        let mut rsx = Rsx::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = rsx.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if rsx.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "RSX should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_rsx_finite() {
        let mut rsx = Rsx::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_rsx_reset() {
        let mut rsx = Rsx::new(10);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsx.is_ready());
        rsx.reset();
        assert!(!rsx.is_ready());
        assert_eq!(rsx.value().main(), 0.0);
    }
}
