//! statistics_catalog.rs: Catalog of Statistical Test indicators
//!
//! Statistical indicators for stationarity tests, cointegration, structural breaks, etc.
//! Contains IndicatorSignature definitions for 26 statistical test indicators.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Statistics;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// ADF Proxy - Augmented Dickey-Fuller test proxy
pub fn signature_adf_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("ADF", CATEGORY)
        .name("ADF Proxy")
        .description("Augmented Dickey-Fuller stationarity test proxy")
        .add_constraint(ParamConstraint::period(20, 500, 100))
        .metadata("test_type", "stationarity")
        .metadata("null_hypothesis", "unit root (non-stationary)")
        .machine_id(BarIndicatorId::Adf) // TODO: Add to enum
        // Note: "ADF" is already the main ID, no need for alias
        .alias("Adf")
        .alias("adf")
        .alias("ADFPROXY")
        .alias("ADFProxy")
        .alias("adfproxy")
        .alias("adf_proxy")
        .alias("ADF_PROXY")
        .alias("Adf_Proxy")
        .build()
}

/// KPSS Proxy - Kwiatkowski-Phillips-Schmidt-Shin test proxy
pub fn signature_kpss_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("KPSS", CATEGORY)
        .name("KPSS Proxy")
        .description("KPSS stationarity test proxy")
        .add_constraint(ParamConstraint::period(20, 500, 100))
        .metadata("test_type", "stationarity")
        .metadata("null_hypothesis", "stationary")
        .machine_id(BarIndicatorId::Kpss) // TODO: Add to enum
        // Note: "KPSS" is already the main ID, no need for alias
        .alias("Kpss")
        .alias("kpss")
        .alias("KPSSPROXY")
        .alias("KPSSProxy")
        .alias("kpssproxy")
        .alias("kpss_proxy")
        .alias("KPSS_PROXY")
        .alias("Kpss_Proxy")
        .build()
}

/// KPSS Trend Proxy - KPSS test with trend component
pub fn signature_kpss_trend_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("KPSS_TREND", CATEGORY)
        .name("KPSS Trend Proxy")
        .description("KPSS stationarity test with trend")
        .add_constraint(ParamConstraint::period(20, 500, 100))
        .metadata("test_type", "stationarity")
        .metadata("trend", "true")
        .machine_id(BarIndicatorId::KpssTrend) // TODO: Add to enum
        // Note: "KPSS_TREND" is already the main ID, no need for alias
        .alias("KpssTrend")
        .alias("kpss_trend")
        .alias("KPSSTRENDPROXY")
        .alias("KPSSTrendProxy")
        .alias("kpsstrendproxy")
        .alias("kpss_trend_proxy")
        .alias("KPSS_TREND_PROXY")
        .alias("Kpss_Trend_Proxy")
        .build()
}

/// KPSS Z Proxy - KPSS z-statistic proxy
pub fn signature_kpss_z_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("KPSS_Z", CATEGORY)
        .name("KPSS Z Proxy")
        .description("KPSS z-statistic proxy")
        .add_constraint(ParamConstraint::period(20, 500, 100))
        .metadata("test_type", "stationarity")
        .machine_id(BarIndicatorId::KpssZ) // TODO: Add to enum
        // Note: "KPSS_Z" is already the main ID, no need for alias
        .alias("KpssZ")
        .alias("kpss_z")
        .alias("KPSSZPROXY")
        .alias("KPSSZProxy")
        .alias("kpsszproxy")
        .alias("kpss_z_proxy")
        .alias("KPSS_Z_PROXY")
        .alias("Kpss_Z_Proxy")
        .build()
}

/// ADF-KPSS Composite - Combined ADF and KPSS test
pub fn signature_adf_kpss_composite() -> IndicatorSignature {
    IndicatorSignature::builder("ADF_KPSS", CATEGORY)
        .name("ADF-KPSS Composite")
        .description("Combined ADF and KPSS stationarity test")
        .add_constraint(ParamConstraint::period(20, 500, 100))
        .metadata("test_type", "stationarity")
        .metadata("composite", "true")
        .machine_id(BarIndicatorId::AdfKpss) // TODO: Add to enum
        // Note: "ADF_KPSS" is already the main ID, no need for alias
        .alias("AdfKpss")
        .alias("adf_kpss")
        .alias("ADFKPSSCOMPOSITE")
        .alias("ADFKPSSComposite")
        .alias("adfkpsscomposite")
        .alias("adf_kpss_composite")
        .alias("ADF_KPSS_COMPOSITE")
        .alias("Adf_Kpss_Composite")
        .build()
}

/// Phillips-Perron Proxy - Phillips-Perron unit root test
pub fn signature_phillips_perron_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("PP", CATEGORY)
        .name("Phillips-Perron Proxy")
        .description("Phillips-Perron unit root test proxy")
        .add_constraint(ParamConstraint::period(20, 500, 100))
        .metadata("test_type", "stationarity")
        .machine_id(BarIndicatorId::Pp) // TODO: Add to enum
        // Note: "PP" is already the main ID, no need for alias
        .alias("Pp")
        .alias("pp")
        .alias("PHILLIPSPERRONPROXY")
        .alias("PhillipsPerronProxy")
        .alias("phillipsperronproxy")
        .alias("phillips_perron_proxy")
        .alias("PHILLIPS_PERRON_PROXY")
        .alias("Phillips_Perron_Proxy")
        .build()
}

/// Zivot-Andrews Proxy - Structural break test
pub fn signature_zivot_andrews_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("ZA", CATEGORY)
        .name("Zivot-Andrews Proxy")
        .description("Zivot-Andrews structural break test")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "structural_break")
        .machine_id(BarIndicatorId::Za) // TODO: Add to enum
        // Note: "ZA" is already the main ID, no need for alias
        .alias("Za")
        .alias("za")
        .alias("ZIVOTANDREWSPROXY")
        .alias("ZivotAndrewsProxy")
        .alias("zivotandrewsproxy")
        .alias("zivot_andrews_proxy")
        .alias("ZIVOT_ANDREWS_PROXY")
        .alias("Zivot_Andrews_Proxy")
        .build()
}

/// ARCH-LM Proxy - ARCH Lagrange Multiplier test for heteroskedasticity
pub fn signature_arch_lm_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("ARCH_LM", CATEGORY)
        .name("ARCH-LM Proxy")
        .description("ARCH Lagrange Multiplier test for conditional heteroskedasticity")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .add_constraint(
            ParamConstraint::new("lags", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
        )
        .metadata("test_type", "heteroskedasticity")
        .machine_id(BarIndicatorId::ArchLm) // TODO: Add to enum
        // Note: "ARCH_LM" is already the main ID, no need for alias
        .alias("ArchLm")
        .alias("arch_lm")
        .alias("ARCHLMPROXY")
        .alias("ARCHLMProxy")
        .alias("archlmproxy")
        .alias("arch_lm_proxy")
        .alias("ARCH_LM_PROXY")
        .alias("Arch_Lm_Proxy")
        .build()
}

/// ARCH-LM P-Value Proxy - ARCH-LM test p-value
pub fn signature_arch_lm_pvalue_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("ARCH_LM_PVAL", CATEGORY)
        .name("ARCH-LM P-Value Proxy")
        .description("ARCH-LM test p-value proxy")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .metadata("test_type", "heteroskedasticity")
        .metadata("output", "p-value")
        .machine_id(BarIndicatorId::ArchLmPval) // TODO: Add to enum
        // Note: "ARCH_LM_PVAL" is already the main ID, no need for alias
        .alias("ArchLmPval")
        .alias("arch_lm_pval")
        .alias("ARCHLMPVALUEPROXY")
        .alias("ARCHLMPValueProxy")
        .alias("archlmpvalueproxy")
        .alias("arch_lm_p_value_proxy")
        .alias("ARCH_LM_P_VALUE_PROXY")
        .alias("Arch_Lm_P_Value_Proxy")
        .build()
}

/// Ljung-Box Test - Autocorrelation test
pub fn signature_ljung_box() -> IndicatorSignature {
    IndicatorSignature::builder("LJUNG_BOX", CATEGORY)
        .name("Ljung-Box Test")
        .description("Ljung-Box test for autocorrelation")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .add_constraint(
            ParamConstraint::new("lags", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
        )
        .metadata("test_type", "autocorrelation")
        .machine_id(BarIndicatorId::LjungBox)
        // Note: "LJUNG_BOX" is already the main ID, no need for alias
        .alias("LjungBox")
        .alias("ljung_box")
        .alias("LJUNGBOXTEST")
        .alias("LjungBoxTest")
        .alias("ljungboxtest")
        .alias("ljung_box_test")
        .alias("LJUNG_BOX_TEST")
        .alias("Ljung_Box_Test")
        .build()
}

/// PACF - Partial Autocorrelation Function
pub fn signature_pacf() -> IndicatorSignature {
    IndicatorSignature::builder("PACF", CATEGORY)
        .name("Partial Autocorrelation Function")
        .description("PACF for identifying AR order")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .add_constraint(
            ParamConstraint::new("lag", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(1))
        )
        .metadata("test_type", "autocorrelation")
        .machine_id(BarIndicatorId::Pacf)
        // Note: "PACF" is already the main ID, no need for alias
        .alias("Pacf")
        .alias("pacf")
        .alias("PARTIALAUTOCORRELATIONFUNCTION")
        .alias("PartialAutocorrelationFunction")
        .alias("partialautocorrelationfunction")
        .alias("partial_autocorrelation_function")
        .alias("PARTIAL_AUTOCORRELATION_FUNCTION")
        .alias("Partial_Autocorrelation_Function")
        .build()
}

/// Engle-Granger Cointegration Proxy
pub fn signature_engle_granger_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("EG_COINT", CATEGORY)
        .name("Engle-Granger Cointegration Proxy")
        .description("Engle-Granger cointegration test proxy")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "cointegration")
        .machine_id(BarIndicatorId::EgCoint) // TODO: Add to enum
        // Note: "EG_COINT" is already the main ID, no need for alias
        .alias("EgCoint")
        .alias("eg_coint")
        .alias("ENGLEGRANGERCOINTEGRATIONPROXY")
        .alias("EngleGrangerCointegrationProxy")
        .alias("englegrangercointegrationproxy")
        .alias("engle_granger_cointegration_proxy")
        .alias("ENGLE_GRANGER_COINTEGRATION_PROXY")
        .alias("Engle_Granger_Cointegration_Proxy")
        .build()
}

/// Engle-Granger ADF Proxy
pub fn signature_engle_granger_adf_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("EG_ADF", CATEGORY)
        .name("Engle-Granger ADF Proxy")
        .description("Engle-Granger residual ADF test")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "cointegration")
        .machine_id(BarIndicatorId::EgAdf) // TODO: Add to enum
        // Note: "EG_ADF" is already the main ID, no need for alias
        .alias("EgAdf")
        .alias("eg_adf")
        .alias("ENGLEGRANGERADFPROXY")
        .alias("EngleGrangerADFProxy")
        .alias("englegrangeradfproxy")
        .alias("engle_granger_adf_proxy")
        .alias("ENGLE_GRANGER_ADF_PROXY")
        .alias("Engle_Granger_Adf_Proxy")
        .build()
}

/// Engle-Granger Trend Proxy
pub fn signature_engle_granger_trend_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("EG_TREND", CATEGORY)
        .name("Engle-Granger Trend Proxy")
        .description("Engle-Granger with trend component")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "cointegration")
        .metadata("trend", "true")
        .machine_id(BarIndicatorId::EgTrend) // TODO: Add to enum
        // Note: "EG_TREND" is already the main ID, no need for alias
        .alias("EgTrend")
        .alias("eg_trend")
        .alias("ENGLEGRANGERTRENDPROXY")
        .alias("EngleGrangerTrendProxy")
        .alias("englegrangertrendproxy")
        .alias("engle_granger_trend_proxy")
        .alias("ENGLE_GRANGER_TREND_PROXY")
        .alias("Engle_Granger_Trend_Proxy")
        .build()
}

/// Cointegration Proxy - Generic cointegration test
pub fn signature_cointegration_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("COINT", CATEGORY)
        .name("Cointegration Proxy")
        .description("Generic cointegration test proxy")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "cointegration")
        .machine_id(BarIndicatorId::Coint) // TODO: Add to enum
        // Note: "COINT" is already the main ID, no need for alias
        .alias("Coint")
        .alias("coint")
        .alias("COINTEGRATIONPROXY")
        .alias("CointegrationProxy")
        .alias("cointegrationproxy")
        .alias("cointegration_proxy")
        .alias("COINTEGRATION_PROXY")
        .alias("Cointegration_Proxy")
        .build()
}

/// Half-Life Mean Reversion
pub fn signature_half_life_mr() -> IndicatorSignature {
    IndicatorSignature::builder("HALF_LIFE_MR", CATEGORY)
        .name("Half-Life Mean Reversion")
        .description("Mean reversion half-life estimation")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .metadata("test_type", "mean_reversion")
        .machine_id(BarIndicatorId::HalfLifeMr)
        // Note: "HALF_LIFE_MR" is already the main ID, no need for alias
        .alias("HalfLifeMr")
        .alias("half_life_mr")
        .alias("HALFLIFEMEANREVERSION")
        .alias("HalfLifeMeanReversion")
        .alias("halflifemeanreversion")
        .alias("half_life_mean_reversion")
        .alias("HALF_LIFE_MEAN_REVERSION")
        .alias("Half_Life_Mean_Reversion")
        .build()
}

/// Residual Stationarity
pub fn signature_residual_stationarity() -> IndicatorSignature {
    IndicatorSignature::builder("RESID_STAT", CATEGORY)
        .name("Residual Stationarity")
        .description("Stationarity test on regression residuals")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .metadata("test_type", "stationarity")
        .metadata("residual_based", "true")
        .machine_id(BarIndicatorId::ResidStat) // TODO: Add to enum
        // Note: "RESID_STAT" is already the main ID, no need for alias
        .alias("ResidStat")
        .alias("resid_stat")
        .alias("RESIDUALSTATIONARITY")
        .alias("ResidualStationarity")
        .alias("residualstationarity")
        .alias("residual_stationarity")
        .alias("RESIDUAL_STATIONARITY")
        .alias("Residual_Stationarity")
        .build()
}

/// Variance Ratio Test
pub fn signature_variance_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("VR", CATEGORY)
        .name("Variance Ratio Test")
        .description("Variance ratio test for random walk")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .add_constraint(
            ParamConstraint::new("lag", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(5))
        )
        .metadata("test_type", "random_walk")
        .machine_id(BarIndicatorId::Vr) // TODO: Add to enum
        // Note: "VR" is already the main ID, no need for alias
        .alias("Vr")
        .alias("vr")
        .alias("VARIANCERATIOTEST")
        .alias("VarianceRatioTest")
        .alias("varianceratiotest")
        .alias("variance_ratio_test")
        .alias("VARIANCE_RATIO_TEST")
        .alias("Variance_Ratio_Test")
        .build()
}

/// Variance Ratio Aggregate
pub fn signature_variance_ratio_aggregate() -> IndicatorSignature {
    IndicatorSignature::builder("VR_AGG", CATEGORY)
        .name("Variance Ratio Aggregate")
        .description("Aggregated variance ratio across multiple lags")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .metadata("test_type", "random_walk")
        .metadata("aggregate", "true")
        .machine_id(BarIndicatorId::VrAgg) // TODO: Add to enum
        // Note: "VR_AGG" is already the main ID, no need for alias
        .alias("VrAgg")
        .alias("vr_agg")
        .alias("VARIANCERATIOAGGREGATE")
        .alias("VarianceRatioAggregate")
        .alias("varianceratioaggregate")
        .alias("variance_ratio_aggregate")
        .alias("VARIANCE_RATIO_AGGREGATE")
        .alias("Variance_Ratio_Aggregate")
        .build()
}

/// Variance Ratio Z Aggregate
pub fn signature_variance_ratio_z_aggregate() -> IndicatorSignature {
    IndicatorSignature::builder("VR_Z_AGG", CATEGORY)
        .name("Variance Ratio Z Aggregate")
        .description("Aggregated variance ratio z-statistics")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .metadata("test_type", "random_walk")
        .metadata("output", "z-statistic")
        .machine_id(BarIndicatorId::VrZAgg) // TODO: Add to enum
        // Note: "VR_Z_AGG" is already the main ID, no need for alias
        .alias("VrZAgg")
        .alias("vr_z_agg")
        .alias("VARIANCERATIOZAGGREGATE")
        .alias("VarianceRatioZAggregate")
        .alias("varianceratiozaggregate")
        .alias("variance_ratio_z_aggregate")
        .alias("VARIANCE_RATIO_Z_AGGREGATE")
        .alias("Variance_Ratio_Z_Aggregate")
        .build()
}

/// CUSUM Break Detector - Cumulative sum structural break detector
pub fn signature_cusum_break_detector() -> IndicatorSignature {
    IndicatorSignature::builder("ST_CUSUM", CATEGORY)
        .name("CUSUM Break Detector")
        .description("Cumulative sum structural break detector")
        .add_constraint(ParamConstraint::period(30, 500, 100))
        .metadata("test_type", "structural_break")
        .machine_id(BarIndicatorId::StCusum)
        // Note: "ST_CUSUM" is already the main ID, no need for alias
        .alias("StCusum")
        .alias("st_cusum")
        .alias("CUSUMBREAKDETECTOR")
        .alias("CUSUMBreakDetector")
        .alias("cusumbreakdetector")
        .alias("cusum_break_detector")
        .alias("CUSUM_BREAK_DETECTOR")
        .alias("Cusum_Break_Detector")
        .build()
}

/// Bai-Perron CUSUM - Bai-Perron multiple break test using CUSUM
pub fn signature_bai_perron_cusum() -> IndicatorSignature {
    IndicatorSignature::builder("BP_CUSUM", CATEGORY)
        .name("Bai-Perron CUSUM")
        .description("Bai-Perron multiple structural break test")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "structural_break")
        .metadata("multiple_breaks", "true")
        .machine_id(BarIndicatorId::BpCusum) // TODO: Add to enum
        // Note: "BP_CUSUM" is already the main ID, no need for alias
        .alias("BpCusum")
        .alias("bp_cusum")
        .alias("BAIPERRONCUSUM")
        .alias("BaiPerronCUSUM")
        .alias("baiperroncusum")
        .alias("bai_perron_cusum")
        .alias("BAI_PERRON_CUSUM")
        .alias("Bai_Perron_Cusum")
        .build()
}

/// Price-Volume Coherence Proxy - Coherence between price and volume
pub fn signature_price_volume_coherence_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("PV_COHERENCE", CATEGORY)
        .name("Price-Volume Coherence Proxy")
        .description("Spectral coherence between price and volume")
        .add_constraint(ParamConstraint::period(50, 500, 100))
        .metadata("test_type", "coherence")
        .machine_id(BarIndicatorId::PvCoherence) // TODO: Add to enum
        // Note: "PV_COHERENCE" is already the main ID, no need for alias
        .alias("PvCoherence")
        .alias("pv_coherence")
        .alias("PRICEVOLUMECOHERENCEPROXY")
        .alias("PriceVolumeCoherenceProxy")
        .alias("pricevolumecoherenceproxy")
        .alias("price_volume_coherence_proxy")
        .alias("PRICE_VOLUME_COHERENCE_PROXY")
        .alias("Price_Volume_Coherence_Proxy")
        .build()
}

/// Price Z-Score - Normalized price score
pub fn signature_price_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("PRICE_ZSCORE", CATEGORY)
        .name("Price Z-Score")
        .description("Z-score normalized price")
        .add_constraint(ParamConstraint::period(20, 500, 50))
        .metadata("test_type", "normalization")
        .machine_id(BarIndicatorId::PriceZscore) // TODO: Add to enum
        // Note: "PRICE_ZSCORE" is already the main ID, no need for alias
        .alias("PriceZscore")
        .alias("price_zscore")
        .alias("PRICEZSCORE")
        .alias("PriceZScore")
        .alias("pricezscore")
        .alias("price_z_score")
        .alias("PRICE_Z_SCORE")
        .alias("Price_Z_Score")
        .build()
}

/// R-Squared - Coefficient of determination
pub fn signature_r_squared() -> IndicatorSignature {
    IndicatorSignature::builder("R_SQUARED", CATEGORY)
        .name("R-Squared")
        .description("Coefficient of determination for trend fit")
        .add_constraint(ParamConstraint::period(10, 500, 20))
        .metadata("test_type", "goodness_of_fit")
        .machine_id(BarIndicatorId::RSquared) // TODO: Add to enum
        // Note: "R_SQUARED" is already the main ID, no need for alias
        .alias("RSquared")
        .alias("r_squared")
        .alias("RSQUARED")
        .alias("rsquared")
        .alias("R_Squared")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Statistics indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ADF", signature_adf_proxy as fn() -> IndicatorSignature),
    ("KPSS", signature_kpss_proxy as fn() -> IndicatorSignature),
    ("KPSS_TREND", signature_kpss_trend_proxy as fn() -> IndicatorSignature),
    ("KPSS_Z", signature_kpss_z_proxy as fn() -> IndicatorSignature),
    ("ADF_KPSS", signature_adf_kpss_composite as fn() -> IndicatorSignature),
    ("PP", signature_phillips_perron_proxy as fn() -> IndicatorSignature),
    ("ZA", signature_zivot_andrews_proxy as fn() -> IndicatorSignature),
    ("ARCH_LM", signature_arch_lm_proxy as fn() -> IndicatorSignature),
    ("ARCH_LM_PVAL", signature_arch_lm_pvalue_proxy as fn() -> IndicatorSignature),
    ("LJUNG_BOX", signature_ljung_box as fn() -> IndicatorSignature),
    ("PACF", signature_pacf as fn() -> IndicatorSignature),
    ("EG_COINT", signature_engle_granger_proxy as fn() -> IndicatorSignature),
    ("EG_ADF", signature_engle_granger_adf_proxy as fn() -> IndicatorSignature),
    ("EG_TREND", signature_engle_granger_trend_proxy as fn() -> IndicatorSignature),
    ("COINT", signature_cointegration_proxy as fn() -> IndicatorSignature),
    ("HALF_LIFE_MR", signature_half_life_mr as fn() -> IndicatorSignature),
    ("RESID_STAT", signature_residual_stationarity as fn() -> IndicatorSignature),
    ("VR", signature_variance_ratio as fn() -> IndicatorSignature),
    ("VR_AGG", signature_variance_ratio_aggregate as fn() -> IndicatorSignature),
    ("VR_Z_AGG", signature_variance_ratio_z_aggregate as fn() -> IndicatorSignature),
    ("ST_CUSUM", signature_cusum_break_detector as fn() -> IndicatorSignature),
    ("BP_CUSUM", signature_bai_perron_cusum as fn() -> IndicatorSignature),
    ("PV_COHERENCE", signature_price_volume_coherence_proxy as fn() -> IndicatorSignature),
    ("PRICE_ZSCORE", signature_price_zscore as fn() -> IndicatorSignature),
    ("R_SQUARED", signature_r_squared as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static STATISTICS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    STATISTICS_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count() {
        assert_eq!(count(), 25); // 25 statistics indicators
    }

    #[test]
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }
}
