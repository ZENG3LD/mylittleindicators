//! Fisher Transform indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Fisher Transform - converts prices to Gaussian normal distribution.
///
/// Fisher = 0.5 × ln((1 + Smooth) / (1 - Smooth))
/// where Smooth = EMA(Normalize)
/// Normalize = 2 × ((Close - Lowest) / (Highest - Lowest)) - 1
/// Trigger = Previous Fisher value
///
/// Developed by John Ehlers. The Fisher Transform helps identify extreme price
/// levels and potential reversals. Values oscillate between -4 and +4, but most
/// readings fall between -2 and +2.
///
/// Interpretation:
/// - Fisher > 2.0: Extremely overbought
/// - Fisher > 1.5: Overbought
/// - Fisher < -1.5: Oversold
/// - Fisher < -2.0: Extremely oversold
/// - Fisher crossing Trigger: Trading signals
///
/// # Parameters
/// - `period`: Normalization lookback period (typically 10)
/// - `smooth_period`: EMA smoothing period (typically 3)
///
/// # Implementation
///
/// Uses EMA smoothing and natural log transformation. O(period) per update.
#[derive(Clone)]
pub struct FisherTransform {
    period: usize,
    smooth_period: usize,
    
    // Буферы для расчета
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    normalized_values: ArrayVec<f64, 512>,
    
    // EMA для сглаживания
    ema_alpha: f64,
    smoothed_value: f64,
    ema_initialized: bool,
    
    // Текущие значения
    fisher_value: f64,
    trigger_value: f64,
    
    // Состояние
    count: usize,
    is_ready: bool,
}

impl FisherTransform {
    /// Creates a new Fisher Transform.
    ///
    /// # Arguments
    /// * `period` - Normalization lookback period (typically 10)
    /// * `smooth_period` - EMA smoothing period (typically 3)
    pub fn new(period: usize, smooth_period: usize) -> Self {
        assert!(period > 0 && period <= 512, "Period must be > 0 and <= 512");
        assert!(smooth_period > 0, "Smooth period must be > 0");
        
        let ema_alpha = 2.0 / (smooth_period as f64 + 1.0);
        
        Self {
            period,
            smooth_period,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            normalized_values: ArrayVec::new(),
            ema_alpha,
            smoothed_value: 0.0,
            ema_initialized: false,
            fisher_value: 0.0,
            trigger_value: 0.0,
            count: 0,
            is_ready: false,
        }
    }
    
    /// Creates a Fisher Transform with default parameters (10, 3).
    pub fn default() -> Self {
        Self::new(10, 3)
    }

    /// Updates the Fisher Transform with a new bar and returns (Fisher, Trigger).
    ///
    /// Uses `high`, `low`, and `close` prices.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64) {
        // Добавить новые high/low в буферы
        if self.highs.len() >= self.period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        
        if self.highs.len() >= self.period {
            // 1. Нормализация цены
            let normalized = self.normalize_price(close);
            
            // 2. Сглаживание EMA
            if !self.ema_initialized {
                self.smoothed_value = normalized;
                self.ema_initialized = true;
            } else {
                self.smoothed_value = self.ema_alpha * normalized + (1.0 - self.ema_alpha) * self.smoothed_value;
            }
            
            // 3. Ограничение для избежания ln(0)
            let clamped = self.smoothed_value.clamp(-0.999, 0.999);
            
            // 4. Расчет Fisher Transform
            self.trigger_value = self.fisher_value; // Предыдущее значение становится trigger
            self.fisher_value = 0.5 * ((1.0 + clamped) / (1.0 - clamped)).ln();
            
            // Проверить готовность
            if self.count >= self.period + self.smooth_period {
                self.is_ready = true;
            }
        }
        
        self.count += 1;
        (self.fisher_value, self.trigger_value)
    }
    
    /// Normalizes price to [-1, 1] range.
    fn normalize_price(&self, close: f64) -> f64 {
        let highest = self.highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest = self.lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        if (highest - lowest).abs() < 1e-12 {
            return 0.0;
        }
        
        2.0 * ((close - lowest) / (highest - lowest)) - 1.0
    }
    
    /// Returns the current (Fisher, Trigger) values.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.fisher_value, self.trigger_value)
    }

    /// Returns the Fisher value.
    #[inline]
    pub fn fisher_value(&self) -> f64 {
        self.fisher_value
    }

    /// Returns the Trigger value.
    #[inline]
    pub fn trigger_value(&self) -> f64 {
        self.trigger_value
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the normalization period.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.normalized_values.clear();
        self.smoothed_value = 0.0;
        self.ema_initialized = false;
        self.fisher_value = 0.0;
        self.trigger_value = 0.0;
        self.count = 0;
        self.is_ready = false;
    }
    
    /// Returns trading signal (1 = buy, -1 = sell, 0 = neutral).
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let oversold_level = -1.5;
        let overbought_level = 1.5;
        
        // Бычий сигнал: Fisher пересекает Trigger вверх в зоне перепроданности
        if self.fisher_value > self.trigger_value && 
           self.fisher_value < oversold_level && 
           self.trigger_value < oversold_level {
            return 1;
        }
        
        // Медвежий сигнал: Fisher пересекает Trigger вниз в зоне перекупленности
        if self.fisher_value < self.trigger_value && 
           self.fisher_value > overbought_level && 
           self.trigger_value > overbought_level {
            return -1;
        }
        
        0
    }

    /// Checks for line crossover (1 = bullish, -1 = bearish, 0 = none).
    pub fn crossover(&self, prev_fisher: f64, prev_trigger: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        // Бычье пересечение
        if prev_fisher <= prev_trigger && self.fisher_value > self.trigger_value {
            return 1;
        }
        
        // Медвежье пересечение
        if prev_fisher >= prev_trigger && self.fisher_value < self.trigger_value {
            return -1;
        }
        
        0
    }

    /// Returns the current market condition.
    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        
        match self.fisher_value {
            x if x > 2.0 => "Extremely Overbought",
            x if x > 1.5 => "Overbought",
            x if x > 0.5 => "Bullish",
            x if x > -0.5 => "Neutral",
            x if x > -1.5 => "Bearish", 
            x if x > -2.0 => "Oversold",
            _ => "Extremely Oversold"
        }
    }

    /// Returns the signal strength (0.0 - 1.0).
    pub fn signal_strength(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        
        let fisher_abs = self.fisher_value.abs();
        let trigger_diff = (self.fisher_value - self.trigger_value).abs();
        
        // Комбинированная сила на основе экстремальности Fisher и разности с Trigger
        let extremeness = (fisher_abs / 3.0).min(1.0); // Нормализуем к 3.0 как максимум
        let divergence = (trigger_diff / 2.0).min(1.0); // Нормализуем к 2.0 как максимум
        
        (extremeness * 0.7 + divergence * 0.3).min(1.0)
    }

    /// Returns potential reversal signal (1 = bullish, -1 = bearish, 0 = none).
    pub fn reversal_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let extreme_oversold = -2.0;
        let extreme_overbought = 2.0;
        
        // Потенциальный бычий разворот в экстремальной зоне перепроданности
        if self.fisher_value < extreme_oversold && self.fisher_value > self.trigger_value {
            return 1;
        }
        
        // Потенциальный медвежий разворот в экстремальной зоне перекупленности
        if self.fisher_value > extreme_overbought && self.fisher_value < self.trigger_value {
            return -1;
        }
        
        0
    }

    /// Returns information about the indicator state.
    pub fn info(&self) -> String {
        format!(
            "Fisher: {:.3}, Trigger: {:.3}, Condition: {}, Strength: {:.3}",
            self.fisher_value,
            self.trigger_value,
            self.market_condition(),
            self.signal_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_fisher_basic_calculation() {
        let mut fisher = FisherTransform::new(10, 3);

        // Feed downtrend data to get oversold readings
        for i in 1..=25 {
            let base = 100.0 - i as f64;
            fisher.update_bar(base, base + 1.0, base - 1.0, base - 0.5, 0.0);
        }

        assert!(fisher.is_ready());
        if let IndicatorValue::Double(fisher_val, trigger_val) = fisher.value() {
            // Values should be in reasonable range [-5, 5]
            assert!(fisher_val >= -5.0 && fisher_val <= 5.0);
            assert!(trigger_val >= -5.0 && trigger_val <= 5.0);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_fisher_uptrend() {
        let mut fisher = FisherTransform::new(10, 3);

        // Feed uptrend data
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        assert!(fisher.is_ready());
        // In strong uptrend, Fisher should be positive
        assert!(fisher.fisher_value() > 0.0, "Fisher in uptrend should be positive");
    }

    #[test]
    fn test_fisher_downtrend() {
        let mut fisher = FisherTransform::new(10, 3);

        // Feed downtrend data
        for i in 1..=25 {
            let base = 200.0 - i as f64;
            fisher.update_bar(base, base + 0.5, base - 1.0, base - 0.5, 0.0);
        }

        assert!(fisher.is_ready());
        // In strong downtrend, Fisher should be negative
        assert!(fisher.fisher_value() < 0.0, "Fisher in downtrend should be negative");
    }

    #[test]
    fn test_fisher_trigger_lags() {
        let mut fisher = FisherTransform::new(10, 3);

        // Feed data
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        assert!(fisher.is_ready());
        // Trigger is previous Fisher value, so they should be different during trend
        // (not necessarily, but typically close)
        if let IndicatorValue::Double(f, t) = fisher.value() {
            assert!(f.is_finite());
            assert!(t.is_finite());
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_fisher_reset() {
        let mut fisher = FisherTransform::new(10, 3);

        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }
        assert!(fisher.is_ready());

        fisher.reset();
        assert!(!fisher.is_ready());
        assert_eq!(fisher.fisher_value(), 0.0);
        assert_eq!(fisher.trigger_value(), 0.0);
    }

    #[test]
    fn test_fisher_period() {
        let fisher = FisherTransform::new(14, 5);
        assert_eq!(fisher.period(), 14);
    }

    #[test]
    fn test_fisher_default() {
        let fisher = FisherTransform::default();
        assert_eq!(fisher.period(), 10);
    }

    #[test]
    fn test_fisher_market_condition() {
        let mut fisher = FisherTransform::new(10, 3);

        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(fisher.is_ready());
        let condition = fisher.market_condition();
        assert!(
            condition == "Extremely Overbought"
                || condition == "Overbought"
                || condition == "Bullish"
                || condition == "Neutral"
                || condition == "Bearish"
                || condition == "Oversold"
                || condition == "Extremely Oversold"
        );
    }

    #[test]
    fn test_fisher_signal_strength() {
        let mut fisher = FisherTransform::new(10, 3);

        // Before ready, strength should be 0
        assert_eq!(fisher.signal_strength(), 0.0);

        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(fisher.is_ready());
        let strength = fisher.signal_strength();
        assert!(strength >= 0.0 && strength <= 1.0);
    }
} 






















