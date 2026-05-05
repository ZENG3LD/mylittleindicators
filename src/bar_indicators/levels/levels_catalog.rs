//! levels_catalog.rs: Catalog of all Price Levels indicators
//!
//! Auto-generated catalog based on actual indicator implementations.
//! Contains IndicatorSignature definitions for 19 price level indicators.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Levels;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Classic Pivot Points - standard pivot levels for support and resistance
pub fn signature_pivot_points() -> IndicatorSignature {
    IndicatorSignature::builder("PIVOT", CATEGORY)
        .name("Classic Pivot Points")
        .description("Classic pivot levels: PP, R1-R3, S1-S3 for support/resistance")
        .add_constraint(ParamConstraint::period(1, 168, 1))
        .metadata("author", "Classic TA")
        .metadata("levels", "7 (PP, R1-R3, S1-S3)")
        .machine_id(BarIndicatorId::Pivot) // TODO: Add to enum
        // Note: "PIVOT" is already the main ID, no need for alias
        .alias("Pivot")
        .alias("pivot")
        .alias("CLASSICPIVOTPOINTS")
        .alias("ClassicPivotPoints")
        .alias("classicpivotpoints")
        .alias("classic_pivot_points")
        .alias("CLASSIC_PIVOT_POINTS")
        .alias("Classic_Pivot_Points")
        .build()
}

/// Floor Trader Pivots - floor trader pivot calculation method
pub fn signature_floor_trader_pivots() -> IndicatorSignature {
    IndicatorSignature::builder("FLOORPIVOT", CATEGORY)
        .name("Floor Trader Pivots")
        .description("Floor trader pivot calculation for intraday levels")
        .add_constraint(ParamConstraint::period(1, 168, 1))
        .metadata("author", "Floor Traders")
        .metadata("use_case", "intraday trading")
        .machine_id(BarIndicatorId::Floorpivot) // TODO: Add to enum
        // Note: "FLOORPIVOT" is already the main ID, no need for alias
        .alias("Floorpivot")
        .alias("floorpivot")
        .alias("FLOORTRADERPIVOTS")
        .alias("FloorTraderPivots")
        .alias("floortraderpivots")
        .alias("floor_trader_pivots")
        .alias("FLOOR_TRADER_PIVOTS")
        .alias("Floor_Trader_Pivots")
        .build()
}

/// Camarilla Pivots - Camarilla pivot levels with 4 support/resistance levels
pub fn signature_camarilla_pivots() -> IndicatorSignature {
    IndicatorSignature::builder("CAMARILLA", CATEGORY)
        .name("Camarilla Pivots")
        .description("Camarilla pivot levels with R1-R4 and S1-S4 using 1.1 multiplier")
        .add_constraint(ParamConstraint::period(1, 168, 24))
        .metadata("author", "Nick Stott")
        .metadata("levels", "9 (PP, R1-R4, S1-S4)")
        .metadata("multiplier", "1.1")
        .machine_id(BarIndicatorId::Camarilla) // TODO: Add to enum
        // Note: "CAMARILLA" is already the main ID, no need for alias
        .alias("Camarilla")
        .alias("camarilla")
        .alias("CAMARILLAPIVOTS")
        .alias("CamarillaPivots")
        .alias("camarillapivots")
        .alias("camarilla_pivots")
        .alias("CAMARILLA_PIVOTS")
        .alias("Camarilla_Pivots")
        .build()
}

/// Woodie Pivots - Woodie's pivot calculation method
pub fn signature_woodie_pivots() -> IndicatorSignature {
    IndicatorSignature::builder("WOODIE", CATEGORY)
        .name("Woodie Pivots")
        .description("Woodie's pivot calculation emphasizing current session")
        .add_constraint(ParamConstraint::period(1, 168, 1))
        .metadata("author", "Woodie")
        .metadata("emphasis", "current session")
        .machine_id(BarIndicatorId::Woodie) // TODO: Add to enum
        // Note: "WOODIE" is already the main ID, no need for alias
        .alias("Woodie")
        .alias("woodie")
        .alias("WOODIEPIVOTS")
        .alias("WoodiePivots")
        .alias("woodiepivots")
        .alias("woodie_pivots")
        .alias("WOODIE_PIVOTS")
        .alias("Woodie_Pivots")
        .build()
}

/// DeMark Pivots - Tom DeMark's pivot calculation
pub fn signature_demark_pivots() -> IndicatorSignature {
    IndicatorSignature::builder("DEMARK", CATEGORY)
        .name("DeMark Pivots")
        .description("Tom DeMark's pivot calculation based on open/close relationship")
        .add_constraint(ParamConstraint::period(1, 168, 1))
        .metadata("author", "Tom DeMark")
        .metadata("feature", "open/close relationship")
        .machine_id(BarIndicatorId::Demark) // TODO: Add to enum
        // Note: "DEMARK" is already the main ID, no need for alias
        .alias("Demark")
        .alias("demark")
        .alias("DEMARKPIVOTS")
        .alias("DeMarkPivots")
        .alias("demarkpivots")
        .alias("demark_pivots")
        .alias("DEMARK_PIVOTS")
        .alias("Demark_Pivots")
        .build()
}

/// Anchored VWAP - volume-weighted average price anchored to calendar periods
pub fn signature_anchored_vwap() -> IndicatorSignature {
    IndicatorSignature::builder("AVWAP", CATEGORY)
        .name("Anchored VWAP")
        .description("Volume-weighted average price anchored to monthly periods")
        .metadata("uses_volume", "true")
        .metadata("anchor", "monthly")
        .metadata("reset", "calendar-based")
        .machine_id(BarIndicatorId::Avwap) // TODO: Add to enum
        // Note: "AVWAP" is already the main ID, no need for alias
        .alias("Avwap")
        .alias("avwap")
        .alias("ANCHOREDVWAP")
        .alias("AnchoredVWAP")
        .alias("anchoredvwap")
        .alias("anchored_vwap")
        .alias("ANCHORED_VWAP")
        .alias("Anchored_Vwap")
        .build()
}

/// AVWAP Multi-Anchor Reversion - reversion probability to multiple AVWAP anchors
pub fn signature_avwap_multi_anchor_reversion() -> IndicatorSignature {
    IndicatorSignature::builder("AVWAPREV", CATEGORY)
        .name("AVWAP Multi-Anchor Reversion")
        .description("Reversion probability to multiple AVWAP anchor points")
        .metadata("uses_volume", "true")
        .metadata("multi_anchor", "true")
        .machine_id(BarIndicatorId::Avwaprev) // TODO: Add to enum
        // Note: "AVWAPREV" is already the main ID, no need for alias
        .alias("Avwaprev")
        .alias("avwaprev")
        .alias("AVWAPMULTIANCHORREVERSION")
        .alias("AVWAPMultiAnchorReversion")
        .alias("avwapmultianchorreversion")
        .alias("avwap_multi_anchor_reversion")
        .alias("AVWAP_MULTI_ANCHOR_REVERSION")
        .alias("Avwap_Multi_Anchor_Reversion")
        .build()
}

/// AVWAP Touch Probability - probability of touching anchored VWAP
pub fn signature_avwap_touch_probability() -> IndicatorSignature {
    IndicatorSignature::builder("AVWAPTOUCH", CATEGORY)
        .name("AVWAP Touch Probability")
        .description("Statistical probability of price touching AVWAP level")
        .metadata("uses_volume", "true")
        .metadata("statistical", "true")
        .machine_id(BarIndicatorId::Avwaptouch) // TODO: Add to enum
        // Note: "AVWAPTOUCH" is already the main ID, no need for alias
        .alias("Avwaptouch")
        .alias("avwaptouch")
        .alias("AVWAPTOUCHPROBABILITY")
        .alias("AVWAPTouchProbability")
        .alias("avwaptouchprobability")
        .alias("avwap_touch_probability")
        .alias("AVWAP_TOUCH_PROBABILITY")
        .alias("Avwap_Touch_Probability")
        .build()
}

/// Break of Structure - detect BOS/CHOCH patterns
pub fn signature_break_of_structure() -> IndicatorSignature {
    IndicatorSignature::builder("BOS", CATEGORY)
        .name("Break of Structure")
        .description("Detects Break of Structure and Change of Character patterns")
        .add_constraint(ParamConstraint::period(2, 50, 10))
        .metadata("pattern", "BOS/CHOCH")
        .metadata("use_case", "structure breaks")
        .machine_id(BarIndicatorId::Bos) // TODO: Add to enum
        // Note: "BOS" is already the main ID, no need for alias
        .alias("Bos")
        .alias("bos")
        .alias("BREAKOFSTRUCTURE")
        .alias("BreakofStructure")
        .alias("breakofstructure")
        .alias("break_of_structure")
        .alias("BREAK_OF_STRUCTURE")
        .alias("Break_Of_Structure")
        .build()
}

/// FVG Detector - Fair Value Gap pattern detector
pub fn signature_fvg_detector() -> IndicatorSignature {
    IndicatorSignature::builder("FVG", CATEGORY)
        .name("Fair Value Gap Detector")
        .description("Detects Fair Value Gaps in 3-bar patterns")
        .metadata("pattern", "3-bar FVG")
        .metadata("type", "bull/bear gaps")
        .machine_id(BarIndicatorId::Fvg) // TODO: Add to enum
        // Note: "FVG" is already the main ID, no need for alias
        .alias("Fvg")
        .alias("fvg")
        .alias("FAIRVALUEGAPDETECTOR")
        .alias("FairValueGapDetector")
        .alias("fairvaluegapdetector")
        .alias("fair_value_gap_detector")
        .alias("FAIR_VALUE_GAP_DETECTOR")
        .alias("Fair_Value_Gap_Detector")
        .build()
}

/// FVG Duration Intensity Score - scoring FVG duration and intensity
pub fn signature_fvg_duration_intensity() -> IndicatorSignature {
    IndicatorSignature::builder("FVGDUR", CATEGORY)
        .name("FVG Duration Intensity Score")
        .description("Measures duration and intensity of Fair Value Gaps")
        .metadata("scoring", "duration + intensity")
        .machine_id(BarIndicatorId::Fvgdur) // TODO: Add to enum
        // Note: "FVGDUR" is already the main ID, no need for alias
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
        .machine_id(BarIndicatorId::Fvgalt) // TODO: Add to enum
        // Note: "FVGALT" is already the main ID, no need for alias
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

/// FVG Reversion Probability - probability of FVG fill
pub fn signature_fvg_reversion_probability() -> IndicatorSignature {
    IndicatorSignature::builder("FVGREV", CATEGORY)
        .name("FVG Reversion Probability")
        .description("Probability of Fair Value Gap being filled within horizon")
        .add_constraint(ParamConstraint::period(1, 50, 10))
        .metadata("statistical", "true")
        .metadata("horizon", "H bars")
        .machine_id(BarIndicatorId::Fvgrev) // TODO: Add to enum
        // Note: "FVGREV" is already the main ID, no need for alias
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

/// High-Low Value Area - value area based on high/low ranges
pub fn signature_hl_value_area() -> IndicatorSignature {
    IndicatorSignature::builder("HLVA", CATEGORY)
        .name("High-Low Value Area")
        .description("Value area calculation based on high/low ranges")
        .metadata("basis", "high/low ranges")
        .machine_id(BarIndicatorId::Hlva) // TODO: Add to enum
        // Note: "HLVA" is already the main ID, no need for alias
        .alias("Hlva")
        .alias("hlva")
        .alias("HIGHLOWVALUEAREA")
        .alias("HighLowValueArea")
        .alias("highlowvaluearea")
        .alias("high_low_value_area")
        .alias("HIGH_LOW_VALUE_AREA")
        .alias("High_Low_Value_Area")
        .build()
}

/// Liquidity Gap Density - measure liquidity gap concentration
pub fn signature_liquidity_gap_density() -> IndicatorSignature {
    IndicatorSignature::builder("LIQGAP", CATEGORY)
        .name("Liquidity Gap Density")
        .description("Measures concentration and density of liquidity gaps")
        .metadata("feature", "gap concentration")
        .machine_id(BarIndicatorId::Liqgap) // TODO: Add to enum
        // Note: "LIQGAP" is already the main ID, no need for alias
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

/// Pivot Anchored VWAP - VWAP anchored to pivot points
pub fn signature_pivot_anchored_vwap() -> IndicatorSignature {
    IndicatorSignature::builder("PIVAVWAP", CATEGORY)
        .name("Pivot Anchored VWAP")
        .description("VWAP anchored to pivot point levels")
        .metadata("uses_volume", "true")
        .metadata("anchor", "pivot points")
        .machine_id(BarIndicatorId::Pivavwap) // TODO: Add to enum
        // Note: "PIVAVWAP" is already the main ID, no need for alias
        .alias("Pivavwap")
        .alias("pivavwap")
        .alias("PIVOTANCHOREDVWAP")
        .alias("PivotAnchoredVWAP")
        .alias("pivotanchoredvwap")
        .alias("pivot_anchored_vwap")
        .alias("PIVOT_ANCHORED_VWAP")
        .alias("Pivot_Anchored_Vwap")
        .build()
}

/// Rolling Midline - average of high/low over rolling window
pub fn signature_rolling_midline() -> IndicatorSignature {
    IndicatorSignature::builder("RMID", CATEGORY)
        .name("Rolling Midline")
        .description("Rolling average of (High + Low) / 2")
        .add_constraint(ParamConstraint::period(1, 200, 20))
        .metadata("formula", "(H+L)/2 average")
        .machine_id(BarIndicatorId::Rmid) // TODO: Add to enum
        // Note: "RMID" is already the main ID, no need for alias
        .alias("Rmid")
        .alias("rmid")
        .alias("ROLLINGMIDLINE")
        .alias("RollingMidline")
        .alias("rollingmidline")
        .alias("rolling_midline")
        .alias("ROLLING_MIDLINE")
        .alias("Rolling_Midline")
        .build()
}

/// Rolling Quartiles - Q1, Q2 (median), Q3 over rolling window
pub fn signature_rolling_quartiles() -> IndicatorSignature {
    IndicatorSignature::builder("RQUART", CATEGORY)
        .name("Rolling Quartiles")
        .description("Rolling Q1, Q2 (median), Q3 quartiles of close prices")
        .add_constraint(ParamConstraint::period(1, 200, 20))
        .metadata("outputs", "Q1, Q2, Q3")
        .metadata("complexity", "O(N log N) per update")
        .machine_id(BarIndicatorId::Rquart) // TODO: Add to enum
        // Note: "RQUART" is already the main ID, no need for alias
        .alias("Rquart")
        .alias("rquart")
        .alias("ROLLINGQUARTILES")
        .alias("RollingQuartiles")
        .alias("rollingquartiles")
        .alias("rolling_quartiles")
        .alias("ROLLING_QUARTILES")
        .alias("Rolling_Quartiles")
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
        .machine_id(BarIndicatorId::Swingstr) // TODO: Add to enum
        // Note: "SWINGSTR" is already the main ID, no need for alias
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

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Levels indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("PIVOT", signature_pivot_points as fn() -> IndicatorSignature),
    ("FLOORPIVOT", signature_floor_trader_pivots as fn() -> IndicatorSignature),
    ("CAMARILLA", signature_camarilla_pivots as fn() -> IndicatorSignature),
    ("WOODIE", signature_woodie_pivots as fn() -> IndicatorSignature),
    ("DEMARK", signature_demark_pivots as fn() -> IndicatorSignature),
    ("AVWAP", signature_anchored_vwap as fn() -> IndicatorSignature),
    ("AVWAPREV", signature_avwap_multi_anchor_reversion as fn() -> IndicatorSignature),
    ("AVWAPTOUCH", signature_avwap_touch_probability as fn() -> IndicatorSignature),
    ("BOS", signature_break_of_structure as fn() -> IndicatorSignature),
    ("FVG", signature_fvg_detector as fn() -> IndicatorSignature),
    ("FVGDUR", signature_fvg_duration_intensity as fn() -> IndicatorSignature),
    ("FVGALT", signature_fvg_intensity_alt as fn() -> IndicatorSignature),
    ("FVGREV", signature_fvg_reversion_probability as fn() -> IndicatorSignature),
    ("HLVA", signature_hl_value_area as fn() -> IndicatorSignature),
    ("LIQGAP", signature_liquidity_gap_density as fn() -> IndicatorSignature),
    ("PIVAVWAP", signature_pivot_anchored_vwap as fn() -> IndicatorSignature),
    ("RMID", signature_rolling_midline as fn() -> IndicatorSignature),
    ("RQUART", signature_rolling_quartiles as fn() -> IndicatorSignature),
    ("SWINGSTR", signature_swing_strength_score as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static LEVELS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
/// use zengeld_chart_indicators::bar_indicators::levels::levels_catalog;
///
/// let sig = levels_catalog::get_signature("PIVOT").unwrap();
/// assert_eq!(sig.id, "PIVOT");
/// ```
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    LEVELS_CATALOG.get(id).map(|f| f())
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
    fn test_get_pivot_signature() {
        let sig = get_signature("PIVOT").unwrap();
        assert_eq!(sig.id, "PIVOT");
        assert_eq!(sig.category, CATEGORY);
        assert_eq!(sig.required_params().len(), 1);
    }

    #[test]
    fn test_get_camarilla_signature() {
        let sig = get_signature("CAMARILLA").unwrap();
        assert_eq!(sig.id, "CAMARILLA");
        assert_eq!(sig.name, "Camarilla Pivots");
    }

    #[test]
    fn test_get_swing_strength_signature() {
        let sig = get_signature("SWINGSTR").unwrap();
        assert_eq!(sig.id, "SWINGSTR");
        // SWINGSTR has 2 required parameters (left, right)
        assert_eq!(sig.required_params().len(), 2);
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
        assert_eq!(count(), 19); // 19 level indicators
    }

    #[test]
    fn test_pivot_validation() {
        let sig = get_signature("PIVOT").unwrap();

        // Valid params
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(200))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("PIVOT").unwrap();
        let params = vec![("period", ParamValue::USize(1))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "PIVOT_1");
    }

    #[test]
    fn test_swing_strength_cache_key() {
        let sig = get_signature("SWINGSTR").unwrap();
        let params = vec![
            ("left", ParamValue::USize(3)),
            ("right", ParamValue::USize(3)),
        ];
        let key = sig.cache_key(&params);
        // Keys are sorted alphabetically
        assert!(key.contains("SWINGSTR"));
        assert!(key.contains("3"));
    }

    #[test]
    fn test_bos_signature() {
        let sig = get_signature("BOS").unwrap();
        assert_eq!(sig.id, "BOS");
        assert_eq!(sig.name, "Break of Structure");
        assert_eq!(sig.required_params().len(), 1);
    }

    #[test]
    fn test_fvg_reversion_signature() {
        let sig = get_signature("FVGREV").unwrap();
        assert_eq!(sig.id, "FVGREV");
        assert_eq!(sig.required_params().len(), 1);
    }
}
