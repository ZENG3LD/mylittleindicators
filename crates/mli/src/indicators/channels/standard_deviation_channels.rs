//! standard_deviation_channels.rs: High-Performance Standard Deviation Channels
//! Каналы стандартного отклонения - статистически обоснованные каналы
//!
//! Особенности:
//! - Использует готовый LinearRegressionMA компонент для центральной линии
//! - Полосы стандартного отклонения (1σ, 2σ, 3σ) от регрессии
//! - Circular buffer O(1) operations
//! - Адаптивные режимы расчета

use crate::indicators::average::lr::LinearRegressionMA;
use crate::engine::ohlcv_field::OhlcvField;

/// Режимы расчета стандартного отклонения
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StandardDeviationMode {
    /// Простое стандартное отклонение
    Simple,
    /// Популяционное стандартное отклонение (n-1)
    Population,
    /// Адаптивное к волатильности
    Adaptive,
}

/// Источник данных для линейной регрессии
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegressionSource {
    /// Цена закрытия
    Close,
    /// Типичная цена (H+L+C)/3
    Typical,
    /// Средняя цена (H+L)/2
    Median,
    /// Взвешенная цена (H+L+2*C)/4
    Weighted,
    /// OHLC4 цена (O+H+L+C)/4
    Ohlc4,
}

/// Сигналы каналов стандартного отклонения
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StandardDeviationSignal {
    /// Сильный пробой вверх (выше 2σ)
    StrongBreakoutUp,
    /// Пробой вверх (выше 1σ)
    BreakoutUp,
    /// Возврат к среднему (к линии регрессии)
    MeanReversion,
    /// Пробой вниз (ниже -1σ)
    BreakoutDown,
    /// Сильный пробой вниз (ниже -2σ)
    StrongBreakoutDown,
    /// Внутри канала
    WithinBands,
}

/// High-Performance Standard Deviation Channels
/// Архитектура: LinearRegressionMA для центральной линии + circular buffer для std dev
#[derive(Debug, Clone)]
pub struct StandardDeviationChannels {
    // Параметры
    period: usize,
    std_multiplier: f64,
    mode: StandardDeviationMode,
    source: RegressionSource,
    
    // Компоненты (используем готовые!)
    regression_ma: LinearRegressionMA,  // ✅ Линейная регрессия через готовый компонент
    
    // Circular buffer для расчета отклонений от регрессии - O(1) operations
    price_buffer: Vec<f64>,
    price_index: usize,
    buffer_filled: bool,
    
    // Стандартное отклонение
    std_deviation: f64,
    
    // Полосы каналов
    upper_band_1: f64,   // +1σ
    upper_band_2: f64,   // +2σ
    upper_band_3: f64,   // +3σ
    lower_band_1: f64,   // -1σ
    lower_band_2: f64,   // -2σ
    lower_band_3: f64,   // -3σ
    
    // Адаптивные параметры
    volatility_factor: f64,
    adaptive_multiplier: f64,
    
    // Статистика
    bar_count: usize,
}

impl StandardDeviationChannels {
    /// Создать каналы стандартного отклонения со стандартными параметрами
    pub fn new(period: usize) -> Self {
        Self::new_custom(
            period,
            2.0,
            StandardDeviationMode::Simple,
            RegressionSource::Close
        )
    }
    
    /// Создать каналы с кастомными параметрами
    /// period - период для линейной регрессии и std dev
    /// std_multiplier - множитель стандартного отклонения (обычно 2.0)
    /// mode - режим расчета std dev (простое, популяционное, адаптивное)
    /// source - источник данных (Close, Typical, etc.)
    pub fn new_custom(
        period: usize,
        std_multiplier: f64,
        mode: StandardDeviationMode,
        source: RegressionSource
    ) -> Self {
        assert!(period > 1, "Period must be >= 2");
        assert!(std_multiplier > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_multiplier,
            mode,
            source,
            regression_ma: LinearRegressionMA::new(period),
            price_buffer: Vec::with_capacity(period),
            price_index: 0,
            buffer_filled: false,
            std_deviation: 0.0,
            upper_band_1: 0.0,
            upper_band_2: 0.0,
            upper_band_3: 0.0,
            lower_band_1: 0.0,
            lower_band_2: 0.0,
            lower_band_3: 0.0,
            volatility_factor: 1.0,
            adaptive_multiplier: 1.0,
            bar_count: 0,
        }
    }

    /// Создать каналы с настраиваемым источником данных OHLCV
    pub fn with_source(
        period: usize,
        std_multiplier: f64,
        mode: StandardDeviationMode,
        source: RegressionSource,
        _ohlcv_source: OhlcvField
    ) -> Self {
        Self::new_custom(period, std_multiplier, mode, source)
    }
    
    /// Создать каналы с простыми параметрами
    pub fn new_simple(period: usize, std_multiplier: f64) -> Self {
        Self::new_custom(
            period,
            std_multiplier,
            StandardDeviationMode::Simple,
            RegressionSource::Close
        )
    }
    
    /// Создать адаптивные каналы
    pub fn new_adaptive(period: usize) -> Self {
        Self::new_custom(
            period,
            2.0,
            StandardDeviationMode::Adaptive,
            RegressionSource::Typical
        )
    }
    
    /// Feed a scalar — the resolved price (const SOURCE = Field{Close}).
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        self.bar_count += 1;

        let regression_value = self.regression_ma.feed(price);

        // Обновляем circular buffer для std dev
        self.update_price_buffer(price);

        if self.buffer_filled && self.regression_ma.is_ready() {
            self.calculate_standard_deviation();
            // No high/low available — skip adaptive parameter update (uses h-l range)
            self.calculate_bands(regression_value);
        }

        (self.upper_band_2, regression_value, self.lower_band_2)
    }
    
    /// Обновить circular buffer цен - O(1) операция
    fn update_price_buffer(&mut self, price: f64) {
        if self.buffer_filled {
            // Перезаписываем старые значения циклически
            self.price_buffer[self.price_index] = price;
        } else {
            // Заполняем буфер в первый раз
            self.price_buffer.push(price);
        }
        
        // Обновляем индекс циклически
        self.price_index = (self.price_index + 1) % self.period;
        
        // Проверяем заполненность буфера
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }
    }
    
    /// Рассчитать стандартное отклонение от регрессионной линии
    fn calculate_standard_deviation(&mut self) {
        let buffer_len = if self.buffer_filled { self.period } else { self.price_buffer.len() };
        let regression_value = self.regression_ma.line();
        
        // Рассчитываем отклонения от регрессионной линии
        let variance = self.price_buffer.iter()
            .take(buffer_len)
            .map(|&price| {
                let diff = price - regression_value;
                diff * diff
            })
            .sum::<f64>();
        
        // Применяем режим расчета
        let denominator = match self.mode {
            StandardDeviationMode::Simple => buffer_len as f64,
            StandardDeviationMode::Population => (buffer_len - 1) as f64,
            StandardDeviationMode::Adaptive => {
                // Адаптивное деление с учетом волатильности
                buffer_len as f64 * self.volatility_factor
            }
        };
        
        self.std_deviation = (variance / denominator).sqrt();
    }
    
    /// Рассчитать полосы каналов
    fn calculate_bands(&mut self, regression_value: f64) {
        let effective_std = self.std_deviation * self.adaptive_multiplier;
        
        // Рассчитываем полосы с разными множителями
        self.upper_band_1 = regression_value + 1.0 * effective_std;
        self.lower_band_1 = regression_value - 1.0 * effective_std;
        
        self.upper_band_2 = regression_value + self.std_multiplier * effective_std;
        self.lower_band_2 = regression_value - self.std_multiplier * effective_std;
        
        self.upper_band_3 = regression_value + 3.0 * effective_std;
        self.lower_band_3 = regression_value - 3.0 * effective_std;
    }
    

    pub fn upper(&self) -> f64 { self.upper_band_2 }
    pub fn middle(&self) -> f64 { self.regression_ma.line() }
    pub fn lower(&self) -> f64 { self.lower_band_2 }

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper_band_2, self.regression_ma.line(), self.lower_band_2)
    }
    
    /// Получить все полосы (1σ, 2σ, 3σ)
    pub fn all_bands(&self) -> (f64, f64, f64, f64, f64, f64, f64) {
        (
            self.upper_band_3,
            self.upper_band_2,
            self.upper_band_1,
            self.regression_ma.line(),
            self.lower_band_1,
            self.lower_band_2,
            self.lower_band_3,
        )
    }
    
    /// Получить значение регрессионной линии
    pub fn regression_line(&self) -> f64 {
        self.regression_ma.line()
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.upper_band_2 - self.lower_band_2
    }
    
    /// Получить позицию цены в канале
    pub fn position_in_channel(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower_band_2) / width
        } else {
            0.5
        }
    }
    
    /// Получить статистику регрессии (slope, intercept, r2, std_dev)
    pub fn regression_stats(&self) -> (f64, f64, f64, f64) {
        (
            self.regression_ma.slope(),
            self.regression_ma.intercept(),
            self.regression_ma.r2(),
            self.std_deviation
        )
    }
    
    /// Получить стандартное отклонение
    pub fn standard_deviation(&self) -> f64 {
        self.std_deviation
    }
    
    /// Генерировать сигнал
    pub fn generate_signal(&self, price: f64) -> StandardDeviationSignal {
        if !self.is_ready() {
            return StandardDeviationSignal::WithinBands;
        }
        
        if price > self.upper_band_2 {
            StandardDeviationSignal::StrongBreakoutUp
        } else if price > self.upper_band_1 {
            StandardDeviationSignal::BreakoutUp
        } else if price < self.lower_band_2 {
            StandardDeviationSignal::StrongBreakoutDown
        } else if price < self.lower_band_1 {
            StandardDeviationSignal::BreakoutDown
        } else {
            let regression_value = self.regression_ma.line();
            let distance_to_regression = (price - regression_value).abs();
            let std_distance = distance_to_regression / self.std_deviation;
            
            if std_distance < 0.5 {
                StandardDeviationSignal::MeanReversion
            } else {
                StandardDeviationSignal::WithinBands
            }
        }
    }
    
    /// Проверить пробой уровня
    pub fn is_breakout(&self, price: f64, sigma_level: f64) -> Option<bool> {
        if !self.is_ready() {
            return None;
        }

        let regression_value = self.regression_ma.line();
        let threshold = regression_value + sigma_level * self.std_deviation;

        if price > threshold {
            Some(true)  // Пробой вверх
        } else if price < (regression_value - sigma_level * self.std_deviation) {
            Some(false) // Пробой вниз
        } else {
            None // Нет пробоя
        }
    }
    
    /// Проверить сигнал возврата к среднему
    pub fn is_mean_reversion_signal(&self, price: f64, prev_price: f64) -> bool {
        if !self.is_ready() {
            return false;
        }

        let regression_value = self.regression_ma.line();

        // Цена движется к регрессионной линии
        let prev_distance = (prev_price - regression_value).abs();
        let current_distance = (price - regression_value).abs();

        current_distance < prev_distance && current_distance < self.std_deviation
    }
    
    /// Получить направление тренда (на основе slope)
    pub fn trend_direction(&self) -> i8 {
        let slope = self.regression_ma.slope();
        
        if slope > 0.001 {
            1  // Восходящий тренд
        } else if slope < -0.001 {
            -1 // Нисходящий тренд
        } else {
            0  // Боковое движение
        }
    }
    
    /// Получить силу тренда (R²)
    pub fn trend_strength(&self) -> f64 {
        self.regression_ma.r2()
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.regression_ma.is_ready() && self.buffer_filled
    }
    
    /// Получить параметры
    pub fn get_params(&self) -> (usize, f64, StandardDeviationMode, RegressionSource) {
        (self.period, self.std_multiplier, self.mode, self.source)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.regression_ma.reset();
        self.price_buffer.clear();
        self.price_index = 0;
        self.buffer_filled = false;
        self.std_deviation = 0.0;
        self.upper_band_1 = 0.0;
        self.upper_band_2 = 0.0;
        self.upper_band_3 = 0.0;
        self.lower_band_1 = 0.0;
        self.lower_band_2 = 0.0;
        self.lower_band_3 = 0.0;
        self.volatility_factor = 1.0;
        self.adaptive_multiplier = 1.0;
        self.bar_count = 0;
    }

}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`StandardDeviationChannels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StandardDeviationChannelsConfig {
    pub period: Param<usize>,
    pub std_multiplier: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for StandardDeviationChannels {
    const ID: IndicatorId = IndicatorId::Stddevchan;
    /// Channel family — regression-based standard-deviation banded channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period) per bar: price buffer rescan. The embedded LR charges via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)], // price_buffer
        inner: &[Port::new(IndicatorId::Lr, &[IndicatorOutputId::LrLine])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::StddevchanUpper),
        Output::price(IndicatorOutputId::StddevchanMiddle),
        Output::price(IndicatorOutputId::StddevchanLower),
    ];
    type Config = StandardDeviationChannelsConfig;
    type Runtime = StandardDeviationChannels;

    fn create(cfg: StandardDeviationChannelsConfig) -> StandardDeviationChannels {
        StandardDeviationChannels::with_source(
            cfg.period.resolved().max(2),
            cfg.std_multiplier.resolved().max(0.1),
            StandardDeviationMode::Simple,
            RegressionSource::Close,
            cfg.source.resolved(),
        )
    }

    fn source_fields(cfg: &StandardDeviationChannelsConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for StandardDeviationChannelsConfig {
    fn defaults() -> Self {
        StandardDeviationChannelsConfig {
            period: Param::Solo(20),
            std_multiplier: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8
        s.std_multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for StandardDeviationChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::StddevchanUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::StddevchanMiddle, "Middle", Color::hex(0x607D8B), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::StddevchanLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_stddevchan() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StandardDeviationChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Stddevchan(cfg).build_solo().unwrap();
        for i in 1..=30usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 0.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite(), "stddevchan middle should be finite, got {}", f.primary());
    }
}

impl Default for StandardDeviationChannels {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_deviation_channels_creation() {
        let sdc = StandardDeviationChannels::new(20);
        assert!(!sdc.is_ready());
        assert_eq!(sdc.channel_width(), 0.0);
    }

    #[test]
    fn test_standard_deviation_channels_warmup() {
        let mut sdc = StandardDeviationChannels::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sdc.feed(price);
        }
        assert!(sdc.is_ready());
    }

    #[test]
    fn test_standard_deviation_channels_bands() {
        let mut sdc = StandardDeviationChannels::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            sdc.feed(price);
        }
        let (u3, u2, u1, mid, l1, l2, l3) = sdc.all_bands();
        assert!(u3 >= u2);
        assert!(u2 >= u1);
        assert!(u1 >= mid);
        assert!(mid >= l1);
        assert!(l1 >= l2);
        assert!(l2 >= l3);
    }

    #[test]
    fn test_standard_deviation_channels_adaptive() {
        let mut sdc = StandardDeviationChannels::new_adaptive(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            sdc.feed(price);
        }
        assert!(sdc.is_ready());
        assert!(sdc.standard_deviation() > 0.0);
    }

    #[test]
    fn test_standard_deviation_channels_reset() {
        let mut sdc = StandardDeviationChannels::new(20);
        for i in 0..25 {
            sdc.feed(100.0 + i as f64);
        }
        sdc.reset();
        assert!(!sdc.is_ready());
        assert_eq!(sdc.channel_width(), 0.0);
    }
} 

