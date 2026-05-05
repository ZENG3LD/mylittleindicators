//! NVI/PVI (Negative/Positive Volume Index) - индикаторы объема Нормана Фосбака
//! NVI обновляется только когда объем падает по сравнению с предыдущим днем
//! PVI обновляется только когда объем растет по сравнению с предыдущим днем
//! Используются для анализа поведения "умных" и "неумных" денег

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// NVI/PVI индикатор
#[derive(Clone)]
pub struct NegativePositiveVolumeIndex {
    // Периоды для скользящих средних
    nvi_ma_period: usize,
    pvi_ma_period: usize,
    
    // Буферы для значений
    nvi_values: ArrayVec<f64, 512>,
    pvi_values: ArrayVec<f64, 512>,
    
    // Скользящие средние для сглаживания
    nvi_ma: MovingAverageProvider,
    pvi_ma: MovingAverageProvider,
    
    // Предыдущие значения
    prev_close: f64,
    prev_volume: f64,
    
    // Текущие значения (начинаем с базового значения 1000)
    nvi_value: f64,
    pvi_value: f64,
    nvi_ma_value: f64,
    pvi_ma_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl NegativePositiveVolumeIndex {
    /// Создать новый NVI/PVI с параметрами по умолчанию (255, 255)
    pub fn new() -> Self {
        Self::with_params(255, 255)
    }
    
    /// Создать новый NVI/PVI с настраиваемыми параметрами
    pub fn with_params(nvi_ma_period: usize, pvi_ma_period: usize) -> Self {
        assert!(nvi_ma_period > 0, "NVI MA period must be greater than 0");
        assert!(pvi_ma_period > 0, "PVI MA period must be greater than 0");
        
        Self {
            nvi_ma_period,
            pvi_ma_period,
            nvi_values: ArrayVec::new(),
            pvi_values: ArrayVec::new(),
            nvi_ma: MovingAverageProvider::new(MovingAverageType::EMA, nvi_ma_period),
            pvi_ma: MovingAverageProvider::new(MovingAverageType::EMA, pvi_ma_period),
            prev_close: 0.0,
            prev_volume: 0.0,
            nvi_value: 1000.0,  // Базовое значение
            pvi_value: 1000.0,  // Базовое значение
            nvi_ma_value: 1000.0,
            pvi_ma_value: 1000.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> (f64, f64) {
        self.bars_count += 1;
        
        if self.bars_count == 1 {
            // Первый бар - инициализируем предыдущие значения
            self.prev_close = close;
            self.prev_volume = volume;
            return (self.nvi_value, self.pvi_value);
        }
        
        // Рассчитываем процентное изменение цены
        let price_change_pct = if self.prev_close.abs() > 1e-12 {
            (close - self.prev_close) / self.prev_close
        } else {
            0.0
        };
        
        // Обновляем NVI только когда объем падает
        if volume < self.prev_volume {
            self.nvi_value *= 1.0 + price_change_pct;
        }
        // Иначе NVI остается неизменным
        
        // Обновляем PVI только когда объем растет
        if volume > self.prev_volume {
            self.pvi_value *= 1.0 + price_change_pct;
        }
        // Иначе PVI остается неизменным
        
        // Добавляем в буферы
        if self.nvi_values.len() >= 512 {
            self.nvi_values.remove(0);
        }
        if self.pvi_values.len() >= 512 {
            self.pvi_values.remove(0);
        }
        
        self.nvi_values.push(self.nvi_value);
        self.pvi_values.push(self.pvi_value);
        
        // Рассчитываем скользящие средние
        self.nvi_ma_value = self.nvi_ma.update_bar(self.nvi_value, self.nvi_value, self.nvi_value, self.nvi_value, 1.0);
        self.pvi_ma_value = self.pvi_ma.update_bar(self.pvi_value, self.pvi_value, self.pvi_value, self.pvi_value, 1.0);
        
        // Обновляем предыдущие значения
        self.prev_close = close;
        self.prev_volume = volume;
        
        // Проверяем готовность
        if self.bars_count >= self.nvi_ma_period.max(self.pvi_ma_period) + 10 {
            self.is_ready = true;
        }
        
        (self.nvi_value, self.pvi_value)
    }
    
    /// Получить значение NVI
    pub fn nvi_value(&self) -> f64 {
        self.nvi_value
    }
    
    /// Получить значение PVI
    pub fn pvi_value(&self) -> f64 {
        self.pvi_value
    }
    
    /// Получить значение NVI MA
    pub fn nvi_ma_value(&self) -> f64 {
        self.nvi_ma_value
    }
    
    /// Получить значение PVI MA
    pub fn pvi_ma_value(&self) -> f64 {
        self.pvi_ma_value
    }
    
    /// Получить все значения
    pub fn values(&self) -> (f64, f64, f64, f64) {
        (self.nvi_value, self.pvi_value, self.nvi_ma_value, self.pvi_ma_value)
    }

    /// Получить значение как IndicatorValue::Double(nvi, pvi)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.nvi_value, self.pvi_value)
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить периоды индикатора
    pub fn periods(&self) -> (usize, usize) {
        (self.nvi_ma_period, self.pvi_ma_period)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.nvi_values.clear();
        self.pvi_values.clear();
        self.nvi_ma.reset();
        self.pvi_ma.reset();
        self.prev_close = 0.0;
        self.prev_volume = 0.0;
        self.nvi_value = 1000.0;
        self.pvi_value = 1000.0;
        self.nvi_ma_value = 1000.0;
        self.pvi_ma_value = 1000.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние рынка на основе NVI
    pub fn nvi_market_condition(&self) -> &'static str {
        if self.nvi_value > self.nvi_ma_value {
            "NVI Bullish"  // "Умные деньги" бычьи
        } else if self.nvi_value < self.nvi_ma_value {
            "NVI Bearish"  // "Умные деньги" медвежьи
        } else {
            "NVI Neutral"
        }
    }
    
    /// Определить состояние рынка на основе PVI
    pub fn pvi_market_condition(&self) -> &'static str {
        if self.pvi_value > self.pvi_ma_value {
            "PVI Bullish"  // "Неумные деньги" бычьи
        } else if self.pvi_value < self.pvi_ma_value {
            "PVI Bearish"  // "Неумные деньги" медвежьи
        } else {
            "PVI Neutral"
        }
    }
    
    /// Получить торговый сигнал на основе NVI
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn nvi_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Сигнал на основе пересечения NVI и его MA
        if self.nvi_value > self.nvi_ma_value {
            1  // Покупка - "умные деньги" бычьи
        } else if self.nvi_value < self.nvi_ma_value {
            -1 // Продажа - "умные деньги" медвежьи
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить торговый сигнал на основе PVI
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn pvi_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Сигнал на основе пересечения PVI и его MA
        if self.pvi_value > self.pvi_ma_value {
            1  // Покупка - "неумные деньги" бычьи
        } else if self.pvi_value < self.pvi_ma_value {
            -1 // Продажа - "неумные деньги" медвежьи
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить комбинированный сигнал
    pub fn combined_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let nvi_sig = self.nvi_signal();
        let pvi_sig = self.pvi_signal();
        
        // Сильный сигнал когда оба индикатора согласны
        if nvi_sig == 1 && pvi_sig == 1 {
            1  // Сильная покупка
        } else if nvi_sig == -1 && pvi_sig == -1 {
            -1 // Сильная продажа
        } else {
            0  // Нейтрально или противоречивые сигналы
        }
    }
    
    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.nvi_values.len() < 3 || self.pvi_values.len() < 3 {
            return 0;
        }
        
        let len = self.nvi_values.len();
        let current_nvi = self.nvi_value;
        let _current_pvi = self.pvi_value;
        let prev_nvi = if len >= 2 { self.nvi_values[len - 2] } else { 1000.0 };
        let _prev_pvi = if len >= 2 { self.pvi_values[len - 2] } else { 1000.0 };
        
        // Сигнал покупки: NVI пересекает MA снизу вверх (приоритет "умным деньгам")
        if prev_nvi <= self.nvi_ma_value && current_nvi > self.nvi_ma_value {
            return 1;
        }
        
        // Сигнал продажи: NVI пересекает MA сверху вниз
        if prev_nvi >= self.nvi_ma_value && current_nvi < self.nvi_ma_value {
            return -1;
        }
        
        0
    }
    
    /// Получить дивергенцию между NVI и PVI
    pub fn nvi_pvi_divergence(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let nvi_trend = if self.nvi_value > self.nvi_ma_value { 1 } else { -1 };
        let pvi_trend = if self.pvi_value > self.pvi_ma_value { 1 } else { -1 };
        
        // Дивергенция когда "умные" и "неумные" деньги движутся в разные стороны
        if nvi_trend == 1 && pvi_trend == -1 {
            1  // "Умные деньги" бычьи, "неумные" медвежьи - потенциальный рост
        } else if nvi_trend == -1 && pvi_trend == 1 {
            -1 // "Умные деньги" медвежьи, "неумные" бычьи - потенциальное падение
        } else {
            0  // Нет дивергенции
        }
    }
    
    /// Получить силу тренда для NVI
    pub fn nvi_trend_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        ((self.nvi_value - self.nvi_ma_value) / self.nvi_ma_value).abs() * 100.0
    }
    
    /// Получить силу тренда для PVI
    pub fn pvi_trend_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        ((self.pvi_value - self.pvi_ma_value) / self.pvi_ma_value).abs() * 100.0
    }
    
    /// Получить статистику по активности "умных" vs "неумных" денег
    pub fn smart_money_activity(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.nvi_values.len() < periods || self.pvi_values.len() < periods {
            return (0.0, 0.0);
        }
        
        let start_idx = self.nvi_values.len() - periods;
        
        // Считаем изменения за период
        let nvi_change = (self.nvi_value - self.nvi_values[start_idx]) / self.nvi_values[start_idx];
        let pvi_change = (self.pvi_value - self.pvi_values[start_idx]) / self.pvi_values[start_idx];
        
        (nvi_change * 100.0, pvi_change * 100.0)
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let (nvi_activity, pvi_activity) = self.smart_money_activity(20);

        format!(
            "NVI: {:.2} (MA: {:.2}), PVI: {:.2} (MA: {:.2}), NVI Activity: {:.2}%, PVI Activity: {:.2}%, Divergence: {}",
            self.nvi_value,
            self.nvi_ma_value,
            self.pvi_value,
            self.pvi_ma_value,
            nvi_activity,
            pvi_activity,
            match self.nvi_pvi_divergence() {
                1 => "Bullish",
                -1 => "Bearish",
                _ => "None"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nvi_pvi_creation() {
        let ind = NegativePositiveVolumeIndex::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.nvi_value(), 1000.0);
        assert_eq!(ind.pvi_value(), 1000.0);
    }

    #[test]
    fn test_nvi_pvi_with_params() {
        let ind = NegativePositiveVolumeIndex::with_params(20, 20);
        assert!(!ind.is_ready());
        assert_eq!(ind.periods(), (20, 20));
    }

    #[test]
    fn test_nvi_pvi_update() {
        let mut ind = NegativePositiveVolumeIndex::with_params(10, 10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let volume = 1000.0 + (i as f64 * 100.0);
            ind.update_bar(price, price + 1.0, price - 1.0, price, volume);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_nvi_pvi_values_finite() {
        let mut ind = NegativePositiveVolumeIndex::with_params(10, 10);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let volume = if i % 2 == 0 { 1000.0 } else { 2000.0 };
            let (nvi, pvi) = ind.update_bar(price, price + 1.0, price - 1.0, price, volume);
            assert!(nvi.is_finite());
            assert!(pvi.is_finite());
        }
    }

    #[test]
    fn test_nvi_pvi_reset() {
        let mut ind = NegativePositiveVolumeIndex::with_params(10, 10);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.nvi_value(), 1000.0);
        assert_eq!(ind.pvi_value(), 1000.0);
    }
} 






















