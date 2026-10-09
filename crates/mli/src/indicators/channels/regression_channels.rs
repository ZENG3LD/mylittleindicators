//! regression_channels.rs: High-Performance Regression Channels
//! Каналы линейной регрессии - статистически обоснованные каналы тренда
//!
//! Особенности:
//! - LinearRegressionMA как средняя линия (трендовая линия)
//! - Стандартные отклонения от регрессионной линии как границы
//! - Slope индикация направления тренда
//! - R² для оценки качества аппроксимации

use crate::indicators::average::lr::LinearRegressionMA;
use crate::engine::ohlcv_field::OhlcvField;

/// Режимы расчета Regression Channels
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum RegressionChannelMode {
    /// Standard - стандартные отклонения от регрессионной линии
    #[default]
    Standard,
    /// Percentage - процентные отклонения от регрессионной линии
    Percentage,
    /// R2Weighted - отклонения взвешенные по R² (чем лучше фит, тем уже канал)
    R2Weighted,
}

/// High-Performance Regression Channels
#[derive(Debug, Clone)]
pub struct RegressionChannels {
    period: usize,
    std_dev_mult: f64,
    mode: RegressionChannelMode,

    // Linear Regression MA для трендовой линии
    lr: LinearRegressionMA,

    // Circular buffer для расчета отклонений от регрессии
    price_buffer: Vec<f64>,
    residuals_buffer: Vec<f64>, // Отклонения от регрессионной линии
    buffer_index: usize,
    buffer_filled: bool,

    // Текущие значения канала
    upper: f64,
    middle: f64, // Regression line
    lower: f64,

    // Регрессионные метрики
    slope: f64,
    intercept: f64,
    r_squared: f64,
    std_dev: f64,

    // Направление тренда
    trend_direction: TrendDirection,
}

/// Направление тренда на основе slope
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    Uptrend,
    Downtrend,
    Sideways,
}

impl RegressionChannels {
    /// Создать Regression Channels с указанными параметрами
    pub fn new(period: usize, std_dev_mult: f64, mode: RegressionChannelMode) -> Self {
        assert!(period > 1, "Period must be >= 2");
        assert!(std_dev_mult > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_dev_mult,
            mode,
            lr: LinearRegressionMA::new(period),
            price_buffer: Vec::with_capacity(period),
            residuals_buffer: Vec::with_capacity(period),
            buffer_index: 0,
            buffer_filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            slope: 0.0,
            intercept: 0.0,
            r_squared: 0.0,
            std_dev: 0.0,
            trend_direction: TrendDirection::Sideways,
        }
    }

    /// Создать Regression Channels с настраиваемым источником данных
    pub fn with_source(period: usize, std_dev_mult: f64, mode: RegressionChannelMode, _source: OhlcvField) -> Self {
        Self::new(period, std_dev_mult, mode)
    }
    
    /// Создать стандартные Regression Channels
    pub fn new_standard(period: usize, std_dev_mult: f64) -> Self {
        Self::new(period, std_dev_mult, RegressionChannelMode::Standard)
    }
    
    /// Создать процентные Regression Channels
    pub fn new_percentage(period: usize, percentage: f64) -> Self {
        Self::new(period, percentage, RegressionChannelMode::Percentage)
    }
    
    /// Создать R² взвешенные Regression Channels
    pub fn new_r2_weighted(period: usize, base_mult: f64) -> Self {
        Self::new(period, base_mult, RegressionChannelMode::R2Weighted)
    }
    
    /// Feed a scalar — the resolved price (const SOURCE = Field{Close}).
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        // Обновляем Linear Regression MA через close-only путь
        self.middle = self.lr.feed(price);

        // Добавляем цену в буфер
        if self.buffer_filled {
            self.price_buffer[self.buffer_index] = price;
        } else {
            self.price_buffer.push(price);
        }

        // Обновляем индекс циклически
        self.buffer_index = (self.buffer_index + 1) % self.period;

        // Проверяем, заполнен ли буфер
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }

        // Рассчитываем каналы если готово
        if self.is_ready() {
            self.update_regression_metrics();
            self.calculate_channels();
        } else {
            self.reset_channels();
        }

        (self.upper, self.middle, self.lower)
    }
    
    /// Обновить метрики регрессии
    fn update_regression_metrics(&mut self) {
        // Получаем метрики из LinearRegressionMA
        self.slope = self.lr.slope();
        self.intercept = self.lr.intercept();
        self.r_squared = self.lr.r2();
        
        // Определяем направление тренда
        self.trend_direction = if self.slope > 0.001 {
            TrendDirection::Uptrend
        } else if self.slope < -0.001 {
            TrendDirection::Downtrend
        } else {
            TrendDirection::Sideways
        };
        
        // Рассчитываем residuals (отклонения от регрессионной линии)
        self.calculate_residuals();
    }
    
    /// Рассчитать residuals (отклонения от регрессионной линии)
    fn calculate_residuals(&mut self) {
        self.residuals_buffer.clear();
        
        let buffer_len = if self.buffer_filled { self.period } else { self.price_buffer.len() };
        
        for i in 0..buffer_len {
            let x = (i + 1) as f64; // X координата (1, 2, 3, ...)
            let actual_price = self.price_buffer[i];
            let predicted_price = self.slope * x + self.intercept;
            let residual = actual_price - predicted_price;
            
            if self.residuals_buffer.len() < 512 {
                self.residuals_buffer.push(residual);
            }
        }
        
        // Рассчитываем стандартное отклонение residuals
        if !self.residuals_buffer.is_empty() {
            let mean_residual = self.residuals_buffer.iter().sum::<f64>() / self.residuals_buffer.len() as f64;
            let variance = self.residuals_buffer.iter()
                .map(|&residual| (residual - mean_residual).powi(2))
                .sum::<f64>() / self.residuals_buffer.len() as f64;
            self.std_dev = variance.sqrt();
        }
    }
    
    /// Рассчитать границы каналов
    fn calculate_channels(&mut self) {
        match self.mode {
            RegressionChannelMode::Standard => {
                self.calculate_standard_channels();
            }
            RegressionChannelMode::Percentage => {
                self.calculate_percentage_channels();
            }
            RegressionChannelMode::R2Weighted => {
                self.calculate_r2_weighted_channels();
            }
        }
    }
    
    /// Рассчитать стандартные каналы
    fn calculate_standard_channels(&mut self) {
        self.upper = self.middle + self.std_dev_mult * self.std_dev;
        self.lower = self.middle - self.std_dev_mult * self.std_dev;
    }
    
    /// Рассчитать процентные каналы
    fn calculate_percentage_channels(&mut self) {
        let percentage_band = self.middle * (self.std_dev_mult / 100.0);
        self.upper = self.middle + percentage_band;
        self.lower = self.middle - percentage_band;
    }
    
    /// Рассчитать R² взвешенные каналы
    fn calculate_r2_weighted_channels(&mut self) {
        // Чем выше R², тем уже канал (лучший фит = меньше неопределенности)
        let r2_weight = if self.r_squared > 0.0 {
            1.0 - self.r_squared // Инвертируем R² (высокий R² = узкий канал)
        } else {
            1.0
        };
        
        let weighted_std_dev = self.std_dev * (0.5 + r2_weight); // Минимум 0.5x, максимум 1.5x
        self.upper = self.middle + self.std_dev_mult * weighted_std_dev;
        self.lower = self.middle - self.std_dev_mult * weighted_std_dev;
    }
    
    /// Сбросить значения каналов
    fn reset_channels(&mut self) {
        self.upper = 0.0;
        self.lower = 0.0;
        self.slope = 0.0;
        self.intercept = 0.0;
        self.r_squared = 0.0;
        self.std_dev = 0.0;
        self.trend_direction = TrendDirection::Sideways;
    }
    

    /// Получить текущие значения канала как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить регрессионную линию (средняя линия)
    pub fn regression_line(&self) -> f64 {
        self.middle
    }

    pub fn middle(&self) -> f64 { self.middle }

    /// Получить верхнюю границу канала
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Получить нижнюю границу канала
    pub fn lower(&self) -> f64 {
        self.lower
    }
    
    /// Получить slope регрессии (наклон тренда)
    pub fn slope(&self) -> f64 {
        self.slope
    }
    
    /// Получить intercept регрессии
    pub fn intercept(&self) -> f64 {
        self.intercept
    }
    
    /// Получить R² (качество аппроксимации)
    pub fn r_squared(&self) -> f64 {
        self.r_squared
    }
    
    /// Получить стандартное отклонение residuals
    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }
    
    /// Получить направление тренда
    pub fn trend_direction(&self) -> TrendDirection {
        self.trend_direction
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        if self.is_ready() {
            self.upper - self.lower
        } else {
            0.0
        }
    }
    
    /// Получить позицию цены в канале (0.0 = нижняя граница, 1.0 = верхняя граница)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        if !self.is_ready() || self.upper == self.lower {
            0.5 // По центру если канал не готов или нулевой ширины
        } else {
            ((price - self.lower) / (self.upper - self.lower)).clamp(0.0, 1.0)
        }
    }
    
    /// Проверить качество регрессии (R² > threshold)
    pub fn is_good_fit(&self, r2_threshold: f64) -> bool {
        self.is_ready() && self.r_squared >= r2_threshold
    }
    
    /// Проверить сильный тренд (высокий R² + значительный slope)
    pub fn is_strong_trend(&self, r2_threshold: f64, slope_threshold: f64) -> bool {
        self.is_good_fit(r2_threshold) && self.slope.abs() >= slope_threshold
    }
    
    /// Предсказать цену на N баров вперед (экстраполяция)
    pub fn predict_price(&self, bars_ahead: usize) -> Option<f64> {
        if !self.is_ready() {
            return None;
        }
        
        let x = (self.period + bars_ahead) as f64;
        Some(self.slope * x + self.intercept)
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.lr.is_ready() && self.buffer_filled
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.lr.reset();
        self.price_buffer.clear();
        self.residuals_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;
        self.reset_channels();
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить множитель стандартного отклонения
    pub fn std_dev_mult(&self) -> f64 {
        self.std_dev_mult
    }
    
    /// Получить режим расчета
    pub fn mode(&self) -> RegressionChannelMode {
        self.mode
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

/// Typed dual-mode config for [`RegressionChannels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RegressionChannelsConfig {
    pub period: Param<usize>,
    pub std_dev_mult: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for RegressionChannels {
    const ID: IndicatorId = IndicatorId::Regchan;
    /// Channel family — regression-based banded channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period) per bar: residuals rescan + price buffer (two Vec windows). The embedded LR
    /// charges its own O(period) cost via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[
            Store::window(StoreKind::Vec), // price_buffer
            Store::window(StoreKind::Vec), // residuals_buffer
        ],
        inner: &[Port::new(IndicatorId::Lr, &[IndicatorOutputId::LrLine])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::RegchanUpper),
        Output::price(IndicatorOutputId::RegchanMiddle),
        Output::price(IndicatorOutputId::RegchanLower),
    ];
    type Config = RegressionChannelsConfig;
    type Runtime = RegressionChannels;

    fn create(cfg: RegressionChannelsConfig) -> RegressionChannels {
        RegressionChannels::with_source(
            cfg.period.resolved().max(2),
            cfg.std_dev_mult.resolved().max(0.1),
            RegressionChannelMode::Standard,
            cfg.source.resolved(),
        )
    }

    fn source_fields(cfg: &RegressionChannelsConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for RegressionChannelsConfig {
    fn defaults() -> Self {
        RegressionChannelsConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8
        s.std_dev_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for RegressionChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::RegchanUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::RegchanMiddle, "Middle", Color::hex(0x607D8B), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::RegchanLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_regchan() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<RegressionChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Regchan(cfg).build_solo().unwrap();
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
        assert!(f.primary().is_finite(), "regchan middle should be finite, got {}", f.primary());
    }
}

impl Default for RegressionChannels {
    fn default() -> Self {
        Self::new_standard(20, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_channels_creation() {
        let rc = RegressionChannels::new_standard(20, 2.0);
        assert!(!rc.is_ready());
        assert_eq!(rc.upper(), 0.0);
        assert_eq!(rc.lower(), 0.0);
    }

    #[test]
    fn test_regression_channels_warmup() {
        let mut rc = RegressionChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rc.feed(price);
        }
        assert!(rc.is_ready());
    }

    #[test]
    fn test_regression_channels_values() {
        let mut rc = RegressionChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            rc.feed(price);
        }
        assert!(rc.upper() >= rc.regression_line());
        assert!(rc.regression_line() >= rc.lower());
    }

    #[test]
    fn test_regression_channels_trend() {
        let mut rc = RegressionChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64 * 2.0;
            rc.feed(price);
        }
        assert!(rc.slope() > 0.0);
        assert_eq!(rc.trend_direction(), TrendDirection::Uptrend);
    }

    #[test]
    fn test_regression_channels_r2() {
        let mut rc = RegressionChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            rc.feed(price);
        }
        assert!(rc.r_squared() >= 0.0 && rc.r_squared() <= 1.0);
    }

    #[test]
    fn test_regression_channels_reset() {
        let mut rc = RegressionChannels::new_standard(20, 2.0);
        for i in 0..25 {
            rc.feed(100.0 + i as f64);
        }
        rc.reset();
        assert!(!rc.is_ready());
        assert_eq!(rc.upper(), 0.0);
        assert_eq!(rc.lower(), 0.0);
    }
} 

