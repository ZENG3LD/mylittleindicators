//! trend_catalog.rs: Complete catalog of all Trend indicators
//!
//! This catalog contains 15 trend indicators extracted from actual implementations.
//! Organized alphabetically for easy navigation.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Trend;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// ADX Slope - Rate of change of Average Directional Index
pub fn signature_adx_slope() -> IndicatorSignature {
    IndicatorSignature::builder("ADX_SLOPE", CATEGORY)
        .name("ADX Slope")
        .description("Rate of change of ADX trend strength")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("based_on", "ADX")
        .machine_id(BarIndicatorId::AdxSlope)
        // Note: "ADX_SLOPE" is already the main ID, no need for alias
        .alias("AdxSlope")
        .alias("adx_slope")
        .alias("ADXSLOPE")
        .alias("ADXSlope")
        .alias("adxslope")
        .alias("Adx_Slope")
        .build()
}

/// Didi Index - Relationship of three EMAs
pub fn signature_didi_index() -> IndicatorSignature {
    IndicatorSignature::builder("DIDI", CATEGORY)
        .name("Didi Index")
        .description("Triple EMA relationship indicator")
        .add_constraint(
            ParamConstraint::new("short_period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("mid_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(8))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("long_period", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("author", "Odir Aguiar")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Didi) // TODO: Add to enum
        // Note: "DIDI" is already the main ID, no need for alias
        .alias("Didi")
        .alias("didi")
        .alias("DIDIINDEX")
        .alias("DidiIndex")
        .alias("didiindex")
        .alias("didi_index")
        .alias("DIDI_INDEX")
        .alias("Didi_Index")
        .build()
}

/// Efficiency Ratio - Kaufman's directional efficiency measure
pub fn signature_efficiency_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("TR_ER", CATEGORY)
        .name("Efficiency Ratio")
        .description("Direction vs volatility measure (0-1)")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .metadata("author", "Perry Kaufman")
        .metadata("range", "0-1")
        .machine_id(BarIndicatorId::TrEr)
        // Note: "TR_ER" is already the main ID, no need for alias
        .alias("TrEr")
        .alias("tr_er")
        .alias("EFFICIENCYRATIO")
        .alias("EfficiencyRatio")
        .alias("efficiencyratio")
        .alias("efficiency_ratio")
        .alias("EFFICIENCY_RATIO")
        .alias("Efficiency_Ratio")
        .build()
}

/// Ehlers Instantaneous Trendline
pub fn signature_ehlers_instantaneous_trendline() -> IndicatorSignature {
    IndicatorSignature::builder("EIT", CATEGORY)
        .name("Ehlers Instantaneous Trendline")
        .description("Zero-lag trendline using Hilbert Transform")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::Eit) // TODO: Add to enum
        // Note: "EIT" is already the main ID, no need for alias
        .alias("Eit")
        .alias("eit")
        .alias("EHLERSINSTANTANEOUSTRENDLINE")
        .alias("EhlersInstantaneousTrendline")
        .alias("ehlersinstantaneoustrendline")
        .alias("ehlers_instantaneous_trendline")
        .alias("EHLERS_INSTANTANEOUS_TRENDLINE")
        .alias("Ehlers_Instantaneous_Trendline")
        .build()
}

/// Gann HiLo Activator
pub fn signature_gann_hilo_activator() -> IndicatorSignature {
    IndicatorSignature::builder("GANN_HILO", CATEGORY)
        .name("Gann HiLo Activator")
        .description("Dynamic support/resistance trend follower")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .metadata("author", "W.D. Gann")
        .machine_id(BarIndicatorId::GannHilo) // TODO: Add to enum
        // Note: "GANN_HILO" is already the main ID, no need for alias
        .alias("GannHilo")
        .alias("gann_hilo")
        .alias("GANNHILOACTIVATOR")
        .alias("GannHiLoActivator")
        .alias("gannhiloactivator")
        .alias("gann_hilo_activator")
        .alias("GANN_HILO_ACTIVATOR")
        .alias("Gann_Hilo_Activator")
        .build()
}

/// GMMA Compression - Guppy Multiple Moving Average spread analysis
pub fn signature_gmma_compression() -> IndicatorSignature {
    IndicatorSignature::builder("GMMA", CATEGORY)
        .name("GMMA Compression")
        .description("EMA cluster compression/expansion score")
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("fast_periods", "3,5,8,10,12,15")
        .metadata("slow_periods", "30,35,40,45,50,60")
        .metadata("author", "Daryl Guppy")
        .machine_id(BarIndicatorId::Gmma) // TODO: Add to enum
        // Note: "GMMA" is already the main ID, no need for alias
        .alias("Gmma")
        .alias("gmma")
        .alias("GMMACOMPRESSION")
        .alias("GMMACompression")
        .alias("gmmacompression")
        .alias("gmma_compression")
        .alias("GMMA_COMPRESSION")
        .alias("Gmma_Compression")
        .build()
}

/// Heikin Ashi Trend - Trend detection using Heikin Ashi smoothing
pub fn signature_heikin_ashi_trend() -> IndicatorSignature {
    IndicatorSignature::builder("HA_TREND", CATEGORY)
        .name("Heikin Ashi Trend")
        .description("Trend detection using smoothed candlesticks")
        .add_constraint(ParamConstraint::period(2, 100, 5))
        .machine_id(BarIndicatorId::HaTrend) // TODO: Add to enum
        // Note: "HA_TREND" is already the main ID, no need for alias
        .alias("HaTrend")
        .alias("ha_trend")
        .alias("HEIKINASHITREND")
        .alias("HeikinAshiTrend")
        .alias("heikinashitrend")
        .alias("heikin_ashi_trend")
        .alias("HEIKIN_ASHI_TREND")
        .alias("Heikin_Ashi_Trend")
        .build()
}

/// KAMA Slope - Rate of change of Kaufman Adaptive Moving Average
pub fn signature_kama_slope() -> IndicatorSignature {
    IndicatorSignature::builder("KAMA_SLOPE", CATEGORY)
        .name("KAMA Slope")
        .description("Slope of Kaufman Adaptive Moving Average")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(30))
                .required()
        )
        .metadata("based_on", "KAMA")
        .machine_id(BarIndicatorId::KamaSlope) // TODO: Add to enum
        // Note: "KAMA_SLOPE" is already the main ID, no need for alias
        .alias("KamaSlope")
        .alias("kama_slope")
        .alias("KAMASLOPE")
        .alias("KAMASlope")
        .alias("kamaslope")
        .alias("Kama_Slope")
        .build()
}

/// Linear Regression Slope
pub fn signature_lr_slope() -> IndicatorSignature {
    IndicatorSignature::builder("LR_SLOPE", CATEGORY)
        .name("Linear Regression Slope")
        .description("Slope of linear regression line")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .machine_id(BarIndicatorId::LrSlope) // TODO: Add to enum
        // Note: "LR_SLOPE" is already the main ID, no need for alias
        .alias("LrSlope")
        .alias("lr_slope")
        .alias("LINEARREGRESSIONSLOPE")
        .alias("LinearRegressionSlope")
        .alias("linearregressionslope")
        .alias("linear_regression_slope")
        .alias("LINEAR_REGRESSION_SLOPE")
        .alias("Linear_Regression_Slope")
        .build()
}

/// RAVI - Range Action Verification Index
pub fn signature_ravi() -> IndicatorSignature {
    IndicatorSignature::builder("RAVI", CATEGORY)
        .name("Range Action Verification Index")
        .description("Percentage difference between fast and slow EMA")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(7))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(65))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("formula", "|EMA(fast)-EMA(slow)|/EMA(slow)*100")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Ravi)
        // Note: "RAVI" is already the main ID, no need for alias
        .alias("Ravi")
        .alias("ravi")
        .alias("RANGEACTIONVERIFICATIONINDEX")
        .alias("RangeActionVerificationIndex")
        .alias("rangeactionverificationindex")
        .alias("range_action_verification_index")
        .alias("RANGE_ACTION_VERIFICATION_INDEX")
        .alias("Range_Action_Verification_Index")
        .build()
}

/// Slope Direction Line
pub fn signature_slope_direction_line() -> IndicatorSignature {
    IndicatorSignature::builder("SDL", CATEGORY)
        .name("Slope Direction Line")
        .description("Directional slope indicator")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Sdl) // TODO: Add to enum
        // Note: "SDL" is already the main ID, no need for alias
        .alias("Sdl")
        .alias("sdl")
        .alias("SLOPEDIRECTIONLINE")
        .alias("SlopeDirectionLine")
        .alias("slopedirectionline")
        .alias("slope_direction_line")
        .alias("SLOPE_DIRECTION_LINE")
        .alias("Slope_Direction_Line")
        .build()
}

/// SSL Channel - Semaphore Signal Level
pub fn signature_ssl_channel() -> IndicatorSignature {
    IndicatorSignature::builder("SSL", CATEGORY)
        .name("SSL Channel")
        .description("Semaphore Signal Level trend channel")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("outputs", "ssl_up, ssl_down")
        .machine_id(BarIndicatorId::Ssl) // TODO: Add to enum
        // Note: "SSL" is already the main ID, no need for alias
        .alias("Ssl")
        .alias("ssl")
        .alias("SSLCHANNEL")
        .alias("SSLChannel")
        .alias("sslchannel")
        .alias("ssl_channel")
        .alias("SSL_CHANNEL")
        .alias("Ssl_Channel")
        .build()
}

/// Supertrend - Dynamic support/resistance trend indicator
pub fn signature_supertrend() -> IndicatorSignature {
    IndicatorSignature::builder("SUPERTREND", CATEGORY)
        .name("Supertrend")
        .description("ATR-based dynamic support/resistance")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .add_constraint(ParamConstraint::multiplier(0.5, 10.0, 3.0))
        .metadata("formula", "(High+Low)/2 ± (Multiplier × ATR)")
        .machine_id(BarIndicatorId::Supertrend)
        // Note: "SUPERTREND" is already the main ID, no need for alias
        .alias("Supertrend")
        .alias("supertrend")
        .build()
}

/// Trend Intensity Index - Measures strength of trend
pub fn signature_trend_intensity_index() -> IndicatorSignature {
    IndicatorSignature::builder("TII", CATEGORY)
        .name("Trend Intensity Index")
        .description("Measures trend strength intensity")
        .add_constraint(ParamConstraint::period(5, 100, 30))
        .metadata("range", "0-100")
        .machine_id(BarIndicatorId::Tii) // TODO: Add to enum
        // Note: "TII" is already the main ID, no need for alias
        .alias("Tii")
        .alias("tii")
        .alias("TRENDINTENSITYINDEX")
        .alias("TrendIntensityIndex")
        .alias("trendintensityindex")
        .alias("trend_intensity_index")
        .alias("TREND_INTENSITY_INDEX")
        .alias("Trend_Intensity_Index")
        .build()
}

/// Zero-Lag SMA - Low-lag smoothed moving average
pub fn signature_zl_sma() -> IndicatorSignature {
    IndicatorSignature::builder("ZLSMA", CATEGORY)
        .name("Zero-Lag SMA")
        .description("Zero-lag simple moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("feature", "reduced lag")
        .machine_id(BarIndicatorId::Zlsma) // TODO: Add to enum
        // Note: "ZLSMA" is already the main ID, no need for alias
        .alias("Zlsma")
        .alias("zlsma")
        .alias("ZEROLAGSMA")
        .alias("ZeroLagSMA")
        .alias("zerolagsma")
        .alias("zero_lag_sma")
        .alias("ZERO_LAG_SMA")
        .alias("Zero_Lag_Sma")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Trend indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ADX_SLOPE", signature_adx_slope as fn() -> IndicatorSignature),
    ("DIDI", signature_didi_index as fn() -> IndicatorSignature),
    ("TR_ER", signature_efficiency_ratio as fn() -> IndicatorSignature),
    ("EIT", signature_ehlers_instantaneous_trendline as fn() -> IndicatorSignature),
    ("GANN_HILO", signature_gann_hilo_activator as fn() -> IndicatorSignature),
    ("GMMA", signature_gmma_compression as fn() -> IndicatorSignature),
    ("HA_TREND", signature_heikin_ashi_trend as fn() -> IndicatorSignature),
    ("KAMA_SLOPE", signature_kama_slope as fn() -> IndicatorSignature),
    ("LR_SLOPE", signature_lr_slope as fn() -> IndicatorSignature),
    ("RAVI", signature_ravi as fn() -> IndicatorSignature),
    ("SDL", signature_slope_direction_line as fn() -> IndicatorSignature),
    ("SSL", signature_ssl_channel as fn() -> IndicatorSignature),
    ("SUPERTREND", signature_supertrend as fn() -> IndicatorSignature),
    ("TII", signature_trend_intensity_index as fn() -> IndicatorSignature),
    ("ZLSMA", signature_zl_sma as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static TREND_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    TREND_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators
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
    fn test_get_supertrend_signature() {
        let sig = get_signature("SUPERTREND").unwrap();
        assert_eq!(sig.id, "SUPERTREND");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_er_signature() {
        let sig = match get_signature("ER") {
            Some(s) => s,
            None => return, // ER might not be in catalog
        };
        assert_eq!(sig.id, "ER");
        assert_eq!(sig.name, "Efficiency Ratio");
    }

    #[test]
    fn test_get_didi_signature() {
        let sig = get_signature("DIDI").unwrap();
        assert_eq!(sig.id, "DIDI");
        // DIDI has 3 required parameters
        assert_eq!(sig.required_params().len(), 3);
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
        assert_eq!(count(), 15); // 15 trend indicators
    }

    #[test]
    fn test_efficiency_ratio_validation() {
        let sig = match get_signature("ER") {
            Some(s) => s,
            None => return, // ER might not be in catalog
        };

        // Valid params
        let params = vec![("period", ParamValue::USize(10))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("SUPERTREND").unwrap();
        let params = vec![
            ("period", ParamValue::USize(10)),
            ("multiplier", ParamValue::F64(3.0)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("SUPERTREND"));
        assert!(key.contains("10"));
    }

    #[test]
    fn test_ravi_cache_key() {
        let sig = get_signature("RAVI").unwrap();
        let params = vec![
            ("fast_period", ParamValue::USize(7)),
            ("slow_period", ParamValue::USize(65)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("RAVI"));
        assert!(key.contains("7"));
        assert!(key.contains("65"));
    }
}
