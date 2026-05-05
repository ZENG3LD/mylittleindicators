//! Parabolic SAR (Stop and Reverse) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Parabolic SAR (Stop and Reverse) - trend-following indicator by J. Welles Wilder.
///
/// SAR = SAR(prev) + AF × (EP - SAR(prev))
/// where:
/// - AF = Acceleration Factor (starts at af_start, increments by af_increment, max af_max)
/// - EP = Extreme Point (highest high in uptrend, lowest low in downtrend)
///
/// The indicator trails price action and flips position when price crosses SAR.
/// Useful for setting trailing stops and identifying trend reversals.
///
/// Interpretation:
/// - Price above SAR: Uptrend, SAR acts as support
/// - Price below SAR: Downtrend, SAR acts as resistance
/// - SAR flip: Potential trend reversal
/// - Higher AF: SAR catches up to price faster
///
/// # Parameters
/// - `af_start`: Initial acceleration factor (typically 0.02)
/// - `af_increment`: AF increment on new extreme (typically 0.02)
/// - `af_max`: Maximum acceleration factor (typically 0.20)
///
/// # Implementation
///
/// Tracks extreme points and adjusts SAR with acceleration. O(1) per update.
#[derive(Debug, Clone)]
pub struct ParabolicSAR {
    af_start: f64,
    af_increment: f64,
    af_max: f64,

    sar: f64,
    trend: i8,
    af: f64,
    ep: f64,

    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,

    is_initialized: bool,
    bars_count: usize,
}

impl ParabolicSAR {
    /// Creates a new Parabolic SAR with default parameters (0.02, 0.02, 0.20).
    pub fn new() -> Self {
        Self::with_params(0.02, 0.02, 0.20)
    }

    /// Creates a new Parabolic SAR with custom parameters.
    ///
    /// # Arguments
    /// * `af_start` - Initial acceleration factor (typically 0.02)
    /// * `af_increment` - AF increment on new extreme (typically 0.02)
    /// * `af_max` - Maximum acceleration factor (typically 0.20)
    pub fn with_params(af_start: f64, af_increment: f64, af_max: f64) -> Self {
        Self {
            af_start,
            af_increment,
            af_max,
            sar: 0.0,
            trend: 1,
            af: af_start,
            ep: 0.0,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            is_initialized: false,
            bars_count: 0,
        }
    }
    
    /// Updates the Parabolic SAR with a new bar and returns the SAR value.
    ///
    /// Uses `high` and `low` prices to calculate SAR and detect trend reversals.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> f64 {
        if self.highs.len() >= 512 {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        
        self.bars_count += 1;
        
        if !self.is_initialized {
            // Инициализация на первых двух барах
            if self.bars_count == 1 {
                self.sar = low;
                self.ep = high;
                self.trend = 1; // Начинаем с восходящего тренда
                return self.sar;
            } else if self.bars_count == 2 {
                // Определяем направление тренда по первым двум барам
                let prev_high = self.highs[0];
                let prev_low = self.lows[0];
                
                if high > prev_high {
                    // Восходящий тренд
                    self.trend = 1;
                    self.sar = prev_low.min(low);
                    self.ep = high;
                } else {
                    // Нисходящий тренд  
                    self.trend = -1;
                    self.sar = prev_high.max(high);
                    self.ep = low;
                }
                
                self.af = self.af_start;
                self.is_initialized = true;
                return self.sar;
            }
        }
        
        // Основной расчет SAR
        let prev_sar = self.sar;
        let new_sar = prev_sar + self.af * (self.ep - prev_sar);
        
        if self.trend == 1 {
            // Восходящий тренд
            self.sar = new_sar;

            // SAR не должен быть выше минимумов последних двух баров
            let len = self.lows.len();
            if len >= 2 {
                let min_low = self.lows[len-2].min(self.lows[len-1]);
                if self.sar > min_low {
                    self.sar = min_low;
                }
            }

            // Проверяем новый максимум
            if high > self.ep {
                self.ep = high;
                self.af = (self.af + self.af_increment).min(self.af_max);
            }

            // Проверяем разворот тренда
            if low <= self.sar {
                self.trend = -1;
                self.sar = self.ep; // SAR становится предыдущим максимумом
                self.ep = low;      // EP становится текущим минимумом
                self.af = self.af_start;
            }
        } else {
            // Нисходящий тренд
            self.sar = new_sar;

            // SAR не должен быть ниже максимумов последних двух баров
            let len = self.highs.len();
            if len >= 2 {
                let max_high = self.highs[len-2].max(self.highs[len-1]);
                if self.sar < max_high {
                    self.sar = max_high;
                }
            }

            // Проверяем новый минимум
            if low < self.ep {
                self.ep = low;
                self.af = (self.af + self.af_increment).min(self.af_max);
            }

            // Проверяем разворот тренда
            if high >= self.sar {
                self.trend = 1;
                self.sar = self.ep; // SAR становится предыдущим минимумом
                self.ep = high;     // EP становится текущим максимумом
                self.af = self.af_start;
            }
        }
        
        self.sar
    }
    
    /// Returns the current SAR value.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.sar)
    }

    /// Returns the current trend direction (1 = uptrend, -1 = downtrend).
    #[inline]
    pub fn trend(&self) -> i8 {
        self.trend
    }

    /// Returns the current acceleration factor.
    #[inline]
    pub fn acceleration_factor(&self) -> f64 {
        self.af
    }

    /// Returns the current extreme point.
    #[inline]
    pub fn extreme_point(&self) -> f64 {
        self.ep
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_initialized && self.bars_count >= 2
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.sar = 0.0;
        self.trend = 1;
        self.af = self.af_start;
        self.ep = 0.0;
        self.highs.clear();
        self.lows.clear();
        self.is_initialized = false;
        self.bars_count = 0;
    }
    
    /// Returns trading signal based on price position relative to SAR.
    ///
    /// Returns: 1 = buy (price above SAR in uptrend), -1 = sell (price below SAR in downtrend), 0 = neutral.
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        if self.trend == 1 && current_price > self.sar {
            1  // Покупка: восходящий тренд, цена выше SAR
        } else if self.trend == -1 && current_price < self.sar {
            -1 // Продажа: нисходящий тренд, цена ниже SAR
        } else {
            0  // Нейтрально или возможный разворот
        }
    }
    
    /// Checks for trend reversal on current bar.
    ///
    /// Returns: 1 = reversal to uptrend, -1 = reversal to downtrend, 0 = no reversal.
    pub fn trend_reversal(&self) -> i8 {
        if !self.is_ready() || self.bars_count < 3 {
            return 0;
        }
        
        // Простая эвристика: если AF сбросился до начального значения,
        // значит произошел разворот на предыдущем баре
        if (self.af - self.af_start).abs() < 1e-10 {
            self.trend
        } else {
            0
        }
    }
    
    /// Returns the stop-loss level for current position.
    #[inline]
    pub fn stop_loss_level(&self) -> f64 {
        self.sar
    }

    /// Returns the distance from current price to SAR as a percentage.
    pub fn distance_to_sar(&self, current_price: f64) -> f64 {
        if current_price.abs() < 1e-12 {
            return 0.0;
        }
        
        ((current_price - self.sar) / current_price) * 100.0
    }
    
    /// Returns trend strength based on price position relative to SAR (0.0 to 1.0).
    pub fn trend_strength(&self, current_price: f64) -> f64 {
        let distance = self.distance_to_sar(current_price).abs();
        // Normalize distance - 5% distance = strength 1.0
        (distance / 5.0).min(1.0)
    }
    
    /// Returns information about the indicator state.
    pub fn info(&self) -> String {
        format!(
            "SAR: {:.4}, Trend: {}, AF: {:.3}, EP: {:.4}",
            self.sar,
            if self.trend == 1 { "UP" } else { "DOWN" },
            self.af,
            self.ep
        )
    }

    /// Returns the indicator parameters (af_start, af_increment, af_max).
    #[inline]
    pub fn params(&self) -> (f64, f64, f64) {
        (self.af_start, self.af_increment, self.af_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_sar_basic_calculation() {
        let mut sar = ParabolicSAR::new();

        // Feed uptrend data
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        assert!(sar.is_ready());
        let value = match sar.value() {
            IndicatorValue::Single(v) => v,
            _ => panic!("Expected Single value"),
        };
        assert!(value > 0.0, "SAR should be positive");
    }

    #[test]
    fn test_sar_uptrend() {
        let mut sar = ParabolicSAR::new();

        // Feed strong uptrend data
        for i in 1..=30 {
            let base = 100.0 + i as f64 * 2.0;
            sar.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        assert!(sar.is_ready());
        // In uptrend, trend should be 1
        assert_eq!(sar.trend(), 1, "Trend should be uptrend");
        // SAR should be below current price in uptrend
        let current_price = 100.0 + 30.0 * 2.0 + 0.5;
        let sar_val = match sar.value() {
            IndicatorValue::Single(v) => v,
            _ => panic!("Expected Single value"),
        };
        assert!(sar_val < current_price, "SAR should be below price in uptrend");
    }

    #[test]
    fn test_sar_downtrend() {
        let mut sar = ParabolicSAR::new();

        // Feed strong downtrend data
        for i in 1..=30 {
            let base = 200.0 - i as f64 * 2.0;
            sar.update_bar(base, base + 0.5, base - 1.0, base - 0.5, 0.0);
        }

        assert!(sar.is_ready());
        // In downtrend, trend should be -1
        assert_eq!(sar.trend(), -1, "Trend should be downtrend");
    }

    #[test]
    fn test_sar_trend_reversal() {
        let mut sar = ParabolicSAR::new();

        // Start with uptrend
        for i in 1..=15 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        let trend_before = sar.trend();
        assert_eq!(trend_before, 1, "Should be in uptrend");

        // Sharp reversal to downtrend
        for i in 1..=15 {
            let base = 115.0 - i as f64 * 3.0;
            sar.update_bar(base, base + 0.5, base - 1.0, base - 0.5, 0.0);
        }

        let trend_after = sar.trend();
        assert_eq!(trend_after, -1, "Should be in downtrend after reversal");
    }

    #[test]
    fn test_sar_acceleration_factor() {
        let mut sar = ParabolicSAR::new();

        // Feed bars to initialize
        for i in 1..=5 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        let af = sar.acceleration_factor();
        assert!(af >= 0.02 && af <= 0.20, "AF should be within bounds");
    }

    #[test]
    fn test_sar_extreme_point() {
        let mut sar = ParabolicSAR::new();

        // Feed uptrend data
        for i in 1..=10 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        let ep = sar.extreme_point();
        // In uptrend, EP should be near the highest high
        assert!(ep >= 109.0 && ep <= 112.0, "EP should be near recent high");
    }

    #[test]
    fn test_sar_reset() {
        let mut sar = ParabolicSAR::new();

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }
        assert!(sar.is_ready());

        sar.reset();
        assert!(!sar.is_ready());
        assert_eq!(sar.trend(), 1); // Default trend
    }

    #[test]
    fn test_sar_params() {
        let sar = ParabolicSAR::with_params(0.01, 0.01, 0.10);
        let (af_start, af_inc, af_max) = sar.params();
        assert_eq!(af_start, 0.01);
        assert_eq!(af_inc, 0.01);
        assert_eq!(af_max, 0.10);
    }

    #[test]
    fn test_sar_trading_signal() {
        let mut sar = ParabolicSAR::new();

        // Feed uptrend data
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        assert!(sar.is_ready());
        let current_price = 121.0;
        let signal = sar.trading_signal(current_price);
        // In uptrend with price above SAR, should be buy signal
        assert_eq!(signal, 1, "Should be buy signal in uptrend");
    }

    #[test]
    fn test_sar_stop_loss_level() {
        let mut sar = ParabolicSAR::new();

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(sar.is_ready());
        let stop_loss = sar.stop_loss_level();
        let sar_val = match sar.value() {
            IndicatorValue::Single(v) => v,
            _ => panic!("Expected Single value"),
        };
        assert_eq!(stop_loss, sar_val, "Stop loss should equal SAR value");
    }

    #[test]
    fn test_sar_trend_strength() {
        let mut sar = ParabolicSAR::new();

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(sar.is_ready());
        let strength = sar.trend_strength(120.0);
        assert!(strength >= 0.0 && strength <= 1.0, "Strength should be normalized");
    }

    #[test]
    fn test_sar_distance_to_sar() {
        let mut sar = ParabolicSAR::new();

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            sar.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(sar.is_ready());
        let distance = sar.distance_to_sar(120.0);
        // Distance should be a percentage
        assert!(distance.abs() < 100.0, "Distance should be a reasonable percentage");
    }
}















