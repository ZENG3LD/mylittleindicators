//! EventConfig — configuration for constructing an `EventInstance`.
//!
//! Mirrors `IndicatorConfig` from `bar_indicators/instance_factory.rs`.

use std::collections::HashMap;
use crate::bar_indicators::instance_factory::IndicatorConfig;
use super::event_id::EventId;

/// Configuration for constructing an event/detector primitive.
#[derive(Debug, Clone)]
pub struct EventConfig {
    /// Which event to create.
    pub id: EventId,
    /// Human-readable label.
    pub name: String,
    /// Period parameters (e.g. lookback, window, left/right bars).
    pub periods: Vec<usize>,
    /// Numeric parameters (thresholds, multipliers, percentages).
    pub additional_params: HashMap<String, f64>,
    /// Boolean flags.
    pub flags: HashMap<String, bool>,
    /// String-typed parameters (enum variant names: "mode", "kind", "direction", etc.).
    pub string_params: HashMap<String, String>,
    /// Inner indicator dependencies.
    ///
    /// Positional semantics per event:
    /// - `Divergence`              — [0] = oscillator
    /// - `Confluence`              — [0..N] = all inner indicators
    /// - `RelativePosition`        — [0] = subject, [1] = reference
    /// - `OscillatorWithDivergence`— [0] = inner oscillator, [1] = optional ATR
    /// - `OscillatorWithVolumeWeight` — [0] = inner indicator
    /// - `LineCross`               — [0] = left line, [1] = right line (if indicators)
    /// - `PriceLineCross`          — [0] = line (if indicator)
    pub inner_indicators: Vec<IndicatorConfig>,
}

impl EventConfig {
    /// Minimal constructor.
    pub fn new(id: EventId, name: String) -> Self {
        Self {
            id,
            name,
            periods: Vec::new(),
            additional_params: HashMap::new(),
            flags: HashMap::new(),
            string_params: HashMap::new(),
            inner_indicators: Vec::new(),
        }
    }

    /// Set periods (replaces existing).
    pub fn with_periods(mut self, periods: Vec<usize>) -> Self {
        self.periods = periods;
        self
    }

    /// Add/override a numeric parameter.
    pub fn with_param(mut self, key: impl Into<String>, value: f64) -> Self {
        self.additional_params.insert(key.into(), value);
        self
    }

    /// Add/override a boolean flag.
    pub fn with_flag(mut self, key: impl Into<String>, value: bool) -> Self {
        self.flags.insert(key.into(), value);
        self
    }

    /// Add/override a string parameter (enum variant name).
    pub fn with_string_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.string_params.insert(key.into(), value.into());
        self
    }

    /// Append an inner indicator dependency.
    pub fn with_inner(mut self, inner: IndicatorConfig) -> Self {
        self.inner_indicators.push(inner);
        self
    }

    /// Helper: get first period or a default.
    pub fn period_or(&self, default: usize) -> usize {
        self.periods.first().copied().unwrap_or(default)
    }

    /// Helper: get nth period or a default.
    pub fn period_n_or(&self, n: usize, default: usize) -> usize {
        self.periods.get(n).copied().unwrap_or(default)
    }

    /// Helper: get numeric param or default.
    pub fn param_or(&self, key: &str, default: f64) -> f64 {
        self.additional_params.get(key).copied().unwrap_or(default)
    }

    /// Helper: get string param or default.
    pub fn str_param_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.string_params.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    /// Helper: get flag or default.
    pub fn flag_or(&self, key: &str, default: bool) -> bool {
        self.flags.get(key).copied().unwrap_or(default)
    }
}
