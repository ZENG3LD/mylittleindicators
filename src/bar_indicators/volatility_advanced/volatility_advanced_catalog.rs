//! volatility_advanced_catalog.rs: Indicator catalog for advanced volatility indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::VolatilityAdvanced;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_hv_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("HV_MOMENTUM", CATEGORY)
        .name("HV Momentum")
        .description("Rate of change of historical volatility over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::HvMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::HistoricalVolatility)
        .alias("hv_momentum")
        .alias("HvMomentum")
        .build()
}

pub fn signature_hv_spike() -> IndicatorSignature {
    IndicatorSignature::builder("HV_SPIKE", CATEGORY)
        .name("HV Spike")
        .description("Detects sudden spikes in historical volatility relative to recent baseline")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::HvSpike)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::HistoricalVolatility)
        .alias("hv_spike")
        .alias("HvSpike")
        .build()
}

pub fn signature_vol_idx_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("VOL_IDX_MOMENTUM", CATEGORY)
        .name("Vol Index Momentum")
        .description("Rate of change of the volatility index over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::VolIdxMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::VolatilityIndex)
        .alias("vol_idx_momentum")
        .alias("VolIdxMomentum")
        .build()
}

pub fn signature_vol_idx_spike() -> IndicatorSignature {
    IndicatorSignature::builder("VOL_IDX_SPIKE", CATEGORY)
        .name("Vol Index Spike")
        .description("Detects sudden spikes in the volatility index signaling fear events")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::VolIdxSpike)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::VolatilityIndex)
        .alias("vol_idx_spike")
        .alias("VolIdxSpike")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("HV_MOMENTUM", signature_hv_momentum as fn() -> IndicatorSignature),
    ("HV_SPIKE", signature_hv_spike as fn() -> IndicatorSignature),
    ("VOL_IDX_MOMENTUM", signature_vol_idx_momentum as fn() -> IndicatorSignature),
    ("VOL_IDX_SPIKE", signature_vol_idx_spike as fn() -> IndicatorSignature),
];

pub static VOLATILITY_ADVANCED_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    VOLATILITY_ADVANCED_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
