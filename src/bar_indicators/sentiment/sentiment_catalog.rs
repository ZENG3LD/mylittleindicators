//! sentiment_catalog.rs: Indicator catalog for sentiment indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::Sentiment;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_agg_trade_flow_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("AGG_TRADE_FLOW_IMBALANCE", CATEGORY)
        .name("Agg Trade Flow Imbalance")
        .description("Buy/sell imbalance from aggregated trade stream")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AggTradeFlowImbalance)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::AggTrade)
        .alias("agg_trade_flow_imbalance")
        .alias("AggTradeFlowImbalance")
        .build()
}

pub fn signature_agg_trade_size_distribution() -> IndicatorSignature {
    IndicatorSignature::builder("AGG_TRADE_SIZE_DISTRIBUTION", CATEGORY)
        .name("Agg Trade Size Distribution")
        .description("Distribution statistics of aggregated trade sizes over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AggTradeSizeDistribution)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::AggTrade)
        .alias("agg_trade_size_distribution")
        .alias("AggTradeSizeDistribution")
        .build()
}

pub fn signature_long_short_extreme_detector() -> IndicatorSignature {
    IndicatorSignature::builder("LONG_SHORT_EXTREME_DETECTOR", CATEGORY)
        .name("Long/Short Extreme Detector")
        .description("Detects extreme long/short ratio levels signaling potential reversals")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LongShortExtremeDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::LongShortRatio)
        .alias("long_short_extreme_detector")
        .alias("LongShortExtremeDetector")
        .build()
}

pub fn signature_long_short_ratio_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("LONG_SHORT_RATIO_MOMENTUM", CATEGORY)
        .name("Long/Short Ratio Momentum")
        .description("Rate of change of the long/short ratio over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LongShortRatioMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::LongShortRatio)
        .alias("long_short_ratio_momentum")
        .alias("LongShortRatioMomentum")
        .build()
}

pub fn signature_ratio_vs_price_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("RATIO_VS_PRICE_DIVERGENCE", CATEGORY)
        .name("Ratio vs Price Divergence")
        .description("Divergence between long/short ratio direction and price direction")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RatioVsPriceDivergence)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::LongShortRatio)
        .alias("ratio_vs_price_divergence")
        .alias("RatioVsPriceDivergence")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("AGG_TRADE_FLOW_IMBALANCE", signature_agg_trade_flow_imbalance as fn() -> IndicatorSignature),
    ("AGG_TRADE_SIZE_DISTRIBUTION", signature_agg_trade_size_distribution as fn() -> IndicatorSignature),
    ("LONG_SHORT_EXTREME_DETECTOR", signature_long_short_extreme_detector as fn() -> IndicatorSignature),
    ("LONG_SHORT_RATIO_MOMENTUM", signature_long_short_ratio_momentum as fn() -> IndicatorSignature),
    ("RATIO_VS_PRICE_DIVERGENCE", signature_ratio_vs_price_divergence as fn() -> IndicatorSignature),
];

pub static SENTIMENT_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    SENTIMENT_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
