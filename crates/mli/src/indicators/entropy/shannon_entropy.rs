//! Shannon Entropy - энтропия Шеннона для оценки предсказуемости рынка
//! Измеряет неопределенность в распределении доходностей
//! Значения: 0.0-1.0 (нормализованная), где 0.0 = полностью предсказуемо, 1.0 = максимально случайно

use std::collections::HashMap;
use crate::engine::ohlcv_field::OhlcvField;

/// Состояние рынка на основе энтропии Шеннона
#[derive(Debug, Clone, PartialEq)]
pub enum MarketEntropyState {
    HighlyPredictable,  // entropy < 0.3
    Moderate,           // 0.3 <= entropy < 0.7
    Random,             // 0.7 <= entropy < 0.9
    Chaotic,            // entropy >= 0.9
}

/// Shannon Entropy индикатор
#[derive(Debug, Clone)]
pub struct ShannonEntropy {
    period: usize,              // Период анализа
    bins: usize,                // Количество корзин для гистограммы

    // Буферы данных
    returns: Vec<f64>, // Буфер логарифмических доходностей
    prev_close: Option<f64>,     // Предыдущая цена для расчета доходности
    
    // Результаты
    entropy: f64,               // Энтропия Шеннона
    normalized_entropy: f64,    // Нормализованная энтропия (0-1)
    predictability_score: f64,  // 1 - normalized_entropy
    
    // Состояние
    count: usize,
    initialized: bool,
}

impl ShannonEntropy {
    pub fn new(period: usize, bins: usize) -> Self {
        Self::with_source(period, bins, OhlcvField::Close)
    }

    pub fn with_source(period: usize, bins: usize, _source: OhlcvField) -> Self {
        // `_source` is part of the config (the factory resolves it via `const SOURCE`
        // and feeds the pre-extracted scalar) — the runtime keeps no source of its own.
        Self {
            period: period.min(512),
            bins: bins.clamp(5, 50), // Ограничиваем разумными пределами
            returns: Vec::with_capacity(period.min(512)),
            prev_close: None,
            entropy: 0.0,
            normalized_entropy: 0.0,
            predictability_score: 1.0,
            count: 0,
            initialized: false,
        }
    }

    /// Создать Shannon Entropy с параметрами по умолчанию
    pub fn new_default(period: usize) -> Self {
        Self::new(period, 20) // 20 bins по умолчанию
    }

    /// Feed a pre-extracted scalar (resolved by the factory from `const SOURCE`).
    pub fn feed(&mut self, value: f64) -> f64 {
        // Рассчитываем логарифмическую доходность
        if let Some(prev_close) = self.prev_close {
            let log_return = (value / prev_close).ln();
            
            // Фильтруем экстремальные значения
            if log_return.is_finite() && log_return.abs() < 1.0 {
                // Добавляем в буфер
                if self.returns.len() >= self.period {
                    self.returns.remove(0);
                }
                self.returns.push(log_return);
                self.count += 1;
                
                // Рассчитываем энтропию если достаточно данных
                if self.returns.len() >= self.period.min(10) {
                    self.calculate_entropy();
                    self.initialized = true;
                }
            }
        }
        
        self.prev_close = Some(value);
        self.normalized_entropy
    }
    
    /// Рассчитать энтропию Шеннона
    fn calculate_entropy(&mut self) {
        if self.returns.is_empty() {
            return;
        }
        
        // Находим диапазон доходностей
        let min_return = self.returns.iter().copied().fold(f64::INFINITY, f64::min);
        let max_return = self.returns.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        
        if (max_return - min_return).abs() < 1e-10 {
            // Все доходности одинаковые - минимальная энтропия
            self.entropy = 0.0;
            self.normalized_entropy = 0.0;
            self.predictability_score = 1.0;
            return;
        }
        
        // Создаем гистограмму
        let bin_width = (max_return - min_return) / self.bins as f64;
        let mut histogram = HashMap::new();
        
        for &return_val in &self.returns {
            let bin_index = ((return_val - min_return) / bin_width).floor() as usize;
            let bin_index = bin_index.min(self.bins - 1);
            *histogram.entry(bin_index).or_insert(0) += 1;
        }
        
        // Рассчитываем энтропию Шеннона: H = -Σ p(xi) * log2(p(xi))
        let total_count = self.returns.len() as f64;
        let mut entropy = 0.0;
        
        for &count in histogram.values() {
            if count > 0 {
                let probability = count as f64 / total_count;
                entropy -= probability * probability.log2();
            }
        }
        
        // Нормализуем энтропию (максимальная энтропия = log2(bins))
        let max_entropy = (self.bins as f64).log2();
        
        self.entropy = entropy;
        self.normalized_entropy = if max_entropy > 0.0 {
            (entropy / max_entropy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.predictability_score = 1.0 - self.normalized_entropy;
    }
    
    /// Получить текущую энтропию Шеннона
    pub fn entropy(&self) -> f64 {
        self.entropy
    }
    
    /// Получить нормализованную энтропию (0-1)
    pub fn normalized_entropy(&self) -> f64 {
        self.normalized_entropy
    }
    
    /// Получить оценку предсказуемости (0-1, где 1 = максимально предсказуемо)
    pub fn predictability_score(&self) -> f64 {
        self.predictability_score
    }
    
    /// Определить состояние рынка
    pub fn market_state(&self) -> MarketEntropyState {
        match self.normalized_entropy {
            e if e < 0.3 => MarketEntropyState::HighlyPredictable,
            e if e < 0.7 => MarketEntropyState::Moderate,
            e if e < 0.9 => MarketEntropyState::Random,
            _ => MarketEntropyState::Chaotic,
        }
    }
    
    /// Получить торговый сигнал на основе энтропии
    pub fn trading_signal(&self) -> i8 {
        match self.market_state() {
            MarketEntropyState::HighlyPredictable => 1,  // Высокая предсказуемость - следуем тренду
            MarketEntropyState::Moderate => 0,           // Умеренная - нейтрально
            MarketEntropyState::Random => -1,            // Случайность - контртренд
            MarketEntropyState::Chaotic => 0,            // Хаос - ждем
        }
    }
    
    /// Получить значение для использования в других индикаторах
    pub fn value(&self) -> f64 {
        self.normalized_entropy
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить количество корзин
    pub fn bins(&self) -> usize {
        self.bins
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.returns.clear();
        self.prev_close = None;
        self.entropy = 0.0;
        self.normalized_entropy = 0.0;
        self.predictability_score = 1.0;
        self.count = 0;
        self.initialized = false;
    }

}

impl Default for ShannonEntropy {
    /// Factory default (Shannon arm): period=100, bins=20, source=Close.
    fn default() -> Self {
        Self::with_source(100, 20, OhlcvField::Close)
    }
}

// ── Contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, StoreKind, Store, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

use crate::contract::Param;

/// Typed config for [`ShannonEntropy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ShannonConfig {
    pub period: Param<usize>,
    pub bins: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for ShannonEntropy {
    const ID: IndicatorId = IndicatorId::Shannon;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Shannon)];
    type Config = ShannonConfig;
    type Runtime = ShannonEntropy;

    fn create(cfg: ShannonConfig) -> ShannonEntropy {
        ShannonEntropy::with_source(cfg.period.resolved(), cfg.bins.resolved(), cfg.source.resolved())
    }

    fn source_fields(cfg: &ShannonConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for ShannonConfig {
    fn defaults() -> Self {
        ShannonConfig {
            period: Param::Solo(100),
            bins: Param::Solo(20),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // bins: Class B (entropy histogram) — 4..=64 step 2
        s.bins = Param::many((4usize..=64).step_by(2).collect());
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ShannonEntropy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Shannon, "Shannon Entropy", Color::hex(0x9C27B0))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn test_shannon_entropy_creation() {
        let se = ShannonEntropy::new(50, 20);
        assert!(!se.is_ready());
        assert_eq!(se.value(), 0.0);
        assert_eq!(se.period(), 50);
        assert_eq!(se.bins(), 20);
    }

    #[test]
    fn test_shannon_entropy_default() {
        let se = ShannonEntropy::new_default(30);
        assert!(!se.is_ready());
        assert_eq!(se.period(), 30);
        assert_eq!(se.bins(), 20);
    }

    #[test]
    fn test_shannon_entropy_warmup() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.feed(price);
        }
        assert!(se.is_ready());
    }

    #[test]
    fn test_shannon_entropy_values_range() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = se.feed(price);
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_shannon_entropy_reset() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            se.feed(100.0 + i as f64);
        }
        se.reset();
        assert!(!se.is_ready());
        assert_eq!(se.value(), 0.0);
    }

    #[test]
    fn test_shannon_entropy_predictability() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.feed(price);
        }
        let score = se.predictability_score();
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_shannon_entropy_market_state() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.feed(price);
        }
        let state = se.market_state();
        assert!(matches!(state, MarketEntropyState::HighlyPredictable | MarketEntropyState::Moderate | MarketEntropyState::Random | MarketEntropyState::Chaotic));
    }

    #[test]
    fn factory_feeds_resolved_shannon() {
        
        let mut f = IndicatorOrder::Shannon(<<ShannonEntropy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.15).sin() * 8.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Shannon);
        assert!(v >= 0.0 && v <= 1.0, "shannon out of [0,1]: {v}");
    }
} 






















