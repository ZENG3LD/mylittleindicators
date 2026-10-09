//! Ehlers Fractal Adaptive Moving Average (FAMA) - фрактальная адаптивная скользящая средняя
//!
//! FAMA адаптируется к изменчивости рынка, используя фрактальную размерность
//! для автоматической настройки периода сглаживания.
//! Чем выше волатильность, тем быстрее реагирует индикатор.
//!
//! Переиспользует существующие компоненты EMA

use crate::engine::ohlcv_field::OhlcvField;

/// Результат Fractal Adaptive MA
#[derive(Debug, Clone, Copy)]
pub struct FractalAdaptiveResult {
    pub fama: f64,                  // Значение FAMA
    pub fractal_dimension: f64,     // Фрактальная размерность
    pub efficiency_ratio: f64,      // Коэффициент эффективности
    pub adaptive_period: f64,       // Адаптивный период
    pub trend_strength: f64,        // Сила тренда
}

impl FractalAdaptiveResult {
    pub fn empty() -> Self {
        Self {
            fama: 0.0,
            fractal_dimension: 0.0,
            efficiency_ratio: 0.0,
            adaptive_period: 0.0,
            trend_strength: 0.0,
        }
    }
}

/// Ehlers Fractal Adaptive Moving Average
#[derive(Debug, Clone)]
pub struct EhlersFractalAdaptiveMa {
    // Адаптивная EMA «на месте»: alpha пересчитывается из фрактального периода каждый
    // бар, накопитель НЕ пересоздаётся (пересоздание рвало бы сглаживание — выход
    // прыгал бы к цене при каждой смене периода).
    fama: f64,
    seeded: bool,

    // Параметры
    period: usize,
    min_period: usize,
    max_period: usize,

    // Буферы для расчетов
    prices: Vec<f64>,

    // Результат
    last_result: FractalAdaptiveResult,

    // Состояние
    initialized: bool,
}

impl EhlersFractalAdaptiveMa {
    pub fn new(period: usize) -> Self {
        let min_period = (period / 4).max(2);
        let max_period = period * 2;

        Self {
            fama: 0.0,
            seeded: false,
            period,
            min_period,
            max_period,
            prices: Vec::with_capacity(period),
            last_result: FractalAdaptiveResult::empty(),
            initialized: false,
        }
    }
    
    pub fn period(&self) -> usize {
        self.period
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized && self.prices.len() >= self.period
    }
    
    /// Обновление с одним значением цены
    pub fn update(&mut self, price: f64) -> f64 {
        self.feed(price)
    }
    
    /// Обновление с OHLCV данными (использует настроенный source)
    pub fn feed(&mut self, value: f64) -> f64 {
        let price = value;

        // Добавляем новую цену
        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        // Прогрев: сглаживаем по базовому периоду, накопитель непрерывен.
        if self.prices.len() < self.period {
            let base_alpha = 2.0 / (self.period as f64 + 1.0);
            self.fama = if self.seeded {
                base_alpha * price + (1.0 - base_alpha) * self.fama
            } else {
                self.seeded = true;
                price
            };
            self.last_result.fama = self.fama;
            return self.fama;
        }

        // Вычисляем фрактальную размерность и коэффициент эффективности
        let fractal_dim = self.calculate_fractal_dimension();
        let efficiency = self.calculate_efficiency_ratio();

        // Адаптивный период -> alpha. Сглаживаем НА МЕСТЕ: alpha меняется каждый бар,
        // состояние FAMA сохраняется (никакого пересоздания/сброса EMA).
        let adaptive_period = self.calculate_adaptive_period(fractal_dim, efficiency);
        let alpha = (2.0 / (adaptive_period + 1.0)).clamp(0.0, 1.0);
        self.fama = if self.seeded {
            alpha * price + (1.0 - alpha) * self.fama
        } else {
            self.seeded = true;
            price
        };

        // Обновляем результат
        self.last_result = FractalAdaptiveResult {
            fama: self.fama,
            fractal_dimension: fractal_dim,
            efficiency_ratio: efficiency,
            adaptive_period,
            trend_strength: efficiency * (2.0 - fractal_dim), // Комбинированная метрика
        };

        self.initialized = true;
        self.fama
    }
    
    /// Вычисление фрактальной размерности
    fn calculate_fractal_dimension(&self) -> f64 {
        if self.prices.len() < self.period {
            return 1.5; // Средняя фрактальная размерность
        }
        
        let mut total_length = 0.0;

        let start_idx = self.prices.len() - self.period;

        // Вычисляем общую длину пути
        for i in 1..self.period {
            let prev = self.prices[start_idx + i - 1];
            let curr = self.prices[start_idx + i];
            total_length += (curr - prev).abs();
        }

        // Прямое расстояние
        let direct_distance = (self.prices[start_idx + self.period - 1] - self.prices[start_idx]).abs();
        
        if direct_distance == 0.0 || total_length == 0.0 {
            return 1.5;
        }
        
        // Фрактальная размерность
        let fractal_dim = (total_length / direct_distance).ln() / (self.period as f64).ln();
        
        // Ограничиваем значения от 1.0 до 2.0
        fractal_dim.clamp(1.0, 2.0)
    }
    
    /// Вычисление коэффициента эффективности (как в AMA)
    fn calculate_efficiency_ratio(&self) -> f64 {
        if self.prices.len() < self.period {
            return 0.0;
        }
        
        let start_idx = self.prices.len() - self.period;
        
        // Направленное движение
        let direction = (self.prices[start_idx + self.period - 1] - self.prices[start_idx]).abs();
        
        // Волатильность (сумма абсолютных изменений)
        let mut volatility = 0.0;
        for i in 1..self.period {
            volatility += (self.prices[start_idx + i] - self.prices[start_idx + i - 1]).abs();
        }
        
        if volatility == 0.0 {
            return 0.0;
        }
        
        direction / volatility
    }
    
    /// Вычисление адаптивного периода
    fn calculate_adaptive_period(&self, fractal_dim: f64, efficiency: f64) -> f64 {
        // Комбинируем фрактальную размерность и эффективность
        let complexity_factor = fractal_dim; // 1.0-2.0
        let efficiency_factor = efficiency;   // 0.0-1.0
        
        // Высокая сложность (fractal_dim близко к 2.0) = медленная адаптация
        // Высокая эффективность = быстрая адаптация  
        let adaptation_speed = efficiency_factor / complexity_factor;
        
        // Интерполируем между min_period и max_period
        let adaptive_period = self.max_period as f64 - 
            adaptation_speed * (self.max_period - self.min_period) as f64;
            
        adaptive_period.max(self.min_period as f64).min(self.max_period as f64)
    }
    
    /// Получить последний результат
    pub fn result(&self) -> FractalAdaptiveResult {
        self.last_result
    }
    
    /// Получить значение FAMA
    pub fn value(&self) -> f64 {
        self.last_result.fama
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.prices.len() >= self.period
    }

    /// Сброс индикатора
    pub fn reset(&mut self) {
        self.prices.clear();
        self.fama = 0.0;
        self.seeded = false;
        self.last_result = FractalAdaptiveResult::empty();
        self.initialized = false;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`EhlersFractalAdaptiveMa`]: period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EhlersfahConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for EhlersfahConfig {
    fn defaults() -> Self {
        EhlersfahConfig {
            period: Param::Solo(20),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        Self::machine_defaults_auto()
    }
}

impl Indicator for EhlersFractalAdaptiveMa {
    const ID: IndicatorId = IndicatorId::Ehlersfa;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Scans a period-deep price `Vec` for the fractal dimension each bar (O(period)).
    /// The adaptive EMA is computed in-place (alpha from the fractal period) — no inner
    /// indicator edge, so the whole cost is this node's own scan + the window store.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Ehlersfa)];
    type Config = EhlersfahConfig;
    type Runtime = EhlersFractalAdaptiveMa;

    fn create(cfg: EhlersfahConfig) -> EhlersFractalAdaptiveMa {
        EhlersFractalAdaptiveMa::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &Self::Config) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for EhlersFractalAdaptiveMa {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Ehlersfa, "Ehlers FA", Color::hex(0x00BCD4))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fama_creation() {
        let fama = EhlersFractalAdaptiveMa::new(20);
        assert_eq!(fama.period(), 20);
        assert!(!fama.is_ready());
        assert_eq!(fama.value(), 0.0);
    }

    #[test]
    fn test_fama_warmup_and_finite() {
        let mut fama = EhlersFractalAdaptiveMa::new(10);
        for i in 1..=40 {
            let v = fama.update(100.0 + i as f64);
            assert!(v.is_finite());
        }
        assert!(fama.is_ready());
    }

    #[test]
    fn test_fama_tracks_uptrend() {
        let mut fama = EhlersFractalAdaptiveMa::new(10);
        for i in 1..=60 {
            fama.update(100.0 + i as f64 * 2.0);
        }
        assert!(fama.is_ready());
        // A smoother that adapts must still rise well above the start in a sustained uptrend.
        assert!(fama.value() > 150.0, "FAMA should track the uptrend, got {}", fama.value());
    }

    #[test]
    fn test_fama_constant_converges_to_price() {
        let mut fama = EhlersFractalAdaptiveMa::new(8);
        for _ in 0..40 {
            fama.update(100.0);
        }
        assert!(fama.is_ready());
        assert!((fama.value() - 100.0).abs() < 1e-6, "FAMA on constant price should converge to it, got {}", fama.value());
    }

    #[test]
    fn test_fama_reset() {
        let mut fama = EhlersFractalAdaptiveMa::new(10);
        for i in 1..=30 {
            fama.update(100.0 + i as f64);
        }
        assert!(fama.is_ready());
        fama.reset();
        assert!(!fama.is_ready());
        assert_eq!(fama.value(), 0.0);
    }

    #[test]
    fn test_fama_contract_create() {
        let cfg = <<EhlersFractalAdaptiveMa as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut fama = <EhlersFractalAdaptiveMa as Indicator>::create(cfg);
        for i in 1..=40 {
            fama.feed(100.0 + i as f64);
        }
        assert!(fama.is_ready());
    }
}