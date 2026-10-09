// High-performance Lowest indicator
// Поиск минимума за N периодов с эффективным буфером
// (c) 2024


#[derive(Clone, Debug)]
pub struct Lowest {
    period: usize,
    buffer: Vec<f64>,
    index: usize,
    filled: bool,
    value: f64,
}

impl Lowest {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: Vec::with_capacity(period),
            index: 0,
            filled: false,
            value: 0.0,
        }
    }
    
    /// Обновить Lowest новым баром (используется low)
    /// Feed ONE resolved scalar (low) — the factory extracts the Low field.
    pub fn feed(&mut self, value: f64) -> f64 {
        self.update(value)
    }
    
    /// Обновить Lowest новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(value);
        } else {
            // Циклический буфер - заменяем старое значение
            self.buffer[self.index] = value;
            self.index = (self.index + 1) % self.period;
            self.filled = true;
        }
        
        // Эффективный поиск минимума в буфере
        self.value = self.buffer.iter().copied().fold(f64::INFINITY, f64::min);
        self.value
    }
    
    pub fn value(&self) -> f64 {
        self.value
    }
    
    pub fn is_ready(&self) -> bool {
        self.filled || self.buffer.len() == self.period
    }
    
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.index = 0;
        self.filled = false;
        self.value = 0.0;
    }
    
    pub fn period(&self) -> usize {
        self.period
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`Lowest`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LowestConfig {
    pub period: Param<usize>,
}

impl Indicator for Lowest {
    const ID: IndicatorId = IndicatorId::Lowest;
    /// No family — Lowest is an atomic PRODUCER, not a pluggable family member
    /// (Lowest ≠ Highest, not interchangeable). The rolling lowest-low it emits is
    /// consumed by Stochastic / Williams %R / Donchian / Aroon via a `Port` to its
    /// `value` port — a port relationship, not a taxonomy.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the bar low — the factory feeds `low` into the rolling min.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Low]));
    /// O(period): each bar folds the window for the min. The window is a period-deep
    /// `Vec` — its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Lowest)];
    type Config = LowestConfig;
    type Runtime = Lowest;

    fn create(cfg: LowestConfig) -> Lowest {
        Lowest::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for LowestConfig {
    fn defaults() -> Self {
        LowestConfig { period: Param::Solo(20) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1) — single axis, auto suffices
    }
}


impl Render for Lowest {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Lowest, "Lowest", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowest_creation() {
        let lowest = Lowest::new(10);
        assert!(!lowest.is_ready());
        assert_eq!(lowest.value(), 0.0);
        assert_eq!(lowest.period(), 10);
    }

    #[test]
    fn test_lowest_finds_min() {
        let mut lowest = Lowest::new(5);
        lowest.update(20.0);
        lowest.update(10.0);
        lowest.update(15.0);
        lowest.update(5.0);
        lowest.update(18.0);
        assert!(lowest.is_ready());
        assert_eq!(lowest.value(), 5.0);
    }

    #[test]
    fn test_lowest_rolling_window() {
        let mut lowest = Lowest::new(3);
        lowest.update(20.0);
        lowest.update(10.0);
        lowest.update(15.0);
        assert_eq!(lowest.value(), 10.0);
        lowest.update(25.0); // pushes out 20
        assert_eq!(lowest.value(), 10.0);
        lowest.update(30.0); // pushes out 10
        assert_eq!(lowest.value(), 15.0);
    }

    #[test]
    fn test_lowest_update_bar() {
        let mut lowest = Lowest::new(5);
        for i in 1..=10 {
            let low = 100.0 - i as f64;
            lowest.feed(low);
        }
        assert!(lowest.is_ready());
        // Last 5 lows: 94, 93, 92, 91, 90
        assert_eq!(lowest.value(), 90.0);
    }

    #[test]
    fn test_lowest_reset() {
        let mut lowest = Lowest::new(5);
        for i in 1..=10 {
            lowest.update(100.0 - i as f64);
        }
        assert!(lowest.is_ready());
        lowest.reset();
        assert!(!lowest.is_ready());
        assert_eq!(lowest.value(), 0.0);
    }
}






















