//! divergence_catalog.rs: Complete catalog of all Divergence indicators
//!
//! This catalog contains divergence detection indicators for price/indicator analysis.
//! Divergence indicators detect when price action and technical indicators disagree,
//! often signaling potential reversals or continuation patterns.
//!
//! Types of divergence:
//! - Regular/Classic Bullish: Price makes lower low, indicator makes higher low (reversal signal)
//! - Regular/Classic Bearish: Price makes higher high, indicator makes lower high (reversal signal)
//! - Hidden Bullish: Price makes higher low, indicator makes lower low (continuation signal)
//! - Hidden Bearish: Price makes lower high, indicator makes higher high (continuation signal)

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Divergence;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// CCI Divergence - Commodity Channel Index divergence detection
pub fn signature_cci_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("CCI_DIV", CATEGORY)
        .name("CCI Divergence")
        .description("Detects divergence between price and CCI indicator")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("lookback_swings", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(10))
                .with_default(ParamValue::USize(5))
        )
        .metadata("indicator", "CCI")
        .metadata("signals", "bullish_regular, bearish_regular, bullish_hidden, bearish_hidden")
        .machine_id(BarIndicatorId::CciDiv) // TODO: Add to enum
        // Note: "CCI_DIV" is already the main ID, no need for alias
        .alias("CciDiv")
        .alias("cci_div")
        .alias("CCIDIVERGENCE")
        .alias("CCIDivergence")
        .alias("ccidivergence")
        .alias("cci_divergence")
        .alias("CCI_DIVERGENCE")
        .alias("Cci_Divergence")
        .build()
}

/// Classic Divergence Detector - Generic regular divergence detection
pub fn signature_classic_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("CLASSIC_DIV", CATEGORY)
        .name("Classic Divergence")
        .description("Detects regular (classic) bullish and bearish divergences between price and any oscillator")
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("min_swing_distance", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
        )
        .metadata("type", "regular/classic")
        .metadata("signals", "bullish, bearish")
        .machine_id(BarIndicatorId::ClassicDiv) // TODO: Add to enum
        // Note: "CLASSIC_DIV" is already the main ID, no need for alias
        .alias("ClassicDiv")
        .alias("classic_div")
        .alias("CLASSICDIVERGENCE")
        .alias("ClassicDivergence")
        .alias("classicdivergence")
        .alias("classic_divergence")
        .alias("CLASSIC_DIVERGENCE")
        .alias("Classic_Divergence")
        .build()
}

/// Divergence Strength - Measures the magnitude of divergence
pub fn signature_divergence_strength() -> IndicatorSignature {
    IndicatorSignature::builder("DIV_STRENGTH", CATEGORY)
        .name("Divergence Strength")
        .description("Quantifies the strength/magnitude of detected divergences (0-100)")
        .add_constraint(ParamConstraint::period(5, 200, 14))
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .metadata("range", "0-100")
        .metadata("interpretation", "0 = no divergence, 100 = maximum divergence")
        .machine_id(BarIndicatorId::DivStrength) // TODO: Add to enum
        // Note: "DIV_STRENGTH" is already the main ID, no need for alias
        .alias("DivStrength")
        .alias("div_strength")
        .alias("DIVERGENCESTRENGTH")
        .alias("DivergenceStrength")
        .alias("divergencestrength")
        .alias("divergence_strength")
        .alias("DIVERGENCE_STRENGTH")
        .alias("Divergence_Strength")
        .build()
}

/// Hidden Divergence Detector - Detects continuation patterns
pub fn signature_hidden_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("HIDDEN_DIV", CATEGORY)
        .name("Hidden Divergence")
        .description("Detects hidden divergences (trend continuation signals)")
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("min_swing_distance", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
        )
        .metadata("type", "hidden/continuation")
        .metadata("signals", "bullish_hidden, bearish_hidden")
        .machine_id(BarIndicatorId::HiddenDiv) // TODO: Add to enum
        // Note: "HIDDEN_DIV" is already the main ID, no need for alias
        .alias("HiddenDiv")
        .alias("hidden_div")
        .alias("HIDDENDIVERGENCE")
        .alias("HiddenDivergence")
        .alias("hiddendivergence")
        .alias("hidden_divergence")
        .alias("HIDDEN_DIVERGENCE")
        .alias("Hidden_Divergence")
        .build()
}

/// MACD Divergence - Moving Average Convergence Divergence divergence
pub fn signature_macd_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("MACD_DIV", CATEGORY)
        .name("MACD Divergence")
        .description("Detects divergence between price and MACD histogram")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(12))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(26))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(9))
        )
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .metadata("indicator", "MACD")
        .metadata("author", "Gerald Appel")
        .machine_id(BarIndicatorId::MacdDiv) // TODO: Add to enum
        // Note: "MACD_DIV" is already the main ID, no need for alias
        .alias("MacdDiv")
        .alias("macd_div")
        .alias("MACDDIVERGENCE")
        .alias("MACDDivergence")
        .alias("macddivergence")
        .alias("macd_divergence")
        .alias("MACD_DIVERGENCE")
        .alias("Macd_Divergence")
        .build()
}

/// MACD Histogram Divergence - Specifically tracks histogram divergence
pub fn signature_macd_histogram_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("MACD_HIST_DIV", CATEGORY)
        .name("MACD Histogram Divergence")
        .description("Detects divergence using MACD histogram exclusively")
        .add_constraint(ParamConstraint::period(5, 100, 12))
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .metadata("based_on", "MACD histogram")
        .machine_id(BarIndicatorId::MacdHistDiv) // TODO: Add to enum
        // Note: "MACD_HIST_DIV" is already the main ID, no need for alias
        .alias("MacdHistDiv")
        .alias("macd_hist_div")
        .alias("MACDHISTOGRAMDIVERGENCE")
        .alias("MACDHistogramDivergence")
        .alias("macdhistogramdivergence")
        .alias("macd_histogram_divergence")
        .alias("MACD_HISTOGRAM_DIVERGENCE")
        .alias("Macd_Histogram_Divergence")
        .build()
}

/// Multi-Indicator Divergence Consensus - Aggregates multiple divergence signals
pub fn signature_multi_indicator_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("MULTI_DIV", CATEGORY)
        .name("Multi-Indicator Divergence")
        .description("Consensus divergence across RSI, MACD, and Stochastic")
        .add_constraint(ParamConstraint::period(5, 200, 14))
        .add_constraint(
            ParamConstraint::new("min_confirmations", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(3))
                .with_default(ParamValue::USize(2))
        )
        .metadata("indicators", "RSI, MACD, Stochastic")
        .metadata("type", "consensus")
        .machine_id(BarIndicatorId::MultiDiv) // TODO: Add to enum
        // Note: "MULTI_DIV" is already the main ID, no need for alias
        .alias("MultiDiv")
        .alias("multi_div")
        .alias("MULTIINDICATORDIVERGENCE")
        .alias("MultiIndicatorDivergence")
        .alias("multiindicatordivergence")
        .alias("multi_indicator_divergence")
        .alias("MULTI_INDICATOR_DIVERGENCE")
        .alias("Multi_Indicator_Divergence")
        .build()
}

/// OBV Divergence - On-Balance Volume divergence
pub fn signature_obv_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("OBV_DIV", CATEGORY)
        .name("OBV Divergence")
        .description("Detects divergence between price and On-Balance Volume")
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .add_constraint(
            ParamConstraint::new("smooth_period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
        )
        .metadata("indicator", "OBV")
        .metadata("uses_volume", "true")
        .metadata("author", "Joseph Granville")
        .machine_id(BarIndicatorId::ObvDiv) // TODO: Add to enum
        // Note: "OBV_DIV" is already the main ID, no need for alias
        .alias("ObvDiv")
        .alias("obv_div")
        .alias("OBVDIVERGENCE")
        .alias("OBVDivergence")
        .alias("obvdivergence")
        .alias("obv_divergence")
        .alias("OBV_DIVERGENCE")
        .alias("Obv_Divergence")
        .build()
}

/// RSI Divergence - Relative Strength Index divergence
pub fn signature_rsi_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("RSI_DIV", CATEGORY)
        .name("RSI Divergence")
        .description("Detects divergence between price and RSI indicator")
        .add_constraint(
            ParamConstraint::new("rsi_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .add_constraint(
            ParamConstraint::new("min_swing_distance", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
        )
        .metadata("indicator", "RSI")
        .metadata("author", "J. Welles Wilder")
        .machine_id(BarIndicatorId::RsiDiv) // TODO: Add to enum
        // Note: "RSI_DIV" is already the main ID, no need for alias
        .alias("RsiDiv")
        .alias("rsi_div")
        .alias("RSIDIVERGENCE")
        .alias("RSIDivergence")
        .alias("rsidivergence")
        .alias("rsi_divergence")
        .alias("RSI_DIVERGENCE")
        .alias("Rsi_Divergence")
        .build()
}

/// Stochastic Divergence - Stochastic oscillator divergence
pub fn signature_stochastic_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("STOCH_DIV", CATEGORY)
        .name("Stochastic Divergence")
        .description("Detects divergence between price and Stochastic oscillator")
        .add_constraint(
            ParamConstraint::new("k_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("d_period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(3))
        )
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .metadata("indicator", "Stochastic")
        .metadata("author", "George Lane")
        .machine_id(BarIndicatorId::StochDiv) // TODO: Add to enum
        // Note: "STOCH_DIV" is already the main ID, no need for alias
        .alias("StochDiv")
        .alias("stoch_div")
        .alias("STOCHASTICDIVERGENCE")
        .alias("StochasticDivergence")
        .alias("stochasticdivergence")
        .alias("stochastic_divergence")
        .alias("STOCHASTIC_DIVERGENCE")
        .alias("Stochastic_Divergence")
        .build()
}

/// Volume Divergence - Price vs Volume divergence
pub fn signature_volume_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("VOL_DIV", CATEGORY)
        .name("Volume Divergence")
        .description("Detects divergence between price movement and volume")
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .add_constraint(
            ParamConstraint::new("volume_ma_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(20))
        )
        .metadata("uses_volume", "true")
        .metadata("type", "volume_analysis")
        .machine_id(BarIndicatorId::VolDiv) // TODO: Add to enum
        // Note: "VOL_DIV" is already the main ID, no need for alias
        .alias("VolDiv")
        .alias("vol_div")
        .alias("VOLUMEDIVERGENCE")
        .alias("VolumeDivergence")
        .alias("volumedivergence")
        .alias("volume_divergence")
        .alias("VOLUME_DIVERGENCE")
        .alias("Volume_Divergence")
        .build()
}

/// Williams %R Divergence - Williams Percent Range divergence
pub fn signature_williams_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("WILLIAMS_DIV", CATEGORY)
        .name("Williams %R Divergence")
        .description("Detects divergence between price and Williams %R indicator")
        .add_constraint(
            ParamConstraint::new("period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("swing_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .metadata("indicator", "Williams %R")
        .metadata("author", "Larry Williams")
        .machine_id(BarIndicatorId::WilliamsDiv) // TODO: Add to enum
        // Note: "WILLIAMS_DIV" is already the main ID, no need for alias
        .alias("WilliamsDiv")
        .alias("williams_div")
        .alias("WILLIAMS%RDIVERGENCE")
        .alias("Williams%RDivergence")
        .alias("williams%rdivergence")
        .alias("williams_%r_divergence")
        .alias("WILLIAMS_%R_DIVERGENCE")
        .alias("Williams_%r_Divergence")
        .build()
}

/// ZigZag-Based Divergence - Uses ZigZag swings for divergence detection
pub fn signature_zigzag_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG_DIV", CATEGORY)
        .name("ZigZag Divergence")
        .description("Detects divergence using ZigZag swing points for precise pivot identification")
        .add_constraint(
            ParamConstraint::new("zigzag_deviation", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.5))
                .with_default(ParamValue::F64(0.05))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("indicator_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(14))
        )
        .add_constraint(
            ParamConstraint::new("min_swings", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(10))
                .with_default(ParamValue::USize(2))
        )
        .metadata("based_on", "ZigZag")
        .metadata("precision", "high")
        .machine_id(BarIndicatorId::ZigzagDiv) // TODO: Add to enum
        // Note: "ZIGZAG_DIV" is already the main ID, no need for alias
        .alias("ZigzagDiv")
        .alias("zigzag_div")
        .alias("ZIGZAGDIVERGENCE")
        .alias("ZigZagDivergence")
        .alias("zigzagdivergence")
        .alias("zigzag_divergence")
        .alias("ZIGZAG_DIVERGENCE")
        .alias("Zigzag_Divergence")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Divergence indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("CCI_DIV", signature_cci_divergence as fn() -> IndicatorSignature),
    ("CLASSIC_DIV", signature_classic_divergence as fn() -> IndicatorSignature),
    ("DIV_STRENGTH", signature_divergence_strength as fn() -> IndicatorSignature),
    ("HIDDEN_DIV", signature_hidden_divergence as fn() -> IndicatorSignature),
    ("MACD_DIV", signature_macd_divergence as fn() -> IndicatorSignature),
    ("MACD_HIST_DIV", signature_macd_histogram_divergence as fn() -> IndicatorSignature),
    ("MULTI_DIV", signature_multi_indicator_divergence as fn() -> IndicatorSignature),
    ("OBV_DIV", signature_obv_divergence as fn() -> IndicatorSignature),
    ("RSI_DIV", signature_rsi_divergence as fn() -> IndicatorSignature),
    ("STOCH_DIV", signature_stochastic_divergence as fn() -> IndicatorSignature),
    ("VOL_DIV", signature_volume_divergence as fn() -> IndicatorSignature),
    ("WILLIAMS_DIV", signature_williams_divergence as fn() -> IndicatorSignature),
    ("ZIGZAG_DIV", signature_zigzag_divergence as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static DIVERGENCE_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    DIVERGENCE_CATALOG.get(id).map(|f| f())
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
    fn test_get_rsi_divergence_signature() {
        let sig = get_signature("RSI_DIV").unwrap();
        assert_eq!(sig.id, "RSI_DIV");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_macd_divergence_signature() {
        let sig = get_signature("MACD_DIV").unwrap();
        assert_eq!(sig.id, "MACD_DIV");
        assert_eq!(sig.required_params().len(), 2); // fast + slow
    }

    #[test]
    fn test_get_classic_divergence_signature() {
        let sig = get_signature("CLASSIC_DIV").unwrap();
        assert_eq!(sig.id, "CLASSIC_DIV");
        assert!(sig.required_params().len() > 0);
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
        assert_eq!(count(), 13); // 13 divergence indicators
    }

    #[test]
    fn test_rsi_divergence_validation() {
        let sig = get_signature("RSI_DIV").unwrap();

        // Valid params
        let params = vec![("rsi_period", ParamValue::USize(14))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("rsi_period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("RSI_DIV").unwrap();
        let params = vec![("rsi_period", ParamValue::USize(14))];
        let key = sig.cache_key(&params);
        assert!(key.contains("RSI_DIV"));
        assert!(key.contains("14"));
    }

    #[test]
    fn test_zigzag_divergence_parameters() {
        let sig = get_signature("ZIGZAG_DIV").unwrap();
        assert_eq!(sig.id, "ZIGZAG_DIV");

        // Check that zigzag_deviation is required
        let required = sig.required_params();
        assert!(required.iter().any(|p| *p == "zigzag_deviation"));
    }

    #[test]
    fn test_volume_divergence_metadata() {
        let sig = get_signature("VOL_DIV").unwrap();
        assert_eq!(sig.get_metadata("uses_volume"), Some("true"));
    }

    #[test]
    fn test_multi_indicator_divergence() {
        let sig = get_signature("MULTI_DIV").unwrap();
        assert_eq!(sig.id, "MULTI_DIV");
        // Description varies - just check it exists
        assert!(!sig.description.is_empty());
    }
}
