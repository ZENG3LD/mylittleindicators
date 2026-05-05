//! Money Flow Index (MFI) - индикатор денежного потока
//! Комбинирует цену и объем для определения давления покупки/продажи
//! Формула: MFI = 100 - (100 / (1 + Money Flow Ratio))
//! где Money Flow Ratio = Positive Money Flow / Negative Money Flow

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Money Flow Index индикатор
#[derive(Clone)]
pub struct Mfi {
    period: usize,
    money_flows: ArrayVec<f64, 512>,  // Positive/Negative money flows
    flow_types: ArrayVec<bool, 512>,  // true = positive, false = negative
    typical_prices: ArrayVec<f64, 512>, // Для сравнения направления
    count: usize,
    value: f64,
}

impl Mfi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            money_flows: ArrayVec::new(),
            flow_types: ArrayVec::new(),
            typical_prices: ArrayVec::new(),
            count: 0,
            value: 50.0, // Нейтральное значение
        }
    }

    /// Обновить MFI новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let typical_price = (high + low + close) / 3.0;
        let raw_money_flow = typical_price * volume;
        
        // Определяем направление потока
        let is_positive = if self.count > 0 {
            typical_price > self.typical_prices[self.typical_prices.len() - 1]
        } else {
            true // Первый бар считаем положительным
        };
        
        // Добавляем данные в буферы
        if self.count < self.period {
            self.money_flows.push(raw_money_flow);
            self.flow_types.push(is_positive);
            self.typical_prices.push(typical_price);
            self.count += 1;
        } else {
            // Сдвигаем буферы
            self.money_flows.remove(0);
            self.flow_types.remove(0);
            self.typical_prices.remove(0);
            
            self.money_flows.push(raw_money_flow);
            self.flow_types.push(is_positive);
            self.typical_prices.push(typical_price);
        }
        
        // Рассчитываем MFI если есть достаточно данных
        if self.count >= self.period {
            let mut positive_flow = 0.0;
            let mut negative_flow = 0.0;
            
            for i in 0..self.money_flows.len() {
                if self.flow_types[i] {
                    positive_flow += self.money_flows[i];
                } else {
                    negative_flow += self.money_flows[i];
                }
            }
            
            if negative_flow == 0.0 {
                self.value = 100.0; // Все потоки положительные
            } else if positive_flow == 0.0 {
                self.value = 0.0;   // Все потоки отрицательные
            } else {
                let money_flow_ratio = positive_flow / negative_flow;
                self.value = 100.0 - (100.0 / (1.0 + money_flow_ratio));
            }
        }
        
        self.value
    }

    /// Получить текущее значение MFI
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }

    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.money_flows.clear();
        self.flow_types.clear();
        self.typical_prices.clear();
        self.count = 0;
        self.value = 50.0;
    }

    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.value {
            v if v >= 80.0 => "Overbought",
            v if v >= 60.0 => "Bullish",
            v if v >= 40.0 => "Neutral",
            v if v >= 20.0 => "Bearish", 
            _ => "Oversold",
        }
    }

    /// Получить силу денежного потока
    pub fn flow_strength(&self) -> f64 {
        if self.value > 50.0 {
            (self.value - 50.0) / 50.0  // 0.0 to 1.0 для бычьего потока
        } else {
            (50.0 - self.value) / 50.0  // 0.0 to 1.0 для медвежьего потока
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfi_creation() {
        let mfi = Mfi::new(14);
        assert!(!mfi.is_ready());
        assert_eq!(mfi.period(), 14);
        assert_eq!(mfi.value().main(), 50.0);
    }

    #[test]
    fn test_mfi_warmup() {
        let mut mfi = Mfi::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mfi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 10.0);
        }
        assert!(mfi.is_ready());
    }

    #[test]
    fn test_mfi_range() {
        let mut mfi = Mfi::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mfi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "MFI should be in [0, 100]");
        }
    }

    #[test]
    fn test_mfi_reset() {
        let mut mfi = Mfi::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            mfi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        mfi.reset();
        assert!(!mfi.is_ready());
        assert_eq!(mfi.value().main(), 50.0);
    }
} 






















