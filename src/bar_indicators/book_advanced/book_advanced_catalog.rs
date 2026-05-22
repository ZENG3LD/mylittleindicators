//! book_advanced_catalog.rs: Indicator catalog for advanced order book indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::BookAdvanced;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_best_level_volatility() -> IndicatorSignature {
    IndicatorSignature::builder("BEST_LEVEL_VOLATILITY", CATEGORY)
        .name("Best Level Volatility")
        .description("Volatility of best bid/ask prices over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BestLevelVolatility)
        .role_kind(IndicatorRoleKind::Volatility)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderBook)
        .alias("best_level_volatility")
        .alias("BestLevelVolatility")
        .build()
}

pub fn signature_bid_ask_asymmetry() -> IndicatorSignature {
    IndicatorSignature::builder("BID_ASK_ASYMMETRY", CATEGORY)
        .name("Bid-Ask Asymmetry")
        .description("Asymmetry between bid and ask depth distributions")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BidAskAsymmetry)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderBook)
        .alias("bid_ask_asymmetry")
        .alias("BidAskAsymmetry")
        .build()
}

pub fn signature_bid_ask_bounce_rate() -> IndicatorSignature {
    IndicatorSignature::builder("BID_ASK_BOUNCE_RATE", CATEGORY)
        .name("Bid-Ask Bounce Rate")
        .description("Rate at which mid price bounces between bid and ask levels")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BidAskBounceRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderBook)
        .alias("bid_ask_bounce_rate")
        .alias("BidAskBounceRate")
        .build()
}

pub fn signature_layer_concentration() -> IndicatorSignature {
    IndicatorSignature::builder("LAYER_CONCENTRATION", CATEGORY)
        .name("Layer Concentration")
        .description("Concentration of order book volume in top N layers vs total depth")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LayerConcentration)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderBook)
        .alias("layer_concentration")
        .alias("LayerConcentration")
        .build()
}

pub fn signature_mid_price_velocity() -> IndicatorSignature {
    IndicatorSignature::builder("MID_PRICE_VELOCITY", CATEGORY)
        .name("Mid Price Velocity")
        .description("Rate of change of the mid price between consecutive book snapshots")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MidPriceVelocity)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderBook)
        .alias("mid_price_velocity")
        .alias("MidPriceVelocity")
        .build()
}

pub fn signature_price_level_density() -> IndicatorSignature {
    IndicatorSignature::builder("PRICE_LEVEL_DENSITY", CATEGORY)
        .name("Price Level Density")
        .description("Number of distinct price levels per unit of price range in the book")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::PriceLevelDensity)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OrderBook)
        .alias("price_level_density")
        .alias("PriceLevelDensity")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("BEST_LEVEL_VOLATILITY", signature_best_level_volatility as fn() -> IndicatorSignature),
    ("BID_ASK_ASYMMETRY", signature_bid_ask_asymmetry as fn() -> IndicatorSignature),
    ("BID_ASK_BOUNCE_RATE", signature_bid_ask_bounce_rate as fn() -> IndicatorSignature),
    ("LAYER_CONCENTRATION", signature_layer_concentration as fn() -> IndicatorSignature),
    ("MID_PRICE_VELOCITY", signature_mid_price_velocity as fn() -> IndicatorSignature),
    ("PRICE_LEVEL_DENSITY", signature_price_level_density as fn() -> IndicatorSignature),
];

pub static BOOK_ADVANCED_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    BOOK_ADVANCED_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
