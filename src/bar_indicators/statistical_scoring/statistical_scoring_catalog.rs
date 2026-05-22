//! statistical_scoring_catalog.rs: Catalog of all Statistical Scoring indicators
//!
//! Contains IndicatorSignature definitions for 6 statistical scoring indicators
//! (normalized scalar outputs: probability, density, tanh-strength, EMA magnitude).

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
    IndicatorRoleKind,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use crate::data_loader::stream_kind::StreamKind;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::StatisticalScoring;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// FVG Reversion Probability - probability of FVG fill
pub fn signature_fvg_reversion_probability() -> IndicatorSignature {
    IndicatorSignature::builder("FVGREV", CATEGORY)
        .name("FVG Reversion Probability")
        .description("Probability of Fair Value Gap being filled within horizon")
        .add_constraint(ParamConstraint::period(1, 50, 10))
        .metadata("statistical", "true")
        .metadata("horizon", "H bars")
        .machine_id(BarIndicatorId::Fvgrev)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("Fvgrev")
        .alias("fvgrev")
        .alias("FVGREVERSIONPROBABILITY")
        .alias("FVGReversionProbability")
        .alias("fvgreversionprobability")
        .alias("fvg_reversion_probability")
        .alias("FVG_REVERSION_PROBABILITY")
        .alias("Fvg_Reversion_Probability")
        .build()
}

/// FVG Duration Intensity Score - scoring FVG duration and intensity
pub fn signature_fvg_duration_intensity() -> IndicatorSignature {
    IndicatorSignature::builder("FVGDUR", CATEGORY)
        .name("FVG Duration Intensity Score")
        .description("Measures duration and intensity of Fair Value Gaps")
        .metadata("scoring", "duration + intensity")
        .machine_id(BarIndicatorId::Fvgdur)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("Fvgdur")
        .alias("fvgdur")
        .alias("FVGDURATIONINTENSITYSCORE")
        .alias("FVGDurationIntensityScore")
        .alias("fvgdurationintensityscore")
        .alias("fvg_duration_intensity_score")
        .alias("FVG_DURATION_INTENSITY_SCORE")
        .alias("Fvg_Duration_Intensity_Score")
        .build()
}

/// FVG Intensity Alternative Score - alternative FVG intensity calculation
pub fn signature_fvg_intensity_alt() -> IndicatorSignature {
    IndicatorSignature::builder("FVGALT", CATEGORY)
        .name("FVG Intensity Alt Score")
        .description("Alternative intensity scoring for Fair Value Gaps")
        .metadata("scoring", "alternative method")
        .machine_id(BarIndicatorId::Fvgalt)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("Fvgalt")
        .alias("fvgalt")
        .alias("FVGINTENSITYALTSCORE")
        .alias("FVGIntensityAltScore")
        .alias("fvgintensityaltscore")
        .alias("fvg_intensity_alt_score")
        .alias("FVG_INTENSITY_ALT_SCORE")
        .alias("Fvg_Intensity_Alt_Score")
        .build()
}

/// Liquidity Gap Density - measure liquidity gap concentration
pub fn signature_liquidity_gap_density() -> IndicatorSignature {
    IndicatorSignature::builder("LIQGAP", CATEGORY)
        .name("Liquidity Gap Density")
        .description("Measures concentration and density of liquidity gaps")
        .metadata("feature", "gap concentration")
        .machine_id(BarIndicatorId::Liqgap)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("Liqgap")
        .alias("liqgap")
        .alias("LIQUIDITYGAPDENSITY")
        .alias("LiquidityGapDensity")
        .alias("liquiditygapdensity")
        .alias("liquidity_gap_density")
        .alias("LIQUIDITY_GAP_DENSITY")
        .alias("Liquidity_Gap_Density")
        .build()
}

/// Swing Strength Score - normalized strength of swing highs/lows
pub fn signature_swing_strength_score() -> IndicatorSignature {
    IndicatorSignature::builder("SWINGSTR", CATEGORY)
        .name("Swing Strength Score")
        .description("Normalized strength score for swing highs and lows")
        .add_constraint(
            ParamConstraint::new("left", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(10))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("right", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(10))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .metadata("normalization", "ATR-normalized")
        .metadata("output", "tanh-bounded")
        .machine_id(BarIndicatorId::Swingstr)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("Swingstr")
        .alias("swingstr")
        .alias("SWINGSTRENGTHSCORE")
        .alias("SwingStrengthScore")
        .alias("swingstrengthscore")
        .alias("swing_strength_score")
        .alias("SWING_STRENGTH_SCORE")
        .alias("Swing_Strength_Score")
        .build()
}

/// Swing Age - bars since last swing (scalar measurement)
pub fn signature_swing_age() -> IndicatorSignature {
    IndicatorSignature::builder("SWING_AGE", CATEGORY)
        .name("Swing Age")
        .description("Age of current swing in bars")
        .add_constraint(ParamConstraint::period(2, 200, 5))
        .machine_id(BarIndicatorId::SwingAge)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Double)
        .validated()
        .alias("SwingAge")
        .alias("swing_age")
        .alias("SWINGAGE")
        .alias("swingage")
        .alias("Swing_Age")
        .build()
}

/// PriceChange24hZScore — rolling Z-score of 24-hour price-change percent
pub fn signature_price_change_24h_z_score() -> IndicatorSignature {
    IndicatorSignature::builder("PRICE_CHANGE_24H_ZSCORE", CATEGORY)
        .name("Price Change 24h Z-Score")
        .description("Rolling Z-score of 24-hour price-change percentage — measures statistical extremity of daily moves")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("input", "price_change_percent_24h from Ticker")
        .metadata("uses_ticker", "true")
        .machine_id(BarIndicatorId::PriceChange24hZScore)
        .input_stream(StreamKind::Ticker)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("PriceChange24hZScore")
        .alias("price_change_24h_z_score")
        .alias("PRICECHANGE24HZSCORE")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("FVGREV", signature_fvg_reversion_probability as fn() -> IndicatorSignature),
    ("FVGDUR", signature_fvg_duration_intensity as fn() -> IndicatorSignature),
    ("FVGALT", signature_fvg_intensity_alt as fn() -> IndicatorSignature),
    ("LIQGAP", signature_liquidity_gap_density as fn() -> IndicatorSignature),
    ("SWINGSTR", signature_swing_strength_score as fn() -> IndicatorSignature),
    ("SWING_AGE", signature_swing_age as fn() -> IndicatorSignature),
    ("PRICE_CHANGE_24H_ZSCORE", signature_price_change_24h_z_score as fn() -> IndicatorSignature),
];

/// Static catalog of all Statistical Scoring indicators
pub static STATISTICAL_SCORING_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    STATISTICAL_SCORING_CATALOG.get(id).map(|f| f())
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
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }

    #[test]
    fn test_count() {
        assert_eq!(count(), 7);
    }

    #[test]
    fn test_fvgrev_signature() {
        let sig = get_signature("FVGREV").unwrap();
        assert_eq!(sig.id, "FVGREV");
        assert_eq!(sig.required_params().len(), 1);
    }

    #[test]
    fn test_swingstr_signature() {
        let sig = get_signature("SWINGSTR").unwrap();
        assert_eq!(sig.id, "SWINGSTR");
        assert_eq!(sig.required_params().len(), 2);
    }

    #[test]
    fn test_swing_age_signature() {
        let sig = get_signature("SWING_AGE").unwrap();
        assert_eq!(sig.id, "SWING_AGE");
        assert_eq!(sig.required_params().len(), 1);
    }
}
