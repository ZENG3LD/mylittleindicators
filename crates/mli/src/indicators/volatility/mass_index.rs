//! Mass Index - индикатор массового индекса Дональда Дорси
//! Mass Index = Сумма(EMA(High-Low, 9) / EMA(EMA(High-Low, 9), 9)) за 25 периодов
//! Используется для определения потенциальных разворотов тренда
//! Значения выше 27 указывают на возможный разворот, ниже 26.5 - на продолжение тренда

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use std::collections::VecDeque;

/// Mass Index индикатор
#[derive(Debug, Clone)]
pub struct MassIndex {
    ema_period: usize,
    sum_period: usize,

    // Кольцо последних `sum_period` отношений + их running-сумма (O(1) Mass Index).
    mass_ratio_values: VecDeque<f64>,
    mass_ratio_sum: f64,
    // История значений Mass Index — кольцо для вспомогательных сигналов.
    mass_index_values: VecDeque<f64>,

    // EMA для первого и второго сглаживания
    first_ema: SmootherSlot,
    second_ema: SmootherSlot,
    
    // Текущие значения
    mass_index_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl MassIndex {
    /// Создать новый Mass Index с параметрами по умолчанию (9, 25)
    pub fn new() -> Self {
        Self::with_params(9, 25)
    }
    
    /// Создать новый Mass Index с настраиваемыми параметрами
    pub fn with_params(ema_period: usize, sum_period: usize) -> Self {
        assert!(ema_period > 0, "EMA period must be greater than 0");
        assert!(sum_period > 0, "Sum period must be greater than 0");
        
        Self {
            ema_period,
            sum_period,
            mass_ratio_values: VecDeque::with_capacity(sum_period),
            mass_ratio_sum: 0.0,
            mass_index_values: VecDeque::with_capacity(512),
            first_ema: SmootherSlot::new(SmootherId::Ema, ema_period),
            second_ema: SmootherSlot::new(SmootherId::Ema, ema_period),
            mass_index_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        self.bars_count += 1;

        // Рассчитываем High - Low
        let high_low = high - low;

        // Первое сглаживание EMA(High-Low)
        let first_ema_value = self.first_ema.feed(high_low);

        // Второе сглаживание EMA(EMA(High-Low))
        let second_ema_value = self.second_ema.feed(first_ema_value);

        // Рассчитываем отношение
        let mass_ratio = if second_ema_value.abs() > 1e-12 {
            first_ema_value / second_ema_value
        } else {
            1.0
        };

        // Кольцо отношений длиной sum_period + running-сумма (O(1)).
        self.mass_ratio_values.push_back(mass_ratio);
        self.mass_ratio_sum += mass_ratio;
        if self.mass_ratio_values.len() > self.sum_period {
            if let Some(old) = self.mass_ratio_values.pop_front() {
                self.mass_ratio_sum -= old;
            }
        }
        // Mass Index = сумма отношений за период
        if self.mass_ratio_values.len() >= self.sum_period {
            self.mass_index_value = self.mass_ratio_sum;
        }

        // История значений Mass Index (для вспомогательных сигналов)
        if self.mass_index_values.len() >= 512 {
            self.mass_index_values.pop_front();
        }
        self.mass_index_values.push_back(self.mass_index_value);
        
        // Проверяем готовность
        if self.bars_count >= self.ema_period * 2 + self.sum_period {
            self.is_ready = true;
        }
        
        self.mass_index_value
    }
    
    /// Получить значение Mass Index
    pub fn value(&self) -> f64 {
        self.mass_index_value
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить параметры индикатора
    pub fn parameters(&self) -> (usize, usize) {
        (self.ema_period, self.sum_period)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.mass_ratio_values.clear();
        self.mass_ratio_sum = 0.0;
        self.mass_index_values.clear();
        self.first_ema.reset();
        self.second_ema.reset();
        self.mass_index_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.mass_index_value {
            v if v > 27.0 => "Potential Reversal Zone",
            v if v > 26.5 => "High Volatility",
            v if v < 26.5 => "Trend Continuation",
            _ => "Normal"
        }
    }
    
    /// Получить сигнал разворота
    /// 1 = потенциальный разворот вверх, -1 = потенциальный разворот вниз, 0 = нет сигнала
    pub fn reversal_signal(&self) -> i8 {
        if !self.is_ready() || self.mass_index_values.len() < 3 {
            return 0;
        }
        
        let len = self.mass_index_values.len();
        let current = self.mass_index_value;
        let prev_1 = if len >= 2 { self.mass_index_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.mass_index_values[len - 3] } else { 0.0 };
        
        // Сигнал разворота: Mass Index поднимается выше 27 и затем опускается ниже 26.5
        if prev_2 <= 27.0 && prev_1 > 27.0 && current < 26.5 {
            // Направление зависит от предыдущего тренда
            // Для простоты возвращаем общий сигнал разворота
            1
        } else {
            0
        }
    }
    
    /// Получить продвинутый сигнал разворота с дополнительными условиями
    pub fn advanced_reversal_signal(&self, price_trend: i8) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let basic_signal = self.reversal_signal();
        
        if basic_signal != 0 {
            // Если есть базовый сигнал, учитываем направление предыдущего тренда
            match price_trend {
                1 => -1,  // Восходящий тренд -> сигнал разворота вниз
                -1 => 1,  // Нисходящий тренд -> сигнал разворота вверх
                _ => 0    // Неопределенный тренд -> нет сигнала
            }
        } else {
            0
        }
    }
    
    /// Получить уровень волатильности
    pub fn volatility_level(&self) -> &'static str {
        match self.mass_index_value {
            v if v > 28.0 => "Extremely High",
            v if v > 27.0 => "Very High",
            v if v > 26.0 => "High",
            v if v > 25.0 => "Moderate",
            _ => "Low"
        }
    }
    
    /// Получить силу сигнала разворота (от 0 до 100)
    pub fn reversal_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        // Сила зависит от того, насколько высоко поднялся Mass Index
        let max_value = if self.mass_index_values.len() >= 10 {
            let start_idx = self.mass_index_values.len() - 10;
            self.mass_index_values.range(start_idx..)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        } else {
            self.mass_index_value
        };
        
        if max_value > 27.0 {
            ((max_value - 27.0) / 3.0 * 100.0).min(100.0)
        } else {
            0.0
        }
    }
    
    /// Получить тренд Mass Index
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.mass_index_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.mass_index_value;
        let past = self.mass_index_values[self.mass_index_values.len() - lookback - 1];
        
        if current > past {
            1  // Растущая волатильность
        } else if current < past {
            -1 // Падающая волатильность
        } else {
            0  // Стабильная волатильность
        }
    }
    
    /// Получить скорость изменения Mass Index
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.mass_index_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.mass_index_value;
        let past = self.mass_index_values[self.mass_index_values.len() - periods - 1];
        
        if past.abs() > 1e-12 {
            (current - past) / past * 100.0
        } else {
            0.0
        }
    }
    
    /// Получить среднее значение Mass Index за период
    pub fn average_value(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.mass_index_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.mass_index_values.len() - periods;
        let n = self.mass_index_values.len() - start_idx;

        self.mass_index_values.range(start_idx..).sum::<f64>() / n as f64
    }
    
    /// Получить экстремумы Mass Index за период
    pub fn extremes(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.mass_index_values.len() < periods {
            return (0.0, 0.0);
        }
        
        let start_idx = self.mass_index_values.len() - periods;

        let max_val = self.mass_index_values.range(start_idx..).fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = self.mass_index_values.range(start_idx..).fold(f64::INFINITY, |a, &b| a.min(b));

        (min_val, max_val)
    }
    
    /// Проверить, находится ли Mass Index в зоне разворота
    pub fn in_reversal_zone(&self) -> bool {
        self.is_ready() && self.mass_index_value > 27.0
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let strength = self.reversal_strength();
        let trend_dir = match self.trend_direction(5) {
            1 => "Rising",
            -1 => "Falling",
            _ => "Stable"
        };

        format!(
            "Mass Index: {:.2}, Condition: {}, Volatility: {}, Strength: {:.1}%, Trend: {}",
            self.mass_index_value,
            self.market_condition(),
            self.volatility_level(),
            strength,
            trend_dir
        )
    }

}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`MassIndex`] — NO smoother slot: Mass Index is
/// definitionally EMA-of-EMA of the bar range (the EMA kernel is not a free choice).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MassIndexConfig {
    pub ema_period: Param<usize>,
    pub sum_period: Param<usize>,
}

impl Indicator for MassIndex {
    const ID: IndicatorId = IndicatorId::Mi;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the bar range — smooths `high - low` twice with EMA.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1) running-sum Mass Index: a `sum_period`-deep ratio ring (evict-and-add) plus a
    /// bounded history ring for the auxiliary signals; the two internal EMAs are O(1)
    /// scalar state (fixed kernel, no slot).
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Deque),
            Store::fixed(StoreKind::Vec, 512),
        ],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Mi)];
    type Config = MassIndexConfig;
    type Runtime = MassIndex;

    fn create(cfg: MassIndexConfig) -> MassIndex {
        MassIndex::with_params(cfg.ema_period.resolved(), cfg.sum_period.resolved())
    }
}

impl crate::contract::Config for MassIndexConfig {
    fn defaults() -> Self {
        MassIndexConfig {
            ema_period: Param::Solo(9),
            sum_period: Param::Solo(25),
        }
    }
    fn machine_defaults() -> Self {
        // ema_period, sum_period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for MassIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Mi, "Vol MI", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_index_creation() {
        let mi = MassIndex::new();
        assert!(!mi.is_ready());
        assert_eq!(mi.value(), 0.0);
    }

    #[test]
    fn test_mass_index_warmup() {
        let mut mi = MassIndex::with_params(9, 25);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mi.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(mi.is_ready());
    }

    #[test]
    fn test_mass_index_values() {
        let mut mi = MassIndex::new();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = mi.feed(&[price + 2.0, price - 2.0]);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_mass_index_market_condition() {
        let mut mi = MassIndex::new();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            mi.feed(&[price + 1.0, price - 1.0]);
        }
        let condition = mi.market_condition();
        assert!(!condition.is_empty());
    }

    #[test]
    fn test_mass_index_reset() {
        let mut mi = MassIndex::new();
        for _i in 0..50 {
            mi.feed(&[101.0, 99.0]);
        }
        mi.reset();
        assert!(!mi.is_ready());
        assert_eq!(mi.value(), 0.0);
    }

    /// The factory resolves the fixed High/Low lanes and feeds the pair; Mass Index runs
    /// end-to-end and stays non-negative.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Mi(<<MassIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Mi) > 0.0, "factory Mass Index should be > 0, got {}", f.read(IndicatorOutputId::Mi));
    }
}

