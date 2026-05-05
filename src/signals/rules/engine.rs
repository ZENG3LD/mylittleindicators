//! Signal Engine - runtime for processing indicator values through configured detectors
//!
//! The SignalEngine takes a SignalProfile and processes IndicatorValue through
//! the configured detectors to generate Signals.

use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::signals::{
    ChannelDetector, CrossoverDetector, DivergenceDetector, HistogramDetector, SignalKind,
    SwingDetector, ThresholdMonitor, TrendDetector, VolatilityDetector, VolumeDetector,
    ZeroCrossDetector,
};
use crate::signals::signal::{Signal, Direction, BarConfirmation, SignalSource};
use super::config::{DetectorConfig, DetectorParams, DetectorType, ValueSource};
use super::profile::SignalProfile;

/// A detector instance with its configuration
enum DetectorInstance {
    Threshold {
        config_id: String,
        monitor: ThresholdMonitor,
        value_source: ValueSource,
    },
    ZeroCross {
        config_id: String,
        detector: ZeroCrossDetector,
        value_source: ValueSource,
    },
    Crossover {
        config_id: String,
        detector: CrossoverDetector,
        line_a: ValueSource,
        line_b: ValueSource,
    },
    Histogram {
        config_id: String,
        detector: HistogramDetector,
        value_source: ValueSource,
    },
    Channel {
        config_id: String,
        detector: ChannelDetector,
        upper_source: ValueSource,
        lower_source: ValueSource,
    },
    Divergence {
        config_id: String,
        detector: DivergenceDetector,
        indicator_source: ValueSource,
    },
    Trend {
        config_id: String,
        detector: TrendDetector,
        value_source: ValueSource,
    },
    Volatility {
        config_id: String,
        detector: VolatilityDetector,
        value_source: ValueSource,
    },
    Volume {
        config_id: String,
        detector: VolumeDetector,
        value_source: ValueSource,
    },
    Swing {
        config_id: String,
        detector: SwingDetector,
        #[allow(dead_code)]
        value_source: ValueSource,
    },
}

impl DetectorInstance {
    /// Create from config
    fn from_config(config: &DetectorConfig) -> Option<Self> {
        match (&config.detector_type, &config.params) {
            (
                DetectorType::Threshold,
                DetectorParams::Threshold {
                    value_source,
                    upper,
                    lower,
                },
            ) => Some(DetectorInstance::Threshold {
                config_id: config.id.clone(),
                monitor: ThresholdMonitor::new(*upper, *lower),
                value_source: value_source.clone(),
            }),

            (
                DetectorType::ZeroCross,
                DetectorParams::ZeroCross {
                    value_source,
                    tolerance,
                },
            ) => Some(DetectorInstance::ZeroCross {
                config_id: config.id.clone(),
                detector: ZeroCrossDetector::with_tolerance(*tolerance),
                value_source: value_source.clone(),
            }),

            (DetectorType::Crossover, DetectorParams::Crossover { line_a, line_b }) => {
                Some(DetectorInstance::Crossover {
                    config_id: config.id.clone(),
                    detector: CrossoverDetector::new(),
                    line_a: line_a.clone(),
                    line_b: line_b.clone(),
                })
            }

            (DetectorType::Histogram, DetectorParams::Histogram { value_source }) => {
                Some(DetectorInstance::Histogram {
                    config_id: config.id.clone(),
                    detector: HistogramDetector::new(),
                    value_source: value_source.clone(),
                })
            }

            (
                DetectorType::Channel,
                DetectorParams::Channel {
                    upper_source,
                    lower_source,
                },
            ) => Some(DetectorInstance::Channel {
                config_id: config.id.clone(),
                detector: ChannelDetector::new(0.001),
                upper_source: upper_source.clone(),
                lower_source: lower_source.clone(),
            }),

            (DetectorType::Divergence, DetectorParams::Divergence { indicator_source, lookback }) => {
                Some(DetectorInstance::Divergence {
                    config_id: config.id.clone(),
                    detector: DivergenceDetector::new(3, *lookback),
                    indicator_source: indicator_source.clone(),
                })
            }

            (DetectorType::Trend, DetectorParams::Trend { value_source, .. }) => {
                Some(DetectorInstance::Trend {
                    config_id: config.id.clone(),
                    detector: TrendDetector::new(),
                    value_source: value_source.clone(),
                })
            }

            (DetectorType::Volatility, DetectorParams::Volatility { value_source, low_threshold, .. }) => {
                Some(DetectorInstance::Volatility {
                    config_id: config.id.clone(),
                    detector: VolatilityDetector::new(0.0, 1.0, *low_threshold),
                    value_source: value_source.clone(),
                })
            }

            (DetectorType::Volume, DetectorParams::Volume { value_source, .. }) => {
                Some(DetectorInstance::Volume {
                    config_id: config.id.clone(),
                    detector: VolumeDetector::new(1.0),
                    value_source: value_source.clone(),
                })
            }

            (DetectorType::Swing, DetectorParams::Swing { lookback, .. }) => {
                Some(DetectorInstance::Swing {
                    config_id: config.id.clone(),
                    detector: SwingDetector::new(*lookback),
                    value_source: ValueSource::Main,
                })
            }

            _ => None,
        }
    }

    /// Process a value and return `(config_id, SignalKind, Direction)` if a signal fired.
    fn process(
        &mut self,
        value: &IndicatorValue,
        price: Option<f64>,
        high: f64,
        low: f64,
        volume: f64,
    ) -> Option<(String, SignalKind, Direction)> {
        match self {
            DetectorInstance::Threshold {
                config_id,
                monitor,
                value_source,
            } => {
                let v = value_source.extract(value)?;
                let (kind, direction) = monitor.update(v)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::ZeroCross {
                config_id,
                detector,
                value_source,
            } => {
                let v = value_source.extract(value)?;
                let (kind, direction) = detector.update(v)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Crossover {
                config_id,
                detector,
                line_a,
                line_b,
            } => {
                let a = if matches!(line_a, ValueSource::Price) {
                    price?
                } else {
                    line_a.extract(value)?
                };
                let b = if matches!(line_b, ValueSource::Price) {
                    price?
                } else {
                    line_b.extract(value)?
                };
                let (kind, direction) = detector.update(a, b)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Histogram {
                config_id,
                detector,
                value_source,
            } => {
                let v = value_source.extract(value)?;
                let (kind, direction) = detector.update(v)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Channel {
                config_id,
                detector,
                upper_source,
                lower_source,
            } => {
                let price_val = price?;
                let upper = upper_source.extract(value)?;
                let lower = lower_source.extract(value)?;
                let (kind, direction) = detector.update(price_val, upper, lower)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Divergence { config_id, detector, indicator_source } => {
                let price_val = price?;
                let ind_val = indicator_source.extract(value)?;
                let (kind, direction) = detector.update(price_val, ind_val)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Trend { config_id, detector, value_source } => {
                let price_val = price?;
                let fast_ma = value_source.extract(value)?;
                let slow_ma = match value {
                    IndicatorValue::Double(_, b) => *b,
                    _ => fast_ma,
                };
                let (kind, direction) = detector.update(price_val, fast_ma, slow_ma)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Volatility { config_id, detector, value_source } => {
                let v = value_source.extract(value)?;
                let (kind, direction) = detector.update(v)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Volume { config_id, detector, value_source } => {
                let v = if matches!(value_source, ValueSource::Main) {
                    volume
                } else {
                    value_source.extract(value)?
                };
                let (kind, direction) = detector.update(v, None)?;
                Some((config_id.clone(), kind, direction))
            }

            DetectorInstance::Swing { config_id, detector, .. } => {
                let (kind, direction) = detector.update(high, low)?;
                Some((config_id.clone(), kind, direction))
            }
        }
    }

    /// Reset detector state
    fn reset(&mut self) {
        match self {
            DetectorInstance::Threshold { monitor, .. } => monitor.reset(),
            DetectorInstance::ZeroCross { detector, .. } => detector.reset(),
            DetectorInstance::Crossover { detector, .. } => detector.reset(),
            DetectorInstance::Histogram { detector, .. } => detector.reset(),
            DetectorInstance::Channel { detector, .. } => {
                *detector = ChannelDetector::new(0.001);
            }
            DetectorInstance::Divergence { detector, .. } => detector.reset(),
            DetectorInstance::Trend { detector, .. } => detector.reset(),
            DetectorInstance::Volatility { detector, .. } => detector.reset(),
            DetectorInstance::Volume { detector, .. } => detector.reset(),
            DetectorInstance::Swing { detector, .. } => detector.reset(),
        }
    }
}

/// Signal Engine - processes indicator values through configured detectors
pub struct SignalEngine {
    /// Indicator ID this engine is for
    indicator_id: BarIndicatorId,

    /// Profile name
    profile_name: String,

    /// Active detector instances
    detectors: Vec<DetectorInstance>,

    /// Bar index counter
    bar_index: usize,

    /// Counter for signals generated
    signal_counter: u64,
}

impl SignalEngine {
    /// Create a new engine from a signal profile
    pub fn from_profile(profile: &SignalProfile) -> Self {
        let detectors = profile
            .enabled_detectors()
            .filter_map(DetectorInstance::from_config)
            .collect();

        Self {
            indicator_id: profile.indicator_id,
            profile_name: profile.name.clone(),
            detectors,
            bar_index: 0,
            signal_counter: 0,
        }
    }

    /// Process an indicator value and return any generated signals
    pub fn process(
        &mut self,
        value: &IndicatorValue,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: i64,
        indicator_name: &str,
        is_last_bar: bool,
    ) -> Vec<Signal> {
        let mut signals = Vec::new();

        for detector in &mut self.detectors {
            if let Some((_detector_id, signal_kind, direction)) =
                detector.process(value, Some(close), high, low, volume)
            {
                self.signal_counter += 1;

                let confirmation = if is_last_bar {
                    BarConfirmation::Pending
                } else {
                    match direction {
                        Direction::Up => {
                            if close < open {
                                BarConfirmation::WickOnly
                            } else {
                                BarConfirmation::Closed
                            }
                        }
                        Direction::Down => {
                            if close > open {
                                BarConfirmation::WickOnly
                            } else {
                                BarConfirmation::Closed
                            }
                        }
                        Direction::Neutral => BarConfirmation::Closed,
                    }
                };

                signals.push(Signal::new(
                    self.signal_counter,
                    self.bar_index,
                    timestamp,
                    close,
                    signal_kind,
                    direction,
                    confirmation,
                    SignalSource::Indicator(indicator_name.to_string()),
                ));
            }
        }

        self.bar_index += 1;
        signals
    }

    /// Process with just indicator value (no price, channel detectors won't work)
    pub fn process_simple(
        &mut self,
        value: &IndicatorValue,
        timestamp: i64,
        indicator_name: &str,
        is_last_bar: bool,
    ) -> Vec<Signal> {
        let price = value.main();
        self.process(value, price, price, price, price, 0.0, timestamp, indicator_name, is_last_bar)
    }

    /// Reset all detector states
    pub fn reset(&mut self) {
        for detector in &mut self.detectors {
            detector.reset();
        }
        self.bar_index = 0;
        self.signal_counter = 0;
    }

    /// Get the indicator ID this engine is for
    pub fn indicator_id(&self) -> BarIndicatorId {
        self.indicator_id
    }

    /// Get the profile name
    pub fn profile_name(&self) -> &str {
        &self.profile_name
    }

    /// Get count of active detectors
    pub fn detector_count(&self) -> usize {
        self.detectors.len()
    }

    /// Get total signals generated
    pub fn signals_generated(&self) -> u64 {
        self.signal_counter
    }

    /// Get current bar index
    pub fn bar_index(&self) -> usize {
        self.bar_index
    }
}

impl std::fmt::Debug for SignalEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalEngine")
            .field("indicator_id", &self.indicator_id)
            .field("profile_name", &self.profile_name)
            .field("detector_count", &self.detectors.len())
            .field("bar_index", &self.bar_index)
            .field("signals_generated", &self.signal_counter)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::rules::defaults;
    use crate::signals::catalog::{ThresholdSub, HistogramSub};

    #[test]
    fn test_engine_from_rsi_profile() {
        let profile = defaults::rsi_profile();
        let engine = SignalEngine::from_profile(&profile);

        assert_eq!(engine.indicator_id(), BarIndicatorId::Rsi);
        assert!(engine.detector_count() > 0);
    }

    #[test]
    fn test_engine_process_rsi_overbought() {
        let profile = defaults::rsi_profile();
        let mut engine = SignalEngine::from_profile(&profile);

        let values = [50.0, 60.0, 70.0, 75.0, 80.0];

        let mut all_signals = Vec::new();
        for &v in &values {
            let signals = engine.process_simple(&IndicatorValue::Single(v), 0, "Test", false);
            all_signals.extend(signals);
        }

        // Should have generated at least one threshold-enter signal (Up = overbought)
        let has_overbought = all_signals
            .iter()
            .any(|s| {
                matches!(s.kind, SignalKind::Threshold(ThresholdSub::Enter))
                    && s.direction == Direction::Up
            });
        assert!(has_overbought, "Expected overbought signal");
    }

    #[test]
    fn test_engine_process_macd_crossover() {
        let profile = defaults::macd_profile();
        let mut engine = SignalEngine::from_profile(&profile);

        let values = [
            IndicatorValue::Macd {
                line: -0.5,
                signal: 0.0,
                histogram: -0.5,
            },
            IndicatorValue::Macd {
                line: -0.2,
                signal: 0.0,
                histogram: -0.2,
            },
            IndicatorValue::Macd {
                line: 0.3,
                signal: 0.0,
                histogram: 0.3,
            },
        ];

        let mut all_signals = Vec::new();
        for v in &values {
            let signals = engine.process_simple(v, 0, "Test", false);
            all_signals.extend(signals);
        }

        // Should have crossover up signal
        let has_crossup = all_signals
            .iter()
            .any(|s| matches!(s.kind, SignalKind::Crossover) && s.direction == Direction::Up);
        assert!(has_crossup, "Expected crossover up signal");
    }

    #[test]
    fn test_engine_process_stochastic() {
        let profile = defaults::stochastic_profile();
        let mut engine = SignalEngine::from_profile(&profile);

        let values = [
            IndicatorValue::Double(20.0, 30.0), // K=20, D=30
            IndicatorValue::Double(28.0, 29.0), // K=28, D=29
            IndicatorValue::Double(35.0, 30.0), // K=35, D=30 — crossover!
        ];

        let mut all_signals = Vec::new();
        for v in &values {
            let signals = engine.process_simple(v, 0, "Test", false);
            all_signals.extend(signals);
        }

        // Should have a crossover signal (any direction)
        let has_cross = all_signals
            .iter()
            .any(|s| matches!(s.kind, SignalKind::Crossover));
        assert!(has_cross, "Expected K/D crossover signal");
    }

    #[test]
    fn test_engine_reset() {
        let profile = defaults::rsi_profile();
        let mut engine = SignalEngine::from_profile(&profile);

        engine.process_simple(&IndicatorValue::Single(75.0), 0, "Test", false);
        engine.process_simple(&IndicatorValue::Single(80.0), 0, "Test", false);

        assert!(engine.bar_index() > 0);

        engine.reset();
        assert_eq!(engine.signals_generated(), 0);
        assert_eq!(engine.bar_index(), 0);
    }

    #[test]
    fn test_engine_with_disabled_detectors() {
        let mut profile = defaults::rsi_profile();
        profile.disable_all();
        profile.set_detector_enabled("ob_os", true);

        let engine = SignalEngine::from_profile(&profile);
        assert_eq!(engine.detector_count(), 1);
    }

    #[test]
    fn test_engine_histogram_detector() {
        let profile = defaults::macd_profile();
        let mut engine = SignalEngine::from_profile(&profile);

        let values = [
            IndicatorValue::Macd {
                line: -0.5,
                signal: -0.3,
                histogram: -0.2,
            },
            IndicatorValue::Macd {
                line: -0.1,
                signal: -0.2,
                histogram: 0.1,
            },
            IndicatorValue::Macd {
                line: 0.2,
                signal: 0.0,
                histogram: 0.2,
            },
        ];

        let mut all_signals = Vec::new();
        for v in &values {
            let signals = engine.process_simple(v, 0, "Test", false);
            all_signals.extend(signals);
        }

        // Should have histogram sign-change signal (Up = positive)
        let has_hist_pos = all_signals
            .iter()
            .any(|s| {
                matches!(s.kind, SignalKind::Histogram(HistogramSub::SignChange))
                    && s.direction == Direction::Up
            });
        assert!(has_hist_pos, "Expected histogram positive signal");
    }
}
