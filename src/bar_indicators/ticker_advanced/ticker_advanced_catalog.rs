//! ticker_advanced_catalog.rs: Indicator catalog for advanced ticker indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::TickerAdvanced;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_ticker_spread_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("TICKER_SPREAD_RATIO", CATEGORY)
        .name("Ticker Spread Ratio")
        .description("Ratio of bid-ask spread to mid price from 24h ticker data")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::TickerSpreadRatio)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Ticker)
        .alias("ticker_spread_ratio")
        .alias("TickerSpreadRatio")
        .build()
}

pub fn signature_volume_24h_z_score() -> IndicatorSignature {
    IndicatorSignature::builder("VOLUME_24H_Z_SCORE", CATEGORY)
        .name("Volume 24h Z-Score")
        .description("Z-score normalization of 24h trading volume")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Volume24hZScore)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Ticker)
        .alias("volume_24h_z_score")
        .alias("Volume24hZScore")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("TICKER_SPREAD_RATIO", signature_ticker_spread_ratio as fn() -> IndicatorSignature),
    ("VOLUME_24H_Z_SCORE", signature_volume_24h_z_score as fn() -> IndicatorSignature),
];

pub static TICKER_ADVANCED_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    TICKER_ADVANCED_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
