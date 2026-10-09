//! Permutation Entropy (PE) - энтропия перестановок для анализа порядковых паттернов
//! Измеряет разнообразие порядковых паттернов в временном ряду
//! Значения: 0.0-1.0 (нормализованная), где 0.0 = один паттерн, 1.0 = все паттерны равновероятны

use std::collections::HashMap;
use crate::engine::ohlcv_field::OhlcvField;

/// Permutation Entropy индикатор
#[derive(Debug, Clone)]
pub struct PermutationEntropy {
    period: usize,              // Период анализа
    order: usize,               // Порядок перестановки (3-7)
    delay: usize,               // Задержка (обычно 1)

    // Буферы
    data: Vec<f64>,   // Буфер цен
    
    // Результаты
    pe: f64,                    // Permutation Entropy
    normalized_pe: f64,         // Нормализованная PE (0-1)
    pattern_diversity: f64,     // Разнообразие паттернов
    
    // Состояние
    count: usize,
    initialized: bool,
}

impl PermutationEntropy {
    pub fn new(period: usize, order: usize, delay: usize) -> Self {
        Self::with_source(period, order, delay, OhlcvField::Close)
    }

    pub fn with_source(period: usize, order: usize, delay: usize, _source: OhlcvField) -> Self {
        // `_source` is config-only (factory resolves it via `const SOURCE`); runtime keeps none.
        Self {
            period: period.min(512),
            order: order.clamp(3, 7), // Ограничиваем разумными пределами
            delay: delay.clamp(1, 5), // Задержка не может быть слишком большой
            data: Vec::with_capacity(period.min(512)),
            pe: 0.0,
            normalized_pe: 0.0,
            pattern_diversity: 0.0,
            count: 0,
            initialized: false,
        }
    }

    /// Создать PE с параметрами по умолчанию
    pub fn new_default(period: usize) -> Self {
        Self::new(period, 3, 1) // Порядок 3, задержка 1
    }

    /// Feed a pre-extracted scalar (resolved by the factory from `const SOURCE`).
    pub fn feed(&mut self, value: f64) -> f64 {
        // Добавляем цену в буфер
        if self.data.len() >= self.period {
            self.data.remove(0);
        }
        self.data.push(value);
        self.count += 1;
        
        // Рассчитываем PE если достаточно данных
        let min_required = self.order + (self.order - 1) * self.delay;
        if self.data.len() >= min_required.max(10) {
            self.calculate_pe();
            self.initialized = true;
        }
        
        self.normalized_pe
    }
    
    /// Рассчитать энтропию перестановок
    fn calculate_pe(&mut self) {
        if self.data.len() < self.order {
            return;
        }
        
        // Извлекаем порядковые паттерны
        let patterns = self.extract_ordinal_patterns();
        
        if patterns.is_empty() {
            return;
        }
        
        // Подсчитываем частоты паттернов
        let mut pattern_counts: HashMap<Vec<usize>, usize> = HashMap::new();
        
        for pattern in patterns {
            *pattern_counts.entry(pattern).or_insert(0) += 1;
        }
        
        // Рассчитываем энтропию перестановок
        let total_patterns = pattern_counts.values().sum::<usize>() as f64;
        let mut entropy = 0.0;
        
        for &count in pattern_counts.values() {
            if count > 0 {
                let probability = count as f64 / total_patterns;
                entropy -= probability * probability.ln();
            }
        }
        
        // Нормализуем энтропию
        let max_entropy = (self.factorial(self.order) as f64).ln();
        
        self.pe = entropy;
        self.normalized_pe = if max_entropy > 0.0 {
            (entropy / max_entropy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        
        // Разнообразие паттернов = количество уникальных паттернов / максимально возможное
        let unique_patterns = pattern_counts.len() as f64;
        let max_possible_patterns = self.factorial(self.order) as f64;
        self.pattern_diversity = unique_patterns / max_possible_patterns;
    }
    
    /// Извлечь порядковые паттерны из данных
    fn extract_ordinal_patterns(&self) -> Vec<Vec<usize>> {
        let mut patterns = Vec::new();
        
        let n = self.data.len();
        let step = self.delay;
        
        // Для каждой возможной позиции
        for i in 0..=(n - self.order * step) {
            let mut values = Vec::new();
            let mut indices = Vec::new();
            
            // Собираем значения с учетом задержки
            for j in 0..self.order {
                let idx = i + j * step;
                if idx < n {
                    values.push(self.data[idx]);
                    indices.push(j);
                }
            }
            
            if values.len() == self.order {
                // Создаем порядковый паттерн
                let mut indexed_values: Vec<(f64, usize)> = values.into_iter().zip(indices).collect();
                indexed_values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                
                let ordinal_pattern: Vec<usize> = indexed_values.into_iter().map(|(_, idx)| idx).collect();
                patterns.push(ordinal_pattern);
            }
        }
        
        patterns
    }
    
    /// Вычислить факториал
    fn factorial(&self, n: usize) -> usize {
        if n <= 1 {
            1
        } else {
            (2..=n).product()
        }
    }
    
    /// Получить текущую энтропию перестановок
    pub fn pe(&self) -> f64 {
        self.pe
    }
    
    /// Получить нормализованную энтропию перестановок (0-1)
    pub fn normalized_pe(&self) -> f64 {
        self.normalized_pe
    }
    
    /// Получить разнообразие паттернов (0-1)
    pub fn pattern_diversity(&self) -> f64 {
        self.pattern_diversity
    }
    
    /// Определить состояние рынка
    pub fn market_state(&self) -> &'static str {
        match self.normalized_pe {
            p if p < 0.3 => "Highly Ordered",
            p if p < 0.6 => "Moderately Ordered",
            p if p < 0.8 => "Disordered",
            _ => "Highly Disordered",
        }
    }
    
    /// Получить торговый сигнал на основе упорядоченности
    pub fn trading_signal(&self) -> i8 {
        match self.normalized_pe {
            p if p < 0.4 => 1,  // Высокая упорядоченность - следуем паттерну
            p if p > 0.8 => -1, // Низкая упорядоченность - контртренд
            _ => 0,             // Нейтрально
        }
    }
    
    /// Получить значение для использования в других индикаторах
    pub fn value(&self) -> f64 {
        self.normalized_pe
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить порядок перестановки
    pub fn order(&self) -> usize {
        self.order
    }
    
    /// Получить задержку
    pub fn delay(&self) -> usize {
        self.delay
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.data.clear();
        self.pe = 0.0;
        self.normalized_pe = 0.0;
        self.pattern_diversity = 0.0;
        self.count = 0;
        self.initialized = false;
    }

}

impl Default for PermutationEntropy {
    /// Factory default (Pe arm): period=100, order=3, delay=1, source=Close.
    fn default() -> Self {
        Self::with_source(100, 3, 1, OhlcvField::Close)
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

/// Typed config for [`PermutationEntropy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PeConfig {
    pub period: Param<usize>,
    pub order: Param<usize>,
    pub delay: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for PermutationEntropy {
    const ID: IndicatorId = IndicatorId::Pe;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(n) per bar — extract ordinal patterns over the window (linear scan).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Pe)];
    type Config = PeConfig;
    type Runtime = PermutationEntropy;

    fn create(cfg: PeConfig) -> PermutationEntropy {
        PermutationEntropy::with_source(
            cfg.period.resolved(),
            cfg.order.resolved(),
            cfg.delay.resolved(),
            cfg.source.resolved(),
        )
    }

    fn source_fields(cfg: &PeConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for PeConfig {
    fn defaults() -> Self {
        PeConfig {
            period: Param::Solo(100),
            order: Param::Solo(3),
            delay: Param::Solo(1),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // order: Class B (permutation entropy order) — bounded by combinatorial explosion, 2..=8
        s.order = Param::range(2, 8, 1);
        // delay: Class A.lag subcase (permutation entropy delay) — 1..=10
        s.delay = Param::range(1, 10, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for PermutationEntropy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Pe, "Perm Entropy", Color::hex(0x3F51B5))
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
    fn test_permutation_entropy_creation() {
        let pe = PermutationEntropy::new(50, 3, 1);
        assert!(!pe.is_ready());
        assert_eq!(pe.value(), 0.0);
        assert_eq!(pe.period(), 50);
        assert_eq!(pe.order(), 3);
        assert_eq!(pe.delay(), 1);
    }

    #[test]
    fn test_permutation_entropy_default() {
        let pe = PermutationEntropy::new_default(30);
        assert!(!pe.is_ready());
        assert_eq!(pe.period(), 30);
        assert_eq!(pe.order(), 3);
    }

    #[test]
    fn test_permutation_entropy_warmup() {
        let mut pe = PermutationEntropy::new_default(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pe.feed(price);
        }
        assert!(pe.is_ready());
    }

    #[test]
    fn test_permutation_entropy_values_range() {
        let mut pe = PermutationEntropy::new_default(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pe.feed(price);
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_permutation_entropy_reset() {
        let mut pe = PermutationEntropy::new_default(20);
        for i in 0..25 {
            pe.feed(100.0 + i as f64);
        }
        pe.reset();
        assert!(!pe.is_ready());
        assert_eq!(pe.value(), 0.0);
    }

    #[test]
    fn test_permutation_entropy_pattern_diversity() {
        let mut pe = PermutationEntropy::new_default(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pe.feed(price);
        }
        let diversity = pe.pattern_diversity();
        assert!(diversity >= 0.0 && diversity <= 1.0);
    }

    #[test]
    fn factory_feeds_resolved_pe() {
        
        let mut f = IndicatorOrder::Pe(<<PermutationEntropy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
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
        let v = f.read(IndicatorOutputId::Pe);
        assert!(v >= 0.0 && v <= 1.0, "pe out of [0,1]: {v}");
    }
} 






















