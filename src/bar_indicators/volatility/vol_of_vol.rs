// Volatility of Volatility: rolling std of ATR (or abs returns) over window

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub enum VoVSource {
    Atr(usize, MovingAverageType),
    AbsReturn,
}

#[derive(Clone)]
pub struct VolOfVol {
    source: VoVSource,
    // internal state
    atr: Option<Atr>,
    prev_close: f64,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl VolOfVol {
    pub fn new(source: VoVSource, window: usize) -> Self {
        let atr = match source {
            VoVSource::Atr(p, t) => Some(Atr::new(p, t)),
            _ => None,
        };
        Self {
            source,
            atr,
            prev_close: 0.0,
            window: window.max(2),
            buf: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        if let Some(a) = self.atr.as_mut() {
            a.reset();
        }
        self.prev_close = 0.0;
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let x = match self.source {
            VoVSource::Atr(_, _) => {
                let a = self.atr.as_mut().unwrap();
                a.update_bar(open, high, low, close, volume)
            }
            VoVSource::AbsReturn => {
                if self.prev_close <= 0.0 {
                    self.prev_close = close.max(1e-12);
                    return self.value;
                }
                let r = (close / self.prev_close).ln().abs();
                self.prev_close = close.max(1e-12);
                r
            }
        };

        // roll std
        let old = self.buf[self.idx];
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        self.sum += x - old;
        self.sumsq += x * x - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            self.value = var.max(0.0).sqrt();
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_of_vol_creation_atr() {
        let vov = VolOfVol::new(VoVSource::Atr(14, MovingAverageType::RMA), 20);
        assert!(!vov.is_ready());
        assert_eq!(vov.value().main(), 0.0);
    }

    #[test]
    fn test_vol_of_vol_creation_abs_return() {
        let vov = VolOfVol::new(VoVSource::AbsReturn, 20);
        assert!(!vov.is_ready());
        assert_eq!(vov.value().main(), 0.0);
    }

    #[test]
    fn test_vol_of_vol_warmup() {
        let mut vov = VolOfVol::new(VoVSource::Atr(14, MovingAverageType::RMA), 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vov.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vov.is_ready());
    }

    #[test]
    fn test_vol_of_vol_non_negative() {
        let mut vov = VolOfVol::new(VoVSource::AbsReturn, 20);
        for i in 0..35 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vov.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "VoV should be non-negative");
        }
    }

    #[test]
    fn test_vol_of_vol_reset() {
        let mut vov = VolOfVol::new(VoVSource::Atr(14, MovingAverageType::RMA), 20);
        for i in 0..30 {
            vov.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vov.reset();
        assert!(!vov.is_ready());
        assert_eq!(vov.value().main(), 0.0);
    }
}
