// High-performance Directional Movement (DM, +DI, -DI, ADX)
// (c) 2024
// OPTIMIZED: O(1) running sum instead of O(n) iter().sum()

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Индикатор Directional Movement (DM):
/// - plus_di: индекс положительного направленного движения (+DI)
/// - minus_di: индекс отрицательного направленного движения (-DI)
/// - adx: индекс силы тренда (ADX)
#[derive(Clone)]
pub struct Dm {
    period: usize,
    tr_buf: ArrayVec<f64, 512>,
    plus_dm_buf: ArrayVec<f64, 512>,
    minus_dm_buf: ArrayVec<f64, 512>,

    // Running sums for O(1) calculation
    sum_tr: f64,
    sum_plus_dm: f64,
    sum_minus_dm: f64,

    index: usize,
    filled: bool,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    plus_di: f64,
    minus_di: f64,
    adx: f64,
    adx_buf: ArrayVec<f64, 512>,
    adx_idx: usize,
    adx_filled: bool,

    // Running sum for ADX
    sum_adx: f64,
}

impl Dm {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            tr_buf: ArrayVec::from([0.0; 512]),
            plus_dm_buf: ArrayVec::from([0.0; 512]),
            minus_dm_buf: ArrayVec::from([0.0; 512]),
            sum_tr: 0.0,
            sum_plus_dm: 0.0,
            sum_minus_dm: 0.0,
            index: 0,
            filled: false,
            prev_high: 0.0,
            prev_low: 0.0,
            prev_close: 0.0,
            plus_di: 0.0,
            minus_di: 0.0,
            adx: 0.0,
            adx_buf: ArrayVec::from([0.0; 512]),
            adx_idx: 0,
            adx_filled: false,
            sum_adx: 0.0,
        }
    }

    /// Обновить DM новым баром (используются high, low, close)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64, f64) {
        if self.index == 0 && !self.filled && self.prev_high == 0.0 && self.prev_low == 0.0 {
            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;
            self.index = 1;
            return (self.plus_di, self.minus_di, self.adx);
        }

        let tr = (high - low)
            .max((high - self.prev_close).abs())
            .max((low - self.prev_close).abs());
        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;
        let plus_dm = if up_move > down_move && up_move > 0.0 { up_move } else { 0.0 };
        let minus_dm = if down_move > up_move && down_move > 0.0 { down_move } else { 0.0 };

        let idx = self.index % self.period;

        // Update ring buffer with O(1) running sum tracking
        if self.filled {
            // Subtract old values
            self.sum_tr -= self.tr_buf[idx];
            self.sum_plus_dm -= self.plus_dm_buf[idx];
            self.sum_minus_dm -= self.minus_dm_buf[idx];
        }

        // Store new values
        self.tr_buf[idx] = tr;
        self.plus_dm_buf[idx] = plus_dm;
        self.minus_dm_buf[idx] = minus_dm;

        // Add new values to running sums
        self.sum_tr += tr;
        self.sum_plus_dm += plus_dm;
        self.sum_minus_dm += minus_dm;

        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;
        self.index += 1;

        if self.index >= self.period {
            self.filled = true;
        }

        if !self.filled {
            self.plus_di = 0.0;
            self.minus_di = 0.0;
            self.adx = 0.0;
            return (self.plus_di, self.minus_di, self.adx);
        }

        // O(1) calculation using running sums
        self.plus_di = if self.sum_tr.abs() < 1e-12 {
            0.0
        } else {
            100.0 * self.sum_plus_dm / self.sum_tr
        };

        self.minus_di = if self.sum_tr.abs() < 1e-12 {
            0.0
        } else {
            100.0 * self.sum_minus_dm / self.sum_tr
        };

        let di_diff = (self.plus_di - self.minus_di).abs();
        let di_sum = self.plus_di + self.minus_di;
        let dx = if di_sum.abs() < 1e-12 {
            0.0
        } else {
            100.0 * di_diff / di_sum
        };

        let adx_idx = self.adx_idx % self.period;

        // Update ADX buffer with O(1) running sum
        if self.adx_filled {
            self.sum_adx -= self.adx_buf[adx_idx];
        }
        self.adx_buf[adx_idx] = dx;
        self.sum_adx += dx;

        self.adx_idx += 1;
        if self.adx_idx >= self.period {
            self.adx_filled = true;
        }

        // O(1) ADX calculation
        self.adx = if self.adx_filled {
            self.sum_adx / self.period as f64
        } else {
            0.0
        };

        (self.plus_di, self.minus_di, self.adx)
    }

    /// Возвращает кортеж: (plus_di, minus_di, adx)
    /// plus_di — индекс положительного направленного движения (+DI)
    /// minus_di — индекс отрицательного направленного движения (-DI)
    /// adx — индекс силы тренда (ADX)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.plus_di, self.minus_di, self.adx)
    }

    pub fn is_ready(&self) -> bool {
        self.filled && self.adx_filled
    }

    pub fn reset(&mut self) {
        self.tr_buf.fill(0.0);
        self.plus_dm_buf.fill(0.0);
        self.minus_dm_buf.fill(0.0);
        self.adx_buf.fill(0.0);
        self.sum_tr = 0.0;
        self.sum_plus_dm = 0.0;
        self.sum_minus_dm = 0.0;
        self.sum_adx = 0.0;
        self.index = 0;
        self.filled = false;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.prev_close = 0.0;
        self.plus_di = 0.0;
        self.minus_di = 0.0;
        self.adx = 0.0;
        self.adx_idx = 0;
        self.adx_filled = false;
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn plus_di(&self) -> f64 {
        self.plus_di
    }

    pub fn minus_di(&self) -> f64 {
        self.minus_di
    }

    pub fn adx(&self) -> f64 {
        self.adx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dm_creation() {
        let dm = Dm::new(14);
        assert!(!dm.is_ready());
        if let IndicatorValue::Triple(plus_di, minus_di, adx) = dm.value() {
            assert_eq!(plus_di, 0.0);
            assert_eq!(minus_di, 0.0);
            assert_eq!(adx, 0.0);
        } else { panic!("Expected Triple"); }
    }

    #[test]
    fn test_dm_basic_calculation() {
        let mut dm = Dm::new(14);

        for i in 1..=50 {
            let price = 100.0 + i as f64;
            let (plus_di, minus_di, adx) = dm.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);

            if dm.is_ready() {
                assert!(plus_di >= 0.0 && plus_di <= 100.0);
                assert!(minus_di >= 0.0 && minus_di <= 100.0);
                assert!(adx >= 0.0 && adx <= 100.0);
            }
        }

        assert!(dm.is_ready());
    }

    #[test]
    fn test_dm_uptrend() {
        let mut dm = Dm::new(14);

        // Strong uptrend
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            dm.update_bar(price, price + 3.0, price - 1.0, price + 2.0, 1000.0);
        }

        if dm.is_ready() {
            // In uptrend, +DI should be > -DI
            assert!(dm.plus_di() > dm.minus_di(), "+DI should be > -DI in uptrend");
        }
    }

    #[test]
    fn test_dm_downtrend() {
        let mut dm = Dm::new(14);

        // Strong downtrend
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            dm.update_bar(price, price + 1.0, price - 3.0, price - 2.0, 1000.0);
        }

        if dm.is_ready() {
            // In downtrend, -DI should be > +DI
            assert!(dm.minus_di() > dm.plus_di(), "-DI should be > +DI in downtrend");
        }
    }

    #[test]
    fn test_dm_adx_strong_trend() {
        let mut dm = Dm::new(14);

        // Strong trending market
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 3.0;
            dm.update_bar(price, price + 2.0, price - 0.5, price + 1.5, 1000.0);
        }

        if dm.is_ready() {
            // ADX should be elevated in strong trend
            assert!(dm.adx() > 0.0, "ADX should be > 0 in trending market");
        }
    }

    #[test]
    fn test_dm_reset() {
        let mut dm = Dm::new(14);

        for i in 1..=50 {
            let price = 100.0 + i as f64;
            dm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(dm.is_ready());

        dm.reset();
        assert!(!dm.is_ready());
        assert_eq!(dm.plus_di(), 0.0);
        assert_eq!(dm.minus_di(), 0.0);
        assert_eq!(dm.adx(), 0.0);
    }

    #[test]
    fn test_dm_period() {
        let dm = Dm::new(14);
        assert_eq!(dm.period(), 14);

        let dm2 = Dm::new(21);
        assert_eq!(dm2.period(), 21);
    }

    #[test]
    fn test_dm_value() {
        let mut dm = Dm::new(14);

        for i in 1..=50 {
            let price = 100.0 + i as f64;
            dm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        if let IndicatorValue::Triple(plus_di, minus_di, adx) = dm.value() {
            assert_eq!(plus_di, dm.plus_di());
            assert_eq!(minus_di, dm.minus_di());
            assert_eq!(adx, dm.adx());
        } else { panic!("Expected Triple"); }
    }

    #[test]
    fn test_dm_getters() {
        let mut dm = Dm::new(14);

        for i in 1..=50 {
            let price = 100.0 + i as f64;
            dm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(dm.plus_di().is_finite());
        assert!(dm.minus_di().is_finite());
        assert!(dm.adx().is_finite());
    }
}
