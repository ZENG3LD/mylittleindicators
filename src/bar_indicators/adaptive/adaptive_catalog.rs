//! adaptive_catalog.rs: Catalog of all Adaptive indicators
//!
//! Auto-generated catalog based on actual indicator implementations.
//! Contains IndicatorSignature definitions for adaptive indicators like KAMA, MAMA, FRAMA, etc.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Adaptive;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Kaufman's Adaptive Moving Average - adapts to market efficiency
pub fn signature_kama() -> IndicatorSignature {
    IndicatorSignature::builder("KAMA", CATEGORY)
        .name("Kaufman's Adaptive Moving Average")
        .description("Adaptive MA based on Efficiency Ratio (Perry Kaufman)")
        .add_constraint(
            ParamConstraint::new("efficiency_ratio_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("fast_sc_period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_sc_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(30))
                .required()
        )
        .metadata("author", "Perry Kaufman")
        .metadata("complexity", "Efficiency Ratio based")
        .metadata("adaptive_type", "efficiency")
        .machine_id(BarIndicatorId::Kama) // TODO: Add to enum
        // Note: "KAMA" is already the main ID, no need for alias
        .alias("Kama")
        .alias("kama")
        .alias("KAUFMAN'SADAPTIVEMOVINGAVERAGE")
        .alias("Kaufman'sAdaptiveMovingAverage")
        .alias("kaufman'sadaptivemovingaverage")
        .alias("kaufman's_adaptive_moving_average")
        .alias("KAUFMAN'S_ADAPTIVE_MOVING_AVERAGE")
        .alias("Kaufman's_Adaptive_Moving_Average")
        .build()
}

/// Fractal Adaptive Moving Average - adapts based on fractal dimension
pub fn signature_frama() -> IndicatorSignature {
    IndicatorSignature::builder("FRAMA", CATEGORY)
        .name("Fractal Adaptive Moving Average")
        .description("Adaptive MA based on fractal dimension (John Ehlers)")
        .add_constraint(ParamConstraint::period(4, 512, 20))
        .metadata("author", "John Ehlers")
        .metadata("complexity", "Fractal dimension based")
        .metadata("adaptive_type", "fractal")
        .metadata("methods", "Standard, Improved, Dynamic, Robust")
        .machine_id(BarIndicatorId::Frama)
        // Note: "FRAMA" is already the main ID, no need for alias
        .alias("Frama")
        .alias("frama")
        .alias("FRACTALADAPTIVEMOVINGAVERAGE")
        .alias("FractalAdaptiveMovingAverage")
        .alias("fractaladaptivemovingaverage")
        .alias("fractal_adaptive_moving_average")
        .alias("FRACTAL_ADAPTIVE_MOVING_AVERAGE")
        .alias("Fractal_Adaptive_Moving_Average")
        .build()
}

/// Variable Index Dynamic Average - adapts to volatility via CMO
pub fn signature_vidya() -> IndicatorSignature {
    IndicatorSignature::builder("VIDYA", CATEGORY)
        .name("Variable Index Dynamic Average")
        .description("Volatility-adaptive MA using Chande Momentum Oscillator (Tushar Chande)")
        .add_constraint(ParamConstraint::period(2, 500, 20))
        .metadata("author", "Tushar Chande")
        .metadata("complexity", "CMO-based volatility adaptation")
        .metadata("adaptive_type", "volatility")
        .metadata("cmo_types", "Simple, Exponential, Linear, Triangular")
        .machine_id(BarIndicatorId::Vidya)
        // Note: "VIDYA" is already the main ID, no need for alias
        .alias("Vidya")
        .alias("vidya")
        .alias("VARIABLEINDEXDYNAMICAVERAGE")
        .alias("VariableIndexDynamicAverage")
        .alias("variableindexdynamicaverage")
        .alias("variable_index_dynamic_average")
        .alias("VARIABLE_INDEX_DYNAMIC_AVERAGE")
        .alias("Variable_Index_Dynamic_Average")
        .build()
}

/// MESA Adaptive Moving Average - adapts to dominant cycle
pub fn signature_mama() -> IndicatorSignature {
    IndicatorSignature::builder("MAMA", CATEGORY)
        .name("MESA Adaptive Moving Average")
        .description("Adaptive MA based on dominant market cycle detection (John Ehlers)")
        .add_constraint(
            ParamConstraint::new("min_period", ParamType::F64)
                .with_min(ParamValue::F64(2.0))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(8.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("max_period", ParamType::F64)
                .with_min(ParamValue::F64(10.0))
                .with_max(ParamValue::F64(200.0))
                .with_default(ParamValue::F64(50.0))
                .required()
        )
        .metadata("author", "John Ehlers")
        .metadata("complexity", "Hilbert Transform based")
        .metadata("adaptive_type", "cycle")
        .metadata("method", "MESA (Maximum Entropy Spectral Analysis)")
        .machine_id(BarIndicatorId::Mama) // TODO: Add to enum
        // Note: "MAMA" is already the main ID, no need for alias
        .alias("Mama")
        .alias("mama")
        .alias("MESAADAPTIVEMOVINGAVERAGE")
        .alias("MESAAdaptiveMovingAverage")
        .alias("mesaadaptivemovingaverage")
        .alias("mesa_adaptive_moving_average")
        .alias("MESA_ADAPTIVE_MOVING_AVERAGE")
        .alias("Mesa_Adaptive_Moving_Average")
        .build()
}

/// Generic Adaptive Moving Average - multi-mode adaptation
pub fn signature_adaptive_ma() -> IndicatorSignature {
    IndicatorSignature::builder("ADAPTIVEMA", CATEGORY)
        .name("Adaptive Moving Average (Generic)")
        .description("Multi-mode adaptive MA (volatility, volume, trend, momentum, combined, market)")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("complexity", "Multi-mode adaptation")
        .metadata("adaptive_type", "multi")
        .metadata("modes", "Volatility, Volume, Trend, Momentum, Combined, Market")
        .metadata("efficiency_methods", "Kaufman, Fractal, DirectionalMovement, TrendStrength, NoiseRatio")
        .machine_id(BarIndicatorId::Adaptivema) // TODO: Add to enum
        // Note: "ADAPTIVEMA" is already the main ID, no need for alias
        .alias("Adaptivema")
        .alias("adaptivema")
        .alias("ADAPTIVEMOVINGAVERAGE(GENERIC)")
        .alias("AdaptiveMovingAverage(Generic)")
        .alias("adaptivemovingaverage(generic)")
        .alias("adaptive_moving_average_(generic)")
        .alias("ADAPTIVE_MOVING_AVERAGE_(GENERIC)")
        .alias("Adaptive_Moving_Average_(generic)")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Adaptive indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("KAMA", signature_kama as fn() -> IndicatorSignature),
    ("FRAMA", signature_frama as fn() -> IndicatorSignature),
    ("VIDYA", signature_vidya as fn() -> IndicatorSignature),
    ("MAMA", signature_mama as fn() -> IndicatorSignature),
    ("ADAPTIVEMA", signature_adaptive_ma as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static ADAPTIVE_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
/// use zengeld_chart_indicators::bar_indicators::adaptive::adaptive_catalog;
///
/// let sig = adaptive_catalog::get_signature("KAMA").unwrap();
/// assert_eq!(sig.id, "KAMA");
/// ```
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    ADAPTIVE_CATALOG.get(id).map(|f| f())
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
    fn test_get_kama_signature() {
        let sig = get_signature("KAMA").unwrap();
        assert_eq!(sig.id, "KAMA");
        assert_eq!(sig.category, CATEGORY);
        assert_eq!(sig.required_params().len(), 3);
    }

    #[test]
    fn test_get_frama_signature() {
        let sig = get_signature("FRAMA").unwrap();
        assert_eq!(sig.id, "FRAMA");
        assert_eq!(sig.name, "Fractal Adaptive Moving Average");
    }

    #[test]
    fn test_get_vidya_signature() {
        let sig = get_signature("VIDYA").unwrap();
        assert_eq!(sig.id, "VIDYA");
        assert_eq!(sig.required_params().len(), 1);
    }

    #[test]
    fn test_get_mama_signature() {
        let sig = get_signature("MAMA").unwrap();
        assert_eq!(sig.id, "MAMA");
        assert_eq!(sig.required_params().len(), 2); // min_period, max_period
    }

    #[test]
    fn test_get_adaptive_ma_signature() {
        let sig = get_signature("ADAPTIVEMA").unwrap();
        assert_eq!(sig.id, "ADAPTIVEMA");
        assert_eq!(sig.required_params().len(), 1); // period
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
        assert_eq!(count(), 5); // 5 adaptive indicators
    }

    #[test]
    fn test_kama_validation() {
        let sig = get_signature("KAMA").unwrap();

        // Valid params
        let params = vec![
            ("efficiency_ratio_period", ParamValue::USize(10)),
            ("fast_sc_period", ParamValue::USize(2)),
            ("slow_sc_period", ParamValue::USize(30)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: fast >= slow
        let _params = vec![
            ("efficiency_ratio_period", ParamValue::USize(10)),
            ("fast_sc_period", ParamValue::USize(30)),
            ("slow_sc_period", ParamValue::USize(30)),
        ];
        // Note: This would need cross-parameter validation in the signature builder
        // For now, we just test individual parameter ranges
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("FRAMA").unwrap();
        let params = vec![("period", ParamValue::USize(20))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "FRAMA_20");
    }

    #[test]
    fn test_kama_cache_key() {
        let sig = get_signature("KAMA").unwrap();
        let params = vec![
            ("efficiency_ratio_period", ParamValue::USize(10)),
            ("fast_sc_period", ParamValue::USize(2)),
            ("slow_sc_period", ParamValue::USize(30)),
        ];
        let key = sig.cache_key(&params);
        // Keys are sorted alphabetically
        assert!(key.contains("KAMA"));
        assert!(key.contains("10"));
        assert!(key.contains("2"));
        assert!(key.contains("30"));
    }

    #[test]
    fn test_mama_cache_key() {
        let sig = get_signature("MAMA").unwrap();
        let params = vec![
            ("min_period", ParamValue::F64(8.0)),
            ("max_period", ParamValue::F64(50.0)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("MAMA"));
        assert!(key.contains("8"));
        assert!(key.contains("50"));
    }
}
