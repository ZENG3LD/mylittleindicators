//! mark_price_advanced_catalog.rs: Indicator catalog for advanced mark price indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::MarkPriceAdvanced;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_mark_price_gap_detector() -> IndicatorSignature {
    IndicatorSignature::builder("MARK_PRICE_GAP_DETECTOR", CATEGORY)
        .name("Mark Price Gap Detector")
        .description("Detects significant gaps between mark price and last traded price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MarkPriceGapDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::MarkPrice)
        .alias("mark_price_gap_detector")
        .alias("MarkPriceGapDetector")
        .build()
}

pub fn signature_mark_price_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("MARK_PRICE_MOMENTUM", CATEGORY)
        .name("Mark Price Momentum")
        .description("Rate of change of mark price over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MarkPriceMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::MarkPrice)
        .alias("mark_price_momentum")
        .alias("MarkPriceMomentum")
        .build()
}

pub fn signature_mark_price_volatility() -> IndicatorSignature {
    IndicatorSignature::builder("MARK_PRICE_VOLATILITY", CATEGORY)
        .name("Mark Price Volatility")
        .description("Rolling standard deviation of mark price returns")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MarkPriceVolatility)
        .role_kind(IndicatorRoleKind::Volatility)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::MarkPrice)
        .alias("mark_price_volatility")
        .alias("MarkPriceVolatility")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("MARK_PRICE_GAP_DETECTOR", signature_mark_price_gap_detector as fn() -> IndicatorSignature),
    ("MARK_PRICE_MOMENTUM", signature_mark_price_momentum as fn() -> IndicatorSignature),
    ("MARK_PRICE_VOLATILITY", signature_mark_price_volatility as fn() -> IndicatorSignature),
];

pub static MARK_PRICE_ADVANCED_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    MARK_PRICE_ADVANCED_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
