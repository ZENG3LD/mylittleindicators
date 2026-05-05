//! ratio_catalog.rs: Complete catalog of all Ratio indicators
//!
//! This catalog contains ratio indicators including efficiency ratios, spread analyzers,
//! and relative metrics. Organized alphabetically for easy navigation.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Ratio;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// Efficiency Ratio - Kaufman's Efficiency Ratio
pub fn signature_efficiency_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("ER", CATEGORY)
        .name("Efficiency Ratio")
        .description("Kaufman's Efficiency Ratio - measures trend efficiency")
        .add_constraint(ParamConstraint::period(2, 512, 10))
        .metadata("author", "Perry Kaufman")
        .metadata("range", "0-1")
        .metadata("description", "Ratio of net price change to sum of absolute price changes")
        .machine_id(BarIndicatorId::Er) // TODO: Add to enum
        // Note: "ER" is already the main ID, no need for alias
        .alias("Er")
        .alias("er")
        .alias("EFFICIENCYRATIO")
        .alias("EfficiencyRatio")
        .alias("efficiencyratio")
        .alias("efficiency_ratio")
        .alias("EFFICIENCY_RATIO")
        .alias("Efficiency_Ratio")
        .build()
}

/// Efficiency Ratio Ring - Ring buffer implementation
pub fn signature_efficiency_ratio_ring() -> IndicatorSignature {
    IndicatorSignature::builder("ER_RING", CATEGORY)
        .name("Efficiency Ratio Ring")
        .description("Ring buffer implementation of Efficiency Ratio for fixed memory")
        .add_constraint(ParamConstraint::period(2, 512, 10))
        .metadata("author", "Perry Kaufman")
        .metadata("range", "0-1")
        .metadata("implementation", "fixed memory ring buffer")
        .machine_id(BarIndicatorId::ErRing) // TODO: Add to enum
        // Note: "ER_RING" is already the main ID, no need for alias
        .alias("ErRing")
        .alias("er_ring")
        .alias("EFFICIENCYRATIORING")
        .alias("EfficiencyRatioRing")
        .alias("efficiencyratioring")
        .alias("efficiency_ratio_ring")
        .alias("EFFICIENCY_RATIO_RING")
        .alias("Efficiency_Ratio_Ring")
        .build()
}

/// Range to ATR Ratio - Daily range relative to ATR
pub fn signature_range_to_atr() -> IndicatorSignature {
    IndicatorSignature::builder("RANGE_ATR", CATEGORY)
        .name("Range to ATR Ratio")
        .description("Ratio of bar range (high-low) to Average True Range")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .metadata("interpretation", ">1 means larger than average range, <1 means smaller")
        .metadata("note", "Traditionally uses Wilder MA for ATR, but can optimize with any type")
        .machine_id(BarIndicatorId::RangeAtr) // TODO: Add to enum
        // Note: "RANGE_ATR" is already the main ID, no need for alias
        .alias("RangeAtr")
        .alias("range_atr")
        .alias("RANGETOATRRATIO")
        .alias("RangetoATRRatio")
        .alias("rangetoatrratio")
        .alias("range_to_atr_ratio")
        .alias("RANGE_TO_ATR_RATIO")
        .alias("Range_To_Atr_Ratio")
        .build()
}

/// Spread Analyzer - Bid-Ask spread analysis
pub fn signature_spread_analyzer() -> IndicatorSignature {
    IndicatorSignature::builder("SPREAD_ANALYZER", CATEGORY)
        .name("Spread Analyzer")
        .description("Analyzes bid-ask spread statistics over time")
        .add_constraint(ParamConstraint::period(1, 512, 20))
        .metadata("outputs", "current_spread, average_spread")
        .metadata("uses_market_data", "bid/ask")
        .machine_id(BarIndicatorId::SpreadAnalyzer)
        // Note: "SPREAD_ANALYZER" is already the main ID, no need for alias
        .alias("SpreadAnalyzer")
        .alias("spread_analyzer")
        .alias("SPREADANALYZER")
        .alias("spreadanalyzer")
        .alias("Spread_Analyzer")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Ratio indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ER", signature_efficiency_ratio as fn() -> IndicatorSignature),
    ("ER_RING", signature_efficiency_ratio_ring as fn() -> IndicatorSignature),
    ("RANGE_ATR", signature_range_to_atr as fn() -> IndicatorSignature),
    ("SPREAD_ANALYZER", signature_spread_analyzer as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static RATIO_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    RATIO_CATALOG.get(id).map(|f| f())
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
    use crate::ParamValue;

    #[test]
    fn test_get_efficiency_ratio_signature() {
        let sig = get_signature("ER").unwrap();
        assert_eq!(sig.id, "ER");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_range_to_atr_signature() {
        let sig = get_signature("RANGE_ATR").unwrap();
        assert_eq!(sig.id, "RANGE_ATR");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_spread_analyzer_signature() {
        let sig = get_signature("SPREAD_ANALYZER").unwrap();
        assert_eq!(sig.id, "SPREAD_ANALYZER");
        assert_eq!(sig.category, CATEGORY);
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
        assert_eq!(count(), 4); // 4 ratio indicators
    }

    #[test]
    fn test_efficiency_ratio_validation() {
        let sig = get_signature("ER").unwrap();

        // Valid params
        let params = vec![("period", ParamValue::USize(10))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_err());

        // Invalid: too large
        let params = vec![("period", ParamValue::USize(1000))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("ER").unwrap();
        let params = vec![("period", ParamValue::USize(10))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "ER_10");
    }

    #[test]
    fn test_range_atr_cache_key() {
        let sig = get_signature("RANGE_ATR").unwrap();
        let params = vec![
            ("period", ParamValue::USize(14)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("RANGE_ATR"));
        assert!(key.contains("14"));
    }

    #[test]
    fn test_spread_analyzer_period_range() {
        let sig = get_signature("SPREAD_ANALYZER").unwrap();

        // Test minimum valid period
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_ok());

        // Test maximum valid period
        let params = vec![("period", ParamValue::USize(512))];
        assert!(sig.validate_params(&params).is_ok());

        // Test out of range
        let params = vec![("period", ParamValue::USize(513))];
        assert!(sig.validate_params(&params).is_err());
    }
}
