//! Signal Profile - collection of detector configurations for an indicator
//!
//! A SignalProfile defines which signals an indicator can generate
//! and how they should be detected.

use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use super::config::{DetectorConfig, DetectorParams, ValueSource};

/// A signal profile defines the signal generation rules for an indicator type
#[derive(Debug, Clone)]
pub struct SignalProfile {
    /// Indicator ID this profile is for
    pub indicator_id: BarIndicatorId,

    /// Display name for this profile
    pub name: String,

    /// Description of what signals this profile generates
    pub description: String,

    /// Collection of detector configurations
    pub detectors: Vec<DetectorConfig>,
}

impl SignalProfile {
    /// Create a new empty signal profile
    pub fn new(indicator_id: BarIndicatorId, name: impl Into<String>) -> Self {
        Self {
            indicator_id,
            name: name.into(),
            description: String::new(),
            detectors: Vec::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a detector configuration
    pub fn with_detector(mut self, detector: DetectorConfig) -> Self {
        self.detectors.push(detector);
        self
    }

    /// Add multiple detector configurations
    pub fn with_detectors(mut self, detectors: impl IntoIterator<Item = DetectorConfig>) -> Self {
        self.detectors.extend(detectors);
        self
    }

    /// Get a detector config by ID
    pub fn get_detector(&self, id: &str) -> Option<&DetectorConfig> {
        self.detectors.iter().find(|d| d.id == id)
    }

    /// Get a mutable detector config by ID
    pub fn get_detector_mut(&mut self, id: &str) -> Option<&mut DetectorConfig> {
        self.detectors.iter_mut().find(|d| d.id == id)
    }

    /// Enable or disable a detector by ID
    pub fn set_detector_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(detector) = self.get_detector_mut(id) {
            detector.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Update threshold values for a threshold detector
    pub fn update_threshold(&mut self, id: &str, upper: f64, lower: f64) -> bool {
        if let Some(detector) = self.get_detector_mut(id) {
            detector.update_thresholds(upper, lower)
        } else {
            false
        }
    }

    /// Get all enabled detectors
    pub fn enabled_detectors(&self) -> impl Iterator<Item = &DetectorConfig> {
        self.detectors.iter().filter(|d| d.enabled)
    }

    /// Get count of enabled detectors
    pub fn enabled_count(&self) -> usize {
        self.detectors.iter().filter(|d| d.enabled).count()
    }

    /// Disable all detectors
    pub fn disable_all(&mut self) {
        for detector in &mut self.detectors {
            detector.enabled = false;
        }
    }

    /// Enable all detectors
    pub fn enable_all(&mut self) {
        for detector in &mut self.detectors {
            detector.enabled = true;
        }
    }

    /// Create a copy with all detectors disabled except specified ones
    pub fn with_only(&self, detector_ids: &[&str]) -> Self {
        let mut profile = self.clone();
        profile.disable_all();
        for id in detector_ids {
            profile.set_detector_enabled(id, true);
        }
        profile
    }

    /// Get all value sources used by enabled detectors
    pub fn value_sources(&self) -> Vec<ValueSource> {
        let mut sources = Vec::new();
        for detector in self.enabled_detectors() {
            match &detector.params {
                DetectorParams::Threshold { value_source, .. }
                | DetectorParams::ZeroCross { value_source, .. }
                | DetectorParams::Histogram { value_source }
                | DetectorParams::Divergence {
                    indicator_source: value_source,
                    ..
                }
                | DetectorParams::Trend { value_source, .. }
                | DetectorParams::Volatility { value_source, .. }
                | DetectorParams::Volume { value_source, .. }
                | DetectorParams::Swing { value_source, .. } => {
                    if !sources.contains(value_source) {
                        sources.push(value_source.clone());
                    }
                }
                DetectorParams::Crossover { line_a, line_b } => {
                    if !sources.contains(line_a) {
                        sources.push(line_a.clone());
                    }
                    if !sources.contains(line_b) {
                        sources.push(line_b.clone());
                    }
                }
                DetectorParams::Channel {
                    upper_source,
                    lower_source,
                } => {
                    if !sources.contains(upper_source) {
                        sources.push(upper_source.clone());
                    }
                    if !sources.contains(lower_source) {
                        sources.push(lower_source.clone());
                    }
                }
            }
        }
        sources
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::rules::config::DetectorConfig;

    #[test]
    fn test_profile_creation() {
        let profile = SignalProfile::new(BarIndicatorId::Rsi, "RSI Default Profile")
            .with_description("Standard RSI signals with 70/30 thresholds")
            .with_detector(DetectorConfig::threshold(
                "ob_os",
                "Overbought/Oversold",
                ValueSource::Main,
                70.0,
                30.0,
            ));

        assert_eq!(profile.indicator_id, BarIndicatorId::Rsi);
        assert_eq!(profile.name, "RSI Default Profile");
        assert_eq!(profile.detectors.len(), 1);
    }

    #[test]
    fn test_profile_detector_management() {
        let mut profile = SignalProfile::new(BarIndicatorId::Rsi, "RSI Profile")
            .with_detector(DetectorConfig::threshold(
                "ob_os",
                "Overbought/Oversold",
                ValueSource::Main,
                70.0,
                30.0,
            ))
            .with_detector(DetectorConfig::threshold(
                "extreme",
                "Extreme Levels",
                ValueSource::Main,
                90.0,
                10.0,
            ));

        assert_eq!(profile.enabled_count(), 2);

        // Disable one detector
        profile.set_detector_enabled("extreme", false);
        assert_eq!(profile.enabled_count(), 1);

        // Get detector
        let detector = profile.get_detector("ob_os").unwrap();
        assert!(detector.enabled);

        // Update threshold
        profile.update_threshold("ob_os", 80.0, 20.0);
        let detector = profile.get_detector("ob_os").unwrap();
        if let DetectorParams::Threshold { upper, lower, .. } = &detector.params {
            assert_eq!(*upper, 80.0);
            assert_eq!(*lower, 20.0);
        }
    }

    #[test]
    fn test_profile_with_only() {
        let profile = SignalProfile::new(BarIndicatorId::Macd, "MACD Profile")
            .with_detector(DetectorConfig::crossover(
                "signal_cross",
                "Signal Line Cross",
                ValueSource::Main,
                ValueSource::Main,
            ))
            .with_detector(DetectorConfig::zero_cross(
                "zero_cross",
                "Zero Line Cross",
                ValueSource::Main,
                0.001,
            ))
            .with_detector(DetectorConfig::histogram(
                "histogram",
                "Histogram",
                ValueSource::Main,
            ));

        let focused = profile.with_only(&["signal_cross"]);
        assert_eq!(focused.enabled_count(), 1);
        assert!(focused.get_detector("signal_cross").unwrap().enabled);
        assert!(!focused.get_detector("zero_cross").unwrap().enabled);
    }
}
