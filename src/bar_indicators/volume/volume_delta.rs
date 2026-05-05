//! Volume Delta Indicator
//! Анализирует баланс покупок/продаж
//! Работает с Bar (OHLCV) и опционально с buy/sell объёмами

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Bar;

/// Volume Delta индикатор с фиксированным окном
#[derive(Debug, Clone)]
pub struct VolumeDelta {
    /// Период для расчета средних
    period: usize,
    /// Буфер дельт (фиксированный размер)
    buffer: ArrayVec<f64, 512>,
    /// Индекс для циклического буфера
    idx: usize,
    /// Сумма дельт в буфере
    sum: f64,
    /// Количество обработанных баров
    count: usize,
    /// Кумулятивная дельта за весь период
    cumulative_delta: f64,
    /// Текущая дельта
    current_delta: f64,
    /// Флаг готовности
    ready: bool,
}

impl VolumeDelta {
    /// Создать новый Volume Delta индикатор
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: ArrayVec::new(),
            idx: 0,
            sum: 0.0,
            count: 0,
            cumulative_delta: 0.0,
            current_delta: 0.0,
            ready: false,
        }
    }

    /// Обновить индикатор с готовыми buy/sell объёмами
    pub fn update_with_delta(&mut self, buy_volume: f64, sell_volume: f64) -> f64 {
        let delta = buy_volume - sell_volume;
        self.process_delta(delta)
    }

    /// Обновить индикатор баром (эвристика по price action)
    pub fn update(&mut self, bar: &Bar) -> f64 {
        let delta = self.estimate_delta_from_bar(bar);
        self.process_delta(delta)
    }

    /// Обновить стандартным update_bar интерфейсом
    pub fn update_bar(&mut self, open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> f64 {
        let delta = if close > open {
            volume // Bullish - покупки
        } else if close < open {
            -volume // Bearish - продажи
        } else {
            0.0 // Нейтрально
        };
        self.process_delta(delta)
    }

    /// Оценка дельты из бара (эвристика)
    fn estimate_delta_from_bar(&self, bar: &Bar) -> f64 {
        let price_delta = bar.close - bar.open;
        if price_delta > 0.0 {
            bar.volume // Покупки
        } else if price_delta < 0.0 {
            -bar.volume // Продажи
        } else {
            0.0 // Нейтрально
        }
    }

    /// Обработка дельты (общая логика)
    fn process_delta(&mut self, delta: f64) -> f64 {
        self.current_delta = delta;
        self.cumulative_delta += delta;

        if self.count < self.period {
            self.buffer.push(delta);
            self.sum += delta;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            let old = self.buffer[self.idx];
            self.sum += delta - old;
            self.buffer[self.idx] = delta;
            self.idx = (self.idx + 1) % self.period;
        }

        self.ready = self.count >= self.period;
        delta
    }

    pub fn current_delta(&self) -> f64 { self.current_delta }
    pub fn cumulative_delta(&self) -> f64 { self.cumulative_delta }
    pub fn average_delta(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.sum / self.count.min(self.period) as f64 }
    }
    pub fn is_ready(&self) -> bool { self.ready }
    pub fn count(&self) -> usize { self.count }
    pub fn period(&self) -> usize { self.period }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.idx = 0;
        self.sum = 0.0;
        self.count = 0;
        self.cumulative_delta = 0.0;
        self.current_delta = 0.0;
        self.ready = false;
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_delta_creation() {
        let vd = VolumeDelta::new(20);
        assert!(!vd.is_ready());
        assert_eq!(vd.period(), 20);
        assert_eq!(vd.current_delta(), 0.0);
    }

    #[test]
    fn test_volume_delta_warmup() {
        let mut vd = VolumeDelta::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vd.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0);
        }
        assert!(vd.is_ready());
    }

    #[test]
    fn test_volume_delta_values() {
        let mut vd = VolumeDelta::new(10);
        // Price going up - should have positive delta
        let delta = vd.update_bar(100.0, 101.0, 99.0, 101.0, 1000.0);
        assert!(delta > 0.0, "Rising close should have positive delta");

        // Price going down - should have negative delta
        let delta = vd.update_bar(101.0, 102.0, 100.0, 100.0, 1000.0);
        assert!(delta < 0.0, "Falling close should have negative delta");
    }

    #[test]
    fn test_volume_delta_cumulative() {
        let mut vd = VolumeDelta::new(10);
        for i in 0..10 {
            let price = 100.0 + i as f64;
            vd.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0);
        }
        let cumulative = vd.cumulative_delta();
        assert!(cumulative.is_finite());
    }

    #[test]
    fn test_volume_delta_reset() {
        let mut vd = VolumeDelta::new(10);
        for i in 0..15 {
            vd.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        vd.reset();
        assert!(!vd.is_ready());
        assert_eq!(vd.current_delta(), 0.0);
        assert_eq!(vd.cumulative_delta(), 0.0);
    }
}
