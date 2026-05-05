// Percentile Channels: upper/lower as rolling percentiles of price (close) or range

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::bar_indicators::utils::math::percentile::quickselect_nth;

#[derive(Clone)]
pub enum PercentileBasis {
    Close,
    Range,
}

#[derive(Clone)]
pub struct PercentileChannels {
    window: usize,
    basis: PercentileBasis,
    source: OhlcvField,
    upper_q: f64,
    lower_q: f64,
    // buffers
    close_buf: Vec<f64>,
    range_buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

impl PercentileChannels {
    pub fn new(window: usize, basis: PercentileBasis, lower_q: f64, upper_q: f64) -> Self {
        Self {
            window,
            basis,
            source: OhlcvField::Close,
            upper_q: upper_q.clamp(0.0, 1.0),
            lower_q: lower_q.clamp(0.0, 1.0),
            close_buf: vec![0.0; window.max(1)],
            range_buf: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    pub fn with_source(window: usize, basis: PercentileBasis, lower_q: f64, upper_q: f64, source: OhlcvField) -> Self {
        Self {
            window,
            basis,
            source,
            upper_q: upper_q.clamp(0.0, 1.0),
            lower_q: lower_q.clamp(0.0, 1.0),
            close_buf: vec![0.0; window.max(1)],
            range_buf: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.close_buf.fill(0.0);
        self.range_buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64, f64) {
        let price = self.source.extract(open, high, low, close, volume);
        self.close_buf[self.idx] = price;
        self.range_buf[self.idx] = (high - low).abs();
        self.idx = (self.idx + 1) % self.window.max(1);
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len == 0 {
            return (self.lower, self.middle, self.upper);
        }

        match self.basis {
            PercentileBasis::Close => {
                self.lower = percentile_of(&self.close_buf, len, self.lower_q);
                self.upper = percentile_of(&self.close_buf, len, self.upper_q);
                self.middle = 0.5 * (self.lower + self.upper);
            }
            PercentileBasis::Range => {
                self.lower = percentile_of(&self.range_buf, len, self.lower_q);
                self.upper = percentile_of(&self.range_buf, len, self.upper_q);
                self.middle = 0.5 * (self.lower + self.upper);
            }
        }
        (self.lower, self.middle, self.upper)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }
}

#[inline]
fn percentile_of(buf: &Vec<f64>, len: usize, q: f64) -> f64 {
    // 🚀 O(n) quickselect instead of O(n log n) sorting
    if len == 0 {
        return 0.0;
    }
    let mut tmp = Vec::with_capacity(len);
    tmp.extend_from_slice(&buf[..len]);
    let pos = (q.clamp(0.0, 1.0) * (len as f64 - 1.0)).round() as usize;
    quickselect_nth(&mut tmp, pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_channels_creation() {
        let pc = PercentileChannels::new(20, PercentileBasis::Close, 0.1, 0.9);
        assert!(!pc.is_ready());
        assert_eq!(pc.upper, 0.0);
        assert_eq!(pc.lower, 0.0);
    }

    #[test]
    fn test_percentile_channels_warmup() {
        let mut pc = PercentileChannels::new(20, PercentileBasis::Close, 0.1, 0.9);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pc.is_ready());
    }

    #[test]
    fn test_percentile_channels_values() {
        let mut pc = PercentileChannels::new(20, PercentileBasis::Close, 0.1, 0.9);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pc.upper >= pc.middle);
        assert!(pc.middle >= pc.lower);
    }

    #[test]
    fn test_percentile_channels_range_basis() {
        let mut pc = PercentileChannels::new(20, PercentileBasis::Range, 0.25, 0.75);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            pc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(pc.is_ready());
        assert!(pc.upper >= pc.lower);
    }

    #[test]
    fn test_percentile_channels_reset() {
        let mut pc = PercentileChannels::new(20, PercentileBasis::Close, 0.1, 0.9);
        for i in 0..25 {
            pc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pc.reset();
        assert!(!pc.is_ready());
        assert_eq!(pc.upper, 0.0);
        assert_eq!(pc.lower, 0.0);
    }
}
