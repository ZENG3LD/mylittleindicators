//! average_catalog.rs: Catalog of all Average/Moving Average indicators
//!
//! Auto-generated catalog based on actual indicator implementations.
//! Contains IndicatorSignature definitions for 22 moving average indicators.
//!
//! Note: Legacy OHLCV-specific MA indicators have been REMOVED.
//! For OHLCV field selection, use MovingAverageWithField wrapper:
//!   MovingAverageWithField::new(MovingAverageType::SMA, 20, OhlcvField::High)

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue, SourceType,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Average;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Simple Moving Average - arithmetic mean over N periods
pub fn signature_sma() -> IndicatorSignature {
    IndicatorSignature::builder("SMA", CATEGORY)
        .name("Simple Moving Average")
        .description("Arithmetic mean of prices over N periods")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("complexity", "O(1) with ring buffer")
        .metadata("author", "Classic TA")
        .machine_id(BarIndicatorId::Sma)
        // Note: "SMA" is already the main ID, no need for alias
        .alias("Sma")
        .alias("sma")
        .alias("SIMPLEMOVINGAVERAGE")
        .alias("SimpleMovingAverage")
        .alias("simplemovingaverage")
        .alias("simple_moving_average")
        .alias("SIMPLE_MOVING_AVERAGE")
        .alias("Simple_Moving_Average")
        .build()
}

/// Exponential Moving Average - weighted MA giving more weight to recent prices
pub fn signature_ema() -> IndicatorSignature {
    IndicatorSignature::builder("EMA", CATEGORY)
        .name("Exponential Moving Average")
        .description("Weighted moving average with exponential decay")
        .add_constraint(ParamConstraint::period(2, 200, 12))
        .metadata("complexity", "O(1)")
        .metadata("author", "Classic TA")
        .machine_id(BarIndicatorId::Ema)
        // Note: "EMA" is already the main ID, no need for alias
        .alias("Ema")
        .alias("ema")
        .alias("EXPONENTIALMOVINGAVERAGE")
        .alias("ExponentialMovingAverage")
        .alias("exponentialmovingaverage")
        .alias("exponential_moving_average")
        .alias("EXPONENTIAL_MOVING_AVERAGE")
        .alias("Exponential_Moving_Average")
        .build()
}

/// Weighted Moving Average - linear weights from 1 to N
pub fn signature_wma() -> IndicatorSignature {
    IndicatorSignature::builder("WMA", CATEGORY)
        .name("Weighted Moving Average")
        .description("Moving average with linear weights")
        .add_constraint(ParamConstraint::period(2, 256, 20))
        .metadata("max_period", "256")
        .machine_id(BarIndicatorId::Wma)  // Canonical name for optimized WMA
        // Note: "WMA" is already the main ID, no need for alias
        .alias("Wma")
        .alias("wma")
        .alias("WEIGHTEDMOVINGAVERAGE")
        .alias("WeightedMovingAverage")
        .alias("weightedmovingaverage")
        .alias("weighted_moving_average")
        .alias("WEIGHTED_MOVING_AVERAGE")
        .alias("Weighted_Moving_Average")
        .build()
}

/// Hull Moving Average - fast and smooth MA by Alan Hull
pub fn signature_hma() -> IndicatorSignature {
    IndicatorSignature::builder("HMA", CATEGORY)
        .name("Hull Moving Average")
        .description("Fast and smooth moving average using WMAs")
        .add_constraint(ParamConstraint::period(2, 200, 16))
        .metadata("author", "Alan Hull")
        .metadata("formula", "WMA(2*WMA(n/2) - WMA(n), sqrt(n))")
        .machine_id(BarIndicatorId::Hma)  // Canonical name for optimized HMA
        // Note: "HMA" is already the main ID, no need for alias
        .alias("Hma")
        .alias("hma")
        .alias("HULLMOVINGAVERAGE")
        .alias("HullMovingAverage")
        .alias("hullmovingaverage")
        .alias("hull_moving_average")
        .alias("HULL_MOVING_AVERAGE")
        .alias("Hull_Moving_Average")
        .build()
}

/// Double Exponential Moving Average
pub fn signature_dema() -> IndicatorSignature {
    IndicatorSignature::builder("DEMA", CATEGORY)
        .name("Double Exponential Moving Average")
        .description("Double smoothed EMA for reduced lag")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("formula", "2*EMA - EMA(EMA)")
        .machine_id(BarIndicatorId::Dema)
        // Note: "DEMA" is already the main ID, no need for alias
        .alias("Dema")
        .alias("dema")
        .alias("DOUBLEEXPONENTIALMOVINGAVERAGE")
        .alias("DoubleExponentialMovingAverage")
        .alias("doubleexponentialmovingaverage")
        .alias("double_exponential_moving_average")
        .alias("DOUBLE_EXPONENTIAL_MOVING_AVERAGE")
        .alias("Double_Exponential_Moving_Average")
        .build()
}

/// Triple Exponential Moving Average
pub fn signature_tema() -> IndicatorSignature {
    IndicatorSignature::builder("TEMA", CATEGORY)
        .name("Triple Exponential Moving Average")
        .description("Triple smoothed EMA for minimal lag")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("formula", "3*EMA - 3*EMA(EMA) + EMA(EMA(EMA))")
        .machine_id(BarIndicatorId::Tema)
        // Note: "TEMA" is already the main ID, no need for alias
        .alias("Tema")
        .alias("tema")
        .alias("TRIPLEEXPONENTIALMOVINGAVERAGE")
        .alias("TripleExponentialMovingAverage")
        .alias("tripleexponentialmovingaverage")
        .alias("triple_exponential_moving_average")
        .alias("TRIPLE_EXPONENTIAL_MOVING_AVERAGE")
        .alias("Triple_Exponential_Moving_Average")
        .build()
}

/// Running Moving Average (Wilder's smoothing)
pub fn signature_rma() -> IndicatorSignature {
    IndicatorSignature::builder("RMA", CATEGORY)
        .name("Running Moving Average")
        .description("Wilder's smoothing method used in RSI")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "J. Welles Wilder")
        .metadata("aka", "Wilder's Moving Average")
        .machine_id(BarIndicatorId::Rma)
        // Note: "RMA" is already the main ID, no need for alias
        .alias("Rma")
        .alias("rma")
        .alias("RUNNINGMOVINGAVERAGE")
        .alias("RunningMovingAverage")
        .alias("runningmovingaverage")
        .alias("running_moving_average")
        .alias("RUNNING_MOVING_AVERAGE")
        .alias("Running_Moving_Average")
        .build()
}

/// Volume Weighted Average Price
pub fn signature_vwap() -> IndicatorSignature {
    IndicatorSignature::builder("VWAP", CATEGORY)
        .name("Volume Weighted Average Price")
        .description("Average price weighted by volume")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("uses_volume", "true")
        .metadata("price_type", "typical (HLC/3)")
        .source_type(SourceType::PriceAndVolume)
        .machine_id(BarIndicatorId::Vwap)
        // Note: "VWAP" is already the main ID, no need for alias
        .alias("Vwap")
        .alias("vwap")
        .alias("VOLUMEWEIGHTEDAVERAGEPRICE")
        .alias("VolumeWeightedAveragePrice")
        .alias("volumeweightedaverageprice")
        .alias("volume_weighted_average_price")
        .alias("VOLUME_WEIGHTED_AVERAGE_PRICE")
        .alias("Volume_Weighted_Average_Price")
        .build()
}

/// Triangular Moving Average
pub fn signature_tma() -> IndicatorSignature {
    IndicatorSignature::builder("TMA", CATEGORY)
        .name("Triangular Moving Average")
        .description("Double-smoothed simple moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("formula", "SMA of SMA")
        .machine_id(BarIndicatorId::Tma)
        // Note: "TMA" is already the main ID, no need for alias
        .alias("Tma")
        .alias("tma")
        .alias("TRIANGULARMOVINGAVERAGE")
        .alias("TriangularMovingAverage")
        .alias("triangularmovingaverage")
        .alias("triangular_moving_average")
        .alias("TRIANGULAR_MOVING_AVERAGE")
        .alias("Triangular_Moving_Average")
        .build()
}

/// Adaptive Moving Average (Kaufman's)
pub fn signature_ama() -> IndicatorSignature {
    IndicatorSignature::builder("AMA", CATEGORY)
        .name("Adaptive Moving Average")
        .description("Kaufman's Adaptive Moving Average based on efficiency ratio")
        .add_constraint(
            ParamConstraint::new("period_efficiency_ratio", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(10))
                .required()
        )
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
        .metadata("author", "Perry Kaufman")
        .metadata("complexity", "Uses Efficiency Ratio")
        .machine_id(BarIndicatorId::Ama)  // Canonical name for optimized AMA
        // Note: "AMA" is already the main ID, no need for alias
        .alias("Ama")
        .alias("ama")
        .alias("ADAPTIVEMOVINGAVERAGE")
        .alias("AdaptiveMovingAverage")
        .alias("adaptivemovingaverage")
        .alias("adaptive_moving_average")
        .alias("ADAPTIVE_MOVING_AVERAGE")
        .alias("Adaptive_Moving_Average")
        .build()
}

/// Fractal Adaptive Moving Average
pub fn signature_frama() -> IndicatorSignature {
    IndicatorSignature::builder("AV_FRAMA", CATEGORY)
        .name("Fractal Adaptive Moving Average")
        .description("Adaptive MA based on fractal dimension")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::AvFrama)
        // Note: "AV_FRAMA" is already the main ID, no need for alias
        .alias("AvFrama")
        .alias("av_frama")
        .alias("FRACTALADAPTIVEMOVINGAVERAGE")
        .alias("FractalAdaptiveMovingAverage")
        .alias("fractaladaptivemovingaverage")
        .alias("fractal_adaptive_moving_average")
        .alias("FRACTAL_ADAPTIVE_MOVING_AVERAGE")
        .alias("Fractal_Adaptive_Moving_Average")
        .build()
}

/// Fractal Adaptive Moving Average (Advanced version)
pub fn signature_frama_advanced() -> IndicatorSignature {
    IndicatorSignature::builder("FRAMAADV", CATEGORY)
        .name("Fractal Adaptive Moving Average Advanced")
        .description("Enhanced FRAMA with additional smoothing")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John Ehlers")
        .metadata("variant", "advanced")
        .machine_id(BarIndicatorId::Framaadv) // TODO: Add to enum
        // Note: "FRAMAADV" is already the main ID, no need for alias
        .alias("Framaadv")
        .alias("framaadv")
        .alias("FRACTALADAPTIVEMOVINGAVERAGEADVANCED")
        .alias("FractalAdaptiveMovingAverageAdvanced")
        .alias("fractaladaptivemovingaverageadvanced")
        .alias("fractal_adaptive_moving_average_advanced")
        .alias("FRACTAL_ADAPTIVE_MOVING_AVERAGE_ADVANCED")
        .alias("Fractal_Adaptive_Moving_Average_Advanced")
        .build()
}

/// Linear Regression Moving Average
pub fn signature_lr() -> IndicatorSignature {
    IndicatorSignature::builder("LR", CATEGORY)
        .name("Linear Regression")
        .description("Linear regression line over N periods")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("aka", "Linear Regression Line")
        .machine_id(BarIndicatorId::Lr) // TODO: Add to enum
        // Note: "LR" is already the main ID, no need for alias
        .alias("Lr")
        .alias("lr")
        .alias("LINEARREGRESSION")
        .alias("LinearRegression")
        .alias("linearregression")
        .alias("linear_regression")
        .alias("LINEAR_REGRESSION")
        .alias("Linear_Regression")
        .build()
}

/// Ehlers Fractal Adaptive Moving Average
pub fn signature_ehlers_fractal_adaptive_ma() -> IndicatorSignature {
    IndicatorSignature::builder("EHLERSFA", CATEGORY)
        .name("Ehlers Fractal Adaptive MA")
        .description("Ehlers' implementation of fractal adaptive smoothing")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::Ehlersfa) // TODO: Add to enum
        // Note: "EHLERSFA" is already the main ID, no need for alias
        .alias("Ehlersfa")
        .alias("ehlersfa")
        .alias("EHLERSFRACTALADAPTIVEMA")
        .alias("EhlersFractalAdaptiveMA")
        .alias("ehlersfractaladaptivema")
        .alias("ehlers_fractal_adaptive_ma")
        .alias("EHLERS_FRACTAL_ADAPTIVE_MA")
        .alias("Ehlers_Fractal_Adaptive_Ma")
        .build()
}

/// Ehlers Zero Lag EMA
pub fn signature_ehlers_zero_lag_ema() -> IndicatorSignature {
    IndicatorSignature::builder("EHLERSZ", CATEGORY)
        .name("Ehlers Zero Lag EMA")
        .description("Zero-lag exponential moving average by Ehlers")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John Ehlers")
        .metadata("feature", "reduced lag")
        .machine_id(BarIndicatorId::Ehlersz) // TODO: Add to enum
        // Note: "EHLERSZ" is already the main ID, no need for alias
        .alias("Ehlersz")
        .alias("ehlersz")
        .alias("EHLERSZEROLAGEMA")
        .alias("EhlersZeroLagEMA")
        .alias("ehlerszerolagema")
        .alias("ehlers_zero_lag_ema")
        .alias("EHLERS_ZERO_LAG_EMA")
        .alias("Ehlers_Zero_Lag_Ema")
        .build()
}

/// Arnaud Legoux Moving Average
pub fn signature_alma() -> IndicatorSignature {
    IndicatorSignature::builder("ALMA", CATEGORY)
        .name("Arnaud Legoux Moving Average")
        .description("Gaussian-weighted moving average")
        .add_constraint(ParamConstraint::period(2, 200, 9))
        .add_constraint(
            ParamConstraint::new("offset", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.85))
        )
        .add_constraint(
            ParamConstraint::new("sigma", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(6.0))
        )
        .metadata("author", "Arnaud Legoux")
        .machine_id(BarIndicatorId::Alma)
        // Note: "ALMA" is already the main ID, no need for alias
        .alias("Alma")
        .alias("alma")
        .alias("ARNAUDLEGOUXMOVINGAVERAGE")
        .alias("ArnaudLegouxMovingAverage")
        .alias("arnaudlegouxmovingaverage")
        .alias("arnaud_legoux_moving_average")
        .alias("ARNAUD_LEGOUX_MOVING_AVERAGE")
        .alias("Arnaud_Legoux_Moving_Average")
        .build()
}

/// Jurik Moving Average
pub fn signature_jurik_ma() -> IndicatorSignature {
    IndicatorSignature::builder("JMA", CATEGORY)
        .name("Jurik Moving Average")
        .description("Low-lag adaptive moving average")
        .add_constraint(ParamConstraint::period(2, 200, 7))
        .add_constraint(
            ParamConstraint::new("phase", ParamType::F64)
                .with_min(ParamValue::F64(-100.0))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(0.0))
        )
        .metadata("author", "Mark Jurik")
        .machine_id(BarIndicatorId::Jma) // TODO: Add to enum
        // Note: "JMA" is already the main ID, no need for alias
        .alias("Jma")
        .alias("jma")
        .alias("JURIKMOVINGAVERAGE")
        .alias("JurikMovingAverage")
        .alias("jurikmovingaverage")
        .alias("jurik_moving_average")
        .alias("JURIK_MOVING_AVERAGE")
        .alias("Jurik_Moving_Average")
        .build()
}

/// McGinley Dynamic
pub fn signature_mcginley_dynamic() -> IndicatorSignature {
    IndicatorSignature::builder("MCGINLEY", CATEGORY)
        .name("McGinley Dynamic")
        .description("Automatically adjusting moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John McGinley")
        .metadata("feature", "auto-adjusting")
        .machine_id(BarIndicatorId::Mcginley) // TODO: Add to enum
        // Note: "MCGINLEY" is already the main ID, no need for alias
        .alias("Mcginley")
        .alias("mcginley")
        .alias("MCGINLEYDYNAMIC")
        .alias("McGinleyDynamic")
        .alias("mcginleydynamic")
        .alias("mcginley_dynamic")
        .alias("MCGINLEY_DYNAMIC")
        .alias("Mcginley_Dynamic")
        .build()
}

/// T3 Moving Average (Tillson)
pub fn signature_t3() -> IndicatorSignature {
    IndicatorSignature::builder("T3", CATEGORY)
        .name("T3 Moving Average")
        .description("Tillson's T3 - smooth and responsive")
        .add_constraint(ParamConstraint::period(2, 200, 5))
        .add_constraint(
            ParamConstraint::new("volume_factor", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.7))
        )
        .metadata("author", "Tim Tillson")
        .machine_id(BarIndicatorId::T3) // TODO: Add to enum
        // Note: "T3" is already the main ID, no need for alias
        .alias("t3")
        .alias("T3MOVINGAVERAGE")
        .alias("T3MovingAverage")
        .alias("t3movingaverage")
        .alias("t3_moving_average")
        .alias("T3_MOVING_AVERAGE")
        .alias("T3_Moving_Average")
        .build()
}

/// Triple Moving Average
pub fn signature_trima() -> IndicatorSignature {
    IndicatorSignature::builder("TRIMA", CATEGORY)
        .name("Triple Moving Average")
        .description("Triangular/Triple smoothed moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .machine_id(BarIndicatorId::Trima)
        // Note: "TRIMA" is already the main ID, no need for alias
        .alias("Trima")
        .alias("trima")
        .alias("TRIPLEMOVINGAVERAGE")
        .alias("TripleMovingAverage")
        .alias("triplemovingaverage")
        .alias("triple_moving_average")
        .alias("TRIPLE_MOVING_AVERAGE")
        .alias("Triple_Moving_Average")
        .build()
}

/// Variable Index Dynamic Average
pub fn signature_vidya() -> IndicatorSignature {
    IndicatorSignature::builder("AV_VIDYA", CATEGORY)
        .name("Variable Index Dynamic Average")
        .description("Volatility-adjusted moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("author", "Tushar Chande")
        .metadata("feature", "volatility-adaptive")
        .metadata("note", "VIDYA uses CMO for volatility and smooths with configurable MA type")
        .machine_id(BarIndicatorId::AvVidya)
        // Note: "AV_VIDYA" is already the main ID, no need for alias
        .alias("AvVidya")
        .alias("av_vidya")
        .alias("VARIABLEINDEXDYNAMICAVERAGE")
        .alias("VariableIndexDynamicAverage")
        .alias("variableindexdynamicaverage")
        .alias("variable_index_dynamic_average")
        .alias("VARIABLE_INDEX_DYNAMIC_AVERAGE")
        .alias("Variable_Index_Dynamic_Average")
        .build()
}

/// Volume Weighted Moving Average
pub fn signature_vwma() -> IndicatorSignature {
    IndicatorSignature::builder("VWMA", CATEGORY)
        .name("Volume Weighted Moving Average")
        .description("SMA weighted by volume")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("uses_volume", "true")
        .source_type(SourceType::PriceAndVolume)
        .machine_id(BarIndicatorId::Vwma)
        // Note: "VWMA" is already the main ID, no need for alias
        .alias("Vwma")
        .alias("vwma")
        .alias("VOLUMEWEIGHTEDMOVINGAVERAGE")
        .alias("VolumeWeightedMovingAverage")
        .alias("volumeweightedmovingaverage")
        .alias("volume_weighted_moving_average")
        .alias("VOLUME_WEIGHTED_MOVING_AVERAGE")
        .alias("Volume_Weighted_Moving_Average")
        .build()
}

// ============================================================================
// Catalog HashMap with Auto-Generated Aliases
// ============================================================================

/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("SMA", signature_sma as fn() -> IndicatorSignature),
    ("EMA", signature_ema as fn() -> IndicatorSignature),
    ("WMA", signature_wma as fn() -> IndicatorSignature),
    ("HMA", signature_hma as fn() -> IndicatorSignature),
    ("DEMA", signature_dema as fn() -> IndicatorSignature),
    ("TEMA", signature_tema as fn() -> IndicatorSignature),
    ("RMA", signature_rma as fn() -> IndicatorSignature),
    ("VWAP", signature_vwap as fn() -> IndicatorSignature),
    ("TMA", signature_tma as fn() -> IndicatorSignature),
    ("AMA", signature_ama as fn() -> IndicatorSignature),
    ("AV_FRAMA", signature_frama as fn() -> IndicatorSignature),
    ("FRAMAADV", signature_frama_advanced as fn() -> IndicatorSignature),
    ("LR", signature_lr as fn() -> IndicatorSignature),
    ("EHLERSFA", signature_ehlers_fractal_adaptive_ma as fn() -> IndicatorSignature),
    ("EHLERSZ", signature_ehlers_zero_lag_ema as fn() -> IndicatorSignature),
    ("ALMA", signature_alma as fn() -> IndicatorSignature),
    ("JMA", signature_jurik_ma as fn() -> IndicatorSignature),
    ("MCGINLEY", signature_mcginley_dynamic as fn() -> IndicatorSignature),
    ("T3", signature_t3 as fn() -> IndicatorSignature),
    ("TRIMA", signature_trima as fn() -> IndicatorSignature),
    ("AV_VIDYA", signature_vidya as fn() -> IndicatorSignature),
    ("VWMA", signature_vwma as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static AVERAGE_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
/// use zengeld_chart_indicators::bar_indicators::average::average_catalog;
///
/// let sig = average_catalog::get_signature("SMA").unwrap();
/// assert_eq!(sig.id, "SMA");
/// ```
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    AVERAGE_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
/// Returns only main IDs, not aliases
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
/// Returns count of unique indicators, not including aliases
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
    fn test_get_sma_signature() {
        let sig = get_signature("SMA").unwrap();
        assert_eq!(sig.id, "SMA");
        assert_eq!(sig.category, CATEGORY);
        assert_eq!(sig.required_params().len(), 1);
    }

    #[test]
    fn test_get_ema_signature() {
        let sig = get_signature("EMA").unwrap();
        assert_eq!(sig.id, "EMA");
        assert_eq!(sig.name, "Exponential Moving Average");
    }

    #[test]
    fn test_get_ama_signature() {
        let sig = get_signature("AMA").unwrap();
        assert_eq!(sig.id, "AMA");
        // AMA has 3 required parameters
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
        assert_eq!(count(), 22); // 22 average indicators (3 LEGACY removed)
    }

    #[test]
    fn test_simple_ma_validation() {
        let sig = get_signature("SMA").unwrap();

        // Valid params
        let params = vec![("period", ParamValue::USize(20))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("SMA").unwrap();
        let params = vec![("period", ParamValue::USize(20))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "SMA_20");
    }

    #[test]
    fn test_ama_cache_key() {
        let sig = get_signature("AMA").unwrap();
        let params = vec![
            ("period_efficiency_ratio", ParamValue::USize(10)),
            ("fast_period", ParamValue::USize(2)),
            ("slow_period", ParamValue::USize(30)),
        ];
        let key = sig.cache_key(&params);
        // Keys are sorted alphabetically
        assert!(key.contains("AMA"));
        assert!(key.contains("10"));
        assert!(key.contains("2"));
        assert!(key.contains("30"));
    }
}
