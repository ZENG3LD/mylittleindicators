use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::volatility::atr::Atr;

/// Random Walk Index (simplified):
/// rwi_up = (High - prev_low) / (ATR(period) * sqrt(period))
/// rwi_down = (prev_high - Low) / (ATR(period) * sqrt(period))
#[derive(Debug, Clone)]
pub struct Rwi {
    period: usize,
    atr: Atr,
    prev_high: Option<f64>,
    prev_low: Option<f64>,
    up: f64,
    down: f64,
}

impl Rwi {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(2),
            atr: Atr::new(period.max(2), MovingAverageType::RMA),
            prev_high: None,
            prev_low: None,
            up: 0.0,
            down: 0.0,
        }
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64) {
        let _ = self.atr.update_bar(0.0, h, l, c, v);
        let atrv = self.atr.value().main();
        let denom = (atrv * (self.period as f64).sqrt()).max(1e-12);
        if let (Some(ph), Some(pl)) = (self.prev_high, self.prev_low) {
            self.up = (h - pl).max(0.0) / denom;
            self.down = (ph - l).max(0.0) / denom;
        }
        self.prev_high = Some(h);
        self.prev_low = Some(l);
        (self.up, self.down)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.up, self.down)
    }
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready() && self.prev_high.is_some() && self.prev_low.is_some()
    }
    pub fn reset(&mut self) {
        self.atr.reset();
        self.prev_high = None;
        self.prev_low = None;
        self.up = 0.0;
        self.down = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rwi_creation() {
        let rwi = Rwi::new(14);
        assert!(!rwi.is_ready());
        assert_eq!(rwi.value(), IndicatorValue::Double(0.0, 0.0));
        assert_eq!(rwi.period(), 14);
    }

    #[test]
    fn test_rwi_uptrend() {
        let mut rwi = Rwi::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            rwi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rwi.is_ready());
        if let IndicatorValue::Double(up, down) = rwi.value() {
            assert!(up > down, "RWI up should > down in uptrend, got up={}, down={}", up, down);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_rwi_downtrend() {
        let mut rwi = Rwi::new(14);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            rwi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rwi.is_ready());
        if let IndicatorValue::Double(up, down) = rwi.value() {
            assert!(down > up, "RWI down should > up in downtrend, got up={}, down={}", up, down);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_rwi_finite_values() {
        let mut rwi = Rwi::new(14);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (up, down) = rwi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(up.is_finite() && down.is_finite(), "RWI values should always be finite");
            assert!(up >= 0.0 && down >= 0.0, "RWI values should be non-negative");
        }
    }

    #[test]
    fn test_rwi_reset() {
        let mut rwi = Rwi::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            rwi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rwi.is_ready());
        rwi.reset();
        assert!(!rwi.is_ready());
        assert_eq!(rwi.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
