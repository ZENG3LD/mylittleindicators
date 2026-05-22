//! open_interest_catalog.rs: Indicator catalog for open interest indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::OpenInterest;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_long_squeeze_detector() -> IndicatorSignature {
    IndicatorSignature::builder("LONG_SQUEEZE_DETECTOR", CATEGORY)
        .name("Long Squeeze Detector")
        .description("Detects long squeeze conditions from open interest + price action")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LongSqueezeDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OpenInterest)
        .alias("long_squeeze_detector")
        .alias("LongSqueezeDetector")
        .build()
}

pub fn signature_oi_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("OI_MOMENTUM", CATEGORY)
        .name("OI Momentum")
        .description("Rate of change of open interest over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::OiMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OpenInterest)
        .alias("oi_momentum")
        .alias("OiMomentum")
        .build()
}

pub fn signature_oi_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("OI_PERCENTILE", CATEGORY)
        .name("OI Percentile")
        .description("Rolling percentile rank of open interest")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::OiPercentile)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OpenInterest)
        .alias("oi_percentile")
        .alias("OiPercentile")
        .build()
}

pub fn signature_oi_price_correlation() -> IndicatorSignature {
    IndicatorSignature::builder("OI_PRICE_CORRELATION", CATEGORY)
        .name("OI Price Correlation")
        .description("Rolling correlation between open interest changes and price changes")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::OiPriceCorrelation)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OpenInterest)
        .alias("oi_price_correlation")
        .alias("OiPriceCorrelation")
        .build()
}

pub fn signature_oi_z_score() -> IndicatorSignature {
    IndicatorSignature::builder("OI_Z_SCORE", CATEGORY)
        .name("OI Z-Score")
        .description("Z-score normalization of open interest over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::OiZScore)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OpenInterest)
        .alias("oi_z_score")
        .alias("OiZScore")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("LONG_SQUEEZE_DETECTOR", signature_long_squeeze_detector as fn() -> IndicatorSignature),
    ("OI_MOMENTUM", signature_oi_momentum as fn() -> IndicatorSignature),
    ("OI_PERCENTILE", signature_oi_percentile as fn() -> IndicatorSignature),
    ("OI_PRICE_CORRELATION", signature_oi_price_correlation as fn() -> IndicatorSignature),
    ("OI_Z_SCORE", signature_oi_z_score as fn() -> IndicatorSignature),
];

pub static OPEN_INTEREST_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    OPEN_INTEREST_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
