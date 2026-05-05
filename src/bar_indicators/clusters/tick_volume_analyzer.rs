//! Tick Volume Analyzer - анализатор тикового объема
//! Работает с Tick данными для детального анализа микроструктуры рынка

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Tick;
use arrayvec::ArrayVec;

/// Анализатор тикового объема
#[derive(Clone)]
pub struct TickVolumeAnalyzer {
    period: usize,
    ticks: ArrayVec<Tick, 1024>,

    // Статистика
    total_volume: f64,
    buy_volume: f64,
    sell_volume: f64,

    // Метрики
    volume_delta: f64,
    volume_ratio: f64, // buy / sell
    tick_count: usize,
    avg_tick_size: f64,

    // Микроструктурные метрики
    avg_spread: f64,
    spread_samples: usize,
}

impl TickVolumeAnalyzer {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ticks: ArrayVec::new(),
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            volume_delta: 0.0,
            volume_ratio: 1.0,
            tick_count: 0,
            avg_tick_size: 0.0,
            avg_spread: 0.0,
            spread_samples: 0,
        }
    }

    /// Обновить анализатор новым тиком
    pub fn update(&mut self, tick: &Tick) {
        // Обновляем объемы
        self.total_volume += tick.size;
        if tick.is_buy {
            self.buy_volume += tick.size;
        } else {
            self.sell_volume += tick.size;
        }

        // Обновляем spread
        if let Some(spread) = tick.spread() {
            self.avg_spread = (self.avg_spread * self.spread_samples as f64 + spread)
                / (self.spread_samples + 1) as f64;
            self.spread_samples += 1;
        }

        self.tick_count += 1;
        self.volume_delta = self.buy_volume - self.sell_volume;
        self.volume_ratio = if self.sell_volume > 0.0 {
            self.buy_volume / self.sell_volume
        } else {
            1.0
        };
        self.avg_tick_size = self.total_volume / self.tick_count as f64;

        // Сохраняем тик в буфер
        if self.ticks.len() >= self.period {
            self.ticks.remove(0);
        }
        if !self.ticks.is_full() {
            self.ticks.push(*tick);
        }
    }

    /// Обновить стандартным update_bar (эмулирует buy/sell по направлению бара)
    /// Если close > open - считаем buy volume, иначе sell volume
    pub fn update_bar(&mut self, o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        self.total_volume += v;

        // Эмуляция buy/sell по направлению бара
        if c > o {
            // Бычий бар - считаем как buy volume
            self.buy_volume += v;
        } else if c < o {
            // Медвежий бар - считаем как sell volume
            self.sell_volume += v;
        } else {
            // Doji - делим пополам
            self.buy_volume += v * 0.5;
            self.sell_volume += v * 0.5;
        }

        self.tick_count += 1;
        self.volume_delta = self.buy_volume - self.sell_volume;
        self.volume_ratio = if self.sell_volume > 0.0 {
            self.buy_volume / self.sell_volume
        } else {
            1.0
        };
        self.avg_tick_size = self.total_volume / self.tick_count as f64;
        self.volume_delta
    }

    pub fn volume_delta(&self) -> f64 { self.volume_delta }
    pub fn volume_ratio(&self) -> f64 { self.volume_ratio }
    pub fn buy_volume(&self) -> f64 { self.buy_volume }
    pub fn sell_volume(&self) -> f64 { self.sell_volume }
    pub fn avg_spread(&self) -> f64 { self.avg_spread }
    pub fn tick_count(&self) -> usize { self.tick_count }

    /// Получить значение как IndicatorValue
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.volume_delta)
    }

    pub fn reset(&mut self) {
        self.ticks.clear();
        self.total_volume = 0.0;
        self.buy_volume = 0.0;
        self.sell_volume = 0.0;
        self.volume_delta = 0.0;
        self.volume_ratio = 1.0;
        self.tick_count = 0;
        self.avg_tick_size = 0.0;
        self.avg_spread = 0.0;
        self.spread_samples = 0;
    }
}

impl TickVolumeAnalyzer {
    pub fn is_ready(&self) -> bool { self.tick_count > 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_volume_analyzer_creation() {
        let ind = TickVolumeAnalyzer::new(100);
        assert!(!ind.is_ready());
        assert_eq!(ind.volume_delta(), 0.0);
    }

    #[test]
    fn test_tick_volume_analyzer_update_bar() {
        let mut ind = TickVolumeAnalyzer::new(100);
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert!(ind.is_ready());
        assert_eq!(ind.tick_count(), 1);
    }

    #[test]
    fn test_tick_volume_analyzer_multiple_updates() {
        let mut ind = TickVolumeAnalyzer::new(100);
        for i in 0..10 {
            ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0 + i as f64);
        }
        assert_eq!(ind.tick_count(), 10);
    }

    #[test]
    fn test_tick_volume_analyzer_reset() {
        let mut ind = TickVolumeAnalyzer::new(100);
        for _ in 0..5 {
            ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.volume_delta(), 0.0);
        assert_eq!(ind.tick_count(), 0);
    }
}
