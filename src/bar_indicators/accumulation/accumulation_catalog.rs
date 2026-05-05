//! accumulation_catalog.rs: Catalog of all Accumulation indicators
//!
//! Auto-generated catalog based on actual indicator implementations.
//! Contains IndicatorSignature definitions for accumulation/distribution indicators.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue, SourceType,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Accumulation;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Accumulation/Distribution Line - shows whether accumulation or distribution is occurring
pub fn signature_accumulation_distribution() -> IndicatorSignature {
    IndicatorSignature::builder("AD", CATEGORY)
        .name("Accumulation/Distribution Line")
        .description("Shows whether accumulation (buying) or distribution (selling) is occurring")
        .source_type(SourceType::PriceAndVolume)
        .metadata("formula", "AD = Previous AD + Money Flow Multiplier × Volume")
        .metadata("author", "Marc Chaikin")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::Ad) // TODO: Add to enum
        // Note: "AD" is already the main ID, no need for alias
        .alias("Ad")
        .alias("ad")
        .alias("ACCUMULATIONDISTRIBUTIONLINE")
        .alias("AccumulationDistributionLine")
        .alias("accumulationdistributionline")
        .alias("accumulation_distribution_line")
        .alias("ACCUMULATION_DISTRIBUTION_LINE")
        .alias("Accumulation_Distribution_Line")
        .build()
}

/// Accumulative Swing Index - cumulative version of the Swing Index
pub fn signature_accumulative_swing_index() -> IndicatorSignature {
    IndicatorSignature::builder("ASI", CATEGORY)
        .name("Accumulative Swing Index")
        .description("Cumulative version of Wilder's Swing Index")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(
            ParamConstraint::new("limit_move", ParamType::F64)
                .with_min(ParamValue::F64(0.0001))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("author", "J. Welles Wilder")
        .metadata("category", "cumulative")
        .machine_id(BarIndicatorId::Asi) // TODO: Add to enum
        // Note: "ASI" is already the main ID, no need for alias
        .alias("Asi")
        .alias("asi")
        .alias("ACCUMULATIVESWINGINDEX")
        .alias("AccumulativeSwingIndex")
        .alias("accumulativeswingindex")
        .alias("accumulative_swing_index")
        .alias("ACCUMULATIVE_SWING_INDEX")
        .alias("Accumulative_Swing_Index")
        .build()
}

/// Chaikin Money Flow - measures buying and selling pressure over a period
pub fn signature_chaikin_money_flow() -> IndicatorSignature {
    IndicatorSignature::builder("CMF", CATEGORY)
        .name("Chaikin Money Flow")
        .description("Volume-weighted measure of accumulation and distribution")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("author", "Marc Chaikin")
        .metadata("range", "-1 to +1")
        .machine_id(BarIndicatorId::Cmf) // TODO: Add to enum
        // Note: "CMF" is already the main ID, no need for alias
        .alias("Cmf")
        .alias("cmf")
        .alias("CHAIKINMONEYFLOW")
        .alias("ChaikinMoneyFlow")
        .alias("chaikinmoneyflow")
        .alias("chaikin_money_flow")
        .alias("CHAIKIN_MONEY_FLOW")
        .alias("Chaikin_Money_Flow")
        .build()
}

/// Chaikin Oscillator - MACD applied to A/D Line
pub fn signature_chaikin_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("CHO", CATEGORY)
        .name("Chaikin Oscillator")
        .description("MACD applied to Accumulation/Distribution Line")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("author", "Marc Chaikin")
        .machine_id(BarIndicatorId::Cho) // TODO: Add to enum
        // Note: "CHO" is already the main ID, no need for alias
        .alias("Cho")
        .alias("cho")
        .alias("CHAIKINOSCILLATOR")
        .alias("ChaikinOscillator")
        .alias("chaikinoscillator")
        .alias("chaikin_oscillator")
        .alias("CHAIKIN_OSCILLATOR")
        .alias("Chaikin_Oscillator")
        .build()
}

/// Demand Index - measures demand using price and volume
pub fn signature_demand_index() -> IndicatorSignature {
    IndicatorSignature::builder("DI", CATEGORY)
        .name("Demand Index")
        .description("Measures buying and selling pressure based on price range and volume")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("category", "volume_based")
        .machine_id(BarIndicatorId::Di) // TODO: Add to enum
        // Note: "DI" is already the main ID, no need for alias
        .alias("Di")
        .alias("di")
        .alias("DEMANDINDEX")
        .alias("DemandIndex")
        .alias("demandindex")
        .alias("demand_index")
        .alias("DEMAND_INDEX")
        .alias("Demand_Index")
        .build()
}

/// Ease of Movement - relates price change to volume
pub fn signature_ease_of_movement() -> IndicatorSignature {
    IndicatorSignature::builder("EOM", CATEGORY)
        .name("Ease of Movement")
        .description("Relates price change to volume, showing how easily price moves")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .add_constraint(
            ParamConstraint::new("scale_factor", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(1000000.0))
                .with_default(ParamValue::F64(1000.0))
        )
        .metadata("author", "Richard Arms")
        .machine_id(BarIndicatorId::Eom) // TODO: Add to enum
        // Note: "EOM" is already the main ID, no need for alias
        .alias("Eom")
        .alias("eom")
        .alias("EASEOFMOVEMENT")
        .alias("EaseofMovement")
        .alias("easeofmovement")
        .alias("ease_of_movement")
        .alias("EASE_OF_MOVEMENT")
        .alias("Ease_Of_Movement")
        .build()
}

/// Force Index - measures the force of bulls and bears using volume
pub fn signature_force_index() -> IndicatorSignature {
    IndicatorSignature::builder("FI", CATEGORY)
        .name("Force Index")
        .description("Measures bull and bear power using volume and price change")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(
            ParamConstraint::new("smoothing_period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(13))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("author", "Alexander Elder")
        .metadata("popular_periods", "2, 13")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Fi) // TODO: Add to enum
        // Note: "FI" is already the main ID, no need for alias
        .alias("Fi")
        .alias("fi")
        .alias("FORCEINDEX")
        .alias("ForceIndex")
        .alias("forceindex")
        .alias("force_index")
        .alias("FORCE_INDEX")
        .alias("Force_Index")
        .build()
}

/// Intraday Intensity - measures accumulation/distribution within the bar
pub fn signature_intraday_intensity() -> IndicatorSignature {
    IndicatorSignature::builder("II", CATEGORY)
        .name("Intraday Intensity")
        .description("Measures accumulation/distribution within the bar")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 21))
        .metadata("category", "intrabar_analysis")
        .machine_id(BarIndicatorId::Ii) // TODO: Add to enum
        // Note: "II" is already the main ID, no need for alias
        .alias("Ii")
        .alias("ii")
        .alias("INTRADAYINTENSITY")
        .alias("IntradayIntensity")
        .alias("intradayintensity")
        .alias("intraday_intensity")
        .alias("INTRADAY_INTENSITY")
        .alias("Intraday_Intensity")
        .build()
}

/// Intraday Intensity Percent - percentage version of Intraday Intensity
pub fn signature_intraday_intensity_percent() -> IndicatorSignature {
    IndicatorSignature::builder("IIP", CATEGORY)
        .name("Intraday Intensity Percent")
        .description("Percentage-based intraday intensity indicator")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 21))
        .metadata("category", "intrabar_analysis")
        .metadata("range", "percentage")
        .machine_id(BarIndicatorId::Iip) // TODO: Add to enum
        // Note: "IIP" is already the main ID, no need for alias
        .alias("Iip")
        .alias("iip")
        .alias("INTRADAYINTENSITYPERCENT")
        .alias("IntradayIntensityPercent")
        .alias("intradayintensitypercent")
        .alias("intraday_intensity_percent")
        .alias("INTRADAY_INTENSITY_PERCENT")
        .alias("Intraday_Intensity_Percent")
        .build()
}

/// Intraday Intensity Ratio - ratio version of Intraday Intensity
pub fn signature_intraday_intensity_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("IIR", CATEGORY)
        .name("Intraday Intensity Ratio")
        .description("Ratio-based intraday intensity indicator")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 21))
        .metadata("category", "intrabar_analysis")
        .machine_id(BarIndicatorId::Iir) // TODO: Add to enum
        // Note: "IIR" is already the main ID, no need for alias
        .alias("Iir")
        .alias("iir")
        .alias("INTRADAYINTENSITYRATIO")
        .alias("IntradayIntensityRatio")
        .alias("intradayintensityratio")
        .alias("intraday_intensity_ratio")
        .alias("INTRADAY_INTENSITY_RATIO")
        .alias("Intraday_Intensity_Ratio")
        .build()
}

/// Twiggs Money Flow - improved version of Chaikin Money Flow
pub fn signature_twiggs_money_flow() -> IndicatorSignature {
    IndicatorSignature::builder("TMF", CATEGORY)
        .name("Twiggs Money Flow")
        .description("Enhanced Chaikin Money Flow with better divergence detection")
        .source_type(SourceType::PriceAndVolume)
        .add_constraint(ParamConstraint::period(5, 100, 21))
        .metadata("author", "Colin Twiggs")
        .metadata("range", "-1 to +1")
        .machine_id(BarIndicatorId::Tmf) // TODO: Add to enum
        // Note: "TMF" is already the main ID, no need for alias
        .alias("Tmf")
        .alias("tmf")
        .alias("TWIGGSMONEYFLOW")
        .alias("TwiggsMoneyFlow")
        .alias("twiggsmoneyflow")
        .alias("twiggs_money_flow")
        .alias("TWIGGS_MONEY_FLOW")
        .alias("Twiggs_Money_Flow")
        .build()
}

/// Williams Accumulation/Distribution - Larry Williams' version of A/D
pub fn signature_williams_ad() -> IndicatorSignature {
    IndicatorSignature::builder("WAD", CATEGORY)
        .name("Williams Accumulation/Distribution")
        .description("Larry Williams' version of Accumulation/Distribution")
        .source_type(SourceType::PriceAndVolume)
        .metadata("author", "Larry Williams")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::Wad) // TODO: Add to enum
        // Note: "WAD" is already the main ID, no need for alias
        .alias("Wad")
        .alias("wad")
        .alias("WILLIAMSACCUMULATIONDISTRIBUTION")
        .alias("WilliamsAccumulationDistribution")
        .alias("williamsaccumulationdistribution")
        .alias("williams_accumulation_distribution")
        .alias("WILLIAMS_ACCUMULATION_DISTRIBUTION")
        .alias("Williams_Accumulation_Distribution")
        .build()
}

/// On Balance Volume - cumulative volume-based indicator
pub fn signature_obv() -> IndicatorSignature {
    IndicatorSignature::builder("OBV", CATEGORY)
        .name("On Balance Volume")
        .description("Cumulative volume indicator based on price direction")
        .source_type(SourceType::PriceAndVolume)
        .metadata("author", "Joe Granville")
        .metadata("category", "cumulative")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::Obv)
        // Note: "OBV" is already the main ID, no need for alias
        .alias("Obv")
        .alias("obv")
        .alias("ONBALANCEVOLUME")
        .alias("OnBalanceVolume")
        .alias("onbalancevolume")
        .alias("on_balance_volume")
        .alias("ON_BALANCE_VOLUME")
        .alias("On_Balance_Volume")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Accumulation indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("AD", signature_accumulation_distribution as fn() -> IndicatorSignature),
    ("ASI", signature_accumulative_swing_index as fn() -> IndicatorSignature),
    ("CHO", signature_chaikin_oscillator as fn() -> IndicatorSignature),
    ("CMF", signature_chaikin_money_flow as fn() -> IndicatorSignature),
    ("DI", signature_demand_index as fn() -> IndicatorSignature),
    ("EOM", signature_ease_of_movement as fn() -> IndicatorSignature),
    ("FI", signature_force_index as fn() -> IndicatorSignature),
    ("II", signature_intraday_intensity as fn() -> IndicatorSignature),
    ("IIP", signature_intraday_intensity_percent as fn() -> IndicatorSignature),
    ("IIR", signature_intraday_intensity_ratio as fn() -> IndicatorSignature),
    ("OBV", signature_obv as fn() -> IndicatorSignature),
    ("TMF", signature_twiggs_money_flow as fn() -> IndicatorSignature),
    ("WAD", signature_williams_ad as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static ACCUMULATION_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
/// use zengeld_chart_indicators::bar_indicators::accumulation::accumulation_catalog;
///
/// let sig = accumulation_catalog::get_signature("CMF").unwrap();
/// assert_eq!(sig.id, "CMF");
/// ```
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    ACCUMULATION_CATALOG.get(id).map(|f| f())
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
    fn test_get_ad_signature() {
        let sig = get_signature("AD").unwrap();
        assert_eq!(sig.id, "AD");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_cmf_signature() {
        let sig = get_signature("CMF").unwrap();
        assert_eq!(sig.id, "CMF");
        assert_eq!(sig.name, "Chaikin Money Flow");
        assert_eq!(sig.required_params().len(), 1);
    }

    #[test]
    fn test_get_force_index_signature() {
        let sig = get_signature("FI").unwrap();
        assert_eq!(sig.id, "FI");
        // Force Index has 1 required parameter (smoothing_period)
        assert_eq!(sig.required_params().len(), 1);
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
        assert_eq!(count(), 13); // 13 accumulation indicators
    }

    #[test]
    fn test_cmf_validation() {
        let sig = get_signature("CMF").unwrap();

        // Valid params
        let params = vec![("period", ParamValue::USize(20))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(4))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("CMF").unwrap();
        let params = vec![("period", ParamValue::USize(20))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "CMF_20");
    }

    #[test]
    fn test_force_index_cache_key() {
        let sig = get_signature("FI").unwrap();
        let params = vec![("smoothing_period", ParamValue::USize(13))];
        let key = sig.cache_key(&params);
        assert!(key.contains("FI"));
        assert!(key.contains("13"));
    }

    #[test]
    fn test_chaikin_oscillator_multi_param() {
        let sig = get_signature("CHO").unwrap();
        let params = vec![
            ("fast_period", ParamValue::USize(3)),
            ("slow_period", ParamValue::USize(10)),
        ];
        assert!(sig.validate_params(&params).is_ok());
    }

    #[test]
    fn test_no_param_indicators() {
        // Test indicators that don't require parameters
        let indicators = vec!["AD", "OBV", "WAD"];

        for id in indicators {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.required_params().len(), 0);
        }
    }
}
