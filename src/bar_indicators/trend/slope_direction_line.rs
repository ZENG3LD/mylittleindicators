// Slope Direction Line (SDL): sign of slope of a smoothing MA

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SlopeDirectionLine {
    period: usize,
    ma_type: MovingAverageType,
    ma: MovingAverageProvider,
    prev: f64,
    curr: f64,
    dir: i8,
}

impl SlopeDirectionLine {
    /// Create Slope Direction Line with default MA type (SMA)
    pub fn new(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::SMA)
    }

    /// Create Slope Direction Line with specified MA type
    pub fn new_with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            period,
            ma_type,
            ma: MovingAverageProvider::new(ma_type, period),
            prev: 0.0,
            curr: 0.0,
            dir: 0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma = MovingAverageProvider::new(self.ma_type, self.period);
        self.prev = 0.0;
        self.curr = 0.0;
        self.dir = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> i8 {
        let v = self.ma.update_bar(open, high, low, close, volume);
        self.prev = self.curr;
        self.curr = v;
        self.dir = if self.curr > self.prev {
            1
        } else if self.curr < self.prev {
            -1
        } else {
            0
        };
        self.dir
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slope_direction_line_creation() {
        let sdl = SlopeDirectionLine::new(10);
        assert!(!sdl.is_ready());
    }

    #[test]
    fn test_slope_direction_line_warmup() {
        let mut sdl = SlopeDirectionLine::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sdl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sdl.is_ready());
    }

    #[test]
    fn test_slope_direction_line_signal_range() {
        let mut sdl = SlopeDirectionLine::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let dir = sdl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(dir >= -1 && dir <= 1, "Direction should be -1, 0, or 1");
        }
    }

    #[test]
    fn test_slope_direction_line_reset() {
        let mut sdl = SlopeDirectionLine::new(10);
        for i in 0..15 {
            sdl.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        sdl.reset();
        assert!(!sdl.is_ready());
    }
}
