//! Average Directional Index (ADX) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Average Directional Index (ADX) - measures trend strength regardless of direction.
///
/// ADX oscillates between 0 and 100:
/// - 0-25: Weak trend (ranging market)
/// - 25-50: Moderate trend
/// - 50-75: Strong trend
/// - 75-100: Very strong trend
///
/// Also provides +DI and -DI for trend direction.
///
/// # Calculation
/// 1. Calculate True Range (TR)
/// 2. Calculate +DM and -DM (Directional Movement)
/// 3. Smooth TR, +DM, -DM using Wilder's smoothing
/// 4. Calculate +DI = 100 × Smoothed(+DM) / Smoothed(TR)
/// 5. Calculate -DI = 100 × Smoothed(-DM) / Smoothed(TR)
/// 6. DX = 100 × |+DI - -DI| / (+DI + -DI)
/// 7. ADX = Smoothed(DX)
///
/// # Implementation
///
/// Uses Wilder's smoothing (RMA). O(1) update complexity.

#[derive(Clone)]
pub struct Adx {
    period: usize,
    
    // ATR для централизованного расчета True Range
    atr: Atr,
    
    // Буферы для расчета TR, +DM, -DM
    tr_sum: f64,
    plus_dm_sum: f64,
    minus_dm_sum: f64,
    
    // Буферы для расчета ADX
    dx_buffer: ArrayVec<f64, 512>,
    dx_index: usize,
    
    // Предыдущие значения
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    
    // Состояние
    count: usize,
    is_initialized: bool,
    
    // Текущие значения
    adx_value: f64,
    plus_di: f64,
    minus_di: f64,
    
    // Сглаживание (Wilder's smoothing)
    smoothing_factor: f64,
}

impl Adx {
    /// Создать новый ADX индикатор
    pub fn new(period: usize) -> Self {
        assert!(period > 0 && period <= 512, "ADX period must be > 0 and <= 512");
        
        Self {
            period,
            atr: Atr::new_wilder(period), // ADX традиционно использует Wilder's smoothing
            tr_sum: 0.0,
            plus_dm_sum: 0.0,
            minus_dm_sum: 0.0,
            dx_buffer: ArrayVec::new(),
            dx_index: 0,
            prev_high: 0.0,
            prev_low: 0.0,
            prev_close: 0.0,
            count: 0,
            is_initialized: false,
            adx_value: 0.0,
            plus_di: 0.0,
            minus_di: 0.0,
            smoothing_factor: 1.0 / period as f64,
        }
    }
    
    /// Обновить ADX новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        if self.count == 0 {
            // Первый бар - инициализация
            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;
            self.count = 1;
            return self.adx_value;
        }
        
        // Расчет True Range через централизованный ATR
        let tr = self.atr.update_bar(_open, high, low, close, _volume);
        
        // Расчет Directional Movement
        let (plus_dm, minus_dm) = self.calculate_directional_movement(high, low);
        
        if self.count <= self.period {
            // Накопление первых period значений
            self.tr_sum += tr;
            self.plus_dm_sum += plus_dm;
            self.minus_dm_sum += minus_dm;
            
            if self.count == self.period {
                // Первый расчет DI и DX
                self.calculate_di_and_dx();
                self.is_initialized = true;
            }
        } else {
            // Wilder's smoothing для TR, +DM, -DM
            self.tr_sum = self.tr_sum - (self.tr_sum * self.smoothing_factor) + tr;
            self.plus_dm_sum = self.plus_dm_sum - (self.plus_dm_sum * self.smoothing_factor) + plus_dm;
            self.minus_dm_sum = self.minus_dm_sum - (self.minus_dm_sum * self.smoothing_factor) + minus_dm;
            
            // Расчет DI и DX
            self.calculate_di_and_dx();
        }
        
        // Обновление предыдущих значений
        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;
        self.count += 1;
        
        self.adx_value
    }
    

    
    /// Рассчитать Directional Movement
    fn calculate_directional_movement(&self, high: f64, low: f64) -> (f64, f64) {
        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;
        
        let plus_dm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        
        let minus_dm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        
        (plus_dm, minus_dm)
    }
    
    /// Рассчитать DI и DX, обновить ADX
    fn calculate_di_and_dx(&mut self) {
        if self.tr_sum.abs() < 1e-12 {
            self.plus_di = 0.0;
            self.minus_di = 0.0;
            return;
        }
        
        // Расчет Directional Indicators
        self.plus_di = (self.plus_dm_sum / self.tr_sum) * 100.0;
        self.minus_di = (self.minus_dm_sum / self.tr_sum) * 100.0;
        
        // Расчет DX
        let di_sum = self.plus_di + self.minus_di;
        let dx = if di_sum.abs() < 1e-12 {
            0.0
        } else {
            ((self.plus_di - self.minus_di).abs() / di_sum) * 100.0
        };
        
        // Обновление ADX
        if self.count <= self.period {
            // Накопление первых period значений DX
            if self.dx_buffer.len() < self.period {
                self.dx_buffer.push(dx);
            }
            
            if self.dx_buffer.len() == self.period {
                // Первый расчет ADX как среднее арифметическое
                self.adx_value = self.dx_buffer.iter().sum::<f64>() / self.period as f64;
            }
        } else {
            // Wilder's smoothing для ADX
            self.adx_value = self.adx_value - (self.adx_value * self.smoothing_factor) + (dx * self.smoothing_factor);
        }
    }
    
    /// Получить текущее значение ADX
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.adx_value)
    }
    
    /// Получить +DI значение
    pub fn plus_di(&self) -> f64 {
        self.plus_di
    }
    
    /// Получить -DI значение
    pub fn minus_di(&self) -> f64 {
        self.minus_di
    }
    
    /// Получить все значения (ADX, +DI, -DI)
    pub fn values(&self) -> (f64, f64, f64) {
        (self.adx_value, self.plus_di, self.minus_di)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_initialized && self.count > self.period * 2
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.atr.reset();
        self.tr_sum = 0.0;
        self.plus_dm_sum = 0.0;
        self.minus_dm_sum = 0.0;
        self.dx_buffer.clear();
        self.dx_index = 0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.prev_close = 0.0;
        self.count = 0;
        self.is_initialized = false;
        self.adx_value = 0.0;
        self.plus_di = 0.0;
        self.minus_di = 0.0;
    }
    
    /// Получить силу тренда как текстовое описание
    pub fn trend_strength(&self) -> &'static str {
        match self.adx_value {
            x if x < 25.0 => "Weak",
            x if x < 50.0 => "Moderate", 
            x if x < 75.0 => "Strong",
            _ => "Very Strong"
        }
    }
    
    /// Получить направление тренда на основе DI
    pub fn trend_direction(&self) -> &'static str {
        if self.plus_di > self.minus_di {
            "Bullish"
        } else if self.minus_di > self.plus_di {
            "Bearish"
        } else {
            "Neutral"
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = сильный бычий сигнал, -1 = сильный медвежий сигнал, 0 = нет сигнала
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let strength_threshold = 25.0;
        let di_diff = (self.plus_di - self.minus_di).abs();
        
        if self.adx_value > strength_threshold && di_diff > 5.0 {
            if self.plus_di > self.minus_di {
                1  // Бычий сигнал
            } else {
                -1 // Медвежий сигнал
            }
        } else {
            0  // Нет сигнала
        }
    }
    
    /// Проверить пересечение DI линий
    /// Возвращает: 1 = +DI пересекает -DI вверх, -1 = -DI пересекает +DI вверх, 0 = нет пересечения
    pub fn di_crossover(&self, prev_plus_di: f64, prev_minus_di: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Бычье пересечение: +DI пересекает -DI снизу вверх
        if prev_plus_di <= prev_minus_di && self.plus_di > self.minus_di {
            return 1;
        }
        
        // Медвежье пересечение: -DI пересекает +DI снизу вверх
        if prev_minus_di <= prev_plus_di && self.minus_di > self.plus_di {
            return -1;
        }
        
        0
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "ADX: {:.2} ({}), +DI: {:.2}, -DI: {:.2}, Direction: {}",
            self.adx_value,
            self.trend_strength(),
            self.plus_di,
            self.minus_di,
            self.trend_direction()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_adx_new() {
        let adx = Adx::new(14);
        assert_eq!(adx.period(), 14);
        assert!(!adx.is_ready());
        assert_eq!(adx.value().main(), 0.0);
    }
    
    #[test]
    fn test_adx_calculation() {
        let mut adx = Adx::new(14);
        
        // Тестовые данные (восходящий тренд)
        let test_data = vec![
            (1.0, 1.10, 1.00, 1.05),
            (1.05, 1.15, 1.03, 1.12),
            (1.12, 1.20, 1.10, 1.18),
            (1.18, 1.25, 1.15, 1.22),
            (1.22, 1.30, 1.20, 1.28),
        ];
        
        for (open, high, low, close) in test_data {
            adx.update_bar(open, high, low, close, 1000.0);
        }
        
        // После нескольких баров должны быть разумные значения
        assert!(adx.plus_di() >= 0.0);
        assert!(adx.minus_di() >= 0.0);
        assert!(adx.value().main() >= 0.0);
    }
    
    #[test]
    fn test_adx_reset() {
        let mut adx = Adx::new(14);
        
        adx.update_bar(1.0, 1.10, 1.00, 1.05, 1000.0);
        adx.update_bar(1.05, 1.15, 1.03, 1.12, 1000.0);
        
        adx.reset();
        
        assert!(!adx.is_ready());
        assert_eq!(adx.value().main(), 0.0);
        assert_eq!(adx.plus_di(), 0.0);
        assert_eq!(adx.minus_di(), 0.0);
    }
    
    #[test]
    fn test_trend_strength_classification() {
        let mut adx = Adx::new(14);
        
        // Simulate different ADX values
        adx.adx_value = 20.0;
        assert_eq!(adx.trend_strength(), "Weak");
        
        adx.adx_value = 35.0;
        assert_eq!(adx.trend_strength(), "Moderate");
        
        adx.adx_value = 60.0;
        assert_eq!(adx.trend_strength(), "Strong");
        
        adx.adx_value = 80.0;
        assert_eq!(adx.trend_strength(), "Very Strong");
    }
} 






















