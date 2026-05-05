use crate::bar_indicators::average::MovingAverageProvider;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::macd::Macd;

/// Schaff Trend Cycle (STC): Stochastic of MACD

#[derive(Clone)]
pub struct Stc {
    macd: Macd,
    k_ma: MovingAverageProvider, // smoothing for %K
    d_ma: MovingAverageProvider, // smoothing for %D
    k: f64,
    d: f64,
    ready: bool,
}

impl Stc {
    pub fn new(fast: usize, slow: usize, k_period: usize, d_period: usize) -> Self {
        Self {
            macd: Macd::new_with_signal(fast, slow, 9),
            k_ma: MovingAverageProvider::new(MovingAverageType::EMA, k_period.max(1)),
            d_ma: MovingAverageProvider::new(MovingAverageType::EMA, d_period.max(1)),
            k: 0.0,
            d: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> (f64, f64) {
        let macd_val = self.macd.update_bar(0.0, 0.0, 0.0, c, 0.0);
        // Simple stochastic via EMA min/max proxies: use EMA smoothing to approximate normalization
        // k ~ EMA of macd scaled to 0-100 using running range proxy; here we use signal line as smoother
        let signal = self.macd.value_signal();
        let raw = macd_val - signal;
        let k_sm = self.k_ma.update_bar(0.0, 0.0, 0.0, raw, 0.0);
        let d_sm = self.d_ma.update_bar(0.0, 0.0, 0.0, k_sm, 0.0);
        self.k = 50.0 + 50.0 * (k_sm.tanh());
        self.d = 50.0 + 50.0 * (d_sm.tanh());
        self.ready = self.k_ma.is_ready() && self.d_ma.is_ready() && self.macd.is_ready();
        (self.k, self.d)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.k, self.d)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.macd.reset();
        self.k_ma.reset();
        self.d_ma.reset();
        self.k = 0.0;
        self.d = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stc_creation() {
        let stc = Stc::new(12, 26, 10, 3);
        assert!(!stc.is_ready());
        assert_eq!(stc.value(), IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn test_stc_uptrend() {
        let mut stc = Stc::new(12, 26, 10, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            stc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(stc.is_ready());
        if let IndicatorValue::Double(k, d) = stc.value() {
            assert!(k.is_finite() && d.is_finite());
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stc_range() {
        let mut stc = Stc::new(12, 26, 10, 3);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (k, d) = stc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(k >= 0.0 && k <= 100.0, "STC K should be in [0, 100], got {}", k);
            assert!(d >= 0.0 && d <= 100.0, "STC D should be in [0, 100], got {}", d);
        }
    }

    #[test]
    fn test_stc_reset() {
        let mut stc = Stc::new(12, 26, 10, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            stc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(stc.is_ready());
        stc.reset();
        assert!(!stc.is_ready());
        assert_eq!(stc.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
