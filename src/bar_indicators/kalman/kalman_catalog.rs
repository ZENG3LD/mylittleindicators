//! kalman_catalog.rs: Catalog of all Kalman Filter indicators
//!
//! Auto-generated catalog based on actual indicator implementations.
//! Contains IndicatorSignature definitions for 11 Kalman filter indicators.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Kalman;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Basic Kalman Filter - optimal state estimation with Kalman filtering
pub fn signature_basic_kalman_filter() -> IndicatorSignature {
    IndicatorSignature::builder("KALMAN", CATEGORY)
        .name("Basic Kalman Filter")
        .description("Optimal state estimation combining model predictions with noisy observations")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("complexity", "Constant velocity model")
        .metadata("outputs", "position, velocity, acceleration")
        .machine_id(BarIndicatorId::Kalman) // TODO: Add to enum
        // Note: "KALMAN" is already the main ID, no need for alias
        .alias("Kalman")
        .alias("kalman")
        .alias("BASICKALMANFILTER")
        .alias("BasicKalmanFilter")
        .alias("basickalmanfilter")
        .alias("basic_kalman_filter")
        .alias("BASIC_KALMAN_FILTER")
        .alias("Basic_Kalman_Filter")
        .build()
}

/// Extended Kalman Filter - for nonlinear systems using linearization
pub fn signature_extended_kalman_filter() -> IndicatorSignature {
    IndicatorSignature::builder("EKF", CATEGORY)
        .name("Extended Kalman Filter")
        .description("Kalman filter for nonlinear systems via Jacobian linearization")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("complexity", "Nonlinear with Jacobian")
        .metadata("models", "friction, nonlinear observation")
        .machine_id(BarIndicatorId::Ekf) // TODO: Add to enum
        // Note: "EKF" is already the main ID, no need for alias
        .alias("Ekf")
        .alias("ekf")
        .alias("EXTENDEDKALMANFILTER")
        .alias("ExtendedKalmanFilter")
        .alias("extendedkalmanfilter")
        .alias("extended_kalman_filter")
        .alias("EXTENDED_KALMAN_FILTER")
        .alias("Extended_Kalman_Filter")
        .build()
}

/// Unscented Kalman Filter - for nonlinear systems using unscented transform
pub fn signature_unscented_kalman_filter() -> IndicatorSignature {
    IndicatorSignature::builder("UKF", CATEGORY)
        .name("Unscented Kalman Filter")
        .description("Kalman filter for nonlinear systems using sigma points")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("complexity", "Unscented transform")
        .metadata("feature", "no Jacobian needed")
        .machine_id(BarIndicatorId::Ukf) // TODO: Add to enum
        // Note: "UKF" is already the main ID, no need for alias
        .alias("Ukf")
        .alias("ukf")
        .alias("UNSCENTEDKALMANFILTER")
        .alias("UnscentedKalmanFilter")
        .alias("unscentedkalmanfilter")
        .alias("unscented_kalman_filter")
        .alias("UNSCENTED_KALMAN_FILTER")
        .alias("Unscented_Kalman_Filter")
        .build()
}

/// Particle Filter - Monte Carlo filter for arbitrary distributions
pub fn signature_particle_filter() -> IndicatorSignature {
    IndicatorSignature::builder("PARTICLE", CATEGORY)
        .name("Particle Filter")
        .description("Monte Carlo particle filter for nonlinear systems with arbitrary distributions")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("num_particles", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(10000))
                .with_default(ParamValue::USize(100))
                .required()
        )
        .metadata("complexity", "Monte Carlo sampling")
        .metadata("feature", "effective sample size tracking")
        .machine_id(BarIndicatorId::Particle) // TODO: Add to enum
        // Note: "PARTICLE" is already the main ID, no need for alias
        .alias("Particle")
        .alias("particle")
        .alias("PARTICLEFILTER")
        .alias("ParticleFilter")
        .alias("particlefilter")
        .alias("particle_filter")
        .alias("PARTICLE_FILTER")
        .alias("Particle_Filter")
        .build()
}

/// Alpha-Beta-Gamma Filter - constant acceleration tracking
pub fn signature_alpha_beta_gamma_filter() -> IndicatorSignature {
    IndicatorSignature::builder("ABGFILTER", CATEGORY)
        .name("Alpha-Beta-Gamma Filter")
        .description("Constant acceleration model tracking filter")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("model", "constant acceleration")
        .metadata("implementation", "EMA chain proxy")
        .machine_id(BarIndicatorId::Abgfilter) // TODO: Add to enum
        // Note: "ABGFILTER" is already the main ID, no need for alias
        .alias("Abgfilter")
        .alias("abgfilter")
        .alias("ALPHABETAGAMMAFILTER")
        .alias("AlphaBetaGammaFilter")
        .alias("alphabetagammafilter")
        .alias("alpha_beta_gamma_filter")
        .alias("ALPHA_BETA_GAMMA_FILTER")
        .alias("Alpha_Beta_Gamma_Filter")
        .build()
}

/// RTS Smoother - backward smoothing over Kalman filter output
pub fn signature_rts_smoother() -> IndicatorSignature {
    IndicatorSignature::builder("RTS", CATEGORY)
        .name("Rauch-Tung-Striebel Smoother")
        .description("Backward smoothing over Kalman filter for optimal past state estimates")
        .metadata("complexity", "Forward-backward pass")
        .metadata("note", "Streaming proxy implementation")
        .machine_id(BarIndicatorId::Rts) // TODO: Add to enum
        // Note: "RTS" is already the main ID, no need for alias
        .alias("Rts")
        .alias("rts")
        .alias("RAUCHTUNGSTRIEBELSMOOTHER")
        .alias("RauchTungStriebelSmoother")
        .alias("rauchtungstriebelsmoother")
        .alias("rauch_tung_striebel_smoother")
        .alias("RAUCH_TUNG_STRIEBEL_SMOOTHER")
        .alias("Rauch_Tung_Striebel_Smoother")
        .build()
}

/// Kalman Trend Slope - velocity and z-score from Kalman filter
pub fn signature_kalman_trend_slope() -> IndicatorSignature {
    IndicatorSignature::builder("KSLOPE", CATEGORY)
        .name("Kalman Trend Slope")
        .description("Kalman filter velocity (trend) with rolling z-score normalization")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("window", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("outputs", "slope, slope_z")
        .machine_id(BarIndicatorId::Kslope) // TODO: Add to enum
        // Note: "KSLOPE" is already the main ID, no need for alias
        .alias("Kslope")
        .alias("kslope")
        .alias("KALMANTRENDSLOPE")
        .alias("KalmanTrendSlope")
        .alias("kalmantrendslope")
        .alias("kalman_trend_slope")
        .alias("KALMAN_TREND_SLOPE")
        .alias("Kalman_Trend_Slope")
        .build()
}

/// Kalman Slope Z-Score - z-score of Kalman velocity over window
pub fn signature_kalman_slope_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("KSLOPEZ", CATEGORY)
        .name("Kalman Slope Z-Score")
        .description("Z-score of Kalman filter velocity over rolling window")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("window", ParamType::USize)
                .with_min(ParamValue::USize(20))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(100))
                .required()
        )
        .metadata("feature", "normalized velocity")
        .machine_id(BarIndicatorId::Kslopez) // TODO: Add to enum
        // Note: "KSLOPEZ" is already the main ID, no need for alias
        .alias("Kslopez")
        .alias("kslopez")
        .alias("KALMANSLOPEZSCORE")
        .alias("KalmanSlopeZScore")
        .alias("kalmanslopezscore")
        .alias("kalman_slope_z_score")
        .alias("KALMAN_SLOPE_Z_SCORE")
        .alias("Kalman_Slope_Z_Score")
        .build()
}

/// Kalman Trend Regime - discrete regime classification by velocity
pub fn signature_kalman_trend_regime() -> IndicatorSignature {
    IndicatorSignature::builder("KREGIME", CATEGORY)
        .name("Kalman Trend Regime")
        .description("Discrete trend regime classification (-1, 0, 1) based on velocity z-score")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("z_window", ParamType::USize)
                .with_min(ParamValue::USize(20))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(100))
                .required()
        )
        .metadata("output", "discrete regime: -1, 0, 1")
        .machine_id(BarIndicatorId::Kregime) // TODO: Add to enum
        // Note: "KREGIME" is already the main ID, no need for alias
        .alias("Kregime")
        .alias("kregime")
        .alias("KALMANTRENDREGIME")
        .alias("KalmanTrendRegime")
        .alias("kalmantrendregime")
        .alias("kalman_trend_regime")
        .alias("KALMAN_TREND_REGIME")
        .alias("Kalman_Trend_Regime")
        .build()
}

/// Kalman Regime Score - continuous regime score with sigmoid mapping
pub fn signature_kalman_regime_score() -> IndicatorSignature {
    IndicatorSignature::builder("KSCR", CATEGORY)
        .name("Kalman Regime Score")
        .description("Continuous regime score (0-1) using tanh transformation of velocity z-score")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("window", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("decay", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.9))
                .required()
        )
        .metadata("output", "continuous 0-1 score")
        .metadata("transform", "tanh sigmoid")
        .machine_id(BarIndicatorId::Kscr) // TODO: Add to enum
        // Note: "KSCR" is already the main ID, no need for alias
        .alias("Kscr")
        .alias("kscr")
        .alias("KALMANREGIMESCORE")
        .alias("KalmanRegimeScore")
        .alias("kalmanregimescore")
        .alias("kalman_regime_score")
        .alias("KALMAN_REGIME_SCORE")
        .alias("Kalman_Regime_Score")
        .build()
}

/// Kalman Regime Composite - combined regime score with ATR and volatility
pub fn signature_kalman_regime_composite() -> IndicatorSignature {
    IndicatorSignature::builder("KCOMP", CATEGORY)
        .name("Kalman Regime Composite")
        .description("Composite regime combining Kalman score with ATR and volatility percentiles")
        .add_constraint(
            ParamConstraint::new("dt", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("process_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("measurement_noise", ParamType::F64)
                .with_min(ParamValue::F64(1e-12))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("k_window", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("decay", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.9))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("atr_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("atr_pct_window", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(100))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("vov_vol_window", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("vov_pct_window", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(100))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("w_regime", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("w_atr", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("w_vov", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("complexity", "Multi-component composite")
        .metadata("combines", "Kalman + ATR + Vol percentiles")
        .machine_id(BarIndicatorId::Kcomp) // TODO: Add to enum
        // Note: "KCOMP" is already the main ID, no need for alias
        .alias("Kcomp")
        .alias("kcomp")
        .alias("KALMANREGIMECOMPOSITE")
        .alias("KalmanRegimeComposite")
        .alias("kalmanregimecomposite")
        .alias("kalman_regime_composite")
        .alias("KALMAN_REGIME_COMPOSITE")
        .alias("Kalman_Regime_Composite")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Kalman indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("KALMAN", signature_basic_kalman_filter as fn() -> IndicatorSignature),
    ("EKF", signature_extended_kalman_filter as fn() -> IndicatorSignature),
    ("UKF", signature_unscented_kalman_filter as fn() -> IndicatorSignature),
    ("PARTICLE", signature_particle_filter as fn() -> IndicatorSignature),
    ("ABGFILTER", signature_alpha_beta_gamma_filter as fn() -> IndicatorSignature),
    ("RTS", signature_rts_smoother as fn() -> IndicatorSignature),
    ("KSLOPE", signature_kalman_trend_slope as fn() -> IndicatorSignature),
    ("KSLOPEZ", signature_kalman_slope_zscore as fn() -> IndicatorSignature),
    ("KREGIME", signature_kalman_trend_regime as fn() -> IndicatorSignature),
    ("KSCR", signature_kalman_regime_score as fn() -> IndicatorSignature),
    ("KCOMP", signature_kalman_regime_composite as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static KALMAN_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
///
/// ## Example
/// ```rust
/// use zengeld_chart_indicators::bar_indicators::kalman::kalman_catalog;
///
/// let sig = kalman_catalog::get_signature("KALMAN").unwrap();
/// assert_eq!(sig.id, "KALMAN");
/// ```
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    KALMAN_CATALOG.get(id).map(|f| f())
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
    fn test_get_kalman_signature() {
        let sig = get_signature("KALMAN").unwrap();
        assert_eq!(sig.id, "KALMAN");
        assert_eq!(sig.category, CATEGORY);
        assert_eq!(sig.required_params().len(), 3);
    }

    #[test]
    fn test_get_ekf_signature() {
        let sig = get_signature("EKF").unwrap();
        assert_eq!(sig.id, "EKF");
        assert_eq!(sig.name, "Extended Kalman Filter");
    }

    #[test]
    fn test_get_kcomp_signature() {
        let sig = get_signature("KCOMP").unwrap();
        assert_eq!(sig.id, "KCOMP");
        // KCOMP has 12 required parameters
        assert_eq!(sig.required_params().len(), 12);
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
        assert_eq!(count(), 11); // 11 Kalman filter indicators
    }

    #[test]
    fn test_kalman_validation() {
        let sig = get_signature("KALMAN").unwrap();

        // Valid params
        let params = vec![
            ("dt", ParamValue::F64(1.0)),
            ("process_noise", ParamValue::F64(1.0)),
            ("measurement_noise", ParamValue::F64(1.0)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![
            ("dt", ParamValue::F64(0.0)),
            ("process_noise", ParamValue::F64(1.0)),
            ("measurement_noise", ParamValue::F64(1.0)),
        ];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("KALMAN").unwrap();
        let params = vec![
            ("dt", ParamValue::F64(1.0)),
            ("process_noise", ParamValue::F64(1.0)),
            ("measurement_noise", ParamValue::F64(1.0)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("KALMAN"));
        assert!(key.contains("1"));
    }

    #[test]
    fn test_kslope_cache_key() {
        let sig = get_signature("KSLOPE").unwrap();
        let params = vec![
            ("dt", ParamValue::F64(1.0)),
            ("process_noise", ParamValue::F64(1.0)),
            ("measurement_noise", ParamValue::F64(1.0)),
            ("window", ParamValue::USize(50)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("KSLOPE"));
        assert!(key.contains("50"));
    }
}
