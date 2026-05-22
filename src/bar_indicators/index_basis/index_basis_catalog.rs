//! index_basis_catalog.rs: Indicator catalog for index/basis indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::IndexBasis;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_basis_extreme() -> IndicatorSignature {
    IndicatorSignature::builder("BASIS_EXTREME", CATEGORY)
        .name("Basis Extreme")
        .description("Detects extreme basis levels (futures-spot spread) signaling potential convergence")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BasisExtreme)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Basis)
        .alias("basis_extreme")
        .alias("BasisExtreme")
        .build()
}

pub fn signature_basis_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("BASIS_MOMENTUM", CATEGORY)
        .name("Basis Momentum")
        .description("Rate of change of the futures-spot basis over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BasisMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Basis)
        .alias("basis_momentum")
        .alias("BasisMomentum")
        .build()
}

pub fn signature_basis_z_score() -> IndicatorSignature {
    IndicatorSignature::builder("BASIS_Z_SCORE", CATEGORY)
        .name("Basis Z-Score")
        .description("Z-score normalization of the futures-spot basis")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BasisZScore)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Basis)
        .alias("basis_z_score")
        .alias("BasisZScore")
        .build()
}

pub fn signature_composite_weight_drift() -> IndicatorSignature {
    IndicatorSignature::builder("COMPOSITE_WEIGHT_DRIFT", CATEGORY)
        .name("Composite Weight Drift")
        .description("Drift in component weights of a composite index over time")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::CompositeWeightDrift)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::CompositeIndex)
        .alias("composite_weight_drift")
        .alias("CompositeWeightDrift")
        .build()
}

pub fn signature_index_component_drift() -> IndicatorSignature {
    IndicatorSignature::builder("INDEX_COMPONENT_DRIFT", CATEGORY)
        .name("Index Component Drift")
        .description("Drift of individual index components relative to the composite index")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::IndexComponentDrift)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::CompositeIndex)
        .alias("index_component_drift")
        .alias("IndexComponentDrift")
        .build()
}

pub fn signature_index_correlation_breakdown() -> IndicatorSignature {
    IndicatorSignature::builder("INDEX_CORRELATION_BREAKDOWN", CATEGORY)
        .name("Index Correlation Breakdown")
        .description("Detects breakdown in correlation between asset and its reference index")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::IndexCorrelationBreakdown)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Basis)
        .alias("index_correlation_breakdown")
        .alias("IndexCorrelationBreakdown")
        .build()
}

pub fn signature_price_vs_index_spread() -> IndicatorSignature {
    IndicatorSignature::builder("PRICE_VS_INDEX_SPREAD", CATEGORY)
        .name("Price vs Index Spread")
        .description("Spread between asset price and reference index price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::PriceVsIndexSpread)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::IndexPrice)
        .alias("price_vs_index_spread")
        .alias("PriceVsIndexSpread")
        .build()
}

pub fn signature_index_price_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("INDEX_PRICE_MOMENTUM", CATEGORY)
        .name("Index Price Momentum")
        .description("EMA-based momentum (slope) of the index/spot price from the mark price feed")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::IndexPriceMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Double)
        .input_stream(StreamKind::IndexPrice)
        .alias("index_price_momentum")
        .alias("IndexPriceMomentum")
        .alias("INDEXPRICEMOMENTUM")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("BASIS_EXTREME", signature_basis_extreme as fn() -> IndicatorSignature),
    ("BASIS_MOMENTUM", signature_basis_momentum as fn() -> IndicatorSignature),
    ("BASIS_Z_SCORE", signature_basis_z_score as fn() -> IndicatorSignature),
    ("COMPOSITE_WEIGHT_DRIFT", signature_composite_weight_drift as fn() -> IndicatorSignature),
    ("INDEX_COMPONENT_DRIFT", signature_index_component_drift as fn() -> IndicatorSignature),
    ("INDEX_CORRELATION_BREAKDOWN", signature_index_correlation_breakdown as fn() -> IndicatorSignature),
    ("PRICE_VS_INDEX_SPREAD", signature_price_vs_index_spread as fn() -> IndicatorSignature),
    ("INDEX_PRICE_MOMENTUM", signature_index_price_momentum as fn() -> IndicatorSignature),
];

pub static INDEX_BASIS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    INDEX_BASIS_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
