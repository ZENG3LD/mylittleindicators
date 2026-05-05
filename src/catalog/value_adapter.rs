//! Value adapter for extracting render-ready values from IndicatorValue
//!
//! This module provides utilities to decompose complex IndicatorValue variants
//! into individual f64 values suitable for rendering.

use crate::bar_indicators::indicator_value::IndicatorValue;
use super::rendering::{
    ValueExtractor, ChannelPart, MacdPart, IchimokuPart, DoublePart, TriplePart,
    CandlePart, AdaptivePart, VolatilityPart, StatTestPart, CandleAnatomyPart, HilbertPart,
    OutputSpec,
};

/// Adapter for extracting values from IndicatorValue
pub struct ValueAdapter;

impl ValueAdapter {
    /// Extract a specific value from IndicatorValue based on extractor
    pub fn extract(value: &IndicatorValue, extractor: &ValueExtractor) -> Option<f64> {
        match (value, extractor) {
            // Main extractor - works with any variant using .main()
            (_, ValueExtractor::Main) => Some(value.main()),

            // Signal extractor
            (IndicatorValue::Signal(s), ValueExtractor::Signal) => Some(*s as f64),
            (_, ValueExtractor::Signal) => None,

            // Flag extractor
            (IndicatorValue::Flag(b), ValueExtractor::Flag) => Some(if *b { 1.0 } else { 0.0 }),
            (_, ValueExtractor::Flag) => None,

            // Channel extractors
            (IndicatorValue::Channel3 { upper, .. }, ValueExtractor::Channel(ChannelPart::Upper)) => Some(*upper),
            (IndicatorValue::Channel3 { middle, .. }, ValueExtractor::Channel(ChannelPart::Middle)) => Some(*middle),
            (IndicatorValue::Channel3 { lower, .. }, ValueExtractor::Channel(ChannelPart::Lower)) => Some(*lower),

            (IndicatorValue::ChannelExtended { upper, .. }, ValueExtractor::Channel(ChannelPart::Upper)) => Some(*upper),
            (IndicatorValue::ChannelExtended { middle, .. }, ValueExtractor::Channel(ChannelPart::Middle)) => Some(*middle),
            (IndicatorValue::ChannelExtended { lower, .. }, ValueExtractor::Channel(ChannelPart::Lower)) => Some(*lower),
            (IndicatorValue::ChannelExtended { bandwidth, .. }, ValueExtractor::Channel(ChannelPart::Bandwidth)) => Some(*bandwidth),
            (IndicatorValue::ChannelExtended { percent_b, .. }, ValueExtractor::Channel(ChannelPart::PercentB)) => Some(*percent_b),
            (_, ValueExtractor::Channel(_)) => None,

            // MACD extractors
            (IndicatorValue::Macd { line, .. }, ValueExtractor::Macd(MacdPart::Line)) => Some(*line),
            (IndicatorValue::Macd { signal, .. }, ValueExtractor::Macd(MacdPart::Signal)) => Some(*signal),
            (IndicatorValue::Macd { histogram, .. }, ValueExtractor::Macd(MacdPart::Histogram)) => Some(*histogram),
            (_, ValueExtractor::Macd(_)) => None,

            // Ichimoku extractors
            (IndicatorValue::Ichimoku { tenkan, .. }, ValueExtractor::Ichimoku(IchimokuPart::Tenkan)) => Some(*tenkan),
            (IndicatorValue::Ichimoku { kijun, .. }, ValueExtractor::Ichimoku(IchimokuPart::Kijun)) => Some(*kijun),
            (IndicatorValue::Ichimoku { senkou_a, .. }, ValueExtractor::Ichimoku(IchimokuPart::SenkouA)) => Some(*senkou_a),
            (IndicatorValue::Ichimoku { senkou_b, .. }, ValueExtractor::Ichimoku(IchimokuPart::SenkouB)) => Some(*senkou_b),
            (IndicatorValue::Ichimoku { chikou, .. }, ValueExtractor::Ichimoku(IchimokuPart::Chikou)) => Some(*chikou),
            (_, ValueExtractor::Ichimoku(_)) => None,

            // Double extractors
            (IndicatorValue::Double(a, _), ValueExtractor::Double(DoublePart::First)) => Some(*a),
            (IndicatorValue::Double(_, b), ValueExtractor::Double(DoublePart::Second)) => Some(*b),
            (_, ValueExtractor::Double(_)) => None,

            // Triple extractors
            (IndicatorValue::Triple(a, _, _), ValueExtractor::Triple(TriplePart::First)) => Some(*a),
            (IndicatorValue::Triple(_, b, _), ValueExtractor::Triple(TriplePart::Second)) => Some(*b),
            (IndicatorValue::Triple(_, _, c), ValueExtractor::Triple(TriplePart::Third)) => Some(*c),
            (_, ValueExtractor::Triple(_)) => None,

            // Candle extractors
            (IndicatorValue::Candle { open, .. }, ValueExtractor::Candle(CandlePart::Open)) => Some(*open),
            (IndicatorValue::Candle { high, .. }, ValueExtractor::Candle(CandlePart::High)) => Some(*high),
            (IndicatorValue::Candle { low, .. }, ValueExtractor::Candle(CandlePart::Low)) => Some(*low),
            (IndicatorValue::Candle { close, .. }, ValueExtractor::Candle(CandlePart::Close)) => Some(*close),
            (_, ValueExtractor::Candle(_)) => None,

            // Adaptive extractors
            (IndicatorValue::Adaptive { value, .. }, ValueExtractor::Adaptive(AdaptivePart::Value)) => Some(*value),
            (IndicatorValue::Adaptive { period, .. }, ValueExtractor::Adaptive(AdaptivePart::Period)) => Some(*period),
            (IndicatorValue::Adaptive { alpha, .. }, ValueExtractor::Adaptive(AdaptivePart::Alpha)) => Some(*alpha),
            (_, ValueExtractor::Adaptive(_)) => None,

            // Volatility extractors
            (IndicatorValue::Volatility { total, .. }, ValueExtractor::Volatility(VolatilityPart::Total)) => Some(*total),
            (IndicatorValue::Volatility { close_close, .. }, ValueExtractor::Volatility(VolatilityPart::CloseClose)) => Some(*close_close),
            (IndicatorValue::Volatility { high_low, .. }, ValueExtractor::Volatility(VolatilityPart::HighLow)) => Some(*high_low),
            (_, ValueExtractor::Volatility(_)) => None,

            // StatTest extractors
            (IndicatorValue::StatTest { statistic, .. }, ValueExtractor::StatTest(StatTestPart::Statistic)) => Some(*statistic),
            (IndicatorValue::StatTest { p_value, .. }, ValueExtractor::StatTest(StatTestPart::PValue)) => Some(*p_value),
            (_, ValueExtractor::StatTest(_)) => None,

            // CandleAnatomy extractors
            (IndicatorValue::CandleAnatomy { body, .. }, ValueExtractor::CandleAnatomy(CandleAnatomyPart::Body)) => Some(*body),
            (IndicatorValue::CandleAnatomy { upper_wick, .. }, ValueExtractor::CandleAnatomy(CandleAnatomyPart::UpperWick)) => Some(*upper_wick),
            (IndicatorValue::CandleAnatomy { lower_wick, .. }, ValueExtractor::CandleAnatomy(CandleAnatomyPart::LowerWick)) => Some(*lower_wick),
            (_, ValueExtractor::CandleAnatomy(_)) => None,

            // Hilbert extractors
            (IndicatorValue::Hilbert { amplitude, .. }, ValueExtractor::Hilbert(HilbertPart::Amplitude)) => Some(*amplitude),
            (IndicatorValue::Hilbert { phase, .. }, ValueExtractor::Hilbert(HilbertPart::Phase)) => Some(*phase),
            (IndicatorValue::Hilbert { frequency, .. }, ValueExtractor::Hilbert(HilbertPart::Frequency)) => Some(*frequency),
            (_, ValueExtractor::Hilbert(_)) => None,
        }
    }

    /// Decompose IndicatorValue into all named outputs based on output specs
    pub fn decompose(value: &IndicatorValue, outputs: &[OutputSpec]) -> Vec<(String, f64)> {
        outputs
            .iter()
            .filter_map(|spec| {
                Self::extract(value, &spec.value_extractor)
                    .map(|v| (spec.name.clone(), v))
            })
            .collect()
    }

    /// Extract channel values (upper, middle, lower) from IndicatorValue
    pub fn extract_channel(value: &IndicatorValue) -> Option<(f64, f64, f64)> {
        match value {
            IndicatorValue::Channel3 { upper, middle, lower } => Some((*upper, *middle, *lower)),
            IndicatorValue::ChannelExtended { upper, middle, lower, .. } => Some((*upper, *middle, *lower)),
            _ => None,
        }
    }

    /// Extract MACD values (line, signal, histogram) from IndicatorValue
    pub fn extract_macd(value: &IndicatorValue) -> Option<(f64, f64, f64)> {
        match value {
            IndicatorValue::Macd { line, signal, histogram } => Some((*line, *signal, *histogram)),
            _ => None,
        }
    }

    /// Extract Ichimoku values from IndicatorValue
    pub fn extract_ichimoku(value: &IndicatorValue) -> Option<(f64, f64, f64, f64, f64)> {
        match value {
            IndicatorValue::Ichimoku { tenkan, kijun, senkou_a, senkou_b, chikou } => {
                Some((*tenkan, *kijun, *senkou_a, *senkou_b, *chikou))
            }
            _ => None,
        }
    }

    /// Check if IndicatorValue is a channel type
    pub fn is_channel(value: &IndicatorValue) -> bool {
        matches!(value, IndicatorValue::Channel3 { .. } | IndicatorValue::ChannelExtended { .. })
    }

    /// Check if IndicatorValue is a MACD type
    pub fn is_macd(value: &IndicatorValue) -> bool {
        matches!(value, IndicatorValue::Macd { .. })
    }

    /// Check if IndicatorValue is an Ichimoku type
    pub fn is_ichimoku(value: &IndicatorValue) -> bool {
        matches!(value, IndicatorValue::Ichimoku { .. })
    }

    /// Get the number of outputs for a given IndicatorValue
    pub fn output_count(value: &IndicatorValue) -> usize {
        match value {
            IndicatorValue::Single(_) => 1,
            IndicatorValue::Signal(_) => 1,
            IndicatorValue::Flag(_) => 1,
            IndicatorValue::Double(_, _) => 2,
            IndicatorValue::Triple(_, _, _) => 3,
            IndicatorValue::Channel3 { .. } => 3,
            IndicatorValue::ChannelExtended { .. } => 5,
            IndicatorValue::Macd { .. } => 3,
            IndicatorValue::Ichimoku { .. } => 5,
            IndicatorValue::Candle { .. } => 4,
            IndicatorValue::Adaptive { .. } => 3,
            IndicatorValue::StatTest { .. } => 2,
            IndicatorValue::Volatility { .. } => 3,
            IndicatorValue::ValueFlag(_, _) => 2,
            IndicatorValue::DoubleFlag(_, _) => 2,
            IndicatorValue::FuzzyCandle { .. } => 5,
            IndicatorValue::CandleAnatomy { .. } => 3,
            IndicatorValue::Hilbert { .. } => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_main() {
        let value = IndicatorValue::Single(42.5);
        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Main),
            Some(42.5)
        );
    }

    #[test]
    fn test_extract_channel() {
        let value = IndicatorValue::Channel3 {
            upper: 110.0,
            middle: 100.0,
            lower: 90.0,
        };

        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Channel(ChannelPart::Upper)),
            Some(110.0)
        );
        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Channel(ChannelPart::Middle)),
            Some(100.0)
        );
        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Channel(ChannelPart::Lower)),
            Some(90.0)
        );
    }

    #[test]
    fn test_extract_macd() {
        let value = IndicatorValue::Macd {
            line: 0.5,
            signal: 0.3,
            histogram: 0.2,
        };

        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Macd(MacdPart::Line)),
            Some(0.5)
        );
        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Macd(MacdPart::Signal)),
            Some(0.3)
        );
        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Macd(MacdPart::Histogram)),
            Some(0.2)
        );
    }

    #[test]
    fn test_extract_ichimoku() {
        let value = IndicatorValue::Ichimoku {
            tenkan: 100.0,
            kijun: 99.0,
            senkou_a: 101.0,
            senkou_b: 98.0,
            chikou: 100.5,
        };

        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Ichimoku(IchimokuPart::Tenkan)),
            Some(100.0)
        );
        assert_eq!(
            ValueAdapter::extract(&value, &ValueExtractor::Ichimoku(IchimokuPart::SenkouB)),
            Some(98.0)
        );
    }

    #[test]
    fn test_decompose() {
        let value = IndicatorValue::Macd {
            line: 0.5,
            signal: 0.3,
            histogram: 0.2,
        };

        let outputs = vec![
            OutputSpec::line("macd", "MACD", "#2196F3", 2.0, ValueExtractor::Macd(MacdPart::Line)),
            OutputSpec::line("signal", "Signal", "#FF5722", 1.0, ValueExtractor::Macd(MacdPart::Signal)),
            OutputSpec::histogram("histogram", "Histogram", "#4CAF50", ValueExtractor::Macd(MacdPart::Histogram)),
        ];

        let result = ValueAdapter::decompose(&value, &outputs);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("macd".to_string(), 0.5));
        assert_eq!(result[1], ("signal".to_string(), 0.3));
        assert_eq!(result[2], ("histogram".to_string(), 0.2));
    }

    #[test]
    fn test_extract_channel_helper() {
        let value = IndicatorValue::Channel3 {
            upper: 110.0,
            middle: 100.0,
            lower: 90.0,
        };

        let (upper, middle, lower) = ValueAdapter::extract_channel(&value).unwrap();
        assert_eq!(upper, 110.0);
        assert_eq!(middle, 100.0);
        assert_eq!(lower, 90.0);
    }

    #[test]
    fn test_output_count() {
        assert_eq!(ValueAdapter::output_count(&IndicatorValue::Single(0.0)), 1);
        assert_eq!(ValueAdapter::output_count(&IndicatorValue::Double(0.0, 0.0)), 2);
        assert_eq!(ValueAdapter::output_count(&IndicatorValue::Macd { line: 0.0, signal: 0.0, histogram: 0.0 }), 3);
        assert_eq!(ValueAdapter::output_count(&IndicatorValue::Ichimoku { tenkan: 0.0, kijun: 0.0, senkou_a: 0.0, senkou_b: 0.0, chikou: 0.0 }), 5);
    }

    #[test]
    fn test_type_checks() {
        let channel = IndicatorValue::Channel3 { upper: 0.0, middle: 0.0, lower: 0.0 };
        let macd = IndicatorValue::Macd { line: 0.0, signal: 0.0, histogram: 0.0 };
        let ichimoku = IndicatorValue::Ichimoku { tenkan: 0.0, kijun: 0.0, senkou_a: 0.0, senkou_b: 0.0, chikou: 0.0 };
        let single = IndicatorValue::Single(0.0);

        assert!(ValueAdapter::is_channel(&channel));
        assert!(!ValueAdapter::is_channel(&single));

        assert!(ValueAdapter::is_macd(&macd));
        assert!(!ValueAdapter::is_macd(&single));

        assert!(ValueAdapter::is_ichimoku(&ichimoku));
        assert!(!ValueAdapter::is_ichimoku(&single));
    }

    #[test]
    fn test_mismatch_returns_none() {
        let single = IndicatorValue::Single(42.0);

        // Trying to extract MACD from a Single value should return None
        assert_eq!(
            ValueAdapter::extract(&single, &ValueExtractor::Macd(MacdPart::Line)),
            None
        );

        // But Main should still work
        assert_eq!(
            ValueAdapter::extract(&single, &ValueExtractor::Main),
            Some(42.0)
        );
    }
}
