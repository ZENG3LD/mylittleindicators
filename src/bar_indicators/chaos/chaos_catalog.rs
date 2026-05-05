//! chaos_catalog.rs: Auto-generated indicator catalog for chaos indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 9 chaos indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Chaos;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Chaos Oscillator
pub fn signature_chaos_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("CHAOS_OSC", CATEGORY)
        .name("Chaos Oscillator")
        .description("Combined chaos indicator: fractal dimension, Hurst exponent, and volatility")
        .add_constraint(ParamConstraint::period(20, 512, 100))
        .add_constraint(
            ParamConstraint::new("complexity_weight", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.4))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("persistence_weight", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.4))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("volatility_weight", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.2))
                .required()
        )
        .metadata("range", "0.0-1.0")
        .metadata("interpretation", "0.0 = ordered market, 1.0 = maximum chaos")
        .machine_id(BarIndicatorId::ChaosOsc) // TODO: Add to enum
        // Note: "CHAOS_OSC" is already the main ID, no need for alias
        .alias("ChaosOsc")
        .alias("chaos_osc")
        .alias("CHAOSOSCILLATOR")
        .alias("ChaosOscillator")
        .alias("chaososcillator")
        .alias("chaos_oscillator")
        .alias("CHAOS_OSCILLATOR")
        .alias("Chaos_Oscillator")
        .build()
}

/// Detrended Fluctuation Analysis (DFA)
pub fn signature_dfa() -> IndicatorSignature {
    IndicatorSignature::builder("DFA", CATEGORY)
        .name("Detrended Fluctuation Analysis")
        .description("Measures long-range correlations in time series via scaling exponent alpha")
        .add_constraint(
            ParamConstraint::new("scale1", ParamType::USize)
                .with_min(ParamValue::USize(8))
                .with_max(ParamValue::USize(32))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("scale2", ParamType::USize)
                .with_min(ParamValue::USize(16))
                .with_max(ParamValue::USize(64))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("scale3", ParamType::USize)
                .with_min(ParamValue::USize(32))
                .with_max(ParamValue::USize(128))
                .with_default(ParamValue::USize(40))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("scale4", ParamType::USize)
                .with_min(ParamValue::USize(64))
                .with_max(ParamValue::USize(256))
                .with_default(ParamValue::USize(80))
                .required()
        )
        .metadata("range", "0.0-2.0")
        .metadata("interpretation", "alpha < 0.5 = anti-persistent, 0.5 = random walk, > 0.5 = persistent")
        .machine_id(BarIndicatorId::Dfa)
        // Note: "DFA" is already the main ID, no need for alias
        .alias("Dfa")
        .alias("dfa")
        .alias("DETRENDEDFLUCTUATIONANALYSIS")
        .alias("DetrendedFluctuationAnalysis")
        .alias("detrendedfluctuationanalysis")
        .alias("detrended_fluctuation_analysis")
        .alias("DETRENDED_FLUCTUATION_ANALYSIS")
        .alias("Detrended_Fluctuation_Analysis")
        .build()
}

/// DFA Percentile
pub fn signature_dfa_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("DFA_PCT", CATEGORY)
        .name("DFA Percentile")
        .description("Normalized percentile rank of DFA alpha value over rolling window")
        .add_constraint(ParamConstraint::period(20, 200, 50))
        .add_constraint(
            ParamConstraint::new("scale1", ParamType::USize)
                .with_min(ParamValue::USize(8))
                .with_max(ParamValue::USize(32))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("scale2", ParamType::USize)
                .with_min(ParamValue::USize(16))
                .with_max(ParamValue::USize(64))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("scale3", ParamType::USize)
                .with_min(ParamValue::USize(32))
                .with_max(ParamValue::USize(128))
                .with_default(ParamValue::USize(40))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("scale4", ParamType::USize)
                .with_min(ParamValue::USize(64))
                .with_max(ParamValue::USize(256))
                .with_default(ParamValue::USize(80))
                .required()
        )
        .metadata("range", "0.0-1.0")
        .metadata("interpretation", "Percentile rank of current DFA alpha in recent history")
        .machine_id(BarIndicatorId::DfaPct) // TODO: Add to enum
        // Note: "DFA_PCT" is already the main ID, no need for alias
        .alias("DfaPct")
        .alias("dfa_pct")
        .alias("DFAPERCENTILE")
        .alias("DFAPercentile")
        .alias("dfapercentile")
        .alias("dfa_percentile")
        .alias("DFA_PERCENTILE")
        .alias("Dfa_Percentile")
        .build()
}

/// Fractal Dimension (Higuchi method)
pub fn signature_fractal_dimension() -> IndicatorSignature {
    IndicatorSignature::builder("FRACTAL_DIM", CATEGORY)
        .name("Fractal Dimension")
        .description("Higuchi fractal dimension: measures complexity and roughness of price series")
        .add_constraint(ParamConstraint::period(20, 512, 100))
        .add_constraint(
            ParamConstraint::new("max_k", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(12))
                .required()
        )
        .metadata("range", "1.0-2.0")
        .metadata("interpretation", "1.0 = strong trend, 1.5 = random walk, 2.0 = maximum noise")
        .machine_id(BarIndicatorId::FractalDim) // TODO: Add to enum
        // Note: "FRACTAL_DIM" is already the main ID, no need for alias
        .alias("FractalDim")
        .alias("fractal_dim")
        .alias("FRACTALDIMENSION")
        .alias("FractalDimension")
        .alias("fractaldimension")
        .alias("fractal_dimension")
        .alias("FRACTAL_DIMENSION")
        .alias("Fractal_Dimension")
        .build()
}

/// Hurst Exponent
pub fn signature_hurst_exponent() -> IndicatorSignature {
    IndicatorSignature::builder("HURST", CATEGORY)
        .name("Hurst Exponent")
        .description("R/S analysis: determines market persistence and mean reversion tendency")
        .add_constraint(ParamConstraint::period(20, 512, 100))
        .metadata("range", "0.0-1.0")
        .metadata("interpretation", "< 0.5 = mean reversion, 0.5 = random walk, > 0.5 = trending")
        .machine_id(BarIndicatorId::Hurst) // TODO: Add to enum
        // Note: "HURST" is already the main ID, no need for alias
        .alias("Hurst")
        .alias("hurst")
        .alias("HURSTEXPONENT")
        .alias("HurstExponent")
        .alias("hurstexponent")
        .alias("hurst_exponent")
        .alias("HURST_EXPONENT")
        .alias("Hurst_Exponent")
        .build()
}

/// Hurst Percentile
pub fn signature_hurst_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("HURST_PCT", CATEGORY)
        .name("Hurst Percentile")
        .description("Normalized percentile rank of Hurst exponent over rolling window")
        .add_constraint(ParamConstraint::period(20, 200, 50))
        .add_constraint(
            ParamConstraint::new("hurst_period", ParamType::USize)
                .with_min(ParamValue::USize(20))
                .with_max(ParamValue::USize(512))
                .with_default(ParamValue::USize(100))
                .required()
        )
        .metadata("range", "0.0-1.0")
        .metadata("interpretation", "Percentile rank of current Hurst exponent in recent history")
        .machine_id(BarIndicatorId::HurstPct) // TODO: Add to enum
        // Note: "HURST_PCT" is already the main ID, no need for alias
        .alias("HurstPct")
        .alias("hurst_pct")
        .alias("HURSTPERCENTILE")
        .alias("HurstPercentile")
        .alias("hurstpercentile")
        .alias("hurst_percentile")
        .alias("HURST_PERCENTILE")
        .alias("Hurst_Percentile")
        .build()
}

/// Williams Alligator
pub fn signature_williams_alligator() -> IndicatorSignature {
    IndicatorSignature::builder("ALLIGATOR", CATEGORY)
        .name("Williams Alligator")
        .description("Three smoothed moving averages (jaw, teeth, lips) for trend identification")
        .metadata("jaw_period", "13")
        .metadata("teeth_period", "8")
        .metadata("lips_period", "5")
        .metadata("interpretation", "Aligned lines = trend, intertwined = range")
        .machine_id(BarIndicatorId::Alligator)
        // Note: "ALLIGATOR" is already the main ID, no need for alias
        .alias("Alligator")
        .alias("alligator")
        .alias("WILLIAMSALLIGATOR")
        .alias("WilliamsAlligator")
        .alias("williamsalligator")
        .alias("williams_alligator")
        .alias("WILLIAMS_ALLIGATOR")
        .alias("Williams_Alligator")
        .build()
}

/// Williams Awesome Oscillator
pub fn signature_williams_awesome_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("AO", CATEGORY)
        .name("Awesome Oscillator")
        .description("Difference between 5-period and 34-period SMA of median prices")
        .metadata("fast_period", "5")
        .metadata("slow_period", "34")
        .metadata("range", "unbounded")
        .metadata("interpretation", "Positive = bullish momentum, negative = bearish momentum")
        .machine_id(BarIndicatorId::Ao) // TODO: Add to enum
        // Note: "AO" is already the main ID, no need for alias
        .alias("Ao")
        .alias("ao")
        .alias("AWESOMEOSCILLATOR")
        .alias("AwesomeOscillator")
        .alias("awesomeoscillator")
        .alias("awesome_oscillator")
        .alias("AWESOME_OSCILLATOR")
        .alias("Awesome_Oscillator")
        .build()
}

/// Williams Acceleration/Deceleration
pub fn signature_williams_ac() -> IndicatorSignature {
    IndicatorSignature::builder("AC", CATEGORY)
        .name("Acceleration/Deceleration")
        .description("Difference between AO and its 5-period SMA: measures momentum acceleration")
        .metadata("ao_smooth_period", "5")
        .metadata("range", "unbounded")
        .metadata("interpretation", "Positive = acceleration, negative = deceleration")
        .machine_id(BarIndicatorId::Ac) // TODO: Add to enum
        // Note: "AC" is already the main ID, no need for alias
        .alias("Ac")
        .alias("ac")
        .alias("ACCELERATIONDECELERATION")
        .alias("AccelerationDeceleration")
        .alias("accelerationdeceleration")
        .alias("acceleration_deceleration")
        .alias("ACCELERATION_DECELERATION")
        .alias("Acceleration_Deceleration")
        .build()
}

/// Williams Market Facilitation Index
pub fn signature_williams_mfi() -> IndicatorSignature {
    IndicatorSignature::builder("WILLIAMS_MFI", CATEGORY)
        .name("Williams Market Facilitation Index")
        .description("Measures price movement efficiency per unit of volume: (H-L)/Volume")
        .metadata("range", "0.0+")
        .metadata("interpretation", "Higher = more efficient price movement per volume")
        .metadata("bar_types", "Green=acceleration, Fade=deceleration, Fake=false breakout, Squat=accumulation")
        .machine_id(BarIndicatorId::WilliamsMfi) // TODO: Add to enum
        // Note: "WILLIAMS_MFI" is already the main ID, no need for alias
        .alias("WilliamsMfi")
        .alias("williams_mfi")
        .alias("WILLIAMSMARKETFACILITATIONINDEX")
        .alias("WilliamsMarketFacilitationIndex")
        .alias("williamsmarketfacilitationindex")
        .alias("williams_market_facilitation_index")
        .alias("WILLIAMS_MARKET_FACILITATION_INDEX")
        .alias("Williams_Market_Facilitation_Index")
        .build()
}

/// Williams Fractals
pub fn signature_williams_fractals() -> IndicatorSignature {
    IndicatorSignature::builder("FRACTALS", CATEGORY)
        .name("Williams Fractals")
        .description("5-bar pattern detector: identifies local highs and lows for reversal points")
        .metadata("lookback", "2")
        .metadata("pattern_bars", "5")
        .metadata("interpretation", "Up fractal = potential resistance, down fractal = potential support")
        .machine_id(BarIndicatorId::Fractals) // TODO: Add to enum
        // Note: "FRACTALS" is already the main ID, no need for alias
        .alias("Fractals")
        .alias("fractals")
        .alias("WILLIAMSFRACTALS")
        .alias("WilliamsFractals")
        .alias("williamsfractals")
        .alias("williams_fractals")
        .alias("WILLIAMS_FRACTALS")
        .alias("Williams_Fractals")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all chaos indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("CHAOS_OSC", signature_chaos_oscillator as fn() -> IndicatorSignature),
    ("DFA", signature_dfa as fn() -> IndicatorSignature),
    ("DFA_PCT", signature_dfa_percentile as fn() -> IndicatorSignature),
    ("FRACTAL_DIM", signature_fractal_dimension as fn() -> IndicatorSignature),
    ("HURST", signature_hurst_exponent as fn() -> IndicatorSignature),
    ("HURST_PCT", signature_hurst_percentile as fn() -> IndicatorSignature),
    ("ALLIGATOR", signature_williams_alligator as fn() -> IndicatorSignature),
    ("AO", signature_williams_awesome_oscillator as fn() -> IndicatorSignature),
    ("AC", signature_williams_ac as fn() -> IndicatorSignature),
    ("WILLIAMS_MFI", signature_williams_mfi as fn() -> IndicatorSignature),
    ("FRACTALS", signature_williams_fractals as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static CHAOS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    CHAOS_CATALOG.get(id).map(|f| f())
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
    fn test_get_hurst_signature() {
        let sig = get_signature("HURST").unwrap();
        assert_eq!(sig.id, "HURST");
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
        assert_eq!(count(), 11);
    }
}
