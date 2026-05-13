//! candles_catalog.rs: Catalog of Candle indicators
//!
//! Contains IndicatorSignature definitions for the 3 remaining candle indicators.
//! Individual pattern detection moved to `events::CandlePatternDetector`.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Candles;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Heikin-Ashi - Smoothed candlestick representation
pub fn signature_heikin_ashi() -> IndicatorSignature {
    IndicatorSignature::builder("HEIKINASHI", CATEGORY)
        .name("Heikin-Ashi")
        .description("Smoothed candlestick representation reducing noise")
        .metadata("author", "Classic TA")
        .metadata("outputs", "open, high, low, close")
        .metadata("complexity", "O(1)")
        .machine_id(BarIndicatorId::Heikinashi)
        .role_kind(IndicatorRoleKind::Pattern)
        .output_kind(IndicatorValueKind::Candle)
        .alias("Heikinashi")
        .alias("heikinashi")
        .alias("HeikinAshi")
        .alias("heikin_ashi")
        .alias("HEIKIN_ASHI")
        .alias("Heikin_Ashi")
        .build()
}

/// Candle Anatomy - Body and wick ratio analysis
pub fn signature_candle_anatomy() -> IndicatorSignature {
    IndicatorSignature::builder("CANDLEANATOMY", CATEGORY)
        .name("Candle Anatomy")
        .description("Analyzes body, upper wick, and lower wick ratios")
        .add_constraint(
            ParamConstraint::new("long_wick_ratio_threshold", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.5))
                .required()
        )
        .metadata("outputs", "body_ratio, upper_wick_ratio, lower_wick_ratio, long_upper_flag, long_lower_flag")
        .metadata("complexity", "O(1)")
        .machine_id(BarIndicatorId::Candleanatomy)
        .role_kind(IndicatorRoleKind::Pattern)
        .output_kind(IndicatorValueKind::CandleAnatomy)
        .alias("Candleanatomy")
        .alias("candleanatomy")
        .alias("CandleAnatomy")
        .alias("candle_anatomy")
        .alias("CANDLE_ANATOMY")
        .alias("Candle_Anatomy")
        .build()
}

/// Wick Spike - Detects unusually long wicks
pub fn signature_wick_spike() -> IndicatorSignature {
    IndicatorSignature::builder("WICKSPIKE", CATEGORY)
        .name("Wick Spike Detector")
        .description("Flags unusually long wicks vs rolling percentile")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .metadata("outputs", "is_upper_spike, is_lower_spike, upper_percentile, lower_percentile")
        .metadata("threshold", "95th percentile")
        .metadata("complexity", "O(n) per update")
        .machine_id(BarIndicatorId::Wickspike)
        .role_kind(IndicatorRoleKind::Pattern)
        .output_kind(IndicatorValueKind::Flag)
        .alias("Wickspike")
        .alias("wickspike")
        .alias("WICKSPIKEDETECTOR")
        .alias("WickSpikeDetector")
        .alias("wickspikedetector")
        .alias("wick_spike_detector")
        .alias("WICK_SPIKE_DETECTOR")
        .alias("Wick_Spike_Detector")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("HEIKINASHI", signature_heikin_ashi as fn() -> IndicatorSignature),
    ("CANDLEANATOMY", signature_candle_anatomy as fn() -> IndicatorSignature),
    ("WICKSPIKE", signature_wick_spike as fn() -> IndicatorSignature),
];

/// Static catalog of Candle indicators
pub static CANDLES_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();

    for &(main_id, func) in BASE_CATALOG {
        let sig = func();
        m.insert(main_id.to_string(), func);
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
    CANDLES_CATALOG.get(id).map(|f| f())
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
    fn test_get_heikin_ashi_signature() {
        let sig = get_signature("HEIKINASHI").unwrap();
        assert_eq!(sig.id, "HEIKINASHI");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_wickspike_signature() {
        let sig = get_signature("WICKSPIKE").unwrap();
        assert_eq!(sig.id, "WICKSPIKE");
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
        assert_eq!(count(), 3);
    }

    #[test]
    fn test_candle_anatomy_validation() {
        let sig = get_signature("CANDLEANATOMY").unwrap();

        // Valid params
        let params = vec![("long_wick_ratio_threshold", ParamValue::F64(0.5))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("long_wick_ratio_threshold", ParamValue::F64(2.0))];
        assert!(sig.validate_params(&params).is_err());
    }
}
