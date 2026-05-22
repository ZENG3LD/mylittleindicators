//! funding_advanced_catalog.rs: Indicator catalog for advanced funding rate indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::FundingAdvanced;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_annualized_funding_rate() -> IndicatorSignature {
    IndicatorSignature::builder("ANNUALIZED_FUNDING_RATE", CATEGORY)
        .name("Annualized Funding Rate")
        .description("Converts periodic funding rate to annualized percentage")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AnnualizedFundingRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("annualized_funding_rate")
        .alias("AnnualizedFundingRate")
        .build()
}

pub fn signature_funding_direction_shift() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_DIRECTION_SHIFT", CATEGORY)
        .name("Funding Direction Shift")
        .description("Detects sign changes in funding rate direction")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingDirectionShift)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("funding_direction_shift")
        .alias("FundingDirectionShift")
        .build()
}

pub fn signature_funding_extreme_alert() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_EXTREME_ALERT", CATEGORY)
        .name("Funding Extreme Alert")
        .description("Fires when funding rate exceeds extreme threshold levels")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingExtremeAlert)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("funding_extreme_alert")
        .alias("FundingExtremeAlert")
        .build()
}

pub fn signature_funding_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_MOMENTUM", CATEGORY)
        .name("Funding Momentum")
        .description("EMA-smoothed funding rate with slope direction")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Double)
        .input_stream(StreamKind::Funding)
        .alias("funding_momentum")
        .alias("FundingMomentum")
        .build()
}

pub fn signature_funding_z_score() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_ZSCORE", CATEGORY)
        .name("Funding Z-Score")
        .description("Rolling Z-score of funding rate vs window mean and standard deviation")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingZScore)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("funding_z_score")
        .alias("FundingZScore")
        .alias("FUNDING_ZSCORE")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ANNUALIZED_FUNDING_RATE", signature_annualized_funding_rate as fn() -> IndicatorSignature),
    ("FUNDING_DIRECTION_SHIFT", signature_funding_direction_shift as fn() -> IndicatorSignature),
    ("FUNDING_EXTREME_ALERT", signature_funding_extreme_alert as fn() -> IndicatorSignature),
    ("FUNDING_MOMENTUM", signature_funding_momentum as fn() -> IndicatorSignature),
    ("FUNDING_ZSCORE", signature_funding_z_score as fn() -> IndicatorSignature),
];

pub static FUNDING_ADVANCED_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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

pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    FUNDING_ADVANCED_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
