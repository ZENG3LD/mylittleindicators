// High-performance Highest indicator
// Поиск максимума за N периодов с эффективным буфером
// (c) 2024


#[derive(Clone, Debug)]
pub struct Highest {
    period: usize,
    buffer: Vec<f64>,
    index: usize,
    filled: bool,
    value: f64,
}

impl Highest {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: Vec::with_capacity(period),
            index: 0,
            filled: false,
            value: 0.0,
        }
    }
    
    /// Обновить Highest новым баром (используется high)
    /// Feed ONE resolved scalar (high) — the factory extracts the High field.
    pub fn feed(&mut self, value: f64) -> f64 {
        self.update(value)
    }
    
    /// Обновить Highest новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(value);
        } else {
            // Циклический буфер - заменяем старое значение
            self.buffer[self.index] = value;
            self.index = (self.index + 1) % self.period;
            self.filled = true;
        }
        
        // Эффективный поиск максимума в буфере
        self.value = self.buffer.iter().copied().fold(f64::NEG_INFINITY, f64::max);
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

/// Own config for [`Highest`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HighestConfig {
    pub period: Param<usize>,
}

impl Indicator for Highest {
    const ID: IndicatorId = IndicatorId::Highest;
    /// No family — Highest is an atomic PRODUCER, not a pluggable family member
    /// (Highest ≠ Lowest, not interchangeable). The rolling highest-high it emits is
    /// consumed by Stochastic / Williams %R / Donchian / Aroon via a `Port` to its
    /// `value` port — a port relationship, not a taxonomy.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the bar high — the factory feeds `high` into the rolling max.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::High]));
    /// O(period): each bar folds the window for the max. The window is a period-deep
    /// `Vec` — its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Highest)];
    type Config = HighestConfig;
    type Runtime = Highest;

    fn create(cfg: HighestConfig) -> Highest {
        Highest::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for HighestConfig {
    fn defaults() -> Self {
        HighestConfig { period: Param::Solo(20) }
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


impl Render for Highest {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Highest, "Highest", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highest_creation() {
        let highest = Highest::new(10);
        assert!(!highest.is_ready());
        assert_eq!(highest.value(), 0.0);
        assert_eq!(highest.period(), 10);
    }

    #[test]
    fn test_highest_finds_max() {
        let mut highest = Highest::new(5);
        highest.update(10.0);
        highest.update(20.0);
        highest.update(15.0);
        highest.update(25.0);
        highest.update(18.0);
        assert!(highest.is_ready());
        assert_eq!(highest.value(), 25.0);
    }

    #[test]
    fn test_highest_rolling_window() {
        let mut highest = Highest::new(3);
        highest.update(10.0);
        highest.update(20.0);
        highest.update(15.0);
        assert_eq!(highest.value(), 20.0);
        highest.update(5.0); // pushes out 10
        assert_eq!(highest.value(), 20.0);
        highest.update(3.0); // pushes out 20
        assert_eq!(highest.value(), 15.0);
    }

    #[test]
    fn test_highest_update_bar() {
        let mut highest = Highest::new(5);
        for i in 1..=10 {
            let high = 100.0 + i as f64;
            highest.feed(high);
        }
        assert!(highest.is_ready());
        // Last 5 highs: 106, 107, 108, 109, 110
        assert_eq!(highest.value(), 110.0);
    }

    #[test]
    fn test_highest_reset() {
        let mut highest = Highest::new(5);
        for i in 1..=10 {
            highest.update(100.0 + i as f64);
        }
        assert!(highest.is_ready());
        highest.reset();
        assert!(!highest.is_ready());
        assert_eq!(highest.value(), 0.0);
    }
}






















