// HL Value Area Proxy: rolling middle and bandwidth percentile from High/Low

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct HlValueArea {
    window: usize,
    mids: Vec<f64>,
    bands: Vec<f64>,
    idx: usize,
    filled: bool,
    mid: f64,
    band: f64,
    band_percentile: f64,
}

impl HlValueArea {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            mids: vec![0.0; window.max(1)],
            bands: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            mid: 0.0,
            band: 0.0,
            band_percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.mids.fill(0.0);
        self.bands.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.mid = 0.0;
        self.band = 0.0;
        self.band_percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        _close: f64,
        _volume: f64,
    ) -> (f64, f64, f64) {
        let mid_now = 0.5 * (high + low);
        let band_now = (high - low).max(0.0);
        self.mid = mid_now;
        self.band = band_now;
        self.mids[self.idx] = mid_now;
        self.bands[self.idx] = band_now;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        // percentile of band within window
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.bands[i] <= band_now {
                    count_le += 1;
                }
            }
            self.band_percentile = count_le as f64 / len as f64;
        } else {
            self.band_percentile = 0.0;
        }
        (self.mid, self.band, self.band_percentile)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.mid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hl_value_area_creation() {
        let hlva = HlValueArea::new(20);
        assert!(!hlva.is_ready());
        assert_eq!(hlva.value().main(), 0.0);
    }

    #[test]
    fn test_hl_value_area_warmup() {
        let mut hlva = HlValueArea::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            hlva.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hlva.is_ready());
    }

    #[test]
    fn test_hl_value_area_values() {
        let mut hlva = HlValueArea::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (mid, band, pct) = hlva.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(mid > 0.0, "Mid should be positive");
            assert!(band >= 0.0, "Band should be non-negative");
            assert!(pct >= 0.0 && pct <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_hl_value_area_reset() {
        let mut hlva = HlValueArea::new(20);
        for i in 0..25 {
            hlva.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        hlva.reset();
        assert!(!hlva.is_ready());
        assert_eq!(hlva.value().main(), 0.0);
    }
}
