//! Detector configuration types
//!
//! This module defines the configuration primitives for signal detectors.
//! Uses BarIndicatorId for type-safe indicator identification.

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Source for extracting values from IndicatorValue
#[derive(Debug, Clone, PartialEq)]
pub enum ValueSource {
    /// Main/primary value (uses IndicatorValue::main())
    Main,

    /// Current price (passed separately to engine.process)
    Price,

    /// First value (for Double, Triple, Channel3 upper)
    First,

    /// Second value (for Double, Triple, Channel3 middle)
    Second,

    /// Third value (for Triple, Channel3 lower)
    Third,

    /// MACD line (for Macd variant)
    MacdLine,

    /// MACD signal (for Macd variant)
    MacdSignal,

    /// MACD histogram (for Macd variant)
    MacdHistogram,

    /// Channel upper band
    ChannelUpper,

    /// Channel middle
    ChannelMiddle,

    /// Channel lower band
    ChannelLower,

    /// Ichimoku Tenkan-sen
    IchimokuTenkan,

    /// Ichimoku Kijun-sen
    IchimokuKijun,

    /// Ichimoku Senkou Span A
    IchimokuSenkouA,

    /// Ichimoku Senkou Span B
    IchimokuSenkouB,

    /// Ichimoku Chikou Span
    IchimokuChikou,

    /// Value by index in as_vec() output
    Index(usize),
}

impl ValueSource {
    /// Extract f64 value from IndicatorValue based on this source
    /// For ValueSource::Price, returns None - caller must handle price separately
    pub fn extract(&self, value: &IndicatorValue) -> Option<f64> {
        match self {
            // Main value - uses the main() method
            ValueSource::Main => Some(value.main()),

            // Price is handled separately by caller (passed to engine.process)
            ValueSource::Price => None,

            // First/Second/Third - positional access
            ValueSource::First => {
                match value {
                    IndicatorValue::Double(a, _) => Some(*a),
                    IndicatorValue::Triple(a, _, _) => Some(*a),
                    IndicatorValue::Channel3 { upper, .. } => Some(*upper),
                    IndicatorValue::ChannelExtended { upper, .. } => Some(*upper),
                    _ => Some(value.main()),
                }
            }
            ValueSource::Second => {
                match value {
                    IndicatorValue::Double(_, b) => Some(*b),
                    IndicatorValue::Triple(_, b, _) => Some(*b),
                    IndicatorValue::Channel3 { middle, .. } => Some(*middle),
                    IndicatorValue::ChannelExtended { middle, .. } => Some(*middle),
                    IndicatorValue::Macd { signal, .. } => Some(*signal),
                    _ => None,
                }
            }
            ValueSource::Third => {
                match value {
                    IndicatorValue::Triple(_, _, c) => Some(*c),
                    IndicatorValue::Channel3 { lower, .. } => Some(*lower),
                    IndicatorValue::ChannelExtended { lower, .. } => Some(*lower),
                    IndicatorValue::Macd { histogram, .. } => Some(*histogram),
                    _ => None,
                }
            }

            // MACD parts
            ValueSource::MacdLine => value.macd_line(),
            ValueSource::MacdSignal => value.macd_signal(),
            ValueSource::MacdHistogram => value.macd_histogram(),

            // Channel parts
            ValueSource::ChannelUpper => value.upper(),
            ValueSource::ChannelMiddle => value.middle(),
            ValueSource::ChannelLower => value.lower(),

            // Ichimoku parts
            ValueSource::IchimokuTenkan => {
                match value {
                    IndicatorValue::Ichimoku { tenkan, .. } => Some(*tenkan),
                    _ => None,
                }
            }
            ValueSource::IchimokuKijun => {
                match value {
                    IndicatorValue::Ichimoku { kijun, .. } => Some(*kijun),
                    _ => None,
                }
            }
            ValueSource::IchimokuSenkouA => {
                match value {
                    IndicatorValue::Ichimoku { senkou_a, .. } => Some(*senkou_a),
                    _ => None,
                }
            }
            ValueSource::IchimokuSenkouB => {
                match value {
                    IndicatorValue::Ichimoku { senkou_b, .. } => Some(*senkou_b),
                    _ => None,
                }
            }
            ValueSource::IchimokuChikou => {
                match value {
                    IndicatorValue::Ichimoku { chikou, .. } => Some(*chikou),
                    _ => None,
                }
            }

            // Index-based access via as_vec()
            ValueSource::Index(i) => {
                let vec = value.as_vec();
                vec.get(*i).copied()
            }
        }
    }
}

/// Type of detector to instantiate
#[derive(Debug, Clone, PartialEq)]
pub enum DetectorType {
    /// Threshold crossing (overbought/oversold)
    Threshold,
    /// Zero line crossing
    ZeroCross,
    /// Two lines crossing each other
    Crossover,
    /// Histogram sign changes
    Histogram,
    /// Channel position (above/below/inside)
    Channel,
    /// Price-indicator divergence
    Divergence,
    /// Trend direction changes
    Trend,
    /// Volatility regime changes
    Volatility,
    /// Volume spikes
    Volume,
    /// Swing highs/lows
    Swing,
}

/// Parameters for different detector types
#[derive(Debug, Clone, PartialEq)]
pub enum DetectorParams {
    /// Threshold detector params
    Threshold {
        value_source: ValueSource,
        upper: f64,
        lower: f64,
    },

    /// Zero cross detector params
    ZeroCross {
        value_source: ValueSource,
        tolerance: f64,
    },

    /// Crossover detector params
    Crossover {
        line_a: ValueSource,
        line_b: ValueSource,
    },

    /// Histogram detector params
    Histogram {
        value_source: ValueSource,
    },

    /// Channel detector params (price vs channel)
    Channel {
        upper_source: ValueSource,
        lower_source: ValueSource,
    },

    /// Divergence detector params
    Divergence {
        indicator_source: ValueSource,
        lookback: usize,
    },

    /// Trend detector params
    Trend {
        value_source: ValueSource,
        smoothing: usize,
    },

    /// Volatility detector params
    Volatility {
        value_source: ValueSource,
        high_threshold: f64,
        low_threshold: f64,
    },

    /// Volume detector params
    Volume {
        value_source: ValueSource,
        spike_multiplier: f64,
    },

    /// Swing detector params
    Swing {
        value_source: ValueSource,
        lookback: usize,
    },
}

/// Configuration for a single detector
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Unique identifier for this detector within a profile
    pub id: String,

    /// Display name for UI
    pub display_name: String,

    /// Type of detector
    pub detector_type: DetectorType,

    /// Whether this detector is enabled
    pub enabled: bool,

    /// Detector-specific parameters
    pub params: DetectorParams,
}

impl DetectorConfig {
    /// Create a new threshold detector config
    pub fn threshold(
        id: impl Into<String>,
        display_name: impl Into<String>,
        source: ValueSource,
        upper: f64,
        lower: f64,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Threshold,
            enabled: true,
            params: DetectorParams::Threshold {
                value_source: source,
                upper,
                lower,
            },
        }
    }

    /// Create a new zero-cross detector config
    pub fn zero_cross(
        id: impl Into<String>,
        display_name: impl Into<String>,
        source: ValueSource,
        tolerance: f64,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::ZeroCross,
            enabled: true,
            params: DetectorParams::ZeroCross {
                value_source: source,
                tolerance,
            },
        }
    }

    /// Create a new crossover detector config
    pub fn crossover(
        id: impl Into<String>,
        display_name: impl Into<String>,
        line_a: ValueSource,
        line_b: ValueSource,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Crossover,
            enabled: true,
            params: DetectorParams::Crossover { line_a, line_b },
        }
    }

    /// Create a price crossover detector (price crosses indicator value)
    /// This generates signals when price crosses above/below the indicator's main value
    pub fn price_crossover(
        id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Crossover,
            enabled: true,
            params: DetectorParams::Crossover {
                line_a: ValueSource::Price,
                line_b: ValueSource::Main,
            },
        }
    }

    /// Create a new histogram detector config
    pub fn histogram(
        id: impl Into<String>,
        display_name: impl Into<String>,
        source: ValueSource,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Histogram,
            enabled: true,
            params: DetectorParams::Histogram {
                value_source: source,
            },
        }
    }

    /// Create a new channel detector config
    pub fn channel(
        id: impl Into<String>,
        display_name: impl Into<String>,
        upper: ValueSource,
        lower: ValueSource,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Channel,
            enabled: true,
            params: DetectorParams::Channel {
                upper_source: upper,
                lower_source: lower,
            },
        }
    }

    /// Create a new divergence detector config
    pub fn divergence(
        id: impl Into<String>,
        display_name: impl Into<String>,
        source: ValueSource,
        lookback: usize,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Divergence,
            enabled: true,
            params: DetectorParams::Divergence {
                indicator_source: source,
                lookback,
            },
        }
    }

    /// Create a new swing detector config
    pub fn swing(
        id: impl Into<String>,
        display_name: impl Into<String>,
        source: ValueSource,
        lookback: usize,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            detector_type: DetectorType::Swing,
            enabled: true,
            params: DetectorParams::Swing {
                value_source: source,
                lookback,
            },
        }
    }

    /// Enable or disable this detector
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Update threshold values (only works for threshold detectors)
    pub fn update_thresholds(&mut self, upper: f64, lower: f64) -> bool {
        if let DetectorParams::Threshold {
            upper: ref mut u,
            lower: ref mut l,
            ..
        } = self.params
        {
            *u = upper;
            *l = lower;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_source_extract_single() {
        let value = IndicatorValue::Single(75.0);
        assert_eq!(ValueSource::Main.extract(&value), Some(75.0));
    }

    #[test]
    fn test_value_source_extract_macd() {
        let value = IndicatorValue::Macd {
            line: 0.5,
            signal: 0.3,
            histogram: 0.2,
        };

        assert_eq!(ValueSource::Main.extract(&value), Some(0.5));
        assert_eq!(ValueSource::MacdLine.extract(&value), Some(0.5));
        assert_eq!(ValueSource::MacdSignal.extract(&value), Some(0.3));
        assert_eq!(ValueSource::MacdHistogram.extract(&value), Some(0.2));
    }

    #[test]
    fn test_value_source_extract_channel() {
        let value = IndicatorValue::Channel3 {
            upper: 110.0,
            middle: 100.0,
            lower: 90.0,
        };

        assert_eq!(ValueSource::Main.extract(&value), Some(100.0));
        assert_eq!(ValueSource::ChannelUpper.extract(&value), Some(110.0));
        assert_eq!(ValueSource::ChannelMiddle.extract(&value), Some(100.0));
        assert_eq!(ValueSource::ChannelLower.extract(&value), Some(90.0));
    }

    #[test]
    fn test_value_source_extract_double() {
        let value = IndicatorValue::Double(80.0, 75.0);

        assert_eq!(ValueSource::Main.extract(&value), Some(80.0));
        assert_eq!(ValueSource::First.extract(&value), Some(80.0));
        assert_eq!(ValueSource::Second.extract(&value), Some(75.0));
    }

    #[test]
    fn test_value_source_extract_ichimoku() {
        let value = IndicatorValue::Ichimoku {
            tenkan: 100.0,
            kijun: 99.0,
            senkou_a: 101.0,
            senkou_b: 98.0,
            chikou: 100.5,
        };

        assert_eq!(ValueSource::IchimokuTenkan.extract(&value), Some(100.0));
        assert_eq!(ValueSource::IchimokuKijun.extract(&value), Some(99.0));
        assert_eq!(ValueSource::IchimokuSenkouA.extract(&value), Some(101.0));
        assert_eq!(ValueSource::IchimokuSenkouB.extract(&value), Some(98.0));
        assert_eq!(ValueSource::IchimokuChikou.extract(&value), Some(100.5));
    }

    #[test]
    fn test_value_source_extract_by_index() {
        let value = IndicatorValue::Triple(1.0, 2.0, 3.0);

        assert_eq!(ValueSource::Index(0).extract(&value), Some(1.0));
        assert_eq!(ValueSource::Index(1).extract(&value), Some(2.0));
        assert_eq!(ValueSource::Index(2).extract(&value), Some(3.0));
        assert_eq!(ValueSource::Index(3).extract(&value), None);
    }

    #[test]
    fn test_detector_config_threshold() {
        let config = DetectorConfig::threshold(
            "ob_os",
            "Overbought/Oversold",
            ValueSource::Main,
            70.0,
            30.0,
        );

        assert_eq!(config.id, "ob_os");
        assert!(config.enabled);
        assert_eq!(config.detector_type, DetectorType::Threshold);

        if let DetectorParams::Threshold { upper, lower, .. } = config.params {
            assert_eq!(upper, 70.0);
            assert_eq!(lower, 30.0);
        } else {
            panic!("Expected Threshold params");
        }
    }

    #[test]
    fn test_detector_config_update_thresholds() {
        let mut config = DetectorConfig::threshold(
            "ob_os",
            "Overbought/Oversold",
            ValueSource::Main,
            70.0,
            30.0,
        );

        assert!(config.update_thresholds(80.0, 20.0));

        if let DetectorParams::Threshold { upper, lower, .. } = config.params {
            assert_eq!(upper, 80.0);
            assert_eq!(lower, 20.0);
        } else {
            panic!("Expected Threshold params");
        }
    }
}
