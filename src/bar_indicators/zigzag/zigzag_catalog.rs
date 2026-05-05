//! zigzag_catalog.rs: Complete catalog of all ZigZag indicators
//!
//! ZigZag indicators filter price movements by percentage/ATR to identify significant swings.
//! Includes classic zigzag, ATR-based, time-based, candle-pattern, and lookahead variants.
//! Contains 5 zigzag indicators extracted from actual implementations.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Zigzag;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// ZigZag ATR - ZigZag using ATR multiplier threshold
/// Uses Average True Range to dynamically adjust the minimum price movement threshold
pub fn signature_zigzag_atr() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG_ATR", CATEGORY)
        .name("ZigZag ATR")
        .description("ZigZag indicator using ATR-based threshold for swing detection")
        .add_constraint(
            ParamConstraint::new("zigzag_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("atr_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("atr_multiplier", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.5))
                .required()
        )
        .metadata("outputs", "swings, last_extreme, direction")
        .metadata("uses_atr", "true")
        .machine_id(BarIndicatorId::ZigzagAtr) // TODO: Add to enum
        // Note: "ZIGZAG_ATR" is already the main ID, no need for alias
        .alias("ZigzagAtr")
        .alias("zigzag_atr")
        .alias("ZIGZAGATR")
        .alias("ZigZagATR")
        .alias("zigzagatr")
        .alias("Zigzag_Atr")
        .build()
}

/// ZigZag Candle - ZigZag based on N-bar swing patterns
/// Identifies swing highs/lows using candle pattern recognition
pub fn signature_zigzag_candle() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG_CANDLE", CATEGORY)
        .name("ZigZag Candle")
        .description("ZigZag using N-bar swing pattern recognition for swing detection")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("swing_bars", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .metadata("outputs", "swings")
        .metadata("method", "candle_pattern")
        .machine_id(BarIndicatorId::ZigzagCandle) // TODO: Add to enum
        // Note: "ZIGZAG_CANDLE" is already the main ID, no need for alias
        .alias("ZigzagCandle")
        .alias("zigzag_candle")
        .alias("ZIGZAGCANDLE")
        .alias("ZigZagCandle")
        .alias("zigzagcandle")
        .alias("Zigzag_Candle")
        .build()
}

/// ZigZag Classic - Traditional percentage/absolute threshold ZigZag
/// The most common ZigZag implementation using fixed thresholds
pub fn signature_zigzag_classic() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG_CLASSIC", CATEGORY)
        .name("ZigZag Classic")
        .description("Classic ZigZag with percentage or absolute price threshold")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("threshold_percent", ParamType::F64)
                .with_min(ParamValue::F64(0.5))
                .with_max(ParamValue::F64(50.0))
                .with_default(ParamValue::F64(5.0))
        )
        .add_constraint(
            ParamConstraint::new("threshold_abs", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(1000.0))
                .with_default(ParamValue::F64(10.0))
        )
        .metadata("outputs", "swings, last_extreme, direction")
        .metadata("note", "Either threshold_percent or threshold_abs can be used")
        .machine_id(BarIndicatorId::ZigzagClassic) // TODO: Add to enum
        // Note: "ZIGZAG_CLASSIC" is already the main ID, no need for alias
        .alias("ZigzagClassic")
        .alias("zigzag_classic")
        .alias("ZIGZAGCLASSIC")
        .alias("ZigZagClassic")
        .alias("zigzagclassic")
        .alias("Zigzag_Classic")
        .build()
}

/// ZigZag Lookahead - ZigZag with lookahead confirmation
/// Waits for N bars to confirm a swing before marking it
pub fn signature_zigzag_lookahead() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG_LOOKAHEAD", CATEGORY)
        .name("ZigZag Lookahead")
        .description("ZigZag with lookahead confirmation to reduce false swings")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("lookahead", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .metadata("outputs", "swings, candidates")
        .metadata("note", "Introduces lag due to confirmation period")
        .machine_id(BarIndicatorId::ZigzagLookahead) // TODO: Add to enum
        // Note: "ZIGZAG_LOOKAHEAD" is already the main ID, no need for alias
        .alias("ZigzagLookahead")
        .alias("zigzag_lookahead")
        .alias("ZIGZAGLOOKAHEAD")
        .alias("ZigZagLookahead")
        .alias("zigzaglookahead")
        .alias("Zigzag_Lookahead")
        .build()
}

/// ZigZag Time - ZigZag based on minimum time between swings
/// Forces a minimum number of bars between consecutive swings
pub fn signature_zigzag_time() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG_TIME", CATEGORY)
        .name("ZigZag Time")
        .description("ZigZag with minimum time (bars) requirement between swings")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("min_bars", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .metadata("outputs", "swings, last_extreme")
        .metadata("method", "time_based")
        .machine_id(BarIndicatorId::ZigzagTime) // TODO: Add to enum
        // Note: "ZIGZAG_TIME" is already the main ID, no need for alias
        .alias("ZigzagTime")
        .alias("zigzag_time")
        .alias("ZIGZAGTIME")
        .alias("ZigZagTime")
        .alias("zigzagtime")
        .alias("Zigzag_Time")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all ZigZag indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ZIGZAG_ATR", signature_zigzag_atr as fn() -> IndicatorSignature),
    ("ZIGZAG_CANDLE", signature_zigzag_candle as fn() -> IndicatorSignature),
    ("ZIGZAG_CLASSIC", signature_zigzag_classic as fn() -> IndicatorSignature),
    ("ZIGZAG_LOOKAHEAD", signature_zigzag_lookahead as fn() -> IndicatorSignature),
    ("ZIGZAG_TIME", signature_zigzag_time as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static ZIGZAG_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    ZIGZAG_CATALOG.get(id).map(|f| f())
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
    fn test_get_zigzag_classic_signature() {
        let sig = get_signature("ZIGZAG_CLASSIC").unwrap();
        assert_eq!(sig.id, "ZIGZAG_CLASSIC");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_zigzag_atr_signature() {
        let sig = get_signature("ZIGZAG_ATR").unwrap();
        assert_eq!(sig.id, "ZIGZAG_ATR");
        assert_eq!(sig.required_params().len(), 3); // zigzag_period + atr_period + atr_multiplier
    }

    #[test]
    fn test_get_zigzag_time_signature() {
        let sig = get_signature("ZIGZAG_TIME").unwrap();
        assert_eq!(sig.id, "ZIGZAG_TIME");
        assert_eq!(sig.required_params().len(), 2); // period + min_bars
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
        assert_eq!(count(), 5); // 5 zigzag indicators
    }

    #[test]
    fn test_zigzag_classic_validation() {
        let sig = get_signature("ZIGZAG_CLASSIC").unwrap();

        // Valid params with threshold_percent
        let params = vec![
            ("period", ParamValue::USize(20)),
            ("threshold_percent", ParamValue::F64(5.0)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Valid params with threshold_abs
        let params = vec![
            ("period", ParamValue::USize(20)),
            ("threshold_abs", ParamValue::F64(10.0)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: period out of range
        let params = vec![
            ("period", ParamValue::USize(2)),
            ("threshold_percent", ParamValue::F64(5.0)),
        ];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_zigzag_atr_validation() {
        let sig = get_signature("ZIGZAG_ATR").unwrap();

        // Valid params
        let params = vec![
            ("zigzag_period", ParamValue::USize(20)),
            ("atr_period", ParamValue::USize(14)),
            ("atr_multiplier", ParamValue::F64(1.5)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: atr_multiplier out of range
        let params = vec![
            ("zigzag_period", ParamValue::USize(20)),
            ("atr_period", ParamValue::USize(14)),
            ("atr_multiplier", ParamValue::F64(0.05)),
        ];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("ZIGZAG_CLASSIC").unwrap();
        let params = vec![
            ("period", ParamValue::USize(20)),
            ("threshold_percent", ParamValue::F64(5.0)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("ZIGZAG_CLASSIC"));
        assert!(key.contains("20"));
    }

    #[test]
    fn test_zigzag_atr_cache_key() {
        let sig = get_signature("ZIGZAG_ATR").unwrap();
        let params = vec![
            ("zigzag_period", ParamValue::USize(20)),
            ("atr_period", ParamValue::USize(14)),
            ("atr_multiplier", ParamValue::F64(1.5)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("ZIGZAG_ATR"));
        assert!(key.contains("20"));
        assert!(key.contains("14"));
    }
}
