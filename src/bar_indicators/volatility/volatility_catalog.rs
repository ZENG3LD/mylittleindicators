//! volatility_catalog.rs: Auto-generated indicator catalog for volatility indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 41 volatility indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Volatility;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Adaptive Bollinger Bands
pub fn signature_adaptive_bollinger_bands() -> IndicatorSignature {
    IndicatorSignature::builder("ABB", CATEGORY)
        .name("Adaptive Bollinger Bands")
        .description("Bollinger Bands with adaptive standard deviation")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Abb) // TODO: Add to enum
        // Note: "ABB" is already the main ID, no need for alias
        .alias("Abb")
        .alias("abb")
        .alias("ADAPTIVEBOLLINGERBANDS")
        .alias("AdaptiveBollingerBands")
        .alias("adaptivebollingerbands")
        .alias("adaptive_bollinger_bands")
        .alias("ADAPTIVE_BOLLINGER_BANDS")
        .alias("Adaptive_Bollinger_Bands")
        .build()
}

/// Adaptive Volatility Regime
pub fn signature_adaptive_volatility_regime() -> IndicatorSignature {
    IndicatorSignature::builder("AVR", CATEGORY)
        .name("Adaptive Volatility Regime")
        .description("Adaptive regime classification based on volatility")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .machine_id(BarIndicatorId::Avr) // TODO: Add to enum
        // Note: "AVR" is already the main ID, no need for alias
        .alias("Avr")
        .alias("avr")
        .alias("ADAPTIVEVOLATILITYREGIME")
        .alias("AdaptiveVolatilityRegime")
        .alias("adaptivevolatilityregime")
        .alias("adaptive_volatility_regime")
        .alias("ADAPTIVE_VOLATILITY_REGIME")
        .alias("Adaptive_Volatility_Regime")
        .build()
}

/// Average True Range
pub fn signature_atr() -> IndicatorSignature {
    IndicatorSignature::builder("ATR", CATEGORY)
        .name("Average True Range")
        .description("Wilder's Average True Range - volatility indicator")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("author", "J. Welles Wilder")
        .metadata("complexity", "O(1) with any MA type")
        .metadata("note", "Traditionally uses Simple MA, but can optimize with any type")
        .machine_id(BarIndicatorId::Atr)
        // Note: "ATR" is already the main ID, no need for alias
        .alias("Atr")
        .alias("atr")
        .alias("AVERAGETRUERANGE")
        .alias("AverageTrueRange")
        .alias("averagetruerange")
        .alias("average_true_range")
        .alias("AVERAGE_TRUE_RANGE")
        .alias("Average_True_Range")
        .build()
}

/// ATR Bandwidth
pub fn signature_atr_bandwidth() -> IndicatorSignature {
    IndicatorSignature::builder("ATRBW", CATEGORY)
        .name("ATR Bandwidth")
        .description("ATR bandwidth as percentage of price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .machine_id(BarIndicatorId::Atrbw) // TODO: Add to enum
        // Note: "ATRBW" is already the main ID, no need for alias
        .alias("Atrbw")
        .alias("atrbw")
        .alias("ATRBANDWIDTH")
        .alias("ATRBandwidth")
        .alias("atrbandwidth")
        .alias("atr_bandwidth")
        .alias("ATR_BANDWIDTH")
        .alias("Atr_Bandwidth")
        .build()
}

/// ATR Channels
pub fn signature_atr_channels() -> IndicatorSignature {
    IndicatorSignature::builder("ATRC", CATEGORY)
        .name("ATR Channels")
        .description("Price channels based on ATR")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::multiplier(0.5, 10.0, 2.0))
        .machine_id(BarIndicatorId::Atrc) // TODO: Add to enum
        // Note: "ATRC" is already the main ID, no need for alias
        .alias("Atrc")
        .alias("atrc")
        .alias("ATRCHANNELS")
        .alias("ATRChannels")
        .alias("atrchannels")
        .alias("atr_channels")
        .alias("ATR_CHANNELS")
        .alias("Atr_Channels")
        .build()
}

/// ATR Percentile
pub fn signature_atr_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("ATRP", CATEGORY)
        .name("ATR Percentile")
        .description("ATR percentile rank over lookback period")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .machine_id(BarIndicatorId::Atrp) // TODO: Add to enum
        // Note: "ATRP" is already the main ID, no need for alias
        .alias("Atrp")
        .alias("atrp")
        .alias("ATRPERCENTILE")
        .alias("ATRPercentile")
        .alias("atrpercentile")
        .alias("atr_percentile")
        .alias("ATR_PERCENTILE")
        .alias("Atr_Percentile")
        .build()
}

/// ATR Percentile Trend
pub fn signature_atr_percentile_trend() -> IndicatorSignature {
    IndicatorSignature::builder("ATRPT", CATEGORY)
        .name("ATR Percentile Trend")
        .description("Trend direction of ATR percentile")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .add_constraint(
            ParamConstraint::new("trend_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Atrpt) // TODO: Add to enum
        // Note: "ATRPT" is already the main ID, no need for alias
        .alias("Atrpt")
        .alias("atrpt")
        .alias("ATRPERCENTILETREND")
        .alias("ATRPercentileTrend")
        .alias("atrpercentiletrend")
        .alias("atr_percentile_trend")
        .alias("ATR_PERCENTILE_TREND")
        .alias("Atr_Percentile_Trend")
        .build()
}

/// ATR Z-Score
pub fn signature_atr_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("ATRZ", CATEGORY)
        .name("ATR Z-Score")
        .description("Standardized ATR using z-score")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .machine_id(BarIndicatorId::Atrz) // TODO: Add to enum
        // Note: "ATRZ" is already the main ID, no need for alias
        .alias("Atrz")
        .alias("atrz")
        .alias("ATRZSCORE")
        .alias("ATRZScore")
        .alias("atrzscore")
        .alias("atr_z_score")
        .alias("ATR_Z_SCORE")
        .alias("Atr_Z_Score")
        .build()
}

/// Bipower Variance
pub fn signature_bipower_variance() -> IndicatorSignature {
    IndicatorSignature::builder("BPV", CATEGORY)
        .name("Bipower Variance")
        .description("Jump-robust volatility estimator")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .metadata("category", "realized_volatility")
        .machine_id(BarIndicatorId::Bpv) // TODO: Add to enum
        // Note: "BPV" is already the main ID, no need for alias
        .alias("Bpv")
        .alias("bpv")
        .alias("BIPOWERVARIANCE")
        .alias("BipowerVariance")
        .alias("bipowervariance")
        .alias("bipower_variance")
        .alias("BIPOWER_VARIANCE")
        .alias("Bipower_Variance")
        .build()
}

/// Chaikin Volatility
pub fn signature_chaikin_volatility() -> IndicatorSignature {
    IndicatorSignature::builder("CV", CATEGORY)
        .name("Chaikin Volatility")
        .description("Rate of change of high-low range")
        .add_constraint(ParamConstraint::period(5, 100, 10))
        .add_constraint(
            ParamConstraint::new("roc_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("author", "Marc Chaikin")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Cv) // TODO: Add to enum
        // Note: "CV" is already the main ID, no need for alias
        .alias("Cv")
        .alias("cv")
        .alias("CHAIKINVOLATILITY")
        .alias("ChaikinVolatility")
        .alias("chaikinvolatility")
        .alias("chaikin_volatility")
        .alias("CHAIKIN_VOLATILITY")
        .alias("Chaikin_Volatility")
        .build()
}

/// Choppiness Index
pub fn signature_choppiness_index() -> IndicatorSignature {
    IndicatorSignature::builder("CHOP", CATEGORY)
        .name("Choppiness Index")
        .description("Market choppiness indicator (100=choppy, 0=trending)")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .metadata("range", "0-100")
        .machine_id(BarIndicatorId::Chop) // TODO: Add to enum
        // Note: "CHOP" is already the main ID, no need for alias
        .alias("Chop")
        .alias("chop")
        .alias("CHOPPINESSINDEX")
        .alias("ChoppinessIndex")
        .alias("choppinessindex")
        .alias("choppiness_index")
        .alias("CHOPPINESS_INDEX")
        .alias("Choppiness_Index")
        .build()
}

/// Close-to-Close Volatility Percentile
pub fn signature_close_to_close_vol_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("C2CVP", CATEGORY)
        .name("Close-to-Close Volatility Percentile")
        .description("Percentile rank of close-to-close volatility")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .machine_id(BarIndicatorId::C2cvp) // TODO: Add to enum
        // Note: "C2CVP" is already the main ID, no need for alias
        .alias("C2cvp")
        .alias("c2cvp")
        .alias("CLOSETOCLOSEVOLATILITYPERCENTILE")
        .alias("ClosetoCloseVolatilityPercentile")
        .alias("closetoclosevolatilitypercentile")
        .alias("close_to_close_volatility_percentile")
        .alias("CLOSE_TO_CLOSE_VOLATILITY_PERCENTILE")
        .alias("Close_To_Close_Volatility_Percentile")
        .build()
}

/// Donchian Channel (volatility context)
pub fn signature_dc() -> IndicatorSignature {
    IndicatorSignature::builder("VO_DC", CATEGORY)
        .name("Donchian Channel")
        .description("High-low channel for volatility measurement")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .machine_id(BarIndicatorId::VoDc)
        // Note: "VO_DC" is already the main ID, no need for alias
        .alias("VoDc")
        .alias("vo_dc")
        .alias("DONCHIANCHANNEL")
        .alias("DonchianChannel")
        .alias("donchianchannel")
        .alias("donchian_channel")
        .alias("DONCHIAN_CHANNEL")
        .alias("Donchian_Channel")
        .build()
}

/// Dynamic Volatility Regime
pub fn signature_dynamic_volatility_regime() -> IndicatorSignature {
    IndicatorSignature::builder("DVR", CATEGORY)
        .name("Dynamic Volatility Regime")
        .description("Dynamic regime classification based on multiple volatility metrics")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .machine_id(BarIndicatorId::Dvr) // TODO: Add to enum
        // Note: "DVR" is already the main ID, no need for alias
        .alias("Dvr")
        .alias("dvr")
        .alias("DYNAMICVOLATILITYREGIME")
        .alias("DynamicVolatilityRegime")
        .alias("dynamicvolatilityregime")
        .alias("dynamic_volatility_regime")
        .alias("DYNAMIC_VOLATILITY_REGIME")
        .alias("Dynamic_Volatility_Regime")
        .build()
}

/// Fuzzy Volatility
pub fn signature_fuzzy() -> IndicatorSignature {
    IndicatorSignature::builder("FUZZY", CATEGORY)
        .name("Fuzzy Volatility")
        .description("Fuzzy logic-based volatility measure")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .machine_id(BarIndicatorId::Fuzzy) // TODO: Add to enum
        // Note: "FUZZY" is already the main ID, no need for alias
        .alias("Fuzzy")
        .alias("fuzzy")
        .alias("FUZZYVOLATILITY")
        .alias("FuzzyVolatility")
        .alias("fuzzyvolatility")
        .alias("fuzzy_volatility")
        .alias("FUZZY_VOLATILITY")
        .alias("Fuzzy_Volatility")
        .build()
}

/// Heterogeneous Autoregressive Realized Volatility
pub fn signature_har_rv() -> IndicatorSignature {
    IndicatorSignature::builder("HAR", CATEGORY)
        .name("HAR Realized Volatility")
        .description("Multi-period realized volatility model")
        .add_constraint(ParamConstraint::period(10, 100, 22))
        .metadata("category", "realized_volatility")
        .machine_id(BarIndicatorId::Har) // TODO: Add to enum
        // Note: "HAR" is already the main ID, no need for alias
        .alias("Har")
        .alias("har")
        .alias("HARREALIZEDVOLATILITY")
        .alias("HARRealizedVolatility")
        .alias("harrealizedvolatility")
        .alias("har_realized_volatility")
        .alias("HAR_REALIZED_VOLATILITY")
        .alias("Har_Realized_Volatility")
        .build()
}

/// Historical Volatility Close-to-Close
pub fn signature_hv_c2c() -> IndicatorSignature {
    IndicatorSignature::builder("HVC2C", CATEGORY)
        .name("Historical Volatility (Close-to-Close)")
        .description("Standard deviation of log returns")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .machine_id(BarIndicatorId::Hvc2c) // TODO: Add to enum
        // Note: "HVC2C" is already the main ID, no need for alias
        .alias("Hvc2c")
        .alias("hvc2c")
        .alias("HISTORICALVOLATILITY(CLOSETOCLOSE)")
        .alias("HistoricalVolatility(ClosetoClose)")
        .alias("historicalvolatility(closetoclose)")
        .alias("historical_volatility_(close_to_close)")
        .alias("HISTORICAL_VOLATILITY_(CLOSE_TO_CLOSE)")
        .alias("Historical_Volatility_(close_To_Close)")
        .build()
}

/// Keltner Channel (volatility context)
pub fn signature_kc() -> IndicatorSignature {
    IndicatorSignature::builder("VO_KC", CATEGORY)
        .name("Keltner Channel")
        .description("ATR-based volatility channel")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::VoKc)
        // Note: "VO_KC" is already the main ID, no need for alias
        .alias("VoKc")
        .alias("vo_kc")
        .alias("KELTNERCHANNEL")
        .alias("KeltnerChannel")
        .alias("keltnerchannel")
        .alias("keltner_channel")
        .alias("KELTNER_CHANNEL")
        .alias("Keltner_Channel")
        .build()
}

/// Klinger Oscillator Periods (volatility aspect)
pub fn signature_kp() -> IndicatorSignature {
    IndicatorSignature::builder("KP", CATEGORY)
        .name("Klinger Periods")
        .description("Volume-weighted volatility periods")
        .add_constraint(ParamConstraint::period(5, 100, 34))
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(13))
                .required()
        )
        .machine_id(BarIndicatorId::Kp)
        // Note: "KP" is already the main ID, no need for alias
        .alias("Kp")
        .alias("kp")
        .alias("KLINGERPERIODS")
        .alias("KlingerPeriods")
        .alias("klingerperiods")
        .alias("klinger_periods")
        .alias("KLINGER_PERIODS")
        .alias("Klinger_Periods")
        .build()
}

/// Mass Index
pub fn signature_mass_index() -> IndicatorSignature {
    IndicatorSignature::builder("VO_MI", CATEGORY)
        .name("Mass Index")
        .description("Trend reversal indicator based on range expansion")
        .add_constraint(ParamConstraint::period(5, 50, 9))
        .add_constraint(
            ParamConstraint::new("sum_period", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(25))
                .required()
        )
        .machine_id(BarIndicatorId::VoMi)
        // Note: "VO_MI" is already the main ID, no need for alias
        .alias("VoMi")
        .alias("vo_mi")
        .alias("MASSINDEX")
        .alias("MassIndex")
        .alias("massindex")
        .alias("mass_index")
        .alias("MASS_INDEX")
        .alias("Mass_Index")
        .build()
}

/// Normalized ATR
pub fn signature_natr() -> IndicatorSignature {
    IndicatorSignature::builder("NATR", CATEGORY)
        .name("Normalized ATR")
        .description("ATR normalized as percentage of close price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Natr)
        // Note: "NATR" is already the main ID, no need for alias
        .alias("Natr")
        .alias("natr")
        .alias("NORMALIZEDATR")
        .alias("NormalizedATR")
        .alias("normalizedatr")
        .alias("normalized_atr")
        .alias("NORMALIZED_ATR")
        .alias("Normalized_Atr")
        .build()
}

/// Narrow Range Pattern
pub fn signature_nr_range() -> IndicatorSignature {
    IndicatorSignature::builder("NR", CATEGORY)
        .name("Narrow Range")
        .description("Detects narrow range bars (NR4, NR7)")
        .add_constraint(ParamConstraint::period(4, 20, 7))
        .machine_id(BarIndicatorId::Nr) // TODO: Add to enum
        // Note: "NR" is already the main ID, no need for alias
        .alias("Nr")
        .alias("nr")
        .alias("NARROWRANGE")
        .alias("NarrowRange")
        .alias("narrowrange")
        .alias("narrow_range")
        .alias("NARROW_RANGE")
        .alias("Narrow_Range")
        .build()
}

/// Parkinson, Garman-Klass, Rogers-Satchell, Yang-Zhang volatility estimators
pub fn signature_park_gk_rs_yz() -> IndicatorSignature {
    IndicatorSignature::builder("PGRY", CATEGORY)
        .name("Advanced Volatility Estimators")
        .description("Parkinson, GK, RS, YZ volatility estimators")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .metadata("category", "realized_volatility")
        .machine_id(BarIndicatorId::Pgry) // TODO: Add to enum
        // Note: "PGRY" is already the main ID, no need for alias
        .alias("Pgry")
        .alias("pgry")
        .alias("ADVANCEDVOLATILITYESTIMATORS")
        .alias("AdvancedVolatilityEstimators")
        .alias("advancedvolatilityestimators")
        .alias("advanced_volatility_estimators")
        .alias("ADVANCED_VOLATILITY_ESTIMATORS")
        .alias("Advanced_Volatility_Estimators")
        .build()
}

/// Range Compression Burst
pub fn signature_range_compression_burst() -> IndicatorSignature {
    IndicatorSignature::builder("RCB", CATEGORY)
        .name("Range Compression Burst")
        .description("Detects compression and expansion of price range")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(
            ParamConstraint::new("threshold", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(1.5))
                .required()
        )
        .machine_id(BarIndicatorId::Rcb) // TODO: Add to enum
        // Note: "RCB" is already the main ID, no need for alias
        .alias("Rcb")
        .alias("rcb")
        .alias("RANGECOMPRESSIONBURST")
        .alias("RangeCompressionBurst")
        .alias("rangecompressionburst")
        .alias("range_compression_burst")
        .alias("RANGE_COMPRESSION_BURST")
        .alias("Range_Compression_Burst")
        .build()
}

/// Range Percentile
pub fn signature_range_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("RP", CATEGORY)
        .name("Range Percentile")
        .description("Percentile rank of price range")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .machine_id(BarIndicatorId::Rp) // TODO: Add to enum
        // Note: "RP" is already the main ID, no need for alias
        .alias("Rp")
        .alias("rp")
        .alias("RANGEPERCENTILE")
        .alias("RangePercentile")
        .alias("rangepercentile")
        .alias("range_percentile")
        .alias("RANGE_PERCENTILE")
        .alias("Range_Percentile")
        .build()
}

/// RBV Jump Test
pub fn signature_rbv_jump_test() -> IndicatorSignature {
    IndicatorSignature::builder("RBVJ", CATEGORY)
        .name("RBV Jump Test")
        .description("Realized bipower variation jump test")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .metadata("category", "jump_detection")
        .machine_id(BarIndicatorId::Rbvj) // TODO: Add to enum
        // Note: "RBVJ" is already the main ID, no need for alias
        .alias("Rbvj")
        .alias("rbvj")
        .alias("RBVJUMPTEST")
        .alias("RBVJumpTest")
        .alias("rbvjumptest")
        .alias("rbv_jump_test")
        .alias("RBV_JUMP_TEST")
        .alias("Rbv_Jump_Test")
        .build()
}

/// Realized Quarticity
pub fn signature_realized_quarticity() -> IndicatorSignature {
    IndicatorSignature::builder("RQ", CATEGORY)
        .name("Realized Quarticity")
        .description("Fourth moment of realized volatility")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .metadata("category", "realized_volatility")
        .machine_id(BarIndicatorId::Rq) // TODO: Add to enum
        // Note: "RQ" is already the main ID, no need for alias
        .alias("Rq")
        .alias("rq")
        .alias("REALIZEDQUARTICITY")
        .alias("RealizedQuarticity")
        .alias("realizedquarticity")
        .alias("realized_quarticity")
        .alias("REALIZED_QUARTICITY")
        .alias("Realized_Quarticity")
        .build()
}

/// Realized Volatility
pub fn signature_realized_vol() -> IndicatorSignature {
    IndicatorSignature::builder("RV", CATEGORY)
        .name("Realized Volatility")
        .description("Sum of squared returns")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .metadata("category", "realized_volatility")
        .machine_id(BarIndicatorId::Rv) // TODO: Add to enum
        // Note: "RV" is already the main ID, no need for alias
        .alias("Rv")
        .alias("rv")
        .alias("REALIZEDVOLATILITY")
        .alias("RealizedVolatility")
        .alias("realizedvolatility")
        .alias("realized_volatility")
        .alias("REALIZED_VOLATILITY")
        .alias("Realized_Volatility")
        .build()
}

/// Realized Volatility Z-Score
pub fn signature_realized_vol_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("RVZ", CATEGORY)
        .name("Realized Volatility Z-Score")
        .description("Standardized realized volatility")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .machine_id(BarIndicatorId::Rvz) // TODO: Add to enum
        // Note: "RVZ" is already the main ID, no need for alias
        .alias("Rvz")
        .alias("rvz")
        .alias("REALIZEDVOLATILITYZSCORE")
        .alias("RealizedVolatilityZScore")
        .alias("realizedvolatilityzscore")
        .alias("realized_volatility_z_score")
        .alias("REALIZED_VOLATILITY_Z_SCORE")
        .alias("Realized_Volatility_Z_Score")
        .build()
}

/// Relative Volatility Index
pub fn signature_rvi() -> IndicatorSignature {
    IndicatorSignature::builder("RVI", CATEGORY)
        .name("Relative Volatility Index")
        .description("RSI applied to standard deviation")
        .add_constraint(ParamConstraint::period(5, 100, 10))
        .add_constraint(
            ParamConstraint::new("stddev_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Rvi)
        // Note: "RVI" is already the main ID, no need for alias
        .alias("Rvi")
        .alias("rvi")
        .alias("RELATIVEVOLATILITYINDEX")
        .alias("RelativeVolatilityIndex")
        .alias("relativevolatilityindex")
        .alias("relative_volatility_index")
        .alias("RELATIVE_VOLATILITY_INDEX")
        .alias("Relative_Volatility_Index")
        .build()
}

/// Squeeze Momentum
pub fn signature_squeeze_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("SQMOM", CATEGORY)
        .name("Squeeze Momentum")
        .description("TTM Squeeze indicator (Bollinger inside Keltner)")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .add_constraint(ParamConstraint::multiplier(1.0, 3.0, 1.5))
        .add_constraint(ParamConstraint::ma_type_named("bb_ma_type", MovingAverageType::SMA))
        .add_constraint(ParamConstraint::ma_type_named("kc_ma_type", MovingAverageType::SMA))
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("bb_ma_desc", "Bollinger Bands MA type")
        .metadata("kc_ma_desc", "Keltner Channel MA type")
        .metadata("author", "John Carter")
        .machine_id(BarIndicatorId::Sqmom) // TODO: Add to enum
        // Note: "SQMOM" is already the main ID, no need for alias
        .alias("Sqmom")
        .alias("sqmom")
        .alias("SQUEEZEMOMENTUM")
        .alias("SqueezeMomentum")
        .alias("squeezemomentum")
        .alias("squeeze_momentum")
        .alias("SQUEEZE_MOMENTUM")
        .alias("Squeeze_Momentum")
        .build()
}

/// True Range
pub fn signature_true_range() -> IndicatorSignature {
    IndicatorSignature::builder("TR", CATEGORY)
        .name("True Range")
        .description("Wilder's True Range (single bar)")
        .metadata("author", "J. Welles Wilder")
        .metadata("parameters", "none")
        .machine_id(BarIndicatorId::Tr) // TODO: Add to enum
        // Note: "TR" is already the main ID, no need for alias
        .alias("Tr")
        .alias("tr")
        .alias("TRUERANGE")
        .alias("TrueRange")
        .alias("truerange")
        .alias("true_range")
        .alias("TRUE_RANGE")
        .alias("True_Range")
        .build()
}

/// Ulcer Index
pub fn signature_ulcer_index() -> IndicatorSignature {
    IndicatorSignature::builder("UI", CATEGORY)
        .name("Ulcer Index")
        .description("Downside volatility measure")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .metadata("category", "downside_risk")
        .machine_id(BarIndicatorId::Ui) // TODO: Add to enum
        // Note: "UI" is already the main ID, no need for alias
        .alias("Ui")
        .alias("ui")
        .alias("ULCERINDEX")
        .alias("UlcerIndex")
        .alias("ulcerindex")
        .alias("ulcer_index")
        .alias("ULCER_INDEX")
        .alias("Ulcer_Index")
        .build()
}

/// Volatility of Volatility
pub fn signature_vol_of_vol() -> IndicatorSignature {
    IndicatorSignature::builder("VOV", CATEGORY)
        .name("Volatility of Volatility")
        .description("Standard deviation of volatility")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .add_constraint(
            ParamConstraint::new("vol_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Vov) // TODO: Add to enum
        // Note: "VOV" is already the main ID, no need for alias
        .alias("Vov")
        .alias("vov")
        .alias("VOLATILITYOFVOLATILITY")
        .alias("VolatilityofVolatility")
        .alias("volatilityofvolatility")
        .alias("volatility_of_volatility")
        .alias("VOLATILITY_OF_VOLATILITY")
        .alias("Volatility_Of_Volatility")
        .build()
}

/// Volatility of Volatility Percentile
pub fn signature_vol_of_vol_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("VOVP", CATEGORY)
        .name("Volatility of Volatility Percentile")
        .description("Percentile rank of volatility of volatility")
        .add_constraint(ParamConstraint::period(10, 100, 50))
        .machine_id(BarIndicatorId::Vovp) // TODO: Add to enum
        // Note: "VOVP" is already the main ID, no need for alias
        .alias("Vovp")
        .alias("vovp")
        .alias("VOLATILITYOFVOLATILITYPERCENTILE")
        .alias("VolatilityofVolatilityPercentile")
        .alias("volatilityofvolatilitypercentile")
        .alias("volatility_of_volatility_percentile")
        .alias("VOLATILITY_OF_VOLATILITY_PERCENTILE")
        .alias("Volatility_Of_Volatility_Percentile")
        .build()
}

/// Volatility of Volatility Percentile Trend
pub fn signature_vol_of_vol_percentile_trend() -> IndicatorSignature {
    IndicatorSignature::builder("VOVPT", CATEGORY)
        .name("Volatility of Volatility Percentile Trend")
        .description("Trend of VoV percentile")
        .add_constraint(ParamConstraint::period(10, 100, 50))
        .add_constraint(
            ParamConstraint::new("trend_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Vovpt) // TODO: Add to enum
        // Note: "VOVPT" is already the main ID, no need for alias
        .alias("Vovpt")
        .alias("vovpt")
        .alias("VOLATILITYOFVOLATILITYPERCENTILETREND")
        .alias("VolatilityofVolatilityPercentileTrend")
        .alias("volatilityofvolatilitypercentiletrend")
        .alias("volatility_of_volatility_percentile_trend")
        .alias("VOLATILITY_OF_VOLATILITY_PERCENTILE_TREND")
        .alias("Volatility_Of_Volatility_Percentile_Trend")
        .build()
}

/// Volatility Breakout Exponential
pub fn signature_volatility_break_exp() -> IndicatorSignature {
    IndicatorSignature::builder("VBEXP", CATEGORY)
        .name("Volatility Breakout Exponential")
        .description("Exponential volatility breakout detection")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(
            ParamConstraint::new("threshold", ParamType::F64)
                .with_min(ParamValue::F64(0.5))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(2.0))
                .required()
        )
        .machine_id(BarIndicatorId::Vbexp) // TODO: Add to enum
        // Note: "VBEXP" is already the main ID, no need for alias
        .alias("Vbexp")
        .alias("vbexp")
        .alias("VOLATILITYBREAKOUTEXPONENTIAL")
        .alias("VolatilityBreakoutExponential")
        .alias("volatilitybreakoutexponential")
        .alias("volatility_breakout_exponential")
        .alias("VOLATILITY_BREAKOUT_EXPONENTIAL")
        .alias("Volatility_Breakout_Exponential")
        .build()
}

/// Volatility Breakout Detector
pub fn signature_volatility_breakout_detector() -> IndicatorSignature {
    IndicatorSignature::builder("VBD", CATEGORY)
        .name("Volatility Breakout Detector")
        .description("Detects volatility expansion breakouts")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .add_constraint(
            ParamConstraint::new("threshold", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(2.0))
                .required()
        )
        .machine_id(BarIndicatorId::Vbd) // TODO: Add to enum
        // Note: "VBD" is already the main ID, no need for alias
        .alias("Vbd")
        .alias("vbd")
        .alias("VOLATILITYBREAKOUTDETECTOR")
        .alias("VolatilityBreakoutDetector")
        .alias("volatilitybreakoutdetector")
        .alias("volatility_breakout_detector")
        .alias("VOLATILITY_BREAKOUT_DETECTOR")
        .alias("Volatility_Breakout_Detector")
        .build()
}

/// Volatility Percentile Rank Bands
pub fn signature_volatility_percentile_rank_bands() -> IndicatorSignature {
    IndicatorSignature::builder("VPRB", CATEGORY)
        .name("Volatility Percentile Rank Bands")
        .description("Bands based on volatility percentile rank")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .add_constraint(
            ParamConstraint::new("upper_pct", ParamType::F64)
                .with_min(ParamValue::F64(50.0))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(80.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("lower_pct", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(50.0))
                .with_default(ParamValue::F64(20.0))
                .required()
        )
        .machine_id(BarIndicatorId::Vprb) // TODO: Add to enum
        // Note: "VPRB" is already the main ID, no need for alias
        .alias("Vprb")
        .alias("vprb")
        .alias("VOLATILITYPERCENTILERANKBANDS")
        .alias("VolatilityPercentileRankBands")
        .alias("volatilitypercentilerankbands")
        .alias("volatility_percentile_rank_bands")
        .alias("VOLATILITY_PERCENTILE_RANK_BANDS")
        .alias("Volatility_Percentile_Rank_Bands")
        .build()
}

/// Volatility Ratio
pub fn signature_vr() -> IndicatorSignature {
    IndicatorSignature::builder("VO_VR", CATEGORY)
        .name("Volatility Ratio")
        .description("Ratio of short-term to long-term volatility")
        .add_constraint(
            ParamConstraint::new("short_period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("long_period", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(30))
                .required()
        )
        .machine_id(BarIndicatorId::VoVr)
        // Note: "VO_VR" is already the main ID, no need for alias
        .alias("VoVr")
        .alias("vo_vr")
        .alias("VOLATILITYRATIO")
        .alias("VolatilityRatio")
        .alias("volatilityratio")
        .alias("volatility_ratio")
        .alias("VOLATILITY_RATIO")
        .alias("Volatility_Ratio")
        .build()
}

/// Williams VIX Fix
pub fn signature_wvf() -> IndicatorSignature {
    IndicatorSignature::builder("WVF", CATEGORY)
        .name("Williams VIX Fix")
        .description("Synthetic VIX using price data")
        .add_constraint(ParamConstraint::period(10, 100, 22))
        .metadata("author", "Larry Williams")
        .machine_id(BarIndicatorId::Wvf)
        // Note: "WVF" is already the main ID, no need for alias
        .alias("Wvf")
        .alias("wvf")
        .alias("WILLIAMSVIXFIX")
        .alias("WilliamsVIXFix")
        .alias("williamsvixfix")
        .alias("williams_vix_fix")
        .alias("WILLIAMS_VIX_FIX")
        .alias("Williams_Vix_Fix")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all volatility indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ABB", signature_adaptive_bollinger_bands as fn() -> IndicatorSignature),
    ("ATR", signature_atr as fn() -> IndicatorSignature),
    ("ATRBW", signature_atr_bandwidth as fn() -> IndicatorSignature),
    ("ATRC", signature_atr_channels as fn() -> IndicatorSignature),
    ("ATRP", signature_atr_percentile as fn() -> IndicatorSignature),
    ("ATRPT", signature_atr_percentile_trend as fn() -> IndicatorSignature),
    ("ATRZ", signature_atr_zscore as fn() -> IndicatorSignature),
    ("AVR", signature_adaptive_volatility_regime as fn() -> IndicatorSignature),
    ("BPV", signature_bipower_variance as fn() -> IndicatorSignature),
    ("C2CVP", signature_close_to_close_vol_percentile as fn() -> IndicatorSignature),
    ("CHOP", signature_choppiness_index as fn() -> IndicatorSignature),
    ("CV", signature_chaikin_volatility as fn() -> IndicatorSignature),
    ("VO_DC", signature_dc as fn() -> IndicatorSignature),
    ("DVR", signature_dynamic_volatility_regime as fn() -> IndicatorSignature),
    ("FUZZY", signature_fuzzy as fn() -> IndicatorSignature),
    ("HAR", signature_har_rv as fn() -> IndicatorSignature),
    ("HVC2C", signature_hv_c2c as fn() -> IndicatorSignature),
    ("VO_KC", signature_kc as fn() -> IndicatorSignature),
    ("KP", signature_kp as fn() -> IndicatorSignature),
    ("VO_MI", signature_mass_index as fn() -> IndicatorSignature),
    ("NATR", signature_natr as fn() -> IndicatorSignature),
    ("NR", signature_nr_range as fn() -> IndicatorSignature),
    ("PGRY", signature_park_gk_rs_yz as fn() -> IndicatorSignature),
    ("RCB", signature_range_compression_burst as fn() -> IndicatorSignature),
    ("RP", signature_range_percentile as fn() -> IndicatorSignature),
    ("RBVJ", signature_rbv_jump_test as fn() -> IndicatorSignature),
    ("RQ", signature_realized_quarticity as fn() -> IndicatorSignature),
    ("RV", signature_realized_vol as fn() -> IndicatorSignature),
    ("RVI", signature_rvi as fn() -> IndicatorSignature),
    ("RVZ", signature_realized_vol_zscore as fn() -> IndicatorSignature),
    ("SQMOM", signature_squeeze_momentum as fn() -> IndicatorSignature),
    ("TR", signature_true_range as fn() -> IndicatorSignature),
    ("UI", signature_ulcer_index as fn() -> IndicatorSignature),
    ("VBEXP", signature_volatility_break_exp as fn() -> IndicatorSignature),
    ("VBD", signature_volatility_breakout_detector as fn() -> IndicatorSignature),
    ("VOV", signature_vol_of_vol as fn() -> IndicatorSignature),
    ("VOVP", signature_vol_of_vol_percentile as fn() -> IndicatorSignature),
    ("VOVPT", signature_vol_of_vol_percentile_trend as fn() -> IndicatorSignature),
    ("VO_VR", signature_vr as fn() -> IndicatorSignature),
    ("VPRB", signature_volatility_percentile_rank_bands as fn() -> IndicatorSignature),
    ("WVF", signature_wvf as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static VOLATILITY_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    VOLATILITY_CATALOG.get(id).map(|f| f())
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
    fn test_get_atr_signature() {
        let sig = get_signature("ATR").unwrap();
        assert_eq!(sig.id, "ATR");
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
        assert_eq!(count(), 41);
    }
}
