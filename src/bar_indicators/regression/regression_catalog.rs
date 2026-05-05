//! regression_catalog.rs: Complete catalog of all Regression indicators
//!
//! This catalog contains 6 regression indicators extracted from actual implementations.
//! Organized alphabetically for easy navigation.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Regression;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// ARIMA - AutoRegressive Integrated Moving Average
pub fn signature_arima() -> IndicatorSignature {
    IndicatorSignature::builder("ARIMA", CATEGORY)
        .name("ARIMA")
        .description("AutoRegressive Integrated Moving Average model")
        .add_constraint(
            ParamConstraint::new("p", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(16))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("d", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(3))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("q", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(16))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .metadata("outputs", "forecast, aic, bic")
        .metadata("min_observations", "30+")
        .machine_id(BarIndicatorId::Arima)
        // Note: "ARIMA" is already the main ID, no need for alias
        .alias("Arima")
        .alias("arima")
        .build()
}

/// ARIMAX - ARIMA with eXogenous variables
pub fn signature_arimax() -> IndicatorSignature {
    IndicatorSignature::builder("ARIMAX", CATEGORY)
        .name("ARIMAX")
        .description("ARIMA with exogenous variables")
        .add_constraint(
            ParamConstraint::new("p", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(16))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("d", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(3))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("q", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(16))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("num_exog_vars", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(16))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .metadata("outputs", "forecast, aic, bic, exog_coefficients")
        .metadata("min_observations", "30+")
        .machine_id(BarIndicatorId::Arimax) // TODO: Add to enum
        // Note: "ARIMAX" is already the main ID, no need for alias
        .alias("Arimax")
        .alias("arimax")
        .build()
}

/// EGARCH - Exponential GARCH with asymmetric effects
pub fn signature_egarch() -> IndicatorSignature {
    IndicatorSignature::builder("EGARCH", CATEGORY)
        .name("EGARCH")
        .description("Exponential GARCH with asymmetric leverage effects")
        .add_constraint(
            ParamConstraint::new("p", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("q", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .metadata("outputs", "volatility, variance, log_likelihood, aic, bic")
        .metadata("features", "asymmetric, leverage_effect")
        .metadata("min_observations", "50+")
        .metadata("author", "Nelson (1991)")
        .machine_id(BarIndicatorId::Egarch) // TODO: Add to enum
        // Note: "EGARCH" is already the main ID, no need for alias
        .alias("Egarch")
        .alias("egarch")
        .build()
}

/// GARCH - Generalized AutoRegressive Conditional Heteroskedasticity
pub fn signature_garch() -> IndicatorSignature {
    IndicatorSignature::builder("GARCH", CATEGORY)
        .name("GARCH")
        .description("Generalized AutoRegressive Conditional Heteroskedasticity model for volatility")
        .add_constraint(
            ParamConstraint::new("p", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("q", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .metadata("outputs", "volatility, variance, forecast_volatility, log_likelihood, aic, bic")
        .metadata("author", "Bollerslev (1986)")
        .metadata("min_observations", "50+")
        .machine_id(BarIndicatorId::Garch)
        // Note: "GARCH" is already the main ID, no need for alias
        .alias("Garch")
        .alias("garch")
        .build()
}

/// Polynomial Regression - polynomial trend fitting
pub fn signature_polynomial_regression() -> IndicatorSignature {
    IndicatorSignature::builder("POLY_REG", CATEGORY)
        .name("Polynomial Regression")
        .description("Polynomial regression for nonlinear trend modeling")
        .add_constraint(
            ParamConstraint::new("degree", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .metadata("outputs", "forecast, r_squared, adjusted_r_squared, mse, rmse, first_derivative, second_derivative, trend_direction")
        .metadata("trend_directions", "StrongUptrend, Uptrend, Sideways, Downtrend, StrongDowntrend")
        .metadata("min_observations", "10+")
        .machine_id(BarIndicatorId::PolyReg) // TODO: Add to enum
        // Note: "POLY_REG" is already the main ID, no need for alias
        .alias("PolyReg")
        .alias("poly_reg")
        .alias("POLYNOMIALREGRESSION")
        .alias("PolynomialRegression")
        .alias("polynomialregression")
        .alias("polynomial_regression")
        .alias("POLYNOMIAL_REGRESSION")
        .alias("Polynomial_Regression")
        .build()
}

/// VAR - Vector AutoRegression
pub fn signature_var() -> IndicatorSignature {
    IndicatorSignature::builder("VAR", CATEGORY)
        .name("VAR")
        .description("Vector AutoRegression model for multivariate time series")
        .add_constraint(
            ParamConstraint::new("p", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("n_vars", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(16))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .metadata("outputs", "forecasts, residual_covariance, impulse_responses, log_likelihood, aic, bic")
        .metadata("features", "multivariate, impulse_response_analysis")
        .metadata("min_observations", "30+")
        .metadata("author", "Sims (1980)")
        .machine_id(BarIndicatorId::Var)
        // Note: "VAR" is already the main ID, no need for alias
        .alias("Var")
        .alias("var")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Regression indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ARIMA", signature_arima as fn() -> IndicatorSignature),
    ("ARIMAX", signature_arimax as fn() -> IndicatorSignature),
    ("EGARCH", signature_egarch as fn() -> IndicatorSignature),
    ("GARCH", signature_garch as fn() -> IndicatorSignature),
    ("POLY_REG", signature_polynomial_regression as fn() -> IndicatorSignature),
    ("VAR", signature_var as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static REGRESSION_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();

    for &(main_id, func) in BASE_CATALOG {
        // Call function once to get signature with aliases
        let sig = func();

        // Insert main ID
        m.insert(main_id.to_string(), func);

        // Auto-insert all aliases from signature
        for alias in &sig.aliases {
            m.insert(alias.clone(), func);
        }
    }

    m
});

// ============================================================================
// Public API
// ============================================================================

/// Get indicator signature by ID
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    REGRESSION_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_arima_signature() {
        let sig = get_signature("ARIMA").unwrap();
        assert_eq!(sig.id, "ARIMA");
        assert_eq!(sig.category, CATEGORY);
        assert_eq!(sig.required_params().len(), 3); // p, d, q
    }

    #[test]
    fn test_get_garch_signature() {
        let sig = get_signature("GARCH").unwrap();
        assert_eq!(sig.id, "GARCH");
        assert_eq!(sig.category, CATEGORY);
        assert_eq!(sig.required_params().len(), 2); // p, q
    }

    #[test]
    fn test_get_polynomial_signature() {
        let sig = get_signature("POLY_REG").unwrap();
        assert_eq!(sig.id, "POLY_REG");
        assert_eq!(sig.required_params().len(), 1); // degree
    }

    #[test]
    fn test_get_var_signature() {
        let sig = get_signature("VAR").unwrap();
        assert_eq!(sig.id, "VAR");
        assert_eq!(sig.required_params().len(), 2); // p, n_vars
    }

    #[test]
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }

    #[test]
    fn test_count() {
        assert_eq!(count(), 6); // 6 regression indicators
    }

    #[test]
    fn test_arima_validation() {
        let sig = get_signature("ARIMA").unwrap();

        // Valid params
        let params = vec![
            ("p", ParamValue::USize(1)),
            ("d", ParamValue::USize(1)),
            ("q", ParamValue::USize(1)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: p out of range
        let params = vec![
            ("p", ParamValue::USize(20)),
            ("d", ParamValue::USize(1)),
            ("q", ParamValue::USize(1)),
        ];
        assert!(sig.validate_params(&params).is_err());

        // Invalid: d out of range
        let params = vec![
            ("p", ParamValue::USize(1)),
            ("d", ParamValue::USize(5)),
            ("q", ParamValue::USize(1)),
        ];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("GARCH").unwrap();
        let params = vec![
            ("p", ParamValue::USize(1)),
            ("q", ParamValue::USize(1)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("GARCH"));
        assert!(key.contains("1"));
    }

    #[test]
    fn test_polynomial_degree_validation() {
        let sig = get_signature("POLY_REG").unwrap();

        // Valid degree
        let params = vec![("degree", ParamValue::USize(3))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: degree = 0
        let params = vec![("degree", ParamValue::USize(0))];
        assert!(sig.validate_params(&params).is_err());

        // Invalid: degree too high
        let params = vec![("degree", ParamValue::USize(10))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_var_multivariate_validation() {
        let sig = get_signature("VAR").unwrap();

        // Valid params
        let params = vec![
            ("p", ParamValue::USize(2)),
            ("n_vars", ParamValue::USize(3)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: n_vars = 1 (need at least 2 for multivariate)
        let params = vec![
            ("p", ParamValue::USize(1)),
            ("n_vars", ParamValue::USize(1)),
        ];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_egarch_vs_garch() {
        let garch_sig = get_signature("GARCH").unwrap();
        let egarch_sig = get_signature("EGARCH").unwrap();

        // Both should have same required params (p, q)
        assert_eq!(garch_sig.required_params().len(), 2);
        assert_eq!(egarch_sig.required_params().len(), 2);

        // But different IDs
        assert_ne!(garch_sig.id, egarch_sig.id);
    }
}
