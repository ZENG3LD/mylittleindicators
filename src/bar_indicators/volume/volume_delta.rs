//! Volume Delta Indicator
//! Анализирует баланс покупок/продаж по реальным buy/sell объёмам.
//! Bar OHLCV не несёт информации об агрессоре сделки — обновление доступно
//! только через `update_with_delta`; `update_bar` — no-op feed.

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Volume Delta индикатор с фиксированным окном
#[derive(Debug, Clone)]
pub struct VolumeDelta {
    /// Период для расчета средних
    period: usize,
    /// Буфер дельт (фиксированный размер)
    buffer: Vec<f64>,
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
            buffer: Vec::with_capacity(period),
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

    /// No-new-information feed: bar OHLCV carries no real aggressor-side
    /// data, so this cannot fabricate a delta. Returns the current per-bar
    /// delta unchanged; real updates arrive via [`Self::update_with_delta`].
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> f64 {
        self.current_delta
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
        for _ in 0..15 {
            vd.update_with_delta(600.0, 400.0);
        }
        assert!(vd.is_ready());
    }

    #[test]
    fn test_volume_delta_values() {
        let mut vd = VolumeDelta::new(10);
        // Real buy pressure - should have positive delta
        let delta = vd.update_with_delta(700.0, 300.0);
        assert!(delta > 0.0, "Buy-dominant volume should have positive delta");

        // Real sell pressure - should have negative delta
        let delta = vd.update_with_delta(300.0, 700.0);
        assert!(delta < 0.0, "Sell-dominant volume should have negative delta");
    }

    #[test]
    fn test_volume_delta_cumulative() {
        let mut vd = VolumeDelta::new(10);
        for i in 0..10 {
            vd.update_with_delta(500.0 + i as f64, 500.0);
        }
        let cumulative = vd.cumulative_delta();
        assert!(cumulative.is_finite());
    }

    #[test]
    fn test_volume_delta_reset() {
        let mut vd = VolumeDelta::new(10);
        for _ in 0..15 {
            vd.update_with_delta(600.0, 400.0);
        }
        vd.reset();
        assert!(!vd.is_ready());
        assert_eq!(vd.current_delta(), 0.0);
        assert_eq!(vd.cumulative_delta(), 0.0);
    }

    #[test]
    fn update_bar_does_not_fabricate() {
        let mut vd = VolumeDelta::new(10);
        vd.update_with_delta(500.0, 0.0);
        let delta = vd.update_bar(100.0, 102.0, 99.0, 101.0, 999.0);
        assert!((delta - 500.0).abs() < 1e-9, "bar feed must not mutate delta");
        let delta = vd.update_bar(101.0, 102.0, 99.0, 100.0, 999.0);
        assert!((delta - 500.0).abs() < 1e-9, "bar feed must not mutate delta");
    }
}
