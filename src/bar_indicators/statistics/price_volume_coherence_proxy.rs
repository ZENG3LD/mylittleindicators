// Price-Volume Coherence proxy via rolling absolute correlation of returns and volume changes

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct PriceVolumeCoherenceProxy {
    window: usize,
    buf_price: Vec<f64>,
    buf_volume: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_price: Option<f64>,
    prev_volume: Option<f64>,
    pub value: f64,
}

impl PriceVolumeCoherenceProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(16);
        Self {
            window: w,
            buf_price: vec![0.0; w],
            buf_volume: vec![0.0; w],
            idx: 0,
            filled: false,
            prev_price: None,
            prev_volume: None,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf_price.fill(0.0);
        self.buf_volume.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_price = None;
        self.prev_volume = None;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        let pr = if let Some(p) = self.prev_price {
            (c / p).ln()
        } else {
            0.0
        };
        let vr = if let Some(u) = self.prev_volume {
            if u > 0.0 {
                (v / u).ln()
            } else {
                0.0
            }
        } else {
            0.0
        };
        self.prev_price = Some(c.max(1e-12));
        self.prev_volume = Some(v.max(1e-9));
        self.buf_price[self.idx] = pr;
        self.buf_volume[self.idx] = vr;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let n = if self.filled { self.window } else { self.idx };
        if n >= 2 {
            let mut sx = 0.0;
            let mut sy = 0.0;
            let mut sxx = 0.0;
            let mut syy = 0.0;
            let mut sxy = 0.0;
            for i in 0..n {
                let x = self.buf_price[i];
                let y = self.buf_volume[i];
                sx += x;
                sy += y;
                sxx += x * x;
                syy += y * y;
                sxy += x * y;
            }
            let nn = n as f64;
            let num = nn * sxy - sx * sy;
            let den = ((nn * sxx - sx * sx) * (nn * syy - sy * sy))
                .max(1e-24)
                .sqrt();
            let r = if den > 0.0 {
                (num / den).abs().min(1.0)
            } else {
                0.0
            };
            self.value = r;
        }
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_volume_coherence_proxy_creation() {
        let pvc = PriceVolumeCoherenceProxy::new(50);
        assert!(!pvc.is_ready());
        assert_eq!(pvc.value, 0.0);
    }

    #[test]
    fn test_price_volume_coherence_proxy_warmup() {
        let mut pvc = PriceVolumeCoherenceProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let vol = 1000.0 + (i as f64 * 0.2).sin() * 200.0;
            pvc.update_bar(price, price + 1.0, price - 1.0, price, vol);
        }
        assert!(pvc.is_ready());
    }

    #[test]
    fn test_price_volume_coherence_proxy_range() {
        let mut pvc = PriceVolumeCoherenceProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let vol = 1000.0 + (i as f64 * 0.3).sin() * 300.0;
            let value = pvc.update_bar(price, price + 1.0, price - 1.0, price, vol);
            assert!(value >= 0.0 && value <= 1.0, "Coherence should be in [0, 1]");
        }
    }

    #[test]
    fn test_price_volume_coherence_proxy_reset() {
        let mut pvc = PriceVolumeCoherenceProxy::new(50);
        for i in 0..60 {
            pvc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pvc.reset();
        assert!(!pvc.is_ready());
        assert_eq!(pvc.value, 0.0);
    }
}
