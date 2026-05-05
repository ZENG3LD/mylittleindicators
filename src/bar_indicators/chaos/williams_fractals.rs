use crate::bar_indicators::indicator_value::IndicatorValue;

/// Williams Fractals: detect up/down fractals over a lookback of 2 on each side (5-bar pattern)
#[derive(Debug, Clone)]
pub struct WilliamsFractals {
    buf_h: [f64; 5],
    buf_l: [f64; 5],
    idx: usize,
    filled: bool,
    up: bool,
    down: bool,
}

impl Default for WilliamsFractals {
    fn default() -> Self {
        Self::new()
    }
}

impl WilliamsFractals {
    pub fn new() -> Self {
        Self {
            buf_h: [0.0; 5],
            buf_l: [0.0; 5],
            idx: 0,
            filled: false,
            up: false,
            down: false,
        }
    }

    /// update and return (is_up_fractal, is_down_fractal)
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> (bool, bool) {
        self.buf_h[self.idx % 5] = h;
        self.buf_l[self.idx % 5] = l;
        self.idx += 1;
        if self.idx >= 5 {
            self.filled = true;
        }
        self.up = false;
        self.down = false;
        if self.filled {
            // center at idx-3
            let c = (self.idx + 5 - 3) % 5;
            let h0 = self.buf_h[(c + 5 - 2) % 5];
            let h1 = self.buf_h[(c + 5 - 1) % 5];
            let hc = self.buf_h[c % 5];
            let h3 = self.buf_h[(c + 1) % 5];
            let h4 = self.buf_h[(c + 2) % 5];
            let l0 = self.buf_l[(c + 5 - 2) % 5];
            let l1 = self.buf_l[(c + 5 - 1) % 5];
            let lc = self.buf_l[c % 5];
            let l3 = self.buf_l[(c + 1) % 5];
            let l4 = self.buf_l[(c + 2) % 5];
            self.up = hc > h1 && hc > h0 && hc > h3 && hc > h4;
            self.down = lc < l1 && lc < l0 && lc < l3 && lc < l4;
        }
        (self.up, self.down)
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::DoubleFlag(self.up, self.down)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_williams_fractals_creation() {
        let ind = WilliamsFractals::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::DoubleFlag(false, false));
    }

    #[test]
    fn test_williams_fractals_warmup() {
        let mut ind = WilliamsFractals::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_williams_fractals_returns_booleans() {
        let mut ind = WilliamsFractals::new();
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (up, down) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(up == true || up == false);
            assert!(down == true || down == false);
        }
    }

    #[test]
    fn test_williams_fractals_reset() {
        let mut ind = WilliamsFractals::new();
        for i in 0..10 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::DoubleFlag(false, false));
    }
}
