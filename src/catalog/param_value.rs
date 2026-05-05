//! param_value.rs: Type descriptors and errors for parameter values
//!
//! 🔥 REFACTOR 5: ParamValue enum has been UNIFIED with ParameterValue
//! This module now only contains ParamType and ParamError.
//! All parameter values use ParameterValue from parameter_grid.rs
//!
//! Migration: Replace `ParamValue::USize(x)` with `ParameterValue::USize(x)`

use std::fmt;

// Re-export ParameterValue as the canonical type
// Users can import from here or from parameter_grid
pub use crate::catalog::parameter_grid::ParameterValue;

// =============================================================================
// ParamValue: Type-safe parameter container (NOW A TYPE ALIAS)
// =============================================================================

// 🔥 REFACTOR 5: ParamValue is now a type alias for ParameterValue
// This ensures backward compatibility while unifying the type system
pub type ParamValue = ParameterValue;

// Extension methods for ParameterValue to add param_type()
impl ParameterValue {
    /// Get the type of this parameter value
    pub fn param_type(&self) -> ParamType {
        match self {
            ParameterValue::Int(_) => ParamType::Int,
            ParameterValue::Float(_) => ParamType::Float,
            ParameterValue::F64(_) => ParamType::F64,
            ParameterValue::MaType(_) => ParamType::MaType,
            ParameterValue::String(_) => ParamType::String,
            ParameterValue::Bool(_) => ParamType::Bool,
            ParameterValue::USize(_) => ParamType::USize,
            ParameterValue::U8(_) => ParamType::U8,
            ParameterValue::Source(_) => ParamType::Source,
        }
    }
}

// =============================================================================
// ParamType: Type descriptor
// =============================================================================

/// Type descriptor for parameter values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamType {
    /// Signed integer (i64)
    Int,

    /// Floating point (generic, from parameter_grid)
    Float,

    /// Unsigned integer (usize)
    USize,

    /// Floating point (f64, explicit)
    F64,

    /// Unsigned 8-bit integer (u8)
    U8,

    /// Boolean (bool)
    Bool,

    /// Moving average type enum
    MaType,

    /// 🔥 UNIVERSAL: String (indicator IDs, enum values)
    String,

    /// OHLCV field selector (Open, High, Low, Close, Volume, HL2, HLC3, OHLC4)
    Source,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::Int => write!(f, "i64"),
            ParamType::Float => write!(f, "float"),
            ParamType::USize => write!(f, "usize"),
            ParamType::F64 => write!(f, "f64"),
            ParamType::U8 => write!(f, "u8"),
            ParamType::Bool => write!(f, "bool"),
            ParamType::MaType => write!(f, "MovingAverageType"),
            ParamType::String => write!(f, "String"),
            ParamType::Source => write!(f, "OhlcvField"),
        }
    }
}

// =============================================================================
// Errors
// =============================================================================

/// Error type for parameter operations
#[derive(Debug, Clone)]
pub enum ParamError {
    /// Type mismatch: tried to extract wrong type
    TypeMismatch {
        expected: ParamType,
        found: ParamType,
    },

    /// Value out of valid range
    OutOfRange {
        param_name: String,
        value: String,
        min: String,
        max: String,
    },

    /// Missing required parameter
    MissingParam {
        param_name: String,
    },
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            ParamError::OutOfRange { param_name, value, min, max } => {
                write!(
                    f,
                    "Parameter '{}' value {} is out of range [{}, {}]",
                    param_name, value, min, max
                )
            }
            ParamError::MissingParam { param_name } => {
                write!(f, "Missing required parameter: {}", param_name)
            }
        }
    }
}

impl std::error::Error for ParamError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::bar_indicators::average::MovingAverageType;
    use super::*;

    #[test]
    fn test_param_value_usize() {
        let param = ParamValue::USize(14);

        assert_eq!(param.as_usize().unwrap(), 14);
        assert!(param.as_f64().is_none() || param.as_f64().unwrap() == 14.0); // USize can convert to f64
        assert!(param.as_u8().is_none() || param.as_u8().unwrap() == 14); // USize can convert to u8 if small enough
        assert!(param.as_bool().is_none());
        assert!(param.as_ma_type().is_none());
    }

    #[test]
    fn test_param_value_f64() {
        let param = ParamValue::F64(2.5);

        assert_eq!(param.as_f64().unwrap(), 2.5);
        assert!(param.as_usize().is_none());
        assert!(param.as_u8().is_none());
    }

    #[test]
    fn test_param_value_u8() {
        let param = ParamValue::U8(7);

        assert_eq!(param.as_u8().unwrap(), 7);
        assert_eq!(param.as_usize().unwrap(), 7); // U8 can convert to usize
        assert_eq!(param.as_f64().unwrap(), 7.0); // U8 can convert to f64
    }

    #[test]
    fn test_param_value_bool() {
        let param = ParamValue::Bool(true);

        assert_eq!(param.as_bool().unwrap(), true);
        assert!(param.as_usize().is_none());
    }

    #[test]
    fn test_param_value_ma_type() {
        let param = ParamValue::MaType(MovingAverageType::SMA);

        assert_eq!(param.as_ma_type().unwrap(), MovingAverageType::SMA);
        assert!(param.as_usize().is_none());
    }

    #[test]
    fn test_param_type() {
        assert_eq!(ParamValue::USize(10).param_type(), ParamType::USize);
        assert_eq!(ParamValue::F64(1.5).param_type(), ParamType::F64);
        assert_eq!(ParamValue::U8(5).param_type(), ParamType::U8);
        assert_eq!(ParamValue::Bool(false).param_type(), ParamType::Bool);
        assert_eq!(
            ParamValue::MaType(MovingAverageType::EMA).param_type(),
            ParamType::MaType
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ParamValue::USize(14)), "14");
        assert_eq!(format!("{}", ParamValue::F64(2.5)), "2.5000");
        assert_eq!(format!("{}", ParamValue::U8(7)), "7");
        assert_eq!(format!("{}", ParamValue::Bool(true)), "true");
    }

    #[test]
    fn test_type_conversion() {
        // Test that USize(10) can convert to f64
        let param = ParamValue::USize(10);
        assert_eq!(param.as_f64().unwrap(), 10.0);
        assert_eq!(param.param_type(), ParamType::USize);
    }
}
