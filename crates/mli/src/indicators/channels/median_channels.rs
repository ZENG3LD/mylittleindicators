//! median_channels.rs: High-Performance Median Channels
//! Каналы на основе медианных значений - устойчивые к выбросам
//!
//! Особенности:
//! - Median Absolute Deviation (MAD) для ширины каналов
//! - Устойчивость к ценовым выбросам
//! - Робастная статистика вместо mean/std
//! - Multiple quantile levels (25%, 75%, 10%, 90%)

use crate::indicators::utils::math::percentile::{median, quickselect_nth};

/// Режимы расчета медианных каналов
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MedianMode {
    /// Простые медианные каналы (медиана ± MAD)
    Simple,
    /// Квантильные каналы (25%-75% квантили)
    Quantile,
    /// Адаптивные медианные каналы (меняющиеся квантили)
    Adaptive,
    /// Множественные уровни (10%, 25%, 75%, 90%)
    MultiLevel,
}

/// Источник данных для медианы
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MedianSource {
    /// Цена закрытия
    Close,
    /// Типичная цена (H+L+C)/3
    Typical,
    /// Медиана OHLC (медиана от O,H,L,C)
    OhlcMedian,
    /// Weighted median (объемно-взвешенная медиана)
    VolumeWeighted,
    /// True median (медиана от всех цен в баре)
    TrueMedian,
}

/// Сигналы медианных каналов
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MedianSignal {
    /// Пробой верхнего MAD уровня
    BreakoutUpperMAD,
    /// Пробой нижнего MAD уровня
    BreakdownLowerMAD,
    /// Возврат к медиане
    ReturnToMedian,
    /// Пробой 75% квантиля
    BreakoutQ75,
    /// Пробой 25% квантиля
    BreakdownQ25,
    /// Достижение экстремального уровня (90%/10%)
    ExtremeLevel,
    /// Сжатие медианного канала
    MedianSqueeze,
    /// Расширение медианного канала
    MedianExpansion,
}

/// Квантильные уровни
#[derive(Debug, Clone, Copy)]
pub struct QuantileLevels {
    pub q10: f64,   // 10% квантиль
    pub q25: f64,   // 25% квантиль (нижний квартиль)
    pub q50: f64,   // 50% квантиль (медиана)
    pub q75: f64,   // 75% квантиль (верхний квартиль)
    pub q90: f64,   // 90% квантиль
}

/// High-Performance Median Channels
#[derive(Debug, Clone)]
pub struct MedianChannels {
    // Параметры
    period: usize,
    mode: MedianMode,
    source: MedianSource,
    mad_multiplier: f64,
    
    // Circular buffer для цен - O(1) operations
    price_buffer: Vec<f64>,
    
    price_index: usize,
    buffer_filled: bool,
    
    // Отсортированные данные для квантилей
    sorted_prices: Vec<f64>,
    
    // Медианные значения
    median: f64,
    mad: f64,  // Median Absolute Deviation
    
    // Квантильные уровни
    quantile_levels: QuantileLevels,
    
    // Каналы
    upper_mad: f64,      // Медиана + MAD
    lower_mad: f64,      // Медиана - MAD
    upper_mad_2: f64,    // Медиана + 2*MAD
    lower_mad_2: f64,    // Медиана - 2*MAD
    upper_mad_3: f64,    // Медиана + 3*MAD
    lower_mad_3: f64,    // Медиана - 3*MAD
    
    // Интерквартильный размах
    iqr: f64,           // Q75 - Q25
    channel_width: f64,
    
    // Адаптивные параметры
    volatility_adj: f64,
    adaptive_quantiles: (f64, f64), // Адаптивные границы
    
    // Робастная статистика
    trimmed_mean: f64,
    winsorized_std: f64,
    
    // Детекция выбросов
    outlier_threshold: f64,
    outlier_count: usize,
    
    // Статистика
    bar_count: usize,
}

impl MedianChannels {
    /// Создать медианные каналы со стандартными параметрами
    pub fn new(period: usize) -> Self {
        Self::new_custom(
            period,
            MedianMode::Simple,
            MedianSource::Close,
            1.4826  // Константа для приведения MAD к std
        )
    }
    
    /// Создать медианные каналы с кастомными параметрами
    pub fn new_custom(
        period: usize,
        mode: MedianMode,
        source: MedianSource,
        mad_multiplier: f64
    ) -> Self {
        assert!(period >= 2, "Period must be >= 2");
        assert!(mad_multiplier > 0.0);

        Self {
            period,
            mode,
            source,
            mad_multiplier,

            price_buffer: Vec::with_capacity(period),

            price_index: 0,
            buffer_filled: false,
            sorted_prices: Vec::with_capacity(period),

            median: 0.0,
            mad: 0.0,
            quantile_levels: QuantileLevels {
                q10: 0.0, q25: 0.0, q50: 0.0, q75: 0.0, q90: 0.0
            },

            upper_mad: 0.0,
            lower_mad: 0.0,
            upper_mad_2: 0.0,
            lower_mad_2: 0.0,
            upper_mad_3: 0.0,
            lower_mad_3: 0.0,

            iqr: 0.0,
            channel_width: 0.0,

            volatility_adj: 1.0,
            adaptive_quantiles: (0.25, 0.75),

            trimmed_mean: 0.0,
            winsorized_std: 0.0,

            outlier_threshold: 2.0,
            outlier_count: 0,

            bar_count: 0,
        }
    }

    /// Создать медианные каналы с настраиваемым источником данных (MedianSource).
    pub fn with_source(
        period: usize,
        mode: MedianMode,
        source: MedianSource,
        mad_multiplier: f64,
    ) -> Self {
        Self::new_custom(period, mode, source, mad_multiplier)
    }
    
    /// Создать квантильные каналы
    pub fn new_quantile(period: usize) -> Self {
        Self::new_custom(
            period,
            MedianMode::Quantile,
            MedianSource::Typical,
            1.4826
        )
    }
    
    /// Создать адаптивные медианные каналы
    pub fn new_adaptive(period: usize) -> Self {
        Self::new_custom(
            period,
            MedianMode::Adaptive,
            MedianSource::Close,
            1.4826
        )
    }
    
    /// Создать множественные уровни
    pub fn new_multilevel(period: usize) -> Self {
        Self::new_custom(
            period,
            MedianMode::MultiLevel,
            MedianSource::Typical,
            1.4826
        )
    }
    
    /// Обновить буферы
    fn update_buffers(&mut self, price: f64) {
        if self.buffer_filled {
            self.price_buffer[self.price_index] = price;
        } else {
            self.price_buffer.push(price);
        }
        self.price_index = (self.price_index + 1) % self.period;
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }
    }
    
    /// Рассчитать медиану и квантили
    fn calculate_median_and_quantiles(&mut self) {
        // 🚀 O(n) quickselect instead of O(n log n) sorting
        self.sorted_prices.clear();
        self.sorted_prices.extend_from_slice(&self.price_buffer[..]);

        let n = self.sorted_prices.len();

        // Рассчитываем квантили с quickselect
        self.quantile_levels.q10 = quickselect_nth(&mut self.sorted_prices, (n * 10) / 100);
        self.quantile_levels.q25 = quickselect_nth(&mut self.sorted_prices, n / 4);
        self.quantile_levels.q50 = quickselect_nth(&mut self.sorted_prices, n / 2);
        self.quantile_levels.q75 = quickselect_nth(&mut self.sorted_prices, (3 * n) / 4);
        self.quantile_levels.q90 = quickselect_nth(&mut self.sorted_prices, (n * 90) / 100);

        // Медиана
        self.median = self.quantile_levels.q50;

        // Интерквартильный размах
        self.iqr = self.quantile_levels.q75 - self.quantile_levels.q25;

        // Робастные статистики
        self.calculate_robust_statistics();
    }
    
    /// Рассчитать квантиль
    fn calculate_quantile(&self, p: f64) -> f64 {
        let n = self.sorted_prices.len() as f64;
        let index = p * (n - 1.0);
        
        if index.fract() == 0.0 {
            self.sorted_prices[index as usize]
        } else {
            let lower_index = index.floor() as usize;
            let upper_index = index.ceil() as usize;
            let fraction = index.fract();
            
            if upper_index < self.sorted_prices.len() {
                self.sorted_prices[lower_index] * (1.0 - fraction) + 
                self.sorted_prices[upper_index] * fraction
            } else {
                self.sorted_prices[lower_index]
            }
        }
    }
    
    /// Рассчитать робастные статистики
    fn calculate_robust_statistics(&mut self) {
        let n = self.sorted_prices.len();
        
        // Trimmed mean (отбрасываем 10% с каждого конца)
        let trim_count = (n as f64 * 0.1) as usize;
        if trim_count > 0 && trim_count * 2 < n {
            let trimmed_sum: f64 = self.sorted_prices[trim_count..n-trim_count].iter().sum();
            self.trimmed_mean = trimmed_sum / (n - 2 * trim_count) as f64;
        } else {
            self.trimmed_mean = self.median;
        }
        
        // Winsorized standard deviation
        let mut winsorized_prices = self.sorted_prices.clone();
        let win_count = (n as f64 * 0.05) as usize; // 5% с каждого конца
        
        if win_count > 0 {
            let lower_bound = winsorized_prices[win_count];
            let upper_bound = winsorized_prices[n - win_count - 1];
            
            for i in 0..win_count {
                winsorized_prices[i] = lower_bound;
                winsorized_prices[n - i - 1] = upper_bound;
            }
        }
        
        let win_mean = winsorized_prices.iter().sum::<f64>() / n as f64;
        let win_variance = winsorized_prices.iter()
            .map(|&x| (x - win_mean).powi(2))
            .sum::<f64>() / n as f64;
        self.winsorized_std = win_variance.sqrt();
    }
    
    /// Рассчитать Median Absolute Deviation
    fn calculate_mad(&mut self) {
        // 🚀 O(n) median function instead of O(n log n) sorting
        let mut deviations: Vec<f64> = self.price_buffer.iter()
            .map(|&price| (price - self.median).abs())
            .collect();

        self.mad = median(&mut deviations);

        // Применяем множитель для приведения к стандартному отклонению
        self.mad *= self.mad_multiplier;
    }
    
    /// Обновить адаптивные параметры
    fn update_adaptive_parameters(&mut self) {
        if matches!(self.mode, MedianMode::Adaptive) {
            // Адаптируем квантили на основе волатильности
            let volatility_ratio = if self.iqr > 0.0 {
                self.mad / self.iqr
            } else {
                1.0
            };
            
            // Расширяем квантили в волатильные периоды
            if volatility_ratio > 1.5 {
                self.adaptive_quantiles = (0.15, 0.85); // Более широкие границы
            } else if volatility_ratio < 0.8 {
                self.adaptive_quantiles = (0.35, 0.65); // Более узкие границы
            } else {
                self.adaptive_quantiles = (0.25, 0.75); // Стандартные квартили
            }
            
            self.volatility_adj = volatility_ratio.clamp(0.5, 2.0);
        }
    }
    
    /// Рассчитать медианные каналы
    fn calculate_median_channels(&mut self) {
        match self.mode {
            MedianMode::Simple => {
                self.upper_mad = self.median + self.mad;
                self.lower_mad = self.median - self.mad;
                self.upper_mad_2 = self.median + 2.0 * self.mad;
                self.lower_mad_2 = self.median - 2.0 * self.mad;
                self.upper_mad_3 = self.median + 3.0 * self.mad;
                self.lower_mad_3 = self.median - 3.0 * self.mad;
            }
            
            MedianMode::Quantile | MedianMode::MultiLevel => {
                self.upper_mad = self.quantile_levels.q75;
                self.lower_mad = self.quantile_levels.q25;
                self.upper_mad_2 = self.quantile_levels.q90;
                self.lower_mad_2 = self.quantile_levels.q10;
                self.upper_mad_3 = self.median + 3.0 * self.iqr;
                self.lower_mad_3 = self.median - 3.0 * self.iqr;
            }
            
            MedianMode::Adaptive => {
                let adaptive_upper = self.calculate_quantile(self.adaptive_quantiles.1);
                let adaptive_lower = self.calculate_quantile(self.adaptive_quantiles.0);
                
                self.upper_mad = adaptive_upper;
                self.lower_mad = adaptive_lower;
                self.upper_mad_2 = self.median + self.mad * self.volatility_adj * 2.0;
                self.lower_mad_2 = self.median - self.mad * self.volatility_adj * 2.0;
                self.upper_mad_3 = self.median + self.mad * self.volatility_adj * 3.0;
                self.lower_mad_3 = self.median - self.mad * self.volatility_adj * 3.0;
            }
        }
        
        self.channel_width = self.upper_mad - self.lower_mad;
    }
    
    /// Детектировать выбросы
    fn detect_outliers(&mut self, current_price: f64) {
        let distance_from_median = (current_price - self.median).abs();
        let outlier_threshold = self.mad * self.outlier_threshold;
        
        if distance_from_median > outlier_threshold {
            self.outlier_count += 1;
        }
    }
    

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper_mad, self.median, self.lower_mad)
    }
    
    /// Получить все уровни MAD
    pub fn all_mad_levels(&self) -> (f64, f64, f64, f64, f64, f64, f64) {
        (
            self.median,
            self.upper_mad,
            self.lower_mad,
            self.upper_mad_2,
            self.lower_mad_2,
            self.upper_mad_3,
            self.lower_mad_3,
        )
    }
    
    /// Получить квантильные уровни
    pub fn quantile_levels(&self) -> QuantileLevels {
        self.quantile_levels
    }
    
    /// Получить медиану
    pub fn median(&self) -> f64 {
        self.median
    }
    
    /// Получить MAD
    pub fn mad(&self) -> f64 {
        self.mad
    }
    
    /// Получить интерквартильный размах
    pub fn iqr(&self) -> f64 {
        self.iqr
    }

    pub fn upper(&self) -> f64 { self.upper_mad }
    pub fn middle(&self) -> f64 { self.median }
    pub fn lower(&self) -> f64 { self.lower_mad }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.channel_width
    }
    
    /// Получить позицию цены в канале
    pub fn position_in_channel(&self, price: f64) -> f64 {
        if self.channel_width > 0.0 {
            (price - self.lower_mad) / self.channel_width
        } else {
            0.5
        }
    }
    
    /// Получить робастные статистики
    pub fn robust_statistics(&self) -> (f64, f64) {
        (self.trimmed_mean, self.winsorized_std)
    }
    
    /// Генерация сигнала
    pub fn generate_signal(&self, current_price: f64, previous_price: f64) -> MedianSignal {
        // Пробои MAD уровней
        if previous_price <= self.upper_mad && current_price > self.upper_mad {
            return MedianSignal::BreakoutUpperMAD;
        }
        if previous_price >= self.lower_mad && current_price < self.lower_mad {
            return MedianSignal::BreakdownLowerMAD;
        }
        
        // Пробои квантилей
        if previous_price <= self.quantile_levels.q75 && current_price > self.quantile_levels.q75 {
            return MedianSignal::BreakoutQ75;
        }
        if previous_price >= self.quantile_levels.q25 && current_price < self.quantile_levels.q25 {
            return MedianSignal::BreakdownQ25;
        }
        
        // Экстремальные уровни
        if current_price > self.quantile_levels.q90 || current_price < self.quantile_levels.q10 {
            return MedianSignal::ExtremeLevel;
        }
        
        // Возврат к медиане
        let distance_to_median = (current_price - self.median).abs();
        let prev_distance_to_median = (previous_price - self.median).abs();
        
        if distance_to_median < prev_distance_to_median && distance_to_median < self.mad * 0.5 {
            return MedianSignal::ReturnToMedian;
        }
        
        // Сжатие/расширение канала (сравниваем с предыдущими значениями)
        MedianSignal::ReturnToMedian
    }
    
    /// Проверить, является ли цена выбросом
    pub fn is_outlier(&self, price: f64) -> bool {
        let distance = (price - self.median).abs();
        distance > self.mad * self.outlier_threshold
    }
    
    /// Получить количество выбросов
    pub fn outlier_count(&self) -> usize {
        self.outlier_count
    }
    
    /// Получить процент выбросов
    pub fn outlier_percentage(&self) -> f64 {
        if self.bar_count > 0 {
            (self.outlier_count as f64 / self.bar_count as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.buffer_filled
    }
    
    /// Получить параметры
    pub fn get_params(&self) -> (usize, MedianMode, MedianSource, f64) {
        (self.period, self.mode, self.source, self.mad_multiplier)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.price_buffer.clear();
        
        self.price_index = 0;
        self.buffer_filled = false;
        self.sorted_prices.clear();
        
        self.median = 0.0;
        self.mad = 0.0;
        self.quantile_levels = QuantileLevels {
            q10: 0.0, q25: 0.0, q50: 0.0, q75: 0.0, q90: 0.0
        };
        
        self.upper_mad = 0.0;
        self.lower_mad = 0.0;
        self.upper_mad_2 = 0.0;
        self.lower_mad_2 = 0.0;
        self.upper_mad_3 = 0.0;
        self.lower_mad_3 = 0.0;
        
        self.iqr = 0.0;
        self.channel_width = 0.0;
        
        self.volatility_adj = 1.0;
        self.adaptive_quantiles = (0.25, 0.75);
        
        self.trimmed_mean = 0.0;
        self.winsorized_std = 0.0;
        
        self.outlier_count = 0;
        self.bar_count = 0;
    }

    /// Transitional bridge for un-contracted embedders that call the OHLCV form.
    // transitional bridge for un-contracted embedders; remove when they convert
    #[inline]

    /// Feed one pre-extracted scalar price — the contract feed path.
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        self.bar_count += 1;
        self.update_buffers(price);
        if self.buffer_filled {
            self.calculate_median_and_quantiles();
            self.calculate_mad();
            self.update_adaptive_parameters();
            self.calculate_median_channels();
            self.detect_outliers(price);
        }
        (self.upper_mad, self.median, self.lower_mad)
    }
}

impl Default for MedianChannels {
    fn default() -> Self {
        Self::new(20)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`MedianChannels`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MedchanConfig {
    pub period: Param<usize>,
}

impl Indicator for MedianChannels {
    const ID: IndicatorId = IndicatorId::Medchan;
    /// Channel family — a robust median/MAD banded channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::Field { default: crate::engine::ohlcv_field::OhlcvField::Close });
    /// O(period) per bar: quickselect rescan of the price buffer. One period-deep Vec store.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::MedchanUpper),
        Output::price(IndicatorOutputId::MedchanMiddle),
        Output::price(IndicatorOutputId::MedchanLower),
    ];
    type Config = MedchanConfig;
    type Runtime = MedianChannels;

    fn create(cfg: MedchanConfig) -> MedianChannels {
        MedianChannels::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &MedchanConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        let _ = cfg;
        let mut out = arrayvec::ArrayVec::new();
        let _ = out.try_push(crate::engine::ohlcv_field::OhlcvField::Close);
        out
    }
}

impl crate::contract::Config for MedchanConfig {
    fn defaults() -> Self {
        MedchanConfig { period: Param::Solo(20) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // A1 2026-07-04: floor from valid_params (runtime assert `period >= 2` in
        // `MedianChannels::new_custom` — a period-1 median/MAD window is undefined).
        let mut s = Self::machine_defaults_auto(); // period→range(1,10000,1) — only axis
        s.period = Param::range(2, 10000, 1);
        s
    }
}


impl Render for MedianChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::MedchanUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::MedchanMiddle, "Middle", Color::hex(0x607D8B), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::MedchanLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_channels_creation() {
        let mc = MedianChannels::new(20);
        assert!(!mc.is_ready());
        assert_eq!(mc.median(), 0.0);
    }

    #[test]
    fn test_median_channels_warmup() {
        let mut mc = MedianChannels::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mc.feed(price);
        }
        assert!(mc.is_ready());
    }

    #[test]
    fn test_median_channels_values() {
        let mut mc = MedianChannels::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = mc.feed(price);
            if mc.is_ready() {
                assert!(upper >= middle);
                assert!(middle >= lower);
            }
        }
    }

    #[test]
    fn test_median_channels_reset() {
        let mut mc = MedianChannels::new(20);
        for i in 0..25 {
            mc.feed(100.0 + i as f64);
        }
        mc.reset();
        assert!(!mc.is_ready());
        assert_eq!(mc.median(), 0.0);
    }

    /// Factory resolves configurable source (default close); wild high/low are ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<MedianChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Medchan(cfg).build_solo().unwrap();
        for i in 0..25usize {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite(), "upper should be finite, got {}", f.primary());
    }
} 






















