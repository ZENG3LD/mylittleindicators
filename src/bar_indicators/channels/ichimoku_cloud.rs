//! ichimoku_cloud.rs: High-Performance Ichimoku Cloud
//! Облако Ишимоку - японская система технического анализа
//!
//! Особенности:
//! - Все 5 компонентов: Tenkan, Kijun, Senkou A/B, Chikou
//! - Circular buffer O(1) operations
//! - Cloud analysis (bullish/bearish, thickness)
//! - Future cloud projection

use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Состояние облака Ишимоку
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CloudState {
    /// Бычье облако (Senkou A выше Senkou B)
    Bullish,
    /// Медвежье облако (Senkou A ниже Senkou B)
    Bearish,
    /// Плоское облако (Senkou A ≈ Senkou B)
    Flat,
}

/// Позиция цены относительно облака
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CloudPosition {
    /// Цена выше облака
    AboveCloud,
    /// Цена внутри облака
    InCloud,
    /// Цена ниже облака
    BelowCloud,
}

/// Сигналы Ichimoku
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IchimokuSignal {
    /// Сильный бычий сигнал
    StrongBuy,
    /// Бычий сигнал
    Buy,
    /// Нейтрально
    Neutral,
    /// Медвежий сигнал
    Sell,
    /// Сильный медвежий сигнал
    StrongSell,
}

/// High-Performance Ichimoku Cloud
#[derive(Debug, Clone)]
pub struct IchimokuCloud {
    // Периоды линий
    tenkan_period: usize,  // Обычно 9
    kijun_period: usize,   // Обычно 26
    senkou_b_period: usize, // Обычно 52
    displacement: usize,    // Обычно 26 (смещение в будущее для Senkou)

    // Circular buffers для high/low - O(1) operations
    tenkan_high_buffer: ArrayVec<f64, 512>,
    tenkan_low_buffer: ArrayVec<f64, 512>,
    tenkan_index: usize,
    tenkan_filled: bool,

    kijun_high_buffer: ArrayVec<f64, 512>,
    kijun_low_buffer: ArrayVec<f64, 512>,
    kijun_index: usize,
    kijun_filled: bool,

    senkou_b_high_buffer: ArrayVec<f64, 512>,
    senkou_b_low_buffer: ArrayVec<f64, 512>,
    senkou_b_index: usize,
    senkou_b_filled: bool,

    // Буфер для сдвига Chikou Span
    chikou_buffer: ArrayVec<f64, 512>,

    // Буфер для будущих значений Senkou (displacement)
    senkou_future_buffer: ArrayVec<(f64, f64), 512>, // (Senkou A, Senkou B)

    // Текущие значения линий
    tenkan_sen: f64,     // Conversion Line
    kijun_sen: f64,      // Base Line
    senkou_span_a: f64,  // Leading Span A (в будущем)
    senkou_span_b: f64,  // Leading Span B (в будущем)
    chikou_span: f64,    // Lagging Span (в прошлом)

    // Текущие значения облака (для текущего времени)
    current_cloud_top: f64,
    current_cloud_bottom: f64,

    // Состояние облака
    cloud_state: CloudState,
    cloud_thickness: f64,

    // Статистика
    bar_count: usize,
}

impl IchimokuCloud {
    /// Создать Ichimoku Cloud с стандартными параметрами (9, 26, 52, 26)
    pub fn new() -> Self {
        Self::new_custom(9, 26, 52, 26)
    }

    /// Создать Ichimoku Cloud с кастомными параметрами
    pub fn new_custom(
        tenkan_period: usize,
        kijun_period: usize,
        senkou_b_period: usize,
        displacement: usize
    ) -> Self {
        assert!(tenkan_period > 0 && tenkan_period <= 512);
        assert!(kijun_period > 0 && kijun_period <= 512);
        assert!(senkou_b_period > 0 && senkou_b_period <= 512);
        assert!(displacement > 0 && displacement <= 512);

        Self {
            tenkan_period,
            kijun_period,
            senkou_b_period,
            displacement,
            tenkan_high_buffer: ArrayVec::new(),
            tenkan_low_buffer: ArrayVec::new(),
            tenkan_index: 0,
            tenkan_filled: false,
            kijun_high_buffer: ArrayVec::new(),
            kijun_low_buffer: ArrayVec::new(),
            kijun_index: 0,
            kijun_filled: false,
            senkou_b_high_buffer: ArrayVec::new(),
            senkou_b_low_buffer: ArrayVec::new(),
            senkou_b_index: 0,
            senkou_b_filled: false,
            chikou_buffer: ArrayVec::new(),
            senkou_future_buffer: ArrayVec::new(),
            tenkan_sen: 0.0,
            kijun_sen: 0.0,
            senkou_span_a: 0.0,
            senkou_span_b: 0.0,
            chikou_span: 0.0,
            current_cloud_top: 0.0,
            current_cloud_bottom: 0.0,
            cloud_state: CloudState::Flat,
            cloud_thickness: 0.0,
            bar_count: 0,
        }
    }

    /// Обновить Ichimoku новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64, f64, f64, f64) {
        self.bar_count += 1;

        // Обновляем все компоненты
        self.update_tenkan_sen(high, low);
        self.update_kijun_sen(high, low);
        self.update_senkou_b_buffer(high, low);
        self.update_senkou_spans();
        self.update_chikou_span(close);
        self.update_current_cloud();
        self.analyze_cloud();

        (self.tenkan_sen, self.kijun_sen, self.senkou_span_a, self.senkou_span_b, self.chikou_span)
    }

    /// Обновить Tenkan-sen (Conversion Line)
    fn update_tenkan_sen(&mut self, high: f64, low: f64) {
        // Circular buffer для Tenkan
        if self.tenkan_filled {
            self.tenkan_high_buffer[self.tenkan_index] = high;
            self.tenkan_low_buffer[self.tenkan_index] = low;
        } else {
            self.tenkan_high_buffer.push(high);
            self.tenkan_low_buffer.push(low);
        }

        self.tenkan_index = (self.tenkan_index + 1) % self.tenkan_period;

        if self.tenkan_high_buffer.len() == self.tenkan_period && !self.tenkan_filled {
            self.tenkan_filled = true;
        }

        // Рассчитываем Tenkan = (Max High + Min Low) / 2 за период
        if self.tenkan_filled {
            let (min_low, max_high) = self.tenkan_high_buffer.iter()
                .zip(self.tenkan_low_buffer.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.tenkan_sen = (max_high + min_low) / 2.0;
        }
    }

    /// Обновить Kijun-sen (Base Line)
    fn update_kijun_sen(&mut self, high: f64, low: f64) {
        // Circular buffer для Kijun
        if self.kijun_filled {
            self.kijun_high_buffer[self.kijun_index] = high;
            self.kijun_low_buffer[self.kijun_index] = low;
        } else {
            self.kijun_high_buffer.push(high);
            self.kijun_low_buffer.push(low);
        }

        self.kijun_index = (self.kijun_index + 1) % self.kijun_period;

        if self.kijun_high_buffer.len() == self.kijun_period && !self.kijun_filled {
            self.kijun_filled = true;
        }

        // Рассчитываем Kijun = (Max High + Min Low) / 2 за период
        if self.kijun_filled {
            let (min_low, max_high) = self.kijun_high_buffer.iter()
                .zip(self.kijun_low_buffer.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.kijun_sen = (max_high + min_low) / 2.0;
        }
    }

    /// Обновить буфер Senkou B (отдельный буфер для периода 52)
    fn update_senkou_b_buffer(&mut self, high: f64, low: f64) {
        // Circular buffer для Senkou B
        if self.senkou_b_filled {
            self.senkou_b_high_buffer[self.senkou_b_index] = high;
            self.senkou_b_low_buffer[self.senkou_b_index] = low;
        } else {
            self.senkou_b_high_buffer.push(high);
            self.senkou_b_low_buffer.push(low);
        }

        self.senkou_b_index = (self.senkou_b_index + 1) % self.senkou_b_period;

        if self.senkou_b_high_buffer.len() == self.senkou_b_period && !self.senkou_b_filled {
            self.senkou_b_filled = true;
        }
    }

    /// Обновить Senkou Spans (Leading Spans)
    fn update_senkou_spans(&mut self) {
        // Senkou Span A = (Tenkan + Kijun) / 2
        if self.tenkan_filled && self.kijun_filled {
            self.senkou_span_a = (self.tenkan_sen + self.kijun_sen) / 2.0;
        }

        // Senkou Span B = (Max High + Min Low) / 2 за senkou_b_period
        if self.senkou_b_filled {
            let (min_low, max_high) = self.senkou_b_high_buffer.iter()
                .zip(self.senkou_b_low_buffer.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.senkou_span_b = (max_high + min_low) / 2.0;
        }

        // Сохраняем Senkou spans в будущий буфер (с displacement)
        if self.senkou_future_buffer.len() >= self.displacement {
            self.senkou_future_buffer.remove(0);
        }
        self.senkou_future_buffer.push((self.senkou_span_a, self.senkou_span_b));
    }

    /// Обновить Chikou Span (Lagging Span)
    fn update_chikou_span(&mut self, close: f64) {
        // Chikou = текущий close, сдвинутый на displacement назад
        if self.chikou_buffer.len() >= self.displacement {
            self.chikou_buffer.remove(0);
        }
        self.chikou_buffer.push(close);

        // Chikou span - это close от displacement баров назад
        if self.chikou_buffer.len() >= self.displacement {
            self.chikou_span = self.chikou_buffer[0];
        }
    }

    /// Обновить текущие значения облака
    fn update_current_cloud(&mut self) {
        // Текущее облако - это Senkou spans от displacement баров назад
        if self.senkou_future_buffer.len() >= self.displacement {
            let (senkou_a_past, senkou_b_past) = self.senkou_future_buffer[0];
            self.current_cloud_top = senkou_a_past.max(senkou_b_past);
            self.current_cloud_bottom = senkou_a_past.min(senkou_b_past);
        }
    }

    /// Анализировать состояние облака
    fn analyze_cloud(&mut self) {
        // Определяем состояние облака
        if self.senkou_span_a > self.senkou_span_b + 0.0001 {
            self.cloud_state = CloudState::Bullish;
        } else if self.senkou_span_a < self.senkou_span_b - 0.0001 {
            self.cloud_state = CloudState::Bearish;
        } else {
            self.cloud_state = CloudState::Flat;
        }

        // Толщина облака
        self.cloud_thickness = (self.current_cloud_top - self.current_cloud_bottom).abs();
    }

    /// Получить все значения Ichimoku (tenkan, kijun, senkou_a, senkou_b, chikou)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Ichimoku {
            tenkan: self.tenkan_sen,
            kijun: self.kijun_sen,
            senkou_a: self.senkou_span_a,
            senkou_b: self.senkou_span_b,
            chikou: self.chikou_span,
        }
    }

    /// Получить все значения Ichimoku как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64, f64, f64) {
        (self.tenkan_sen, self.kijun_sen, self.senkou_span_a, self.senkou_span_b, self.chikou_span)
    }

    /// Получить текущее облако (для текущего времени)
    pub fn current_cloud(&self) -> (f64, f64) {
        (self.current_cloud_top, self.current_cloud_bottom)
    }

    /// Получить будущее облако (Senkou spans)
    pub fn future_cloud(&self) -> (f64, f64) {
        (self.senkou_span_a, self.senkou_span_b)
    }

    /// Получить Tenkan-sen
    pub fn tenkan_sen(&self) -> f64 {
        self.tenkan_sen
    }

    /// Получить Kijun-sen
    pub fn kijun_sen(&self) -> f64 {
        self.kijun_sen
    }

    /// Получить Chikou Span
    pub fn chikou_span(&self) -> f64 {
        self.chikou_span
    }

    /// Получить состояние облака
    pub fn cloud_state(&self) -> CloudState {
        self.cloud_state
    }

    /// Получить толщину облака
    pub fn cloud_thickness(&self) -> f64 {
        self.cloud_thickness
    }

    /// Определить позицию цены относительно облака
    pub fn price_cloud_position(&self, price: f64) -> CloudPosition {
        if price > self.current_cloud_top {
            CloudPosition::AboveCloud
        } else if price < self.current_cloud_bottom {
            CloudPosition::BelowCloud
        } else {
            CloudPosition::InCloud
        }
    }

    /// Генерация сигнала Ichimoku
    pub fn generate_signal(&self, current_price: f64) -> IchimokuSignal {
        let mut bullish_signals = 0;
        let mut bearish_signals = 0;

        // 1. Tenkan vs Kijun
        if self.tenkan_sen > self.kijun_sen {
            bullish_signals += 1;
        } else if self.tenkan_sen < self.kijun_sen {
            bearish_signals += 1;
        }

        // 2. Цена vs облако
        match self.price_cloud_position(current_price) {
            CloudPosition::AboveCloud => bullish_signals += 2,
            CloudPosition::BelowCloud => bearish_signals += 2,
            CloudPosition::InCloud => {}, // Нейтрально
        }

        // 3. Состояние облака
        match self.cloud_state {
            CloudState::Bullish => bullish_signals += 1,
            CloudState::Bearish => bearish_signals += 1,
            CloudState::Flat => {},
        }

        // 4. Chikou vs цена в прошлом
        if self.chikou_span > current_price {
            bullish_signals += 1;
        } else if self.chikou_span < current_price {
            bearish_signals += 1;
        }

        // Генерация итогового сигнала
        let signal_strength = bullish_signals - bearish_signals;

        match signal_strength {
            3..=5 => IchimokuSignal::StrongBuy,
            1..=2 => IchimokuSignal::Buy,
            0 => IchimokuSignal::Neutral,
            -2..=-1 => IchimokuSignal::Sell,
            -5..=-3 => IchimokuSignal::StrongSell,
            _ => IchimokuSignal::Neutral,
        }
    }

    /// Проверить пробой Kijun-sen
    pub fn is_kijun_breakout(&self, current_price: f64, previous_price: f64) -> Option<bool> {
        if previous_price <= self.kijun_sen && current_price > self.kijun_sen {
            Some(true) // Пробой вверх
        } else if previous_price >= self.kijun_sen && current_price < self.kijun_sen {
            Some(false) // Пробой вниз
        } else {
            None
        }
    }

    /// Проверить TK-Cross (пересечение Tenkan и Kijun)
    pub fn is_tk_cross(&self, prev_tenkan: f64, prev_kijun: f64) -> Option<bool> {
        let current_tk_diff = self.tenkan_sen - self.kijun_sen;
        let prev_tk_diff = prev_tenkan - prev_kijun;

        if prev_tk_diff <= 0.0 && current_tk_diff > 0.0 {
            Some(true) // Золотой крест (bullish)
        } else if prev_tk_diff >= 0.0 && current_tk_diff < 0.0 {
            Some(false) // Мертвый крест (bearish)
        } else {
            None
        }
    }

    /// Проверить, готов ли индикатор
    /// Требует заполнения всех буферов включая displacement для корректных Senkou/Chikou
    pub fn is_ready(&self) -> bool {
        self.tenkan_filled && self.kijun_filled && self.senkou_b_filled
            && self.chikou_buffer.len() >= self.displacement
            && self.senkou_future_buffer.len() >= self.displacement
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.tenkan_high_buffer.clear();
        self.tenkan_low_buffer.clear();
        self.tenkan_index = 0;
        self.tenkan_filled = false;

        self.kijun_high_buffer.clear();
        self.kijun_low_buffer.clear();
        self.kijun_index = 0;
        self.kijun_filled = false;

        self.senkou_b_high_buffer.clear();
        self.senkou_b_low_buffer.clear();
        self.senkou_b_index = 0;
        self.senkou_b_filled = false;

        self.chikou_buffer.clear();
        self.senkou_future_buffer.clear();

        self.tenkan_sen = 0.0;
        self.kijun_sen = 0.0;
        self.senkou_span_a = 0.0;
        self.senkou_span_b = 0.0;
        self.chikou_span = 0.0;
        self.current_cloud_top = 0.0;
        self.current_cloud_bottom = 0.0;
        self.cloud_state = CloudState::Flat;
        self.cloud_thickness = 0.0;
        self.bar_count = 0;
    }

    /// Получить параметры
    pub fn get_params(&self) -> (usize, usize, usize, usize) {
        (self.tenkan_period, self.kijun_period, self.senkou_b_period, self.displacement)
    }
}

impl Default for IchimokuCloud {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ichimoku_cloud_creation() {
        let ic = IchimokuCloud::new();
        assert!(!ic.is_ready());
        let (tenkan, kijun, _senkou_b, _displacement) = ic.get_params();
        assert_eq!(tenkan, 9);
        assert_eq!(kijun, 26);
    }

    #[test]
    fn test_ichimoku_cloud_warmup() {
        let mut ic = IchimokuCloud::new();
        for i in 0..55 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ic.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ic.is_ready());
    }

    #[test]
    fn test_ichimoku_cloud_values() {
        let mut ic = IchimokuCloud::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (tenkan, kijun, _senkou_a, _senkou_b, _chikou) = ic.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if ic.is_ready() {
                assert!(tenkan.is_finite());
                assert!(kijun.is_finite());
            }
        }
    }

    #[test]
    fn test_ichimoku_cloud_reset() {
        let mut ic = IchimokuCloud::new();
        for i in 0..80 {
            ic.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ic.reset();
        assert!(!ic.is_ready());
    }
}
