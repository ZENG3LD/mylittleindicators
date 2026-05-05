//! High-performance moving average abstraction for indicators
//! Делегирует всю логику оптимизированным структурам базовых MA.
//!
//! This module provides a unified MovingAverage enum and factory for fast, lightweight
//! smoothing operations inside indicators (RSI, ATR, Bollinger Bands, etc.).
//!
//! Design principles:
//! - Only FAST variants (Wma, Hma, Ama optimized)
//! - Only LIGHTWEIGHT types (no Frama, Alma, T3, McGinley, Lr)
//! - Dual-mode support via MovingAverageWithField for OHLCV fields
//! - Stack allocation only (no Box, no dyn)
//!
//! For heavyweight or specialized MAs, use them directly as standalone indicators.

use crate::bar_indicators::average::sma::Sma;
use crate::bar_indicators::average::wma::Wma;
use crate::bar_indicators::average::rma::Rma;
use crate::bar_indicators::average::ema::Ema;
use crate::bar_indicators::average::dema::Dema;
use crate::bar_indicators::average::hma::Hma;
use crate::bar_indicators::average::tema::Tema;
use crate::bar_indicators::average::tma::Tma;
use crate::bar_indicators::average::vwap::Vwap;
use crate::bar_indicators::average::vwma::Vwma;
use crate::bar_indicators::average::ama::Ama;
use crate::bar_indicators::indicator_value::IndicatorValue;

// Re-export OhlcvField from its new location for backward compatibility
// The canonical location is now crate::bar_indicators::ohlcv_field::OhlcvField
pub use crate::bar_indicators::ohlcv_field::OhlcvField;

// Unified MovingAverageType enum with UPPERCASE names matching catalog IDs
// This ensures consistency between enum variants, catalog IDs, and factory mappings
// WMA/HMA refer to the FAST variants from the factory (Wma/Hma implementations)
// AMA refers to AMA_RING implementation from the factory

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MovingAverageType {
    SMA,   // Simple Moving Average
    WMA,   // Weighted Moving Average (fast variant)
    EMA,   // Exponential Moving Average
    RMA,   // Wilder's Moving Average (RMA)
    DEMA,  // Double Exponential Moving Average
    TEMA,  // Triple Exponential Moving Average
    HMA,   // Hull Moving Average (fast variant)
    TMA,   // Triangular Moving Average
    VWMA,  // Volume-Weighted Moving Average
    VWAP,  // Volume-Weighted Average Price
    AMA,   // Adaptive Moving Average (AMA_RING)
}

// Display trait: just return the variant name (already matches catalog IDs)
impl std::fmt::Display for MovingAverageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Lightweight MovingAverageProvider enum - internal factory for fast smoothing operations.
/// Contains only optimized variants suitable for internal use in indicators.
///
/// ⚠️  This is an INTERNAL PROVIDER - indicators should work with MovingAverageType (catalog enum).
/// ⚠️  Direct pattern matching on this enum is discouraged - store MovingAverageType instead.
///
/// For heavyweight MAs (Frama, Alma, T3, McGinley, Lr), use them directly
/// as standalone indicators from IndicatorInstance.
#[derive(Debug, Clone)]
pub enum MovingAverageProvider {
    Sma(Sma),
    Wma(Wma),
    Rma(Rma),
    Ema(Ema),
    Dema(Dema),
    Tema(Tema),
    Tma(Tma),
    Hma(Hma),
    Ama(Ama),
    Vwma(Vwma),
    Vwap(Vwap),
}

impl MovingAverageProvider {
    pub fn period(&self) -> usize {
        match self {
            Self::Sma(ma) => ma.period(),
            Self::Wma(ma) => ma.period(),
            Self::Rma(ma) => ma.period(),
            Self::Ema(ma) => ma.period(),
            Self::Dema(ma) => ma.period(),
            Self::Tema(ma) => ma.period(),
            Self::Tma(ma) => ma.period(),
            Self::Hma(ma) => ma.period(),
            Self::Ama(ma) => ma.period(),
            Self::Vwma(ma) => ma.period(),
            Self::Vwap(ma) => ma.period(),
        }
    }
    pub fn new(ma_type: MovingAverageType, period: usize) -> Self {
        // Enforce minimal / maximal period constraints for certain MA implementations
        // - HMA implementation uses fixed-size arrays length MAX_PERIOD (=64)
        // - Efficiency Ratio inside AMA requires period > 1
        // Add other guards as necessary to avoid runtime panics during massive grid search

        match ma_type {
            MovingAverageType::SMA => Self::Sma(Sma::new(period.max(1))),
            MovingAverageType::WMA => Self::Wma(Wma::new(period.max(1))),
            MovingAverageType::EMA => Self::Ema(Ema::new(period.max(1))),
            MovingAverageType::RMA => Self::Rma(Rma::new(period.max(1))),
            MovingAverageType::DEMA => Self::Dema(Dema::new(period.max(1))),
            MovingAverageType::TEMA => Self::Tema(Tema::new(period.max(1))),
            MovingAverageType::HMA => Self::Hma(Hma::new(period)),
            MovingAverageType::TMA => Self::Tma(Tma::new(period.max(1))),
            MovingAverageType::VWMA => Self::Vwma(Vwma::new(period.max(1))),
            MovingAverageType::VWAP => Self::Vwap(Vwap::new(period.max(1))),
            MovingAverageType::AMA => {
                let p = period.max(2);
                Self::Ama(Ama::new(p, 2, 30))
            }
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        match self {
            Self::Sma(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Wma(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Rma(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Ema(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Dema(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Tema(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Tma(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Hma(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Ama(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Vwma(ma) => ma.update_bar(open, high, low, close, volume),
            Self::Vwap(ma) => ma.update_bar(open, high, low, close, volume),
        }
    }

    pub fn value(&self) -> IndicatorValue {
        match self {
            Self::Sma(ma) => ma.value(),
            Self::Wma(ma) => ma.value(),
            Self::Rma(ma) => ma.value(),
            Self::Ema(ma) => ma.value(),
            Self::Dema(ma) => ma.value(),
            Self::Tema(ma) => ma.value(),
            Self::Tma(ma) => ma.value(),
            Self::Hma(ma) => ma.value(),
            Self::Ama(ma) => ma.value(),
            Self::Vwma(ma) => ma.value(),
            Self::Vwap(ma) => ma.value(),
        }
    }

    pub fn is_ready(&self) -> bool {
        match self {
            Self::Sma(ma) => ma.is_ready(),
            Self::Wma(ma) => ma.is_ready(),
            Self::Rma(ma) => ma.is_ready(),
            Self::Ema(ma) => ma.is_ready(),
            Self::Dema(ma) => ma.is_ready(),
            Self::Tema(ma) => ma.is_ready(),
            Self::Tma(ma) => ma.is_ready(),
            Self::Hma(ma) => ma.is_ready(),
            Self::Ama(ma) => ma.is_ready(),
            Self::Vwma(ma) => ma.is_ready(),
            Self::Vwap(ma) => ma.is_ready(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Sma(ma) => ma.reset(),
            Self::Wma(ma) => ma.reset(),
            Self::Rma(ma) => ma.reset(),
            Self::Ema(ma) => ma.reset(),
            Self::Dema(ma) => ma.reset(),
            Self::Tema(ma) => ma.reset(),
            Self::Tma(ma) => ma.reset(),
            Self::Hma(ma) => ma.reset(),
            Self::Ama(ma) => ma.reset(),
            Self::Vwma(ma) => ma.reset(),
            Self::Vwap(ma) => ma.reset(),
        }
    }
}

/// Dual-mode MovingAverage with OHLCV field selection.
///
/// Wraps a MovingAverageProvider and extracts the specified OHLCV field
/// before feeding it to the MA. This replaces the legacy OhlcvAverage trait.
///
/// Usage:
/// ```rust
/// use zengeld_chart_indicators::bar_indicators::average::{
///     MovingAverageType, MovingAverageProvider, MovingAverageWithField, OhlcvField
/// };
///
/// // MA on Close (default) - use MovingAverageProvider directly
/// let mut ma = MovingAverageProvider::new(MovingAverageType::SMA, 20);
/// let val = ma.update_bar(100.0, 105.0, 99.0, 103.0, 1000.0);
///
/// // MA on specific field - use MovingAverageWithField
/// let mut ma_high = MovingAverageWithField::new(MovingAverageType::SMA, 20, OhlcvField::High);
/// let val_high = ma_high.update_bar(100.0, 105.0, 99.0, 103.0, 1000.0);
/// ```
#[derive(Debug, Clone)]
pub struct MovingAverageWithField {
    ma: MovingAverageProvider,
    field: OhlcvField,
}

impl MovingAverageWithField {
    pub fn new(ma_type: MovingAverageType, period: usize, field: OhlcvField) -> Self {
        Self {
            ma: MovingAverageProvider::new(ma_type, period),
            field,
        }
    }

    #[inline]
    fn extract_field(&self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.field.extract(open, high, low, close, volume)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.extract_field(open, high, low, close, volume);
        // Pass extracted value as all OHLCV fields (MA only uses close anyway)
        self.ma.update_bar(value, value, value, value, value)
    }

    pub fn value(&self) -> IndicatorValue {
        self.ma.value()
    }

    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }

    pub fn reset(&mut self) {
        self.ma.reset()
    }

    pub fn period(&self) -> usize {
        self.ma.period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average_type_display() {
        assert_eq!(format!("{}", MovingAverageType::SMA), "SMA");
        assert_eq!(format!("{}", MovingAverageType::EMA), "EMA");
        assert_eq!(format!("{}", MovingAverageType::HMA), "HMA");
    }

    #[test]
    fn test_provider_all_types() {
        let types = [
            MovingAverageType::SMA,
            MovingAverageType::WMA,
            MovingAverageType::EMA,
            MovingAverageType::RMA,
            MovingAverageType::DEMA,
            MovingAverageType::TEMA,
            MovingAverageType::HMA,
            MovingAverageType::TMA,
            MovingAverageType::VWMA,
            MovingAverageType::VWAP,
            MovingAverageType::AMA,
        ];

        for ma_type in types {
            let mut ma = MovingAverageProvider::new(ma_type, 10);
            for i in 1..=20 {
                ma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 1000.0);
            }
            assert!(ma.value().main() > 0.0, "{:?} should produce value", ma_type);
        }
    }

    #[test]
    fn test_provider_reset() {
        let mut ma = MovingAverageProvider::new(MovingAverageType::SMA, 5);
        for i in 1..=10 {
            ma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(ma.is_ready());

        ma.reset();
        assert!(!ma.is_ready());
    }

    #[test]
    fn test_with_field_close() {
        let mut ma = MovingAverageWithField::new(MovingAverageType::SMA, 3, OhlcvField::Close);
        ma.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        ma.update_bar(105.0, 115.0, 95.0, 110.0, 1000.0);
        ma.update_bar(110.0, 120.0, 100.0, 115.0, 1000.0);

        assert!(ma.is_ready());
        // SMA of Close: (105+110+115)/3 = 110
        assert!((ma.value().main() - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_with_field_high() {
        let mut ma = MovingAverageWithField::new(MovingAverageType::SMA, 3, OhlcvField::High);
        ma.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        ma.update_bar(105.0, 115.0, 95.0, 110.0, 1000.0);
        ma.update_bar(110.0, 120.0, 100.0, 115.0, 1000.0);

        assert!(ma.is_ready());
        // SMA of High: (110+115+120)/3 = 115
        assert!((ma.value().main() - 115.0).abs() < 0.01);
    }

    #[test]
    fn test_with_field_low() {
        let mut ma = MovingAverageWithField::new(MovingAverageType::SMA, 3, OhlcvField::Low);
        ma.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        ma.update_bar(105.0, 115.0, 95.0, 110.0, 1000.0);
        ma.update_bar(110.0, 120.0, 100.0, 115.0, 1000.0);

        assert!(ma.is_ready());
        // SMA of Low: (90+95+100)/3 = 95
        assert!((ma.value().main() - 95.0).abs() < 0.01);
    }

}


















