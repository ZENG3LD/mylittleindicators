//! trend_stop_catalog.rs: Indicator catalog for trend stop indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 11 trend stop indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::TrendStop;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// ATR Trailing Stop
pub fn signature_atr_trailing_stop() -> IndicatorSignature {
    IndicatorSignature::builder("ATRTS", CATEGORY)
        .name("ATR Trailing Stop")
        .description("Trailing stop levels based on ATR from highest/lowest prices")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::multiplier(0.5, 10.0, 2.0))
        .metadata("type", "trailing")
        .metadata("complexity", "O(1)")
        .machine_id(BarIndicatorId::Atrts) // TODO: Add to enum
        // Note: "ATRTS" is already the main ID, no need for alias
        .alias("Atrts")
        .alias("atrts")
        .alias("ATRTRAILINGSTOP")
        .alias("ATRTrailingStop")
        .alias("atrtrailingstop")
        .alias("atr_trailing_stop")
        .alias("ATR_TRAILING_STOP")
        .alias("Atr_Trailing_Stop")
        .build()
}

/// Chandelier Stop
pub fn signature_chandelier_stop() -> IndicatorSignature {
    IndicatorSignature::builder("CHAND", CATEGORY)
        .name("Chandelier Stop")
        .description("Stop levels based on ATR from highest high / lowest low")
        .add_constraint(ParamConstraint::period(5, 200, 22))
        .add_constraint(ParamConstraint::multiplier(1.0, 10.0, 3.0))
        .metadata("type", "chandelier")
        .metadata("author", "Chuck LeBeau")
        .machine_id(BarIndicatorId::Chand) // TODO: Add to enum
        // Note: "CHAND" is already the main ID, no need for alias
        .alias("Chand")
        .alias("chand")
        .alias("CHANDELIERSTOP")
        .alias("ChandelierStop")
        .alias("chandelierstop")
        .alias("chandelier_stop")
        .alias("CHANDELIER_STOP")
        .alias("Chandelier_Stop")
        .build()
}

/// Chande Kroll Stop
pub fn signature_chande_kroll_stop() -> IndicatorSignature {
    IndicatorSignature::builder("CKS", CATEGORY)
        .name("Chande Kroll Stop")
        .description("ATR-based stop levels variant with separate high/low periods")
        .add_constraint(
            ParamConstraint::new("atr_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("k", ParamType::F64)
                .with_min(ParamValue::F64(0.5))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(1.5))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("hh_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("ll_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .metadata("author", "Tushar Chande and Stanley Kroll")
        .machine_id(BarIndicatorId::Cks) // TODO: Add to enum
        // Note: "CKS" is already the main ID, no need for alias
        .alias("Cks")
        .alias("cks")
        .alias("CHANDEKROLLSTOP")
        .alias("ChandeKrollStop")
        .alias("chandekrollstop")
        .alias("chande_kroll_stop")
        .alias("CHANDE_KROLL_STOP")
        .alias("Chande_Kroll_Stop")
        .build()
}

/// Donchian Stop
pub fn signature_donchian_stop() -> IndicatorSignature {
    IndicatorSignature::builder("DONS", CATEGORY)
        .name("Donchian Stop")
        .description("Stop levels based on Donchian channel boundaries")
        .add_constraint(
            ParamConstraint::new("upper_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("lower_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("offset", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(0.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("use_percentage", ParamType::Bool)
                .with_default(ParamValue::Bool(false))
                .required()
        )
        .metadata("author", "Richard Donchian")
        .machine_id(BarIndicatorId::Dons) // TODO: Add to enum
        // Note: "DONS" is already the main ID, no need for alias
        .alias("Dons")
        .alias("dons")
        .alias("DONCHIANSTOP")
        .alias("DonchianStop")
        .alias("donchianstop")
        .alias("donchian_stop")
        .alias("DONCHIAN_STOP")
        .alias("Donchian_Stop")
        .build()
}

/// Donchian Breakout
pub fn signature_donchian_breakout() -> IndicatorSignature {
    IndicatorSignature::builder("DONBO", CATEGORY)
        .name("Donchian Breakout")
        .description("Breakout signals relative to Donchian Channel")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("type", "breakout_signal")
        .metadata("output", "[-1, 0, 1]")
        .machine_id(BarIndicatorId::Donbo) // TODO: Add to enum
        // Note: "DONBO" is already the main ID, no need for alias
        .alias("Donbo")
        .alias("donbo")
        .alias("DONCHIANBREAKOUT")
        .alias("DonchianBreakout")
        .alias("donchianbreakout")
        .alias("donchian_breakout")
        .alias("DONCHIAN_BREAKOUT")
        .alias("Donchian_Breakout")
        .build()
}

/// Keltner Stop
pub fn signature_keltner_stop() -> IndicatorSignature {
    IndicatorSignature::builder("KELTS", CATEGORY)
        .name("Keltner Stop")
        .description("Stop levels based on Keltner channel bands")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .add_constraint(ParamConstraint::ma_type_named("center_ma", MovingAverageType::EMA))
        .add_constraint(ParamConstraint::ma_type_named("atr_ma", MovingAverageType::RMA))
        .metadata("type", "channel")
        .metadata("author", "Chester W. Keltner")
        .metadata("center_ma_desc", "Center line MA type")
        .metadata("atr_ma_desc", "ATR smoothing MA type")
        .machine_id(BarIndicatorId::Kelts) // TODO: Add to enum
        // Note: "KELTS" is already the main ID, no need for alias
        .alias("Kelts")
        .alias("kelts")
        .alias("KELTNERSTOP")
        .alias("KeltnerStop")
        .alias("keltnerstop")
        .alias("keltner_stop")
        .alias("KELTNER_STOP")
        .alias("Keltner_Stop")
        .build()
}

/// PSAR Stop
pub fn signature_psar_stop() -> IndicatorSignature {
    IndicatorSignature::builder("PSARS", CATEGORY)
        .name("PSAR Stop")
        .description("Stop levels based on Parabolic SAR")
        .add_constraint(
            ParamConstraint::new("af_start", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.1))
                .with_default(ParamValue::F64(0.02))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("af_increment", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.1))
                .with_default(ParamValue::F64(0.02))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("af_max", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.20))
                .required()
        )
        .metadata("author", "J. Welles Wilder")
        .metadata("type", "parabolic")
        .machine_id(BarIndicatorId::Psars) // TODO: Add to enum
        // Note: "PSARS" is already the main ID, no need for alias
        .alias("Psars")
        .alias("psars")
        .alias("PSARSTOP")
        .alias("PSARStop")
        .alias("psarstop")
        .alias("psar_stop")
        .alias("PSAR_STOP")
        .alias("Psar_Stop")
        .build()
}

/// SuperTrend Stop
pub fn signature_supertrend_stop() -> IndicatorSignature {
    IndicatorSignature::builder("SUPTS", CATEGORY)
        .name("SuperTrend Stop")
        .description("Stop levels based on SuperTrend indicator")
        .add_constraint(ParamConstraint::period(5, 200, 10))
        .add_constraint(ParamConstraint::multiplier(1.0, 10.0, 3.0))
        .metadata("type", "trend_following")
        .metadata("complexity", "O(1)")
        .machine_id(BarIndicatorId::Supts) // TODO: Add to enum
        // Note: "SUPTS" is already the main ID, no need for alias
        .alias("Supts")
        .alias("supts")
        .alias("SUPERTRENDSTOP")
        .alias("SuperTrendStop")
        .alias("supertrendstop")
        .alias("supertrend_stop")
        .alias("SUPERTREND_STOP")
        .alias("Supertrend_Stop")
        .build()
}

/// Swing Stop
pub fn signature_swing_stop() -> IndicatorSignature {
    IndicatorSignature::builder("TS_SWINGS", CATEGORY)
        .name("Swing Stop")
        .description("Stop levels based on swing highs and lows")
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("min_swing_size", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(0.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("offset", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(0.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("use_percentage", ParamType::Bool)
                .with_default(ParamValue::Bool(false))
                .required()
        )
        .metadata("type", "swing")
        .machine_id(BarIndicatorId::TsSwings)
        // Note: "TS_SWINGS" is already the main ID, no need for alias
        .alias("TsSwings")
        .alias("ts_swings")
        .alias("SWINGSTOP")
        .alias("SwingStop")
        .alias("swingstop")
        .alias("swing_stop")
        .alias("SWING_STOP")
        .alias("Swing_Stop")
        .build()
}

/// Volatility Stop
pub fn signature_volatility_stop() -> IndicatorSignature {
    IndicatorSignature::builder("VOLTS", CATEGORY)
        .name("Volatility Stop")
        .description("Adaptive stop levels based on various volatility measures")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 10.0, 2.0))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .add_constraint(
            ParamConstraint::new("volatility_type", ParamType::U8)
                .with_min(ParamValue::U8(0))
                .with_max(ParamValue::U8(2))
                .with_default(ParamValue::U8(0))
                .required()
        )
        .metadata("type", "adaptive")
        .metadata("complexity", "O(n) for StdDev, O(1) for ATR/Range")
        .metadata("volatility_types", "0=StandardDeviation, 1=ATR, 2=Range")
        .machine_id(BarIndicatorId::Volts) // TODO: Add to enum
        // Note: "VOLTS" is already the main ID, no need for alias
        .alias("Volts")
        .alias("volts")
        .alias("VOLATILITYSTOP")
        .alias("VolatilityStop")
        .alias("volatilitystop")
        .alias("volatility_stop")
        .alias("VOLATILITY_STOP")
        .alias("Volatility_Stop")
        .build()
}

/// Volatility Stop ATR
pub fn signature_volatility_stop_atr() -> IndicatorSignature {
    IndicatorSignature::builder("VOLTS_ATR", CATEGORY)
        .name("Volatility Stop (ATR)")
        .description("Adaptive stop levels based on ATR volatility")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 10.0, 2.0))
        .metadata("type", "adaptive")
        .metadata("volatility_measure", "ATR")
        .machine_id(BarIndicatorId::VoltsAtr) // TODO: Add to enum
        // Note: "VOLTS_ATR" is already the main ID, no need for alias
        .alias("VoltsAtr")
        .alias("volts_atr")
        .alias("VOLATILITYSTOP(ATR)")
        .alias("VolatilityStop(ATR)")
        .alias("volatilitystop(atr)")
        .alias("volatility_stop_(atr)")
        .alias("VOLATILITY_STOP_(ATR)")
        .alias("Volatility_Stop_(atr)")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all trend stop indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ATRTS", signature_atr_trailing_stop as fn() -> IndicatorSignature),
    ("CHAND", signature_chandelier_stop as fn() -> IndicatorSignature),
    ("CKS", signature_chande_kroll_stop as fn() -> IndicatorSignature),
    ("DONBO", signature_donchian_breakout as fn() -> IndicatorSignature),
    ("DONS", signature_donchian_stop as fn() -> IndicatorSignature),
    ("KELTS", signature_keltner_stop as fn() -> IndicatorSignature),
    ("PSARS", signature_psar_stop as fn() -> IndicatorSignature),
    ("SUPTS", signature_supertrend_stop as fn() -> IndicatorSignature),
    ("TS_SWINGS", signature_swing_stop as fn() -> IndicatorSignature),
    ("VOLTS", signature_volatility_stop as fn() -> IndicatorSignature),
    ("VOLTS_ATR", signature_volatility_stop_atr as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static TREND_STOP_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    TREND_STOP_CATALOG.get(id).map(|f| f())
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
    fn test_get_atr_trailing_stop_signature() {
        let sig = get_signature("ATRTS").unwrap();
        assert_eq!(sig.id, "ATRTS");
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

    #[test]
    fn test_category_consistency() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.category, IndicatorCategory::TrendStop);
        }
    }
}
