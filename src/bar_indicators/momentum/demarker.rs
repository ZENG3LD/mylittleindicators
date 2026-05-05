/// DeMarker (DeM) oscillator
use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct Demarker {
    period: usize,
    sum_up: f64,
    sum_down: f64,
    prev_high: f64,
    prev_low: f64,
    initialized: bool,
    value: f64,
}

impl Demarker {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            sum_up: 0.0,
            sum_down: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            initialized: false,
            value: 0.0,
        }
    }

    /// Rolling sums via simple decay (EMA-like) to keep O(1) and avoid large buffers.
    /// For strict SMA window one could use ring buffer of diffs; here we use Wilder smoothing for robustness.
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> f64 {
        if !self.initialized {
            self.prev_high = h;
            self.prev_low = l;
            self.initialized = true;
            self.value = 0.0;
            return self.value;
        }
        let up = (h - self.prev_high).max(0.0);
        let down = (self.prev_low - l).max(0.0);
        // Wilder-like smoothing
        let p = self.period as f64;
        self.sum_up = (self.sum_up * (p - 1.0) + up) / p;
        self.sum_down = (self.sum_down * (p - 1.0) + down) / p;
        let denom = self.sum_up + self.sum_down;
        self.value = if denom > 0.0 {
            self.sum_up / denom
        } else {
            0.0
        };
        self.prev_high = h;
        self.prev_low = l;
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    pub fn reset(&mut self) {
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demarker_creation() {
        let dem = Demarker::new(14);
        assert!(!dem.is_ready());
        assert_eq!(dem.value().main(), 0.0);
        assert_eq!(dem.period(), 14);
    }

    #[test]
    fn test_demarker_basic_calculation() {
        let mut dem = Demarker::new(14);

        for i in 1..=30 {
            let price = 100.0 + i as f64;
            let value = dem.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);

            if i > 1 {
                // DeMarker oscillates between 0 and 1
                assert!(value >= 0.0 && value <= 1.0, "DeMarker should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_demarker_uptrend() {
        let mut dem = Demarker::new(14);

        // Strong uptrend - highs increasing more than lows
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            dem.update_bar(price, price + 3.0, price - 0.5, price + 2.0, 1000.0);
        }

        // In uptrend, DeMarker tends to be > 0.5
        if dem.is_ready() {
            assert!(dem.value().main() > 0.3, "DeMarker in uptrend should be elevated");
        }
    }

    #[test]
    fn test_demarker_downtrend() {
        let mut dem = Demarker::new(14);

        // Strong downtrend - lows decreasing more than highs
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            dem.update_bar(price, price + 0.5, price - 3.0, price - 2.0, 1000.0);
        }

        // In downtrend, DeMarker tends to be < 0.5
        if dem.is_ready() {
            assert!(dem.value().main() < 0.7, "DeMarker in downtrend should be suppressed");
        }
    }

    #[test]
    fn test_demarker_reset() {
        let mut dem = Demarker::new(14);

        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dem.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(dem.is_ready());

        dem.reset();
        assert!(!dem.is_ready());
        assert_eq!(dem.value().main(), 0.0);
    }

    #[test]
    fn test_demarker_period() {
        let dem = Demarker::new(14);
        assert_eq!(dem.period(), 14);

        let dem2 = Demarker::new(21);
        assert_eq!(dem2.period(), 21);
    }

    #[test]
    fn test_demarker_is_ready() {
        let mut dem = Demarker::new(14);
        assert!(!dem.is_ready());

        // First update initializes
        dem.update_bar(100.0, 101.0, 99.0, 100.5, 1000.0);
        assert!(dem.is_ready()); // Initialized after first bar
    }

    #[test]
    fn test_demarker_sideways() {
        let mut dem = Demarker::new(14);

        // Sideways market - equal up and down movements
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 2.0;
            dem.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        // In sideways, DeMarker should be around 0.5
        if dem.is_ready() {
            let value = dem.value().main();
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_demarker_extreme_values() {
        let mut dem = Demarker::new(5);

        // Extreme upward movement
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 5.0;
            dem.update_bar(price, price + 10.0, price - 0.1, price + 8.0, 1000.0);
        }

        if dem.is_ready() {
            let value = dem.value().main();
            // Should be close to 1.0 in extreme uptrend
            assert!(value > 0.5, "DeMarker should be high in extreme uptrend, got {}", value);
        }
    }
}
