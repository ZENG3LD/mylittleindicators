//! Adaptive Bollinger Bands - адаптивные полосы Боллинджера
//!
//! Улучшенная версия классических полос Боллинджера, где период и множитель
//! автоматически адаптируются к текущим рыночным условиям на основе
//! волатильности (ATR) и momentum.

use crate::engine::contract_engine::SmootherSlot;
use crate::engine::contract_engine::SmootherId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::indicators::volatility::atr::Atr;

/// Результат Adaptive Bollinger Bands
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBollingerBandsResult {
    pub upper_band: f64,         // Верхняя полоса
    pub middle_band: f64,        // Средняя линия (адаптивная MA)
    pub lower_band: f64,         // Нижняя полоса
    pub bandwidth: f64,          // Ширина канала (upper - lower)
    pub percent_b: f64,          // %B - позиция цены в канале (0-1)
    pub squeeze_ratio: f64,      // Коэффициент сжатия (0-1, где 0 = максимальное сжатие)
    pub adaptive_period: f64,    // Текущий адаптивный период
    pub adaptive_multiplier: f64, // Текущий адаптивный множитель
    pub market_regime: i8,       // Режим рынка: 1 (тренд), 0 (флэт), -1 (волатильность)
}

impl AdaptiveBollingerBandsResult {
    pub fn empty() -> Self {
        Self {
            upper_band: 0.0,
            middle_band: 0.0,
            lower_band: 0.0,
            bandwidth: 0.0,
            percent_b: 0.5,
            squeeze_ratio: 0.5,
            adaptive_period: 20.0,
            adaptive_multiplier: 2.0,
            market_regime: 0,
        }
    }

    /// Определить позицию цены относительно полос
    pub fn price_position(&self, price: f64) -> &'static str {
        if price > self.upper_band {
            "Выше верхней полосы"
        } else if price < self.lower_band {
            "Ниже нижней полосы"
        } else if self.percent_b > 0.8 {
            "Близко к верхней полосе"
        } else if self.percent_b < 0.2 {
            "Близко к нижней полосе"
        } else {
            "В середине канала"
        }
    }

    /// Получить описание режима рынка
    pub fn market_regime_name(&self) -> &'static str {
        match self.market_regime {
            1 => "Трендовый",
            -1 => "Высокая волатильность",
            _ => "Флэтовый",
        }
    }

    /// Определить состояние сжатия
    pub fn squeeze_state(&self) -> &'static str {
        match self.squeeze_ratio {
            x if x < 0.3 => "Сильное сжатие",
            x if x < 0.5 => "Умеренное сжатие",
            x if x < 0.7 => "Нормальная ширина",
            _ => "Расширение",
        }
    }
}

/// Adaptive Bollinger Bands — адаптивные полосы Боллинджера с автоматической адаптацией.
///
/// PURE core: receives the full OHLCV candle via [`AdaptiveBollingerBands::feed`]
/// (factory flavor `Fields`). The configurable price source is extracted internally
/// via `self.source.extract(...)`. ATR is computed from H/L/C internally.
///
/// The central-line MA type is stored as a cached [`SmootherId`] (the narrow typed
/// shape key) so the slot can be rebuilt when the adaptive period shifts.
#[derive(Debug, Clone)]
pub struct AdaptiveBollingerBands {
    adaptive_ma: SmootherSlot,        // Адаптивная скользящая средняя
    adaptive_ma_period: usize,        // Текущий период adaptive_ma (для update_adaptive_ma)
    /// Cached smoother shape for the adaptive central-line MA.
    /// Stored so `update_adaptive_ma` can rebuild `SmootherSlot::new(self.adaptive_ma_id, new_period)`
    /// when the adaptive period shifts.
    adaptive_ma_id: SmootherId,
    atr: Atr,                         // ATR для анализа волатильности
    atr_ma_id: SmootherId,            // Cached ATR smoother id (kept for `reset` reconstruction)
    volatility_ma: SmootherSlot,      // MA для сглаживания волатильности (SMA, hardcoded)
    bandwidth_ma: SmootherSlot,       // MA для анализа ширины канала (SMA, hardcoded)

    // Буферы для расчетов
    prices: Vec<f64>,
    std_devs: Vec<f64>,     // Стандартные отклонения
    bandwidths: Vec<f64>,   // История ширины канала
    periods: Vec<f64>,      // История адаптивных периодов

    // Параметры адаптации
    base_period: usize,              // Базовый период
    min_period: usize,               // Минимальный период
    max_period: usize,               // Максимальный период
    base_multiplier: f64,            // Базовый множитель
    min_multiplier: f64,             // Минимальный множитель
    max_multiplier: f64,             // Максимальный множитель
    source: OhlcvField,              // Источник данных (Close, HL2, HLC3, etc.)

    // Внутренние периоды вспомогательных MA
    volatility_ma_period: usize,
    bandwidth_ma_period: usize,

    // Текущие адаптивные параметры
    current_period: f64,
    current_multiplier: f64,

    // Результат
    current_result: AdaptiveBollingerBandsResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl Default for AdaptiveBollingerBands {
    fn default() -> Self {
        Self::from_base_params(20, 2.0)
    }
}

impl AdaptiveBollingerBands {
    /// Создать новые Adaptive Bollinger Bands с параметрами по умолчанию
    pub fn new() -> Self {
        Self::from_base_params(20, 2.0)
    }

    /// Создать из базовых параметров с автоматическим вычислением min/max диапазонов.
    ///
    /// Это режим "auto" - min/max вычисляются автоматически:
    /// - min_period = base_period / 2 (минимум 5)
    /// - max_period = base_period * 2
    /// - min_multiplier = base_multiplier / 2 (минимум 0.5)
    /// - max_multiplier = base_multiplier * 1.5
    pub fn from_base_params(base_period: usize, base_multiplier: f64) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");

        let min_period = (base_period / 2).max(5);
        let max_period = base_period * 2;
        let min_multiplier = (base_multiplier / 2.0).max(0.5);
        let max_multiplier = base_multiplier * 1.5;

        Self::with_parameters_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier
        )
    }

    /// Создать с полной ручной конфигурацией всех 6 параметров.
    pub fn with_parameters(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64
    ) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(min_period > 0 && min_period <= base_period, "Invalid min period");
        assert!(max_period >= base_period, "Invalid max period");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");
        assert!(min_multiplier > 0.0 && min_multiplier <= base_multiplier, "Invalid min multiplier");
        assert!(max_multiplier >= base_multiplier, "Invalid max multiplier");

        Self::with_parameters_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier
        )
    }

    /// Создать с полной ручной конфигурацией + выбором типов MA/ATR и вспомогательных периодов.
    ///
    /// - `adaptive_ma_id`        — smoother shape for the central adaptive line (default EMA)
    /// - `atr_ma_id`             — smoother shape inside ATR (default RMA = Wilder)
    /// - `volatility_ma_period`  — период MA сглаживания волатильности (default 10)
    /// - `bandwidth_ma_period`   — период MA анализа ширины канала (default 20)
    #[allow(clippy::too_many_arguments)]
    pub fn with_full_config(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64,
        adaptive_ma_id: SmootherId,
        atr_ma_id: SmootherId,
        volatility_ma_period: usize,
        bandwidth_ma_period: usize,
    ) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(min_period > 0 && min_period <= base_period, "Invalid min period");
        assert!(max_period >= base_period, "Invalid max period");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");
        assert!(min_multiplier > 0.0 && min_multiplier <= base_multiplier, "Invalid min multiplier");
        assert!(max_multiplier >= base_multiplier, "Invalid max multiplier");
        assert!(volatility_ma_period > 0, "Volatility MA period must be > 0");
        assert!(bandwidth_ma_period > 0, "Bandwidth MA period must be > 0");

        Self::with_full_config_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier,
            adaptive_ma_id, atr_ma_id,
            volatility_ma_period, bandwidth_ma_period,
        )
    }

    fn with_parameters_internal(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64
    ) -> Self {
        Self::with_full_config_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier,
            SmootherId::Ema,
            SmootherId::Rma,
            10,
            20,
        )
    }

    fn with_full_config_internal(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64,
        adaptive_ma_id: SmootherId,
        atr_ma_id: SmootherId,
        volatility_ma_period: usize,
        bandwidth_ma_period: usize,
    ) -> Self {
        Self {
            adaptive_ma: SmootherSlot::new(adaptive_ma_id, base_period),
            adaptive_ma_period: base_period,
            adaptive_ma_id,
            atr: Atr::from_smoother(14, atr_ma_id),
            atr_ma_id,
            volatility_ma: SmootherSlot::new(SmootherId::Sma, volatility_ma_period),
            bandwidth_ma: SmootherSlot::new(SmootherId::Sma, bandwidth_ma_period),

            prices: Vec::with_capacity(64),
            std_devs: Vec::with_capacity(32),
            bandwidths: Vec::with_capacity(32),
            periods: Vec::with_capacity(16),

            base_period,
            min_period,
            max_period,
            base_multiplier,
            min_multiplier,
            max_multiplier,
            source: OhlcvField::Close,

            volatility_ma_period,
            bandwidth_ma_period,

            current_period: base_period as f64,
            current_multiplier: base_multiplier,

            current_result: AdaptiveBollingerBandsResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Создать с полной ручной конфигурацией всех параметров и источником данных.
    pub fn with_parameters_and_source(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64,
        source: OhlcvField
    ) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(min_period > 0 && min_period <= base_period, "Invalid min period");
        assert!(max_period >= base_period, "Invalid max period");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");
        assert!(min_multiplier > 0.0 && min_multiplier <= base_multiplier, "Invalid min multiplier");
        assert!(max_multiplier >= base_multiplier, "Invalid max multiplier");

        let mut instance = Self::with_parameters_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier
        );
        instance.source = source;
        instance
    }

    /// Создать из базовых параметров с источником данных (auto mode).
    pub fn from_base_params_with_source(base_period: usize, base_multiplier: f64, source: OhlcvField) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");

        let mut instance = Self::from_base_params(base_period, base_multiplier);
        instance.source = source;
        instance
    }

    /// Feed the resolved `[open, high, low, close, volume]` lanes (in `SOURCE` order).
    /// The configurable price source is extracted internally from the full bar; ATR needs
    /// H/L/C. The whole candle is delivered so any `source` selection resolves internally.
    pub fn feed(&mut self, lanes: &[f64]) -> AdaptiveBollingerBandsResult {
        let open = lanes[0];
        let high = lanes[1];
        let low = lanes[2];
        let close = lanes[3];
        let volume = lanes[4];
        // Используем настраиваемый источник данных
        let price = self.source.extract(open, high, low, close, volume);

        // Добавляем цену в буфер
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        // 1. Обновляем ATR (переиспользуем существующий компонент)
        let atr_value = self.atr.feed(&[high, low, close]);

        // 2. Адаптируем параметры на основе волатильности
        self.adapt_parameters(atr_value);

        // 3. Пересоздаем адаптивную MA если период изменился значительно
        self.update_adaptive_ma();

        // 4. Обновляем адаптивную MA
        let middle_band = self.adaptive_ma.feed(price);

        // 5. Рассчитываем стандартное отклонение
        let std_dev = self.calculate_adaptive_std_dev(close, middle_band);

        // 6. Рассчитываем полосы
        let upper_band = middle_band + (self.current_multiplier * std_dev);
        let lower_band = middle_band - (self.current_multiplier * std_dev);

        // 7. Рассчитываем дополнительные метрики
        self.calculate_additional_metrics(close, upper_band, middle_band, lower_band);

        // 8. Определяем режим рынка
        self.determine_market_regime();

        // Обновляем результат
        self.current_result.upper_band = upper_band;
        self.current_result.middle_band = middle_band;
        self.current_result.lower_band = lower_band;
        self.current_result.adaptive_period = self.current_period;
        self.current_result.adaptive_multiplier = self.current_multiplier;

        // Готов после накопления достаточных данных
        if self.adaptive_ma.is_ready() && self.prices.len() >= self.base_period {
            self.is_ready = true;
        }

        self.update_count += 1;
        self.current_result
    }

    /// Адаптировать параметры на основе волатильности
    fn adapt_parameters(&mut self, atr_value: f64) {
        if self.update_count < 20 {
            return; // Недостаточно данных для адаптации
        }

        // Сглаживаем волатильность
        let smoothed_volatility = self.volatility_ma.feed(atr_value);

        // Нормализуем волатильность (отношение к базовому значению)
        let volatility_ratio = if smoothed_volatility > 0.0 {
            atr_value / smoothed_volatility
        } else {
            1.0
        };

        // Адаптируем период: высокая волатильность = короткий период
        let period_adjustment = 1.0 / volatility_ratio.sqrt();
        self.current_period = (self.base_period as f64 * period_adjustment)
            .max(self.min_period as f64)
            .min(self.max_period as f64);

        // Адаптируем множитель: высокая волатильность = меньший множитель
        let multiplier_adjustment = volatility_ratio.sqrt();
        self.current_multiplier = (self.base_multiplier * multiplier_adjustment)
            .max(self.min_multiplier)
            .min(self.max_multiplier);

        // Сохраняем период для анализа
        if self.periods.len() >= 16 {
            self.periods.remove(0);
        }
        self.periods.push(self.current_period);
    }

    /// Обновить адаптивную MA при значительном изменении периода.
    ///
    /// Uses the cached `self.adaptive_ma_id` (a [`SmootherId`]) to rebuild
    /// `SmootherSlot` with the new period.
    fn update_adaptive_ma(&mut self) {
        let current_ma_period = self.adaptive_ma_period;
        let new_period = self.current_period as usize;

        // Пересоздаем MA если период изменился более чем на 20%
        let diff: f64 = (new_period as f64 - current_ma_period as f64).abs();
        if diff / current_ma_period as f64 > 0.2 {
            self.adaptive_ma = SmootherSlot::new(self.adaptive_ma_id, new_period);
            self.adaptive_ma_period = new_period;
        }
    }

    /// Рассчитать адаптивное стандартное отклонение
    fn calculate_adaptive_std_dev(&mut self, _current_price: f64, _middle_band: f64) -> f64 {
        let period = self.current_period as usize;
        let available_data = self.prices.len().min(period);

        if available_data < 2 {
            return 0.1; // Минимальное значение
        }

        // Рассчитываем стандартное отклонение за адаптивный период
        let start_idx = self.prices.len() - available_data;
        let prices_slice = &self.prices[start_idx..];

        let mean = prices_slice.iter().sum::<f64>() / available_data as f64;
        let variance = prices_slice.iter()
            .map(|&price| (price - mean).powi(2))
            .sum::<f64>() / available_data as f64;

        let std_dev = variance.sqrt();

        // Сохраняем для анализа
        if self.std_devs.len() >= 32 {
            self.std_devs.remove(0);
        }
        self.std_devs.push(std_dev);

        std_dev
    }

    /// Рассчитать дополнительные метрики
    fn calculate_additional_metrics(&mut self, price: f64, upper: f64, _middle: f64, lower: f64) {
        // Ширина канала
        let bandwidth = upper - lower;

        // Сохраняем ширину канала
        if self.bandwidths.len() >= 32 {
            self.bandwidths.remove(0);
        }
        self.bandwidths.push(bandwidth);

        // Сглаженная ширина канала
        let _smoothed_bandwidth = self.bandwidth_ma.feed(bandwidth);

        // %B - позиция цены в канале
        let percent_b = if bandwidth > 0.0 {
            (price - lower) / bandwidth
        } else {
            0.5
        };

        // Коэффициент сжатия
        let squeeze_ratio = if self.bandwidths.len() >= 10 {
            let recent_bandwidths = &self.bandwidths[self.bandwidths.len() - 10..];
            let max_bandwidth = recent_bandwidths.iter().fold(0.0f64, |a, &b| a.max(b));

            if max_bandwidth > 0.0 {
                bandwidth / max_bandwidth
            } else {
                0.5
            }
        } else {
            0.5
        };

        // Обновляем результат
        self.current_result.bandwidth = bandwidth;
        self.current_result.percent_b = percent_b;
        self.current_result.squeeze_ratio = squeeze_ratio;
    }

    /// Определить режим рынка
    fn determine_market_regime(&mut self) {
        if !self.is_ready || self.periods.len() < 5 {
            self.current_result.market_regime = 0;
            return;
        }

        let squeeze_ratio = self.current_result.squeeze_ratio;
        let bandwidth = self.current_result.bandwidth;

        // Анализируем стабильность адаптивного периода
        let recent_periods = &self.periods[self.periods.len().saturating_sub(5)..];
        let period_variance = if recent_periods.len() >= 2 {
            let mean: f64 = recent_periods.iter().sum::<f64>() / recent_periods.len() as f64;
            recent_periods.iter()
                .map(|&p| (p - mean).powi(2))
                .sum::<f64>() / recent_periods.len() as f64
        } else {
            0.0
        };

        // Определяем режим
        if squeeze_ratio < 0.3 {
            self.current_result.market_regime = 0; // Флэт (сжатие)
        } else if period_variance < 1.0 && bandwidth > 0.0 {
            self.current_result.market_regime = 1; // Тренд (стабильный период)
        } else {
            self.current_result.market_regime = -1; // Высокая волатильность
        }
    }


    pub fn upper(&self) -> f64 { self.current_result.upper_band }
    pub fn middle(&self) -> f64 { self.current_result.middle_band }
    pub fn lower(&self) -> f64 { self.current_result.lower_band }
    pub fn bandwidth(&self) -> f64 { self.current_result.bandwidth }
    pub fn percent_b(&self) -> f64 { self.current_result.percent_b }
    pub fn squeeze_ratio(&self) -> f64 { self.current_result.squeeze_ratio }

    /// Получить полный результат
    pub fn result(&self) -> AdaptiveBollingerBandsResult {
        self.current_result
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.adaptive_ma = SmootherSlot::new(self.adaptive_ma_id, self.base_period);
        self.adaptive_ma_period = self.base_period;
        self.atr = Atr::from_smoother(14, self.atr_ma_id);
        self.volatility_ma = SmootherSlot::new(SmootherId::Sma, self.volatility_ma_period);
        self.bandwidth_ma = SmootherSlot::new(SmootherId::Sma, self.bandwidth_ma_period);

        self.prices.clear();
        self.std_devs.clear();
        self.bandwidths.clear();
        self.periods.clear();

        self.current_period = self.base_period as f64;
        self.current_multiplier = self.base_multiplier;

        self.current_result = AdaptiveBollingerBandsResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.current_period as usize
    }

    /// Генерировать торговый сигнал
    pub fn trading_signal(&self, price: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }

        let result = self.current_result;

        // Сигналы в зависимости от режима рынка
        match result.market_regime {
            1 => {
                // Трендовый режим - торговля на пробоях
                if price > result.upper_band {
                    1 // Пробой вверх
                } else if price < result.lower_band {
                    -1 // Пробой вниз
                } else {
                    0
                }
            }
            0 => {
                // Флэтовый режим - торговля на отскоках
                if result.percent_b > 0.9 {
                    -1 // Продажа у верхней полосы
                } else if result.percent_b < 0.1 {
                    1 // Покупка у нижней полосы
                } else {
                    0
                }
            }
            _ => {
                // Высокая волатильность - осторожные сигналы
                if result.percent_b > 0.95 && result.squeeze_ratio > 0.7 {
                    -1
                } else if result.percent_b < 0.05 && result.squeeze_ratio > 0.7 {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// Генерировать сигнал сжатия (squeeze)
    pub fn squeeze_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }

        let squeeze_ratio = self.current_result.squeeze_ratio;

        if squeeze_ratio < 0.2 {
            return 1; // Сильное сжатие - ожидаем пробой
        } else if squeeze_ratio > 0.8 {
            return -1; // Расширение - возможное окончание движения
        }

        0
    }

    /// Получить информацию о текущем состоянии
    pub fn info(&self, price: f64) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal(price) {
            1 => "Покупка",
            -1 => "Продажа",
            _ => "Нет сигнала",
        };

        format!(
            "Adaptive BB: {:.2}-{:.2}-{:.2}, Период: {:.0}, Множитель: {:.1}, Режим: {}, {}, %B: {:.1}%, Сигнал: {}",
            result.lower_band,
            result.middle_band,
            result.upper_band,
            result.adaptive_period,
            result.adaptive_multiplier,
            result.market_regime_name(),
            result.squeeze_state(),
            result.percent_b * 100.0,
            signal
        )
    }

    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("upper_band".to_string(), self.current_result.upper_band);
        values.insert("middle_band".to_string(), self.current_result.middle_band);
        values.insert("lower_band".to_string(), self.current_result.lower_band);
        values.insert("bandwidth".to_string(), self.current_result.bandwidth);
        values.insert("percent_b".to_string(), self.current_result.percent_b);
        values.insert("squeeze_ratio".to_string(), self.current_result.squeeze_ratio);
        values.insert("adaptive_period".to_string(), self.current_result.adaptive_period);
        values.insert("adaptive_multiplier".to_string(), self.current_result.adaptive_multiplier);
        values.insert("market_regime".to_string(), self.current_result.market_regime as f64);
        values
    }

    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, usize, f64, f64, f64) {
        (self.base_period, self.min_period, self.max_period,
         self.base_multiplier, self.min_multiplier, self.max_multiplier)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Slot, UpdateComplexity, Store, StoreKind};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Typed dual-mode contract config for [`AdaptiveBollingerBands`].
///
/// Two smoother slots:
/// - `adaptive_ma`: member + period for the central adaptive line (period shifts dynamically;
///   the member kind is config-fixed via the slot order). Default Ema at `base_period` (20).
/// - `atr_ma`: member + period for the internal ATR smoother. Default Rma at `base_period` (20).
///
/// The internal `volatility_ma` and `bandwidth_ma` remain as hardcoded SMA slots
/// (internal mechanism; not user-configurable).
///
/// `valid_params` enforces `min_period <= base_period <= max_period` and
/// `min_multiplier <= base_multiplier <= max_multiplier`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AdaptiveBollingerBandsConfig {
    pub base_period: Param<usize>,
    /// Absolute lower period guardrail (default 10, must be ≤ base_period). Sweepable.
    pub min_period: Param<usize>,
    /// Absolute upper period guardrail (default 40, must be ≥ base_period). Sweepable.
    pub max_period: Param<usize>,
    pub base_multiplier: Param<f64>,
    /// Absolute lower band-multiplier guardrail (default 1.0, must be ≤ base_multiplier). Sweepable.
    pub min_multiplier: Param<f64>,
    /// Absolute upper band-multiplier guardrail (default 3.0, must be ≥ base_multiplier). Sweepable.
    pub max_multiplier: Param<f64>,
    pub source: Param<OhlcvField>,
    /// Member + period for the central adaptive MA. The member kind is fixed at construction
    /// and cached in the runtime for mid-stream SmootherSlot rebuilds when the adaptive
    /// period shifts.
    #[slot]
    pub adaptive_ma: Param<SmootherSlotOrder>,
    /// Member + period for the ATR's internal smoother.
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

impl Indicator for AdaptiveBollingerBands {
    const ID: IndicatorId = IndicatorId::Adaptivebb;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// `Fields` flavor: the factory feeds the full OHLCV candle. The configurable price
    /// `source` is extracted internally (so any field selection resolves in-core), and ATR
    /// requires H/L/C directly — hence the whole bar [O,H,L,C,V] is delivered.
    const SOURCE: Option<crate::contract::SourceAxis> = Some(crate::contract::SourceAxis::KlineSlice(&[
        OhlcvField::Open, OhlcvField::High, OhlcvField::Low, OhlcvField::Close, OhlcvField::Volume,
    ]));
    /// Own base = four Vec buffers (prices, std_devs, bandwidths, periods). Two
    /// `SmootherSlot`s (volatility_ma + bandwidth_ma) are fixed-SMA internals and
    /// contribute to the linear-update class (adapt_parameters rescans the buffer).
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[
        Store::window(StoreKind::Vec), // prices (up to 64 deep)
        Store::fixed(StoreKind::Vec, 32), // std_devs
        Store::fixed(StoreKind::Vec, 32), // bandwidths
        Store::fixed(StoreKind::Vec, 16), // periods
    ]);
    const SLOTS: &'static [Slot] = AdaptiveBollingerBandsConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AdaptivebbUpper),
        Output::price(IndicatorOutputId::AdaptivebbMiddle),
        Output::price(IndicatorOutputId::AdaptivebbLower),
        Output::magnitude(IndicatorOutputId::AdaptivebbBandwidth),
        Output::percent(IndicatorOutputId::AdaptivebbPercentB),
        Output::percent(IndicatorOutputId::AdaptivebbSqueezeRatio),
    ];
    type Config = AdaptiveBollingerBandsConfig;
    type Runtime = AdaptiveBollingerBands;

    fn create(cfg: AdaptiveBollingerBandsConfig) -> AdaptiveBollingerBands {
        let adaptive_ma_order = cfg.adaptive_ma.resolved();
        let atr_ma_order = cfg.atr_ma.resolved();
        let adaptive_ma_id = adaptive_ma_order.id();
        let atr_ma_id = atr_ma_order.id();
        AdaptiveBollingerBands::with_full_config_internal(
            cfg.base_period.resolved(),
            cfg.min_period.resolved(),
            cfg.max_period.resolved(),
            cfg.base_multiplier.resolved(),
            cfg.min_multiplier.resolved(),
            cfg.max_multiplier.resolved(),
            adaptive_ma_id,
            atr_ma_id,
            10, // volatility_ma_period: internal fixed default
            20, // bandwidth_ma_period: internal fixed default
        )
    }

    fn slot_members(cfg: &AdaptiveBollingerBandsConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AdaptiveBollingerBandsConfig {
    fn defaults() -> Self {
        // Old defaults: base_period=20, period_min_mult=0.5, period_max_mult=2.0,
        //   base_multiplier=2.0, mult_min_ratio=0.5, mult_max_ratio=1.5.
        // Derived absolutes (from old create() logic):
        //   min_period = (20*0.5).round()=10, max(5)=10, min(20)=10
        //   max_period = (20*2.0).round()=40, max(20)=40
        //   min_multiplier = (2.0*0.5)=1.0, max(0.5)=1.0, min(2.0)=1.0
        //   max_multiplier = (2.0*1.5)=3.0, max(2.0)=3.0
        AdaptiveBollingerBandsConfig {
            base_period: Param::Solo(20),
            min_period: Param::Solo(10),
            max_period: Param::Solo(40),
            base_multiplier: Param::Solo(2.0),
            min_multiplier: Param::Solo(1.0),
            max_multiplier: Param::Solo(3.0),
            source: Param::Solo(OhlcvField::Close),
            adaptive_ma: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 20 })),
            atr_ma: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 20 })),
        }
    }
    fn valid_params(&self) -> Result<(), String> {
        let min_p = self.min_period.resolved();
        let base_p = self.base_period.resolved();
        let max_p = self.max_period.resolved();
        if min_p > base_p {
            return Err(format!("min_period({min_p}) > base_period({base_p})"));
        }
        if base_p > max_p {
            return Err(format!("base_period({base_p}) > max_period({max_p})"));
        }
        let min_m = self.min_multiplier.resolved();
        let base_m = self.base_multiplier.resolved();
        let max_m = self.max_multiplier.resolved();
        if min_m > base_m {
            return Err(format!("min_multiplier({min_m}) > base_multiplier({base_m})"));
        }
        if base_m > max_m {
            return Err(format!("base_multiplier({base_m}) > max_multiplier({max_m})"));
        }
        Ok(())
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // base_period/min_period/max_period→range(2,4048,1), source→all 8
        s.base_multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s.min_multiplier = Param::many(sweep_f64(0.1, 5.0, 0.1));   // absolute lower band-mult
        s.max_multiplier = Param::many(sweep_f64(0.1, 15.0, 0.1));  // absolute upper band-mult
        s
    }
}


impl Render for AdaptiveBollingerBands {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::AdaptivebbUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::AdaptivebbMiddle, "Middle", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::AdaptivebbLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_bollinger_bands_creation() {
        let abb = AdaptiveBollingerBands::new();
        assert!(!abb.is_ready());
        assert_eq!(abb.parameters().0, 20);
    }

    #[test]
    fn test_with_full_config_non_default_types() {
        // richer ctor: SMA for adaptive_ma, EMA for atr
        let mut abb = AdaptiveBollingerBands::with_full_config(
            20, 10, 40, 2.0, 1.0, 3.0,
            SmootherId::Sma,
            SmootherId::Ema,
            8, 16,
        );
        assert!(!abb.is_ready());
        for i in 0..30 {
            let p = 100.0 + i as f64 * 0.5;
            let r = abb.feed(&[p, p + 1.0, p - 1.0, p, 1000.0]);
            assert!(r.upper_band.is_finite());
            assert!(r.lower_band.is_finite());
        }
        assert!(abb.is_ready());
    }

    #[test]
    fn test_adaptive_bollinger_bands_with_parameters() {
        let abb = AdaptiveBollingerBands::with_parameters(14, 7, 28, 2.5, 1.5, 3.5);
        assert_eq!(abb.parameters(), (14, 7, 28, 2.5, 1.5, 3.5));
    }

    #[test]
    fn test_from_base_params_auto_calculation() {
        let abb = AdaptiveBollingerBands::from_base_params(20, 2.0);
        let (base_p, min_p, max_p, base_m, min_m, max_m) = abb.parameters();
        assert_eq!(base_p, 20);
        assert_eq!(min_p, 10);
        assert_eq!(max_p, 40);
        assert_eq!(base_m, 2.0);
        assert_eq!(min_m, 1.0);
        assert_eq!(max_m, 3.0);
    }

    #[test]
    fn test_from_base_params_high_multiplier() {
        let abb = AdaptiveBollingerBands::from_base_params(20, 4.0);
        let (_, _, _, base_m, min_m, max_m) = abb.parameters();
        assert_eq!(base_m, 4.0);
        assert_eq!(min_m, 2.0);
        assert_eq!(max_m, 6.0);
    }

    #[test]
    fn test_from_base_params_small_period() {
        let abb = AdaptiveBollingerBands::from_base_params(8, 2.0);
        let (base_p, min_p, max_p, _, _, _) = abb.parameters();
        assert_eq!(base_p, 8);
        assert_eq!(min_p, 5);
        assert_eq!(max_p, 16);
    }

    #[test]
    fn test_from_base_params_small_multiplier() {
        let abb = AdaptiveBollingerBands::from_base_params(20, 0.8);
        let (_, _, _, base_m, min_m, max_m) = abb.parameters();
        assert_eq!(base_m, 0.8);
        assert_eq!(min_m, 0.5);
        assert!((max_m - 1.2).abs() < 1e-10);
    }

    #[test]
    fn test_adaptive_bollinger_bands_update() {
        let mut abb = AdaptiveBollingerBands::new();

        for i in 0..30 {
            let base_price = 100.0;
            let trend = i as f64 * 0.1;
            let volatility = if i > 15 { 3.0 } else { 1.0 };

            let high = base_price + trend + volatility;
            let low = base_price + trend - volatility;
            let close = base_price + trend + (volatility * 0.5 * (i as f64 * 0.1).sin());

            let result = abb.feed(&[base_price + trend, high, low, close, 1000.0]);

            if i > 25 {
                assert!(abb.is_ready());
                assert!(result.upper_band > result.middle_band);
                assert!(result.middle_band > result.lower_band);
                assert!(result.bandwidth > 0.0);
                assert!(result.percent_b >= 0.0 && result.percent_b <= 1.5);
                assert!(result.squeeze_ratio >= 0.0 && result.squeeze_ratio <= 1.0);
                assert!(result.adaptive_period >= abb.min_period as f64);
                assert!(result.adaptive_period <= abb.max_period as f64);
            }
        }
    }

    #[test]
    fn test_adaptation_to_volatility() {
        let mut abb = AdaptiveBollingerBands::new();

        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.01);
            let _result = abb.feed(&[price, price + 0.01, price - 0.01, price, 1000.0]);
        }
        let low_vol_period = abb.current_period;
        let low_vol_multiplier = abb.current_multiplier;

        for i in 15..30 {
            let price = 100.0 + (i as f64 * 0.5 * (i as f64).sin());
            let _result = abb.feed(&[price, price + 2.0, price - 2.0, price, 1000.0]);
        }
        let high_vol_period = abb.current_period;
        let high_vol_multiplier = abb.current_multiplier;

        if abb.is_ready() {
            assert!(high_vol_period <= low_vol_period);
            assert!(high_vol_multiplier != low_vol_multiplier);
        }
    }

    #[test]
    fn test_trading_signals() {
        let mut abb = AdaptiveBollingerBands::new();

        for i in 0..25 {
            let price = 100.0 + i as f64 * 0.2;
            let _result = abb.feed(&[price, price + 0.5, price - 0.5, price, 1000.0]);
        }

        if abb.is_ready() {
            let result = abb.result();

            let upper_signal = abb.trading_signal(result.upper_band + 1.0);
            let lower_signal = abb.trading_signal(result.lower_band - 1.0);
            let middle_signal = abb.trading_signal(result.middle_band);
            let squeeze_signal = abb.squeeze_signal();

            assert!(upper_signal >= -1 && upper_signal <= 1);
            assert!(lower_signal >= -1 && lower_signal <= 1);
            assert!(middle_signal >= -1 && middle_signal <= 1);
            assert!(squeeze_signal >= -1 && squeeze_signal <= 1);
        }
    }

    #[test]
    fn test_contract_create() {
        let cfg = <<AdaptiveBollingerBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut abb = <AdaptiveBollingerBands as Indicator>::create(cfg);
        for i in 0..30 {
            let p = 100.0 + i as f64 * 0.5;
            abb.feed(&[p, p + 1.0, p - 1.0, p, 1000.0]);
        }
        assert!(abb.is_ready());
    }
}
