// Traders Dynamic Index (TDI) — RSI with Bollinger Band signal lines.
//
// Algorithm (Dean Malone / published specification):
//   1. rsi_value  = RSI(close, rsi_period)           — raw RSI in [0, 100]
//   2. signal     = EMA(rsi_value, signal_period)     — fast signal line
//   3. bb_basis   = SMA(rsi_value, band_period)       — BB midline
//   4. bb_dev     = StdDev(rsi_value, band_period) * 1.6185  — BB width (published constant)
//   5. upper_band = bb_basis + bb_dev
//   6. lower_band = bb_basis - bb_dev
//
// Output: Triple(rsi_value, signal, bb_basis)
// Bands accessible via upper_band() / lower_band().
//
// Note: rsi_value is normalised to [0, 1] (matching Rsi::update_bar output convention).
// All band arithmetic preserves the same scale.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct Tdi {
    rsi: Rsi,
    signal_ma: MovingAverageProvider,
    // Bollinger band on RSI: rolling SMA + stddev
    band_period: usize,
    band_buf: Vec<f64>,
    band_idx: usize,
    band_sum: f64,
    band_sum_sq: f64,
    rsi_value: f64,
    signal_value: f64,
    /// Bollinger band midline (SMA of RSI over band_period).
    band_basis: f64,
    /// Upper Bollinger band (basis + 1.6185 * stddev).
    band_upper: f64,
    /// Lower Bollinger band (basis - 1.6185 * stddev).
    band_lower: f64,
}

impl Tdi {
    /// Create TDI.
    ///
    /// - `rsi_period`    — RSI lookback (default 13)
    /// - `signal_period` — fast EMA on RSI for signal line (default 2)
    /// - `band_period`   — Bollinger Band SMA period on RSI (default 34)
    pub fn new(rsi_period: usize, signal_period: usize, band_period: usize) -> Self {
        let bp = band_period.max(2);
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            signal_ma: MovingAverageProvider::new(MovingAverageType::EMA, signal_period.max(1)),
            band_period: bp,
            band_buf: Vec::with_capacity(bp),
            band_idx: 0,
            band_sum: 0.0,
            band_sum_sq: 0.0,
            rsi_value: 0.5,
            signal_value: 0.5,
            band_basis: 0.5,
            band_upper: 0.5,
            band_lower: 0.5,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.signal_ma.reset();
        self.band_buf.clear();
        self.band_idx = 0;
        self.band_sum = 0.0;
        self.band_sum_sq = 0.0;
        self.rsi_value = 0.5;
        self.signal_value = 0.5;
        self.band_basis = 0.5;
        self.band_upper = 0.5;
        self.band_lower = 0.5;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }

    #[inline]
    pub fn values(&self) -> (f64, f64, f64) {
        (self.rsi_value, self.signal_value, self.band_basis)
    }

    #[inline]
    pub fn upper_band(&self) -> f64 {
        self.band_upper
    }

    #[inline]
    pub fn lower_band(&self) -> f64 {
        self.band_lower
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.rsi_value, self.signal_value, self.band_basis)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64, f64) {
        let r = self.rsi.update_bar(o, h, l, c, v);
        self.rsi_value = r;
        self.signal_value = self.signal_ma.update_bar(0.0, 0.0, 0.0, r, 0.0);

        // Rolling Bollinger band on RSI
        if self.band_buf.len() < self.band_period {
            self.band_buf.push(r);
            self.band_sum += r;
            self.band_sum_sq += r * r;
        } else {
            let old = self.band_buf[self.band_idx];
            self.band_buf[self.band_idx] = r;
            self.band_sum += r - old;
            self.band_sum_sq += r * r - old * old;
            self.band_idx = (self.band_idx + 1) % self.band_period;
        }

        let n = self.band_buf.len() as f64;
        let mean = self.band_sum / n;
        let variance = (self.band_sum_sq / n - mean * mean).max(0.0);
        let std_dev = variance.sqrt();

        // Published TDI constant: 1.6185 (approximately sqrt(2*pi/2.4) ≈ Malone's choice)
        const TDI_BB_MULT: f64 = 1.6185;
        self.band_basis = mean;
        self.band_upper = mean + TDI_BB_MULT * std_dev;
        self.band_lower = mean - TDI_BB_MULT * std_dev;

        (self.rsi_value, self.signal_value, self.band_basis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tdi_creation() {
        let tdi = Tdi::new(14, 9, 5);
        assert!(!tdi.is_ready());
        // rsi_value = 0.5 on construction, so values() returns (0.5, 0.5, 0.5)
        let (rsi, sig, basis) = tdi.values();
        assert!((rsi - 0.5).abs() < 1e-10);
        assert!((sig - 0.5).abs() < 1e-10);
        assert!((basis - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_tdi_uptrend() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            tdi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tdi.is_ready());
        let (rsi, _, _) = tdi.values();
        assert!(rsi > 0.5, "TDI RSI should be > 0.5 in uptrend, got {}", rsi);
    }

    #[test]
    fn test_tdi_downtrend() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            tdi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tdi.is_ready());
        let (rsi, _, _) = tdi.values();
        assert!(rsi < 0.5, "TDI RSI should be < 0.5 in downtrend, got {}", rsi);
    }

    #[test]
    fn test_tdi_finite_values() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (rsi, signal, band) = tdi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(rsi.is_finite() && signal.is_finite() && band.is_finite());
        }
    }

    #[test]
    fn test_tdi_bands_ordered() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            tdi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(tdi.upper_band() >= tdi.lower_band(),
            "Upper band should be >= lower band");
    }

    #[test]
    fn test_tdi_reset() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            tdi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tdi.is_ready());
        tdi.reset();
        assert!(!tdi.is_ready());
        let (rsi, sig, basis) = tdi.values();
        assert!((rsi - 0.5).abs() < 1e-10);
        assert!((sig - 0.5).abs() < 1e-10);
        assert!((basis - 0.5).abs() < 1e-10);
    }
}
