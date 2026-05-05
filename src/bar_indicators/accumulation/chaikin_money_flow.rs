//! Chaikin Money Flow (CMF) - индикатор денежного потока Марка Чайкина
//! Измеряет количество накопления или распределения за период
//! CMF = Σ(Money Flow Volume) / Σ(Volume) за N периодов
//! Money Flow Volume = Money Flow Multiplier × Volume
//! Money Flow Multiplier = ((Close - Low) - (High - Close)) / (High - Low)
//!
//! OPTIMIZED: O(1) running sum instead of O(n) iter().sum()

use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Chaikin Money Flow индикатор
#[derive(Clone)]
pub struct ChaikinMoneyFlow {
    period: usize,

    // Буферы для денежного потока и объема (VecDeque for O(1) pop_front)
    money_flow_volumes: VecDeque<f64>,
    volumes: VecDeque<f64>,

    // Running sums for O(1) calculation
    sum_money_flow: f64,
    sum_volume: f64,

    // Текущее значение
    cmf_value: f64,

    // Состояние
    index: usize,
    filled: bool,
}

impl ChaikinMoneyFlow {
    /// Создать новый Chaikin Money Flow с заданным периодом
    pub fn new(period: usize) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");

        Self {
            period,
            money_flow_volumes: VecDeque::with_capacity(period),
            volumes: VecDeque::with_capacity(period),
            sum_money_flow: 0.0,
            sum_volume: 0.0,
            cmf_value: 0.0,
            index: 0,
            filled: false,
        }
    }

    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        // Рассчитываем Money Flow Multiplier
        let range = high - low;
        let money_flow_multiplier = if range.abs() < 1e-12 {
            0.0  // Если диапазон нулевой
        } else {
            ((close - low) - (high - close)) / range
        };

        // Рассчитываем Money Flow Volume
        let money_flow_volume = money_flow_multiplier * volume;

        // O(1) operations: pop old value, subtract from running sum, add new value
        if self.money_flow_volumes.len() >= self.period {
            let old_mf = self.money_flow_volumes.pop_front().unwrap();
            self.sum_money_flow -= old_mf;
        }
        self.money_flow_volumes.push_back(money_flow_volume);
        self.sum_money_flow += money_flow_volume;

        if self.volumes.len() >= self.period {
            let old_vol = self.volumes.pop_front().unwrap();
            self.sum_volume -= old_vol;
        }
        self.volumes.push_back(volume);
        self.sum_volume += volume;

        self.index += 1;

        // Проверяем заполненность
        if self.money_flow_volumes.len() >= self.period {
            self.filled = true;
        }

        // Рассчитываем CMF - O(1) using running sums
        if self.filled {
            self.cmf_value = if self.sum_volume.abs() < 1e-12 {
                0.0
            } else {
                self.sum_money_flow / self.sum_volume
            };
        }

        self.cmf_value
    }

    /// Получить текущее значение CMF
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.cmf_value)
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Получить период индикатора
    pub fn period(&self) -> usize {
        self.period
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.money_flow_volumes.clear();
        self.volumes.clear();
        self.sum_money_flow = 0.0;
        self.sum_volume = 0.0;
        self.cmf_value = 0.0;
        self.index = 0;
        self.filled = false;
    }

    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.cmf_value {
            v if v >= 0.2 => "Strong Accumulation",
            v if v >= 0.05 => "Accumulation",
            v if v <= -0.2 => "Strong Distribution",
            v if v <= -0.05 => "Distribution",
            _ => "Neutral"
        }
    }

    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        match self.cmf_value {
            v if v >= 0.1 => 1,   // Накопление - покупка
            v if v <= -0.1 => -1, // Распределение - продажа
            _ => 0                // Нейтрально
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaikin_money_flow_creation() {
        let cmf = ChaikinMoneyFlow::new(20);
        assert!(!cmf.is_ready());
        assert_eq!(cmf.value().main(), 0.0);
    }

    #[test]
    fn test_chaikin_money_flow_warmup() {
        let mut cmf = ChaikinMoneyFlow::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cmf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cmf.is_ready());
    }

    #[test]
    fn test_chaikin_money_flow_range() {
        let mut cmf = ChaikinMoneyFlow::new(20);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = cmf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "CMF should be in [-1, 1]");
        }
    }

    #[test]
    fn test_chaikin_money_flow_reset() {
        let mut cmf = ChaikinMoneyFlow::new(20);
        for i in 0..25 {
            cmf.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        cmf.reset();
        assert!(!cmf.is_ready());
        assert_eq!(cmf.value().main(), 0.0);
    }
}
