// Candle Anatomy: body, upper wick, lower wick ratios and flags

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone, Copy, Debug, Default)]
pub struct CandleAnatomyValue {
    pub body: f64,
    pub upper_wick: f64,
    pub lower_wick: f64,
    pub long_upper: bool,
    pub long_lower: bool,
}

#[derive(Clone)]
pub struct CandleAnatomy {
    pub value: CandleAnatomyValue,
    pub lower_thr: f64,
    pub upper_thr: f64,
}

impl CandleAnatomy {
    pub fn new(long_wick_ratio_threshold: f64) -> Self {
        Self {
            value: CandleAnatomyValue::default(),
            lower_thr: long_wick_ratio_threshold,
            upper_thr: long_wick_ratio_threshold,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.value = CandleAnatomyValue::default();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        _volume: f64,
    ) -> CandleAnatomyValue {
        let range = (high - low).abs();
        if range <= 1e-12 {
            self.value = CandleAnatomyValue {
                body: 0.0,
                upper_wick: 0.0,
                lower_wick: 0.0,
                long_upper: false,
                long_lower: false,
            };
            return self.value;
        }
        let body = (close - open).abs() / range;
        let upper = (high - open.max(close)).max(0.0) / range;
        let lower = (open.min(close) - low).max(0.0) / range;
        self.value = CandleAnatomyValue {
            body,
            upper_wick: upper,
            lower_wick: lower,
            long_upper: upper >= self.upper_thr,
            long_lower: lower >= self.lower_thr,
        };
        self.value
    }

    /// Получить значение как CandleAnatomyValue (legacy)
    #[inline]
    pub fn anatomy_value(&self) -> CandleAnatomyValue {
        self.value
    }

    /// Получить значение в виде IndicatorValue
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::CandleAnatomy {
            body: self.value.body,
            upper_wick: self.value.upper_wick,
            lower_wick: self.value.lower_wick,
            long_upper: self.value.long_upper,
            long_lower: self.value.long_lower,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_anatomy_creation() {
        let ind = CandleAnatomy::new(0.3);
        assert!(ind.is_ready());
        assert_eq!(ind.value.body, 0.0);
    }

    #[test]
    fn test_candle_anatomy_update() {
        let mut ind = CandleAnatomy::new(0.3);
        let result = ind.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        assert!(result.body >= 0.0 && result.body <= 1.0);
        assert!(result.upper_wick >= 0.0 && result.upper_wick <= 1.0);
        assert!(result.lower_wick >= 0.0 && result.lower_wick <= 1.0);
    }

    #[test]
    fn test_candle_anatomy_long_wicks() {
        let mut ind = CandleAnatomy::new(0.3);
        // Long lower wick candle (hammer-like)
        let result = ind.update_bar(100.0, 102.0, 90.0, 101.0, 1000.0);
        assert!(result.lower_wick > result.upper_wick);
    }

    #[test]
    fn test_candle_anatomy_reset() {
        let mut ind = CandleAnatomy::new(0.3);
        ind.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        ind.reset();
        assert_eq!(ind.value.body, 0.0);
        assert_eq!(ind.value.upper_wick, 0.0);
        assert_eq!(ind.value.lower_wick, 0.0);
    }
}
