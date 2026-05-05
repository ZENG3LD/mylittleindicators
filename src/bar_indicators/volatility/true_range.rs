/// True Range: max(high-low, |high-prev_close|, |low-prev_close|)
use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct TrueRange {
    prev_close: Option<f64>,
    value: f64,
}

impl Default for TrueRange {
    fn default() -> Self {
        Self::new()
    }
}

impl TrueRange {
    pub fn new() -> Self {
        Self {
            prev_close: None,
            value: 0.0,
        }
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        let range_hl = (h - l).abs();
        let tr = if let Some(pc) = self.prev_close {
            range_hl.max((h - pc).abs()).max((l - pc).abs())
        } else {
            range_hl
        };
        self.value = tr;
        self.prev_close = Some(c);
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.prev_close.is_some()
    }
    pub fn reset(&mut self) {
        self.prev_close = None;
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_range_creation() {
        let tr = TrueRange::new();
        assert!(!tr.is_ready());
        assert_eq!(tr.value().main(), 0.0);
    }

    #[test]
    fn test_true_range_first_bar() {
        let mut tr = TrueRange::new();
        let value = tr.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert_eq!(value, 10.0); // high - low = 105 - 95
        assert!(tr.is_ready());
    }

    #[test]
    fn test_true_range_gap_up() {
        let mut tr = TrueRange::new();
        tr.update_bar(100.0, 105.0, 95.0, 104.0, 1000.0);
        // Next bar gaps up: prev_close=104, high=115, low=110
        let value = tr.update_bar(110.0, 115.0, 110.0, 113.0, 1000.0);
        // TR = max(115-110, |115-104|, |110-104|) = max(5, 11, 6) = 11
        assert_eq!(value, 11.0);
    }

    #[test]
    fn test_true_range_gap_down() {
        let mut tr = TrueRange::new();
        tr.update_bar(100.0, 105.0, 95.0, 96.0, 1000.0);
        // Next bar gaps down: prev_close=96, high=90, low=85
        let value = tr.update_bar(90.0, 90.0, 85.0, 87.0, 1000.0);
        // TR = max(90-85, |90-96|, |85-96|) = max(5, 6, 11) = 11
        assert_eq!(value, 11.0);
    }

    #[test]
    fn test_true_range_reset() {
        let mut tr = TrueRange::new();
        tr.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        tr.update_bar(102.0, 107.0, 97.0, 105.0, 1000.0);
        tr.reset();
        assert!(!tr.is_ready());
        assert_eq!(tr.value().main(), 0.0);
    }
}
