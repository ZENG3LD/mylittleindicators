//! volume_catalog.rs: Indicator catalog for volume indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 17 volume indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint, ParamType, ParamValue, SourceType,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Volume;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Money Flow Index
pub fn signature_mfi() -> IndicatorSignature {
    IndicatorSignature::builder("MFI", CATEGORY)
        .name("Money Flow Index")
        .description("Volume-weighted RSI using typical price")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .metadata("range", "0-100")
        .metadata("author", "Gene Quong and Avrum Soudack")
        .machine_id(BarIndicatorId::Mfi)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "MFI" is already the main ID, no need for alias
        .alias("Mfi")
        .alias("mfi")
        .alias("MONEYFLOWINDEX")
        .alias("MoneyFlowIndex")
        .alias("moneyflowindex")
        .alias("money_flow_index")
        .alias("MONEY_FLOW_INDEX")
        .alias("Money_Flow_Index")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Negative/Positive Volume Index
pub fn signature_nvi_pvi() -> IndicatorSignature {
    IndicatorSignature::builder("NVI_PVI", CATEGORY)
        .name("Negative/Positive Volume Index")
        .description("Cumulative indicators based on volume changes")
        .metadata("outputs", "nvi, pvi")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::NviPvi)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "NVI_PVI" is already the main ID, no need for alias
        .alias("NviPvi")
        .alias("nvi_pvi")
        .alias("NEGATIVEPOSITIVEVOLUMEINDEX")
        .alias("NegativePositiveVolumeIndex")
        .alias("negativepositivevolumeindex")
        .alias("negative_positive_volume_index")
        .alias("NEGATIVE_POSITIVE_VOLUME_INDEX")
        .alias("Negative_Positive_Volume_Index")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Point of Control Detector
pub fn signature_poc_detector() -> IndicatorSignature {
    IndicatorSignature::builder("POC", CATEGORY)
        .name("Point of Control Detector")
        .description("Detects price level with highest volume")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .metadata("category", "volume_profile")
        .machine_id(BarIndicatorId::Poc)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .validated()
        // Note: "POC" is already the main ID, no need for alias
        .alias("Poc")
        .alias("poc")
        .alias("POINTOFCONTROLDETECTOR")
        .alias("PointofControlDetector")
        .alias("pointofcontroldetector")
        .alias("point_of_control_detector")
        .alias("POINT_OF_CONTROL_DETECTOR")
        .alias("Point_Of_Control_Detector")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Percentage Volume Oscillator
pub fn signature_pvo() -> IndicatorSignature {
    IndicatorSignature::builder("PVO", CATEGORY)
        .name("Percentage Volume Oscillator")
        .description("MACD applied to volume")
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
                .required()
        )
        .add_constraint(
            ParamConstraint::new("fast_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("slow_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("signal_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .machine_id(BarIndicatorId::Pvo)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "PVO" is already the main ID, no need for alias
        .alias("Pvo")
        .alias("pvo")
        .alias("PERCENTAGEVOLUMEOSCILLATOR")
        .alias("PercentageVolumeOscillator")
        .alias("percentagevolumeoscillator")
        .alias("percentage_volume_oscillator")
        .alias("PERCENTAGE_VOLUME_OSCILLATOR")
        .alias("Percentage_Volume_Oscillator")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Price Volume Trend
pub fn signature_pvt() -> IndicatorSignature {
    IndicatorSignature::builder("PVT", CATEGORY)
        .name("Price Volume Trend")
        .description("Cumulative volume based on price change percentage")
        .metadata("category", "cumulative")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::Pvt)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "PVT" is already the main ID, no need for alias
        .alias("Pvt")
        .alias("pvt")
        .alias("PRICEVOLUMETREND")
        .alias("PriceVolumeTrend")
        .alias("pricevolumetrend")
        .alias("price_volume_trend")
        .alias("PRICE_VOLUME_TREND")
        .alias("Price_Volume_Trend")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Relative Volume
pub fn signature_relative_volume() -> IndicatorSignature {
    IndicatorSignature::builder("RVOL", CATEGORY)
        .name("Relative Volume")
        .description("Current volume compared to average volume")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .machine_id(BarIndicatorId::Rvol)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "RVOL" is already the main ID, no need for alias
        .alias("Rvol")
        .alias("rvol")
        .alias("RELATIVEVOLUME")
        .alias("RelativeVolume")
        .alias("relativevolume")
        .alias("relative_volume")
        .alias("RELATIVE_VOLUME")
        .alias("Relative_Volume")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume Flow Indicator
pub fn signature_vfi() -> IndicatorSignature {
    IndicatorSignature::builder("VFI", CATEGORY)
        .name("Volume Flow Indicator")
        .description("Combines price, volume and volatility")
        .add_constraint(ParamConstraint::period(10, 200, 130))
        .add_constraint(
            ParamConstraint::new("coef", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.2))
                .required()
        )
        .machine_id(BarIndicatorId::Vfi)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VFI" is already the main ID, no need for alias
        .alias("Vfi")
        .alias("vfi")
        .alias("VOLUMEFLOWINDICATOR")
        .alias("VolumeFlowIndicator")
        .alias("volumeflowindicator")
        .alias("volume_flow_indicator")
        .alias("VOLUME_FLOW_INDICATOR")
        .alias("Volume_Flow_Indicator")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Volume Delta
pub fn signature_volume_delta() -> IndicatorSignature {
    IndicatorSignature::builder("VDELTA", CATEGORY)
        .name("Volume Delta")
        .description("Difference between buying and selling volume")
        .metadata("requirements", "bid_ask_volume")
        .machine_id(BarIndicatorId::Vdelta)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .validated()
        // Note: "VDELTA" is already the main ID, no need for alias
        .alias("Vdelta")
        .alias("vdelta")
        .alias("VOLUMEDELTA")
        .alias("VolumeDelta")
        .alias("volumedelta")
        .alias("volume_delta")
        .alias("VOLUME_DELTA")
        .alias("Volume_Delta")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume Oscillator
pub fn signature_volume_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("VO", CATEGORY)
        .name("Volume Oscillator")
        .description("Difference between two volume moving averages")
        .add_constraint(
            ParamConstraint::new("short_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("long_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Vo)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VO" is already the main ID, no need for alias
        .alias("Vo")
        .alias("vo")
        .alias("VOLUMEOSCILLATOR")
        .alias("VolumeOscillator")
        .alias("volumeoscillator")
        .alias("volume_oscillator")
        .alias("VOLUME_OSCILLATOR")
        .alias("Volume_Oscillator")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume Profile
pub fn signature_volume_profile() -> IndicatorSignature {
    IndicatorSignature::builder("VPROFILE", CATEGORY)
        .name("Volume Profile")
        .description("Distribution of volume across price levels")
        .add_constraint(
            ParamConstraint::new("tick_size", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("session_duration", ParamType::USize)
                .with_min(ParamValue::USize(0))
                .with_max(ParamValue::USize(86400))
                .with_default(ParamValue::USize(0))
                .required()
        )
        .metadata("outputs", "poc, vah, val, profile")
        .machine_id(BarIndicatorId::Vprofile)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .validated()
        // Note: "VPROFILE" is already the main ID, no need for alias
        .alias("Vprofile")
        .alias("vprofile")
        .alias("VOLUMEPROFILE")
        .alias("VolumeProfile")
        .alias("volumeprofile")
        .alias("volume_profile")
        .alias("VOLUME_PROFILE")
        .alias("Volume_Profile")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume Z-Score
pub fn signature_volume_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("VZ", CATEGORY)
        .name("Volume Z-Score")
        .description("Standardized volume using z-score")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .machine_id(BarIndicatorId::Vz)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VZ" is already the main ID, no need for alias
        .alias("Vz")
        .alias("vz")
        .alias("VOLUMEZSCORE")
        .alias("VolumeZScore")
        .alias("volumezscore")
        .alias("volume_z_score")
        .alias("VOLUME_Z_SCORE")
        .alias("Volume_Z_Score")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume-synchronized Probability of Informed Trading (VPIN)
pub fn signature_vpin() -> IndicatorSignature {
    IndicatorSignature::builder("VPIN", CATEGORY)
        .name("VPIN")
        .description("Order flow toxicity indicator")
        .add_constraint(ParamConstraint::period(10, 100, 50))
        .metadata("category", "microstructure")
        .metadata("requirements", "volume_buckets")
        .machine_id(BarIndicatorId::Vpin)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VPIN" is already the main ID, no need for alias
        .alias("Vpin")
        .alias("vpin")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume Price Trend
pub fn signature_vpt() -> IndicatorSignature {
    IndicatorSignature::builder("VPT", CATEGORY)
        .name("Volume Price Trend")
        .description("Cumulative volume adjusted by price change")
        .metadata("category", "cumulative")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::Vpt)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VPT" is already the main ID, no need for alias
        .alias("Vpt")
        .alias("vpt")
        .alias("VOLUMEPRICETREND")
        .alias("VolumePriceTrend")
        .alias("volumepricetrend")
        .alias("volume_price_trend")
        .alias("VOLUME_PRICE_TREND")
        .alias("Volume_Price_Trend")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Volume Rate of Change
pub fn signature_vroc() -> IndicatorSignature {
    IndicatorSignature::builder("VROC", CATEGORY)
        .name("Volume Rate of Change")
        .description("Rate of change applied to volume")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .machine_id(BarIndicatorId::Vroc)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VROC" is already the main ID, no need for alias
        .alias("Vroc")
        .alias("vroc")
        .alias("VOLUMERATEOFCHANGE")
        .alias("VolumeRateofChange")
        .alias("volumerateofchange")
        .alias("volume_rate_of_change")
        .alias("VOLUME_RATE_OF_CHANGE")
        .alias("Volume_Rate_Of_Change")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Volume Zone Oscillator
pub fn signature_vzo() -> IndicatorSignature {
    IndicatorSignature::builder("VZO", CATEGORY)
        .name("Volume Zone Oscillator")
        .description("Relates volume to price direction")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .metadata("range", "-60 to +60")
        .machine_id(BarIndicatorId::Vzo)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        // Note: "VZO" is already the main ID, no need for alias
        .alias("Vzo")
        .alias("vzo")
        .alias("VOLUMEZONEOSCILLATOR")
        .alias("VolumeZoneOscillator")
        .alias("volumezoneoscillator")
        .alias("volume_zone_oscillator")
        .alias("VOLUME_ZONE_OSCILLATOR")
        .alias("Volume_Zone_Oscillator")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Session VWAP
pub fn signature_session_vwap() -> IndicatorSignature {
    IndicatorSignature::builder("SESSION_VWAP", CATEGORY)
        .name("Session VWAP")
        .description("Cumulative VWAP with explicit session reset — caller resets at session boundary")
        .metadata("formula", "Σ(typical_price × volume) / Σ(volume)")
        .metadata("typical_price", "(high + low + close) / 3")
        .machine_id(BarIndicatorId::SessionVwap)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("SessionVwap")
        .alias("session_vwap")
        .alias("SESSIONVWAP")
        .alias("SessionVWAP")
        .alias("sessionvwap")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Cumulative Volume Delta (rolling)
pub fn signature_cvd() -> IndicatorSignature {
    IndicatorSignature::builder("CVD", CATEGORY)
        .name("Cumulative Volume Delta")
        .description("Rolling sum of synthetic buy/sell delta estimated from candle direction")
        .add_constraint(ParamConstraint::period(10, 500, 50))
        .metadata("estimate", "close > open → +volume; close < open → −volume")
        .machine_id(BarIndicatorId::Cvd)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Single)
        .validated()
        .alias("Cvd")
        .alias("cvd")
        .alias("CUMULATIVEVOLUMEDELTMA")
        .alias("CumulativeVolumeDelta")
        .alias("cumulative_volume_delta")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Rolling Volume Profile (POC / VAH / VAL)
pub fn signature_rvp() -> IndicatorSignature {
    IndicatorSignature::builder("RVP", CATEGORY)
        .name("Rolling Volume Profile")
        .description("Point of Control, Value Area High, Value Area Low over a rolling window")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .add_constraint(
            ParamConstraint::new("bucket_size", ParamType::F64)
                .with_default(ParamValue::F64(1.0))
        )
        .add_constraint(
            ParamConstraint::new("value_area_pct", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.99))
                .with_default(ParamValue::F64(0.7))
        )
        .metadata("outputs", "poc, vah, val")
        .machine_id(BarIndicatorId::Rvp)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Triple)
        .validated()
        .alias("Rvp")
        .alias("rvp")
        .alias("ROLLINGVOLUMEPROFILE")
        .alias("RollingVolumeProfile")
        .alias("rolling_volume_profile")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Trade Flow Imbalance
pub fn signature_trade_flow_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("TRADE_FLOW_IMBALANCE", CATEGORY)
        .name("Trade Flow Imbalance")
        .description("Rolling signed buy/sell volume imbalance from tick stream [-1, 1]")
        .add_constraint(ParamConstraint::period(2, 10000, 100))
        .metadata("range", "-1 to +1")
        .metadata("source", "tick_stream")
        .machine_id(BarIndicatorId::TradeFlowImbalance)
        .role_kind(IndicatorRoleKind::OscillatorBounded)
        .output_kind(IndicatorValueKind::Double)
        .validated()
        .alias("TradeFlowImbalance")
        .alias("trade_flow_imbalance")
        .alias("TFI")
        .alias("tfi")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Uptick/Downtick Volume
pub fn signature_uptick_downtick_volume() -> IndicatorSignature {
    IndicatorSignature::builder("UPTICK_DOWNTICK_VOLUME", CATEGORY)
        .name("Uptick/Downtick Volume")
        .description("Rolling separate buy (uptick) and sell (downtick) volume from tick stream")
        .add_constraint(ParamConstraint::period(2, 10000, 100))
        .metadata("source", "tick_stream")
        .machine_id(BarIndicatorId::UptickDowntickVolume)
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Double)
        .validated()
        .alias("UptickDowntickVolume")
        .alias("uptick_downtick_volume")
        .alias("UDV")
        .alias("udv")
        .source_type(SourceType::VolumeOnly)
        .build()
}

/// Aggressor Imbalance — rolling buy/sell tick frequency ratio
pub fn signature_aggressor_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("AGGRESSOR_IMBALANCE", CATEGORY)
        .name("Aggressor Imbalance")
        .description("Rolling buy-count vs sell-count ratio from tick stream [-1, 1]")
        .add_constraint(ParamConstraint::period(5, 5000, 50))
        .metadata("outputs", "imbalance in [-1, 1] (+1=all buys, -1=all sells)")
        .metadata("uses_ticks", "true")
        .machine_id(BarIndicatorId::AggressorImbalance)
        .role_kind(IndicatorRoleKind::OscillatorBounded)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .alias("AggressorImbalance")
        .alias("aggressor_imbalance")
        .alias("AGGRESSORIMBALANCE")
        .build()
}

/// Large Trade Filter — flags ticks above N× rolling median size
pub fn signature_large_trade_filter() -> IndicatorSignature {
    IndicatorSignature::builder("LARGE_TRADE_FILTER", CATEGORY)
        .name("Large Trade Filter")
        .description("Flags ticks whose size exceeds multiplier × rolling median size")
        .add_constraint(ParamConstraint::period(5, 5000, 50))
        .add_constraint(
            ParamConstraint::new("multiplier", ParamType::F64)
                .with_min(ParamValue::F64(1.1))
                .with_max(ParamValue::F64(50.0))
                .with_default(ParamValue::F64(3.0))
        )
        .metadata("outputs", "signal (+1=large buy, -1=large sell, 0=normal), size_ratio")
        .metadata("uses_ticks", "true")
        .machine_id(BarIndicatorId::LargeTradeFilter)
        .role_kind(IndicatorRoleKind::Pattern)
        .output_kind(IndicatorValueKind::Double)
        .requires_l2()
        .alias("LargeTradeFilter")
        .alias("large_trade_filter")
        .alias("LARGETRADEFILTER")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all volume indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("CVD", signature_cvd as fn() -> IndicatorSignature),
    ("MFI", signature_mfi as fn() -> IndicatorSignature),
    ("NVI_PVI", signature_nvi_pvi as fn() -> IndicatorSignature),
    ("POC", signature_poc_detector as fn() -> IndicatorSignature),
    ("PVO", signature_pvo as fn() -> IndicatorSignature),
    ("PVT", signature_pvt as fn() -> IndicatorSignature),
    ("RVOL", signature_relative_volume as fn() -> IndicatorSignature),
    ("RVP", signature_rvp as fn() -> IndicatorSignature),
    ("SESSION_VWAP", signature_session_vwap as fn() -> IndicatorSignature),
    ("VDELTA", signature_volume_delta as fn() -> IndicatorSignature),
    ("VFI", signature_vfi as fn() -> IndicatorSignature),
    ("VO", signature_volume_oscillator as fn() -> IndicatorSignature),
    ("VPIN", signature_vpin as fn() -> IndicatorSignature),
    ("TRADE_FLOW_IMBALANCE", signature_trade_flow_imbalance as fn() -> IndicatorSignature),
    ("UPTICK_DOWNTICK_VOLUME", signature_uptick_downtick_volume as fn() -> IndicatorSignature),
    ("AGGRESSOR_IMBALANCE", signature_aggressor_imbalance as fn() -> IndicatorSignature),
    ("LARGE_TRADE_FILTER", signature_large_trade_filter as fn() -> IndicatorSignature),
    ("VPROFILE", signature_volume_profile as fn() -> IndicatorSignature),
    ("VPT", signature_vpt as fn() -> IndicatorSignature),
    ("VROC", signature_vroc as fn() -> IndicatorSignature),
    ("VZ", signature_volume_zscore as fn() -> IndicatorSignature),
    ("VZO", signature_vzo as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static VOLUME_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    VOLUME_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators
pub fn count() -> usize {
    BASE_CATALOG.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mfi_signature() {
        let sig = get_signature("MFI").unwrap();
        assert_eq!(sig.id, "MFI");
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
        assert_eq!(count(), 22);
    }
}
