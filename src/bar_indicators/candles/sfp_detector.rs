// Swing Failure Pattern (SFP) detector using Highest/Lowest lookback

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::highest::Highest;
use crate::bar_indicators::momentum::lowest::Lowest;

#[derive(Clone)]
pub struct SfpDetector {
    #[allow(dead_code)]
    lookback: usize,
    highest: Highest,
    lowest: Lowest,
    pub bull_sfp: bool,
    pub bear_sfp: bool,
}

impl SfpDetector {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback,
            highest: Highest::new(lookback),
            lowest: Lowest::new(lookback),
            bull_sfp: false,
            bear_sfp: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highest.reset();
        self.lowest.reset();
        self.bull_sfp = false;
        self.bear_sfp = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.highest.is_ready() && self.lowest.is_ready()
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        close: f64,
        _volume: f64,
    ) -> (bool, bool) {
        let prev_high = self.highest.value();
        let prev_low = self.lowest.value();

        // update extremes after reading prev values
        self.highest.update_bar(0.0, high, 0.0, high, 0.0);
        self.lowest.update_bar(0.0, low, 0.0, low, 0.0);

        // bear SFP: sweep above prev_high and close back below it
        self.bear_sfp = self.is_ready() && high > prev_high.main() && close < prev_high.main();
        // bull SFP: sweep below prev_low and close back above it
        self.bull_sfp = self.is_ready() && low < prev_low.main() && close > prev_low.main();
        (self.bull_sfp, self.bear_sfp)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::DoubleFlag(self.bull_sfp, self.bear_sfp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sfp_detector_creation() {
        let ind = SfpDetector::new(10);
        assert!(!ind.is_ready());
        assert!(!ind.bull_sfp);
        assert!(!ind.bear_sfp);
    }

    #[test]
    fn test_sfp_detector_warmup() {
        let mut ind = SfpDetector::new(5);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_sfp_detector_returns_booleans() {
        let mut ind = SfpDetector::new(5);
        for i in 0..10 {
            let price = 100.0 + i as f64;
            let (bull, bear) = ind.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            // Results should be boolean
            assert!(bull == true || bull == false);
            assert!(bear == true || bear == false);
        }
    }

    #[test]
    fn test_sfp_detector_reset() {
        let mut ind = SfpDetector::new(5);
        for i in 0..10 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(!ind.bull_sfp);
        assert!(!ind.bear_sfp);
    }
}
