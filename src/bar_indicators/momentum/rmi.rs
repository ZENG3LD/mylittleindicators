use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Relative Momentum Index (RMI)
/// RMI_t = 100 * EMA_n( max(0, C_t - C_{t-m}) ) / ( EMA_n(max(0, C_t - C_{t-m})) + EMA_n(max(0, C_{t-m} - C_t)) )
#[derive(Debug, Clone)]
pub struct Rmi {
    momentum_lookback: usize,
    ma_type: MovingAverageType,
    ema_period: usize,
    up_ma: MovingAverageProvider,
    down_ma: MovingAverageProvider,
    closes: VecDeque<f64>,
    value: f64,
    ready: bool,
}

impl Rmi {
    pub fn new(momentum_lookback: usize, ema_period: usize) -> Self {
        Self::new_with_ma_type(momentum_lookback, ema_period, MovingAverageType::RMA)
    }

    pub fn new_default(momentum_lookback: usize, ema_period: usize) -> Self {
        Self::new_with_ma_type(momentum_lookback, ema_period, MovingAverageType::RMA)
    }

    pub fn new_with_ma_type(momentum_lookback: usize, ema_period: usize, ma_type: MovingAverageType) -> Self {
        let m = momentum_lookback.max(1);
        let n = ema_period.max(1);
        Self {
            momentum_lookback: m,
            ma_type,
            ema_period: n,
            up_ma: MovingAverageProvider::new(ma_type, n),
            down_ma: MovingAverageProvider::new(ma_type, n),
            closes: VecDeque::with_capacity(m + 1),
            value: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.closes.push_back(c);
        if self.closes.len() > self.momentum_lookback + 1 {
            self.closes.pop_front();
        }

        if self.closes.len() <= self.momentum_lookback {
            self.value = 0.0;
            self.ready = false;
            return self.value;
        }

        let base = self.closes[0];
        let diff = c - base;
        let up = if diff > 0.0 { diff } else { 0.0 };
        let down = if diff < 0.0 { -diff } else { 0.0 };

        let up_avg = self.up_ma.update_bar(0.0, 0.0, 0.0, up, 0.0);
        let down_avg = self.down_ma.update_bar(0.0, 0.0, 0.0, down, 0.0);
        let denom = up_avg + down_avg;
        self.value = if denom > 0.0 {
            100.0 * (up_avg / denom)
        } else {
            50.0
        };
        self.ready = self.up_ma.is_ready() && self.down_ma.is_ready();
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.closes.clear();
        self.up_ma = MovingAverageProvider::new(self.ma_type, self.ema_period);
        self.down_ma = MovingAverageProvider::new(self.ma_type, self.ema_period);
        self.value = 0.0;
        self.ready = false;
    }

    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    pub fn momentum_lookback(&self) -> usize {
        self.momentum_lookback
    }

    pub fn ema_period(&self) -> usize {
        self.ema_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rmi_creation() {
        let rmi = Rmi::new(5, 14);
        assert!(!rmi.is_ready());
        assert_eq!(rmi.value().main(), 0.0);
        assert_eq!(rmi.momentum_lookback(), 5);
        assert_eq!(rmi.ema_period(), 14);
    }

    #[test]
    fn test_rmi_default() {
        let rmi = Rmi::new_default(5, 14);
        assert!(!rmi.is_ready());
    }

    #[test]
    fn test_rmi_with_ma_type() {
        let rmi = Rmi::new_with_ma_type(5, 14, MovingAverageType::EMA);
        assert!(!rmi.is_ready());
    }

    #[test]
    fn test_rmi_uptrend() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            rmi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rmi.is_ready());
        // In uptrend, RMI should be > 50
        assert!(rmi.value().main() > 50.0, "RMI should be > 50 in uptrend, got {}", rmi.value().main());
    }

    #[test]
    fn test_rmi_downtrend() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            rmi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rmi.is_ready());
        // In downtrend, RMI should be < 50
        assert!(rmi.value().main() < 50.0, "RMI should be < 50 in downtrend, got {}", rmi.value().main());
    }

    #[test]
    fn test_rmi_range_bounds() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rmi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if rmi.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "RMI should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_rmi_reset() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            rmi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rmi.is_ready());
        rmi.reset();
        assert!(!rmi.is_ready());
        assert_eq!(rmi.value().main(), 0.0);
    }

    #[test]
    fn test_rmi_set_ma_type() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            rmi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rmi.is_ready());
        rmi.set_ma_type(MovingAverageType::EMA);
        assert!(!rmi.is_ready()); // should reset
    }

    #[test]
    fn test_rmi_finite_values() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = rmi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "RMI should always be finite");
        }
    }
}
