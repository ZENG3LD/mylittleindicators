//! constraints.rs: Parameter constraints and validation for universal indicator system
//!
//! Provides constraint definitions and validation logic for indicator parameters.
//! Ensures that parameter values fall within valid ranges before indicator construction.

use crate::catalog::param_value::{ParamValue, ParamType, ParamError};
use std::fmt;

// =============================================================================
// ParamConstraint: Constraint definition for a single parameter
// =============================================================================

/// Constraint definition for a single parameter
///
/// Defines the valid range, type, default value, and whether a parameter
/// is required for indicator construction.
///
/// ## Examples
///
/// ```rust
/// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
///
/// // RSI period constraint: 2-200, default 14, required
/// let rsi_period = ParamConstraint::period(2, 200, 14);
/// assert!(rsi_period.required);
///
/// // Bollinger Bands std_dev: 0.5-5.0, default 2.0
/// let bb_stddev = ParamConstraint::multiplier(0.5, 5.0, 2.0);
/// assert_eq!(bb_stddev.name, "multiplier");
///
/// // Use Wilder smoothing: bool flag, default true
/// let use_wilder = ParamConstraint::flag("use_wilder", true);
/// assert_eq!(use_wilder.name, "use_wilder");
/// ```
#[derive(Debug, Clone)]
pub struct ParamConstraint {
    /// Parameter name (e.g., "period", "multiplier", "use_wilder")
    pub name: String,

    /// Expected parameter type
    pub param_type: ParamType,

    /// Minimum allowed value (inclusive)
    /// None means no lower bound
    pub min: Option<ParamValue>,

    /// Maximum allowed value (inclusive)
    /// None means no upper bound
    pub max: Option<ParamValue>,

    /// Default value if not provided
    pub default: Option<ParamValue>,

    /// Whether this parameter must be provided
    pub required: bool,
}

impl ParamConstraint {
    /// Create a new parameter constraint
    pub fn new(name: impl Into<String>, param_type: ParamType) -> Self {
        Self {
            name: name.into(),
            param_type,
            min: None,
            max: None,
            default: None,
            required: false,
        }
    }

    /// Set minimum value
    pub fn with_min(mut self, min: ParamValue) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value
    pub fn with_max(mut self, max: ParamValue) -> Self {
        self.max = Some(max);
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: ParamValue) -> Self {
        self.default = Some(default);
        self
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Validate a parameter value against this constraint
    ///
    /// ## Errors
    ///
    /// - `ParamError::TypeMismatch`: Value has wrong type
    /// - `ParamError::OutOfRange`: Value outside min/max bounds
    pub fn validate(&self, value: &ParamValue) -> Result<(), ParamError> {
        // Check type matches
        if value.param_type() != self.param_type {
            return Err(ParamError::TypeMismatch {
                expected: self.param_type,
                found: value.param_type(),
            });
        }

        // Check min bound
        if let Some(ref min_val) = self.min {
            if !self.check_min(value, min_val) {
                return Err(ParamError::OutOfRange {
                    param_name: self.name.clone(),
                    value: format!("{}", value),
                    min: format!("{}", min_val),
                    max: self.max.as_ref().map(|v| format!("{}", v)).unwrap_or_else(|| "∞".to_string()),
                });
            }
        }

        // Check max bound
        if let Some(ref max_val) = self.max {
            if !self.check_max(value, max_val) {
                return Err(ParamError::OutOfRange {
                    param_name: self.name.clone(),
                    value: format!("{}", value),
                    min: self.min.as_ref().map(|v| format!("{}", v)).unwrap_or_else(|| "-∞".to_string()),
                    max: format!("{}", max_val),
                });
            }
        }

        Ok(())
    }

    /// Check if value >= min
    fn check_min(&self, value: &ParamValue, min: &ParamValue) -> bool {
        match (value, min) {
            (ParamValue::USize(v), ParamValue::USize(m)) => v >= m,
            (ParamValue::F64(v), ParamValue::F64(m)) => v >= m,
            (ParamValue::U8(v), ParamValue::U8(m)) => v >= m,
            _ => true, // Type mismatch handled earlier
        }
    }

    /// Check if value <= max
    fn check_max(&self, value: &ParamValue, max: &ParamValue) -> bool {
        match (value, max) {
            (ParamValue::USize(v), ParamValue::USize(m)) => v <= m,
            (ParamValue::F64(v), ParamValue::F64(m)) => v <= m,
            (ParamValue::U8(v), ParamValue::U8(m)) => v <= m,
            _ => true, // Type mismatch handled earlier
        }
    }

    // =============================================================================
    // Builder methods for common constraint patterns
    // =============================================================================

    /// Create period constraint (usize, typically 2-200)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let rsi_period = ParamConstraint::period(2, 200, 14);
    /// assert!(rsi_period.required);
    /// ```
    pub fn period(min: usize, max: usize, default: usize) -> Self {
        Self::new("period", ParamType::USize)
            .with_min(ParamValue::USize(min))
            .with_max(ParamValue::USize(max))
            .with_default(ParamValue::USize(default))
            .required()
    }

    /// Create multiplier constraint (f64, typically 0.1-10.0)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let bb_stddev = ParamConstraint::multiplier(0.5, 5.0, 2.0);
    /// assert_eq!(bb_stddev.name, "multiplier");
    /// ```
    pub fn multiplier(min: f64, max: f64, default: f64) -> Self {
        Self::new("multiplier", ParamType::F64)
            .with_min(ParamValue::F64(min))
            .with_max(ParamValue::F64(max))
            .with_default(ParamValue::F64(default))
            .required()
    }

    /// Create threshold constraint (f64, typically 0-100)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let rsi_overbought = ParamConstraint::threshold("overbought", 50.0, 100.0, 70.0);
    /// assert_eq!(rsi_overbought.name, "overbought");
    /// ```
    pub fn threshold(name: impl Into<String>, min: f64, max: f64, default: f64) -> Self {
        Self::new(name, ParamType::F64)
            .with_min(ParamValue::F64(min))
            .with_max(ParamValue::F64(max))
            .with_default(ParamValue::F64(default))
    }

    /// Create flag constraint (bool)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let use_wilder = ParamConstraint::flag("use_wilder", true);
    /// assert_eq!(use_wilder.name, "use_wilder");
    /// ```
    pub fn flag(name: impl Into<String>, default: bool) -> Self {
        Self::new(name, ParamType::Bool)
            .with_default(ParamValue::Bool(default))
    }

    /// Create small period constraint (u8, for permutation entropy etc.)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let perm_entropy = ParamConstraint::small_period(2, 10, 7);
    /// assert!(perm_entropy.required);
    /// ```
    pub fn small_period(min: u8, max: u8, default: u8) -> Self {
        Self::new("period", ParamType::U8)
            .with_min(ParamValue::U8(min))
            .with_max(ParamValue::U8(max))
            .with_default(ParamValue::U8(default))
            .required()
    }

    /// Create MA type constraint (enum, no validation needed)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::average::MovingAverageType;
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let ma_type = ParamConstraint::ma_type(MovingAverageType::SMA);
    /// assert_eq!(ma_type.name, "ma_type");
    /// ```
    pub fn ma_type(default: crate::bar_indicators::average::moving_average::MovingAverageType) -> Self {
        Self::new("ma_type", ParamType::MaType)
            .with_default(ParamValue::MaType(default))
    }

    /// Creates a named MA type constraint for indicators with multiple independent MA types
    ///
    /// # Example
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::average::MovingAverageType;
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let fast_ma = ParamConstraint::ma_type_named("fast_ma", MovingAverageType::EMA);
    /// assert_eq!(fast_ma.name, "fast_ma");
    /// ```
    pub fn ma_type_named(name: &str, default: crate::bar_indicators::average::moving_average::MovingAverageType) -> Self {
        Self::new(name, ParamType::MaType)
            .with_default(ParamValue::MaType(default))
    }

    /// Create source field constraint (OhlcvField)
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
    /// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
    /// let source = ParamConstraint::source("source", OhlcvField::Close);
    /// assert_eq!(source.name, "source");
    /// ```
    pub fn source(name: impl Into<String>, default: crate::bar_indicators::ohlcv_field::OhlcvField) -> Self {
        Self::new(name, ParamType::Source)
            .with_default(ParamValue::Source(default))
    }
}

impl fmt::Display for ParamConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.param_type)?;

        if let (Some(ref min), Some(ref max)) = (&self.min, &self.max) {
            write!(f, " [{}, {}]", min, max)?;
        } else if let Some(ref min) = self.min {
            write!(f, " [{}, ∞)", min)?;
        } else if let Some(ref max) = self.max {
            write!(f, " (-∞, {}]", max)?;
        }

        if let Some(ref default) = self.default {
            write!(f, " = {}", default)?;
        }

        if self.required {
            write!(f, " (required)")?;
        }

        Ok(())
    }
}

// =============================================================================
// ConstraintSet: Collection of constraints for an indicator
// =============================================================================

/// Collection of parameter constraints for an indicator
///
/// ## Example
///
/// ```rust
/// use zengeld_chart_indicators::catalog::constraints::{ParamConstraint, ConstraintSet};
/// use zengeld_chart_indicators::catalog::param_value::ParamValue;
///
/// let mut constraints = ConstraintSet::new("RSI");
/// constraints.add(ParamConstraint::period(2, 200, 14));
/// constraints.add(ParamConstraint::flag("use_wilder", true));
///
/// // Validate parameter values
/// let params = vec![
///     ("period", ParamValue::USize(14)),
///     ("use_wilder", ParamValue::Bool(true)),
/// ];
/// assert!(constraints.validate_all(&params).is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct ConstraintSet {
    /// Indicator name
    pub indicator_name: String,

    /// List of parameter constraints
    pub constraints: Vec<ParamConstraint>,
}

impl ConstraintSet {
    /// Create new constraint set for an indicator
    pub fn new(indicator_name: impl Into<String>) -> Self {
        Self {
            indicator_name: indicator_name.into(),
            constraints: Vec::new(),
        }
    }

    /// Add a parameter constraint
    pub fn add(&mut self, constraint: ParamConstraint) {
        self.constraints.push(constraint);
    }

    /// Add multiple constraints
    pub fn add_all(&mut self, constraints: Vec<ParamConstraint>) {
        self.constraints.extend(constraints);
    }

    /// Get constraint by parameter name
    pub fn get(&self, param_name: &str) -> Option<&ParamConstraint> {
        self.constraints.iter().find(|c| c.name == param_name)
    }

    /// Validate all provided parameters
    ///
    /// ## Errors
    ///
    /// - `ParamError::MissingParam`: Required parameter not provided
    /// - `ParamError::TypeMismatch`: Parameter has wrong type
    /// - `ParamError::OutOfRange`: Parameter value out of bounds
    pub fn validate_all(&self, params: &[(&str, ParamValue)]) -> Result<(), ParamError> {
        // Check all required parameters are provided
        for constraint in &self.constraints {
            if constraint.required {
                let found = params.iter().any(|(name, _)| *name == constraint.name);
                if !found {
                    return Err(ParamError::MissingParam {
                        param_name: constraint.name.clone(),
                    });
                }
            }
        }

        // Validate each provided parameter
        for (name, value) in params {
            if let Some(constraint) = self.get(name) {
                constraint.validate(value)?;
            }
            // Note: Unknown parameters are allowed (forward compatibility)
        }

        Ok(())
    }

    /// Get default value for a parameter
    pub fn get_default(&self, param_name: &str) -> Option<&ParamValue> {
        self.get(param_name).and_then(|c| c.default.as_ref())
    }

    /// Get all parameter names
    pub fn param_names(&self) -> Vec<&str> {
        self.constraints.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get all required parameter names
    pub fn required_params(&self) -> Vec<&str> {
        self.constraints
            .iter()
            .filter(|c| c.required)
            .map(|c| c.name.as_str())
            .collect()
    }
}

impl fmt::Display for ConstraintSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Constraints for {}:", self.indicator_name)?;
        for constraint in &self.constraints {
            writeln!(f, "  {}", constraint)?;
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::average::moving_average::MovingAverageType;

    #[test]
    fn test_period_constraint() {
        let constraint = ParamConstraint::period(2, 200, 14);

        // Valid values
        assert!(constraint.validate(&ParamValue::USize(14)).is_ok());
        assert!(constraint.validate(&ParamValue::USize(2)).is_ok());
        assert!(constraint.validate(&ParamValue::USize(200)).is_ok());

        // Invalid: too small
        assert!(constraint.validate(&ParamValue::USize(1)).is_err());

        // Invalid: too large
        assert!(constraint.validate(&ParamValue::USize(201)).is_err());

        // Invalid: wrong type
        assert!(constraint.validate(&ParamValue::F64(14.0)).is_err());
    }

    #[test]
    fn test_multiplier_constraint() {
        let constraint = ParamConstraint::multiplier(0.5, 5.0, 2.0);

        // Valid values
        assert!(constraint.validate(&ParamValue::F64(2.0)).is_ok());
        assert!(constraint.validate(&ParamValue::F64(0.5)).is_ok());
        assert!(constraint.validate(&ParamValue::F64(5.0)).is_ok());

        // Invalid: too small
        assert!(constraint.validate(&ParamValue::F64(0.4)).is_err());

        // Invalid: too large
        assert!(constraint.validate(&ParamValue::F64(5.1)).is_err());
    }

    #[test]
    fn test_threshold_constraint() {
        let constraint = ParamConstraint::threshold("overbought", 50.0, 100.0, 70.0);

        assert_eq!(constraint.name, "overbought");
        assert!(constraint.validate(&ParamValue::F64(70.0)).is_ok());
        assert!(constraint.validate(&ParamValue::F64(49.9)).is_err());
        assert!(constraint.validate(&ParamValue::F64(100.1)).is_err());
    }

    #[test]
    fn test_flag_constraint() {
        let constraint = ParamConstraint::flag("use_wilder", true);

        assert_eq!(constraint.name, "use_wilder");
        assert!(constraint.validate(&ParamValue::Bool(true)).is_ok());
        assert!(constraint.validate(&ParamValue::Bool(false)).is_ok());
        assert!(constraint.validate(&ParamValue::USize(1)).is_err());
    }

    #[test]
    fn test_small_period_constraint() {
        let constraint = ParamConstraint::small_period(2, 10, 7);

        assert!(constraint.validate(&ParamValue::U8(7)).is_ok());
        assert!(constraint.validate(&ParamValue::U8(2)).is_ok());
        assert!(constraint.validate(&ParamValue::U8(10)).is_ok());
        assert!(constraint.validate(&ParamValue::U8(1)).is_err());
        assert!(constraint.validate(&ParamValue::U8(11)).is_err());
    }

    #[test]
    fn test_ma_type_constraint() {
        let constraint = ParamConstraint::ma_type(MovingAverageType::SMA);

        assert!(constraint.validate(&ParamValue::MaType(MovingAverageType::SMA)).is_ok());
        assert!(constraint.validate(&ParamValue::MaType(MovingAverageType::EMA)).is_ok());
        assert!(constraint.validate(&ParamValue::USize(14)).is_err());
    }

    #[test]
    fn test_constraint_set_validation() {
        let mut set = ConstraintSet::new("RSI");
        set.add(ParamConstraint::period(2, 200, 14));
        set.add(ParamConstraint::flag("use_wilder", true));

        // Valid parameters
        let params = vec![
            ("period", ParamValue::USize(14)),
            ("use_wilder", ParamValue::Bool(true)),
        ];
        assert!(set.validate_all(&params).is_ok());

        // Missing required parameter
        let params = vec![
            ("use_wilder", ParamValue::Bool(true)),
        ];
        assert!(set.validate_all(&params).is_err());

        // Invalid value
        let params = vec![
            ("period", ParamValue::USize(1)),
            ("use_wilder", ParamValue::Bool(true)),
        ];
        assert!(set.validate_all(&params).is_err());
    }

    #[test]
    fn test_constraint_set_defaults() {
        let mut set = ConstraintSet::new("SMA");
        set.add(ParamConstraint::period(2, 200, 20));

        assert_eq!(set.get_default("period"), Some(&ParamValue::USize(20)));
        assert_eq!(set.get_default("nonexistent"), None);
    }

    #[test]
    fn test_constraint_set_param_names() {
        let mut set = ConstraintSet::new("MACD");
        set.add(ParamConstraint::period(2, 50, 12).with_min(ParamValue::USize(2)));
        set.add(ParamConstraint::period(2, 50, 26).with_min(ParamValue::USize(2)));
        set.add(ParamConstraint::period(2, 20, 9).with_min(ParamValue::USize(2)));

        let names = set.param_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"period"));
    }

    #[test]
    fn test_display_constraint() {
        let constraint = ParamConstraint::period(2, 200, 14);
        let display = format!("{}", constraint);
        assert!(display.contains("period"));
        assert!(display.contains("usize"));
        assert!(display.contains("[2, 200]"));
        assert!(display.contains("14"));
        assert!(display.contains("required"));
    }

    #[test]
    fn test_display_constraint_set() {
        let mut set = ConstraintSet::new("RSI");
        set.add(ParamConstraint::period(2, 200, 14));
        set.add(ParamConstraint::flag("use_wilder", true));

        let display = format!("{}", set);
        assert!(display.contains("RSI"));
        assert!(display.contains("period"));
        assert!(display.contains("use_wilder"));
    }
}
