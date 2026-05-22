//! microstructure_catalog.rs: Indicator catalog for microstructure indicators (block trade, L3)

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::Microstructure;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_block_trade_flow() -> IndicatorSignature {
    IndicatorSignature::builder("BLOCK_TRADE_FLOW", CATEGORY)
        .name("Block Trade Flow")
        .description("Directional flow of block trades (large institutional prints)")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BlockTradeFlow)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::BlockTrade)
        .alias("block_trade_flow")
        .alias("BlockTradeFlow")
        .build()
}

pub fn signature_block_trade_impact() -> IndicatorSignature {
    IndicatorSignature::builder("BLOCK_TRADE_IMPACT", CATEGORY)
        .name("Block Trade Impact")
        .description("Price impact measurement of block trade executions")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BlockTradeImpact)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::BlockTrade)
        .alias("block_trade_impact")
        .alias("BlockTradeImpact")
        .build()
}

pub fn signature_block_trade_size_anomaly() -> IndicatorSignature {
    IndicatorSignature::builder("BLOCK_TRADE_SIZE_ANOMALY", CATEGORY)
        .name("Block Trade Size Anomaly")
        .description("Z-score of block trade size relative to recent baseline")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BlockTradeSizeAnomaly)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::BlockTrade)
        .alias("block_trade_size_anomaly")
        .alias("BlockTradeSizeAnomaly")
        .build()
}

pub fn signature_l3_cancel_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("L3_CANCEL_RATIO", CATEGORY)
        .name("L3 Cancel Ratio")
        .description("Ratio of cancelled orders to placed orders from L3 order book data")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::L3CancelRatio)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderbookL3)
        .alias("l3_cancel_ratio")
        .alias("L3CancelRatio")
        .build()
}

pub fn signature_l3_large_order_tracker() -> IndicatorSignature {
    IndicatorSignature::builder("L3_LARGE_ORDER_TRACKER", CATEGORY)
        .name("L3 Large Order Tracker")
        .description("Tracks placement and cancellation of large individual orders")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::L3LargeOrderTracker)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderbookL3)
        .alias("l3_large_order_tracker")
        .alias("L3LargeOrderTracker")
        .build()
}

pub fn signature_l3_order_rate() -> IndicatorSignature {
    IndicatorSignature::builder("L3_ORDER_RATE", CATEGORY)
        .name("L3 Order Rate")
        .description("Rate of order placements, modifications, and cancellations from L3 data")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::L3OrderRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderbookL3)
        .alias("l3_order_rate")
        .alias("L3OrderRate")
        .build()
}

pub fn signature_l3_spoofer_score() -> IndicatorSignature {
    IndicatorSignature::builder("L3_SPOOFER_SCORE", CATEGORY)
        .name("L3 Spoofer Score")
        .description("Spoofing likelihood score from L3 order placement/cancellation patterns")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::L3SpooferScore)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderbookL3)
        .alias("l3_spoofer_score")
        .alias("L3SpooferScore")
        .build()
}

pub fn signature_quote_lifecycle_tracker() -> IndicatorSignature {
    IndicatorSignature::builder("QUOTE_LIFECYCLE_TRACKER", CATEGORY)
        .name("Quote Lifecycle Tracker")
        .description("Tracks average lifetime of quotes from placement to fill or cancel")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::QuoteLifecycleTracker)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderbookL3)
        .alias("quote_lifecycle_tracker")
        .alias("QuoteLifecycleTracker")
        .build()
}

pub fn signature_quote_stuffing_detector() -> IndicatorSignature {
    IndicatorSignature::builder("QUOTE_STUFFING_DETECTOR", CATEGORY)
        .name("Quote Stuffing Detector")
        .description("Detects quote stuffing via abnormal order submission rate patterns")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::QuoteStuffingDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderbookL3)
        .alias("quote_stuffing_detector")
        .alias("QuoteStuffingDetector")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("BLOCK_TRADE_FLOW", signature_block_trade_flow as fn() -> IndicatorSignature),
    ("BLOCK_TRADE_IMPACT", signature_block_trade_impact as fn() -> IndicatorSignature),
    ("BLOCK_TRADE_SIZE_ANOMALY", signature_block_trade_size_anomaly as fn() -> IndicatorSignature),
    ("L3_CANCEL_RATIO", signature_l3_cancel_ratio as fn() -> IndicatorSignature),
    ("L3_LARGE_ORDER_TRACKER", signature_l3_large_order_tracker as fn() -> IndicatorSignature),
    ("L3_ORDER_RATE", signature_l3_order_rate as fn() -> IndicatorSignature),
    ("L3_SPOOFER_SCORE", signature_l3_spoofer_score as fn() -> IndicatorSignature),
    ("QUOTE_LIFECYCLE_TRACKER", signature_quote_lifecycle_tracker as fn() -> IndicatorSignature),
    ("QUOTE_STUFFING_DETECTOR", signature_quote_stuffing_detector as fn() -> IndicatorSignature),
];

pub static MICROSTRUCTURE_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    MICROSTRUCTURE_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
