use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Relative Vigor Index (RVGI): numerator = SMA((close-open)), denominator = SMA((high-low))
#[derive(Debug, Clone)]
pub struct Rvgi {
    num_ma: MovingAverageProvider,
    den_ma: MovingAverageProvider,
    signal_ma: MovingAverageProvider,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Rvgi {
    pub fn new(period: usize, signal_period: usize) -> Self {
        let ma = MovingAverageType::SMA;
        Self {
            num_ma: MovingAverageProvider::new(ma, period.max(1)),
            den_ma: MovingAverageProvider::new(ma, period.max(1)),
            signal_ma: MovingAverageProvider::new(ma, signal_period.max(1)),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let num = close - open;
        let den = (high - low).max(1e-12);
        self.num_ma.update_bar(0.0, 0.0, 0.0, num, 0.0);
        self.den_ma.update_bar(0.0, 0.0, 0.0, den, 0.0);
        let n = self.num_ma.value().main();
        let d = self.den_ma.value().main();
        let ratio = if d.abs() < 1e-12 { 0.0 } else { n / d };
        self.value = ratio;
        if self.num_ma.is_ready() && self.den_ma.is_ready() {
            self.signal_ma.update_bar(0.0, 0.0, 0.0, self.value, 0.0);
            self.signal = self.signal_ma.value().main();
        }
        self.ready = self.num_ma.is_ready() && self.den_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }

    /// Получить все значения RVGI как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.value, self.signal)
    }

    /// Значение RVGI линии (для обратной совместимости)
    pub fn value_rvgi(&self) -> f64 {
        self.value
    }

    /// Значение сигнальной линии
    pub fn value_signal(&self) -> f64 {
        self.signal
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.num_ma.reset();
        self.den_ma.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rvgi_creation() {
        let rvgi = Rvgi::new(10, 4);
        assert!(!rvgi.is_ready());
        assert_eq!(rvgi.value_rvgi(), 0.0);
        assert_eq!(rvgi.value_signal(), 0.0);
    }

    #[test]
    fn test_rvgi_uptrend() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=30 {
            let open = 100.0 + (i - 1) as f64 * 2.0;
            let close = 100.0 + i as f64 * 2.0;
            rvgi.update_bar(open, close + 1.0, open - 1.0, close, 1000.0);
        }
        assert!(rvgi.is_ready());
        assert!(rvgi.value_rvgi() > 0.0, "RVGI should be > 0 in uptrend, got {}", rvgi.value_rvgi());
    }

    #[test]
    fn test_rvgi_downtrend() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=30 {
            let open = 200.0 - (i - 1) as f64 * 2.0;
            let close = 200.0 - i as f64 * 2.0;
            rvgi.update_bar(open, open + 1.0, close - 1.0, close, 1000.0);
        }
        assert!(rvgi.is_ready());
        assert!(rvgi.value_rvgi() < 0.0, "RVGI should be < 0 in downtrend, got {}", rvgi.value_rvgi());
    }

    #[test]
    fn test_rvgi_finite_values() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rvgi.update_bar(price - 1.0, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "RVGI should always be finite");
        }
    }

    #[test]
    fn test_rvgi_reset() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            rvgi.update_bar(price - 1.0, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rvgi.is_ready());
        rvgi.reset();
        assert!(!rvgi.is_ready());
        assert_eq!(rvgi.value_rvgi(), 0.0);
        assert_eq!(rvgi.value_signal(), 0.0);
    }
}
