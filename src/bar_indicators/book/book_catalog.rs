//! book_catalog.rs: Indicator catalog for order book indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 2 order book indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Book;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Book Imbalance Ratio
pub fn signature_book_imbalance_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_IMB", CATEGORY)
        .name("Book Imbalance Ratio")
        .description("Ratio of best bid to best ask sizes in order book")
        .metadata("range", "0-1")
        .metadata("parameters", "none")
        .metadata("requirements", "order_book_data")
        .metadata("interpretation", "Closer to 0 = ask pressure, closer to 1 = bid pressure")
        .machine_id(BarIndicatorId::BookImb)
        .role_kind(IndicatorRoleKind::Volume)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        // Note: "BOOK_IMB" is already the main ID, no need for alias
        .alias("BookImb")
        .alias("book_imb")
        .alias("BOOKIMBALANCERATIO")
        .alias("BookImbalanceRatio")
        .alias("bookimbalanceratio")
        .alias("book_imbalance_ratio")
        .alias("BOOK_IMBALANCE_RATIO")
        .alias("Book_Imbalance_Ratio")
        .build()
}

/// Microprice — bid-ask size-weighted mid price
pub fn signature_microprice() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_MICROPRICE", CATEGORY)
        .name("Microprice")
        .description("Bid-ask size-weighted mid price (better next-trade price predictor than simple mid)")
        .metadata("range", "positive real (price units)")
        .metadata("parameters", "none")
        .metadata("requirements", "L2 orderbook top-of-book")
        .metadata("interpretation", "Weighted toward side with smaller quote; pulls toward pressure side")
        .machine_id(BarIndicatorId::BookMicroprice)
        .role_kind(IndicatorRoleKind::Smoother)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .alias("Microprice")
        .alias("microprice")
        .alias("MICROPRICE")
        .alias("book_microprice")
        .alias("BookMicroprice")
        .build()
}

/// Order Book Slope
pub fn signature_order_book_slope() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_SLOPE", CATEGORY)
        .name("Order Book Slope")
        .description("Slope proxy using normalized volume vs price spread")
        .metadata("calculation", "ln(volume) / (high - low)")
        .metadata("category", "depth_proxy")
        .metadata("interpretation", "Higher values indicate steeper order book")
        .machine_id(BarIndicatorId::BookSlope)
        .role_kind(IndicatorRoleKind::Volume)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        // Note: "BOOK_SLOPE" is already the main ID, no need for alias
        .alias("BookSlope")
        .alias("book_slope")
        .alias("ORDERBOOKSLOPE")
        .alias("OrderBookSlope")
        .alias("orderbookslope")
        .alias("order_book_slope")
        .alias("ORDER_BOOK_SLOPE")
        .alias("Order_Book_Slope")
        .build()
}

/// Order Flow Imbalance
pub fn signature_order_flow_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("OFI", CATEGORY)
        .name("Order Flow Imbalance")
        .description("Analyzes buy/sell imbalance across price levels")
        .add_constraint(
            ParamConstraint::new("period", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("tick_size", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("outputs", "total_imbalance, avg_imbalance, dominant_side, strength")
        .metadata("requirements", "volume_bars_with_buy_sell_split")
        .metadata("interpretation", "Positive = buy pressure, negative = sell pressure")
        .machine_id(BarIndicatorId::Ofi)
        .role_kind(IndicatorRoleKind::Volume)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        // Note: "OFI" is already the main ID, no need for alias
        .alias("Ofi")
        .alias("ofi")
        .alias("ORDERFLOWIMBALANCE")
        .alias("OrderFlowImbalance")
        .alias("orderflowimbalance")
        .alias("order_flow_imbalance")
        .alias("ORDER_FLOW_IMBALANCE")
        .alias("Order_Flow_Imbalance")
        .build()
}

/// Liquidity Sweep — detects large orders consuming multiple price levels
pub fn signature_liquidity_sweep() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_LIQUIDITY_SWEEP", CATEGORY)
        .name("Liquidity Sweep")
        .description("Detects large orders sweeping ask (buy sweep) or bid (sell sweep) levels")
        .metadata("outputs", "direction (+1/-1/0), magnitude (price distance swept)")
        .metadata("requirements", "L2 orderbook consecutive snapshots")
        .metadata("interpretation", "+1 = buy sweep, -1 = sell sweep, 0 = no sweep")
        .machine_id(BarIndicatorId::LiquiditySweep)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Double)
        .requires_l2()
        .alias("LiquiditySweep")
        .alias("liquidity_sweep")
        .alias("LIQUIDITYSWEEP")
        .alias("book_liquidity_sweep")
        .build()
}

/// Book Pressure — slope momentum of bid/ask depth changes
pub fn signature_book_pressure() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_PRESSURE", CATEGORY)
        .name("Book Pressure")
        .description("Slope momentum of bid vs ask depth over rolling window: bid_slope - ask_slope")
        .add_constraint(
            ParamConstraint::new("period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("levels", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .metadata("outputs", "pressure (positive = bullish, negative = bearish)")
        .metadata("requirements", "L2 orderbook snapshots")
        .metadata("interpretation", "Positive = bid pressure growing faster; negative = ask growing faster")
        .machine_id(BarIndicatorId::BookPressure)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .alias("BookPressure")
        .alias("book_pressure")
        .alias("BOOKPRESSURE")
        .build()
}

/// Spread Distribution — rolling percentile rank of bid-ask spread
pub fn signature_spread_distribution() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_SPREAD_DIST", CATEGORY)
        .name("Spread Distribution")
        .description("Rolling percentile rank of bid-ask spread (100=tightest, 0=widest)")
        .add_constraint(
            ParamConstraint::new("period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(500))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("outputs", "spread, percentile (0-100)")
        .metadata("requirements", "L2 orderbook top-of-book")
        .metadata("interpretation", "100 = tightest historical spread; 0 = widest")
        .machine_id(BarIndicatorId::SpreadDistribution)
        .role_kind(IndicatorRoleKind::OscillatorBounded)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Double)
        .requires_l2()
        .alias("SpreadDistribution")
        .alias("spread_distribution")
        .alias("SPREADDISTRIBUTION")
        .alias("book_spread_dist")
        .build()
}

/// Order Book Velocity — rolling average rate of orderbook level changes per snapshot
pub fn signature_order_book_velocity() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_OBV", CATEGORY)
        .name("Order Book Velocity")
        .description("Rolling average number of orderbook level changes per snapshot")
        .add_constraint(
            ParamConstraint::new("period", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .metadata("outputs", "avg_changes_per_snapshot")
        .metadata("requirements", "L2 orderbook consecutive snapshots")
        .metadata("interpretation", "Higher = more active orderbook; lower = stable market")
        .machine_id(BarIndicatorId::OrderBookVelocity)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .alias("OrderBookVelocity")
        .alias("order_book_velocity")
        .alias("ORDERBOOKVELOCITY")
        .alias("book_obv")
        .build()
}

/// Queue Imbalance
pub fn signature_queue_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("QUEUE_IMB", CATEGORY)
        .name("Queue Imbalance")
        .description("Level-1 imbalance proxy from OHLCV: (close - mid) / range")
        .metadata("calculation", "(close - (high+low)/2) / (high-low)")
        .metadata("range", "-1 to +1")
        .metadata("category", "level1_proxy")
        .metadata("interpretation", "Positive = buying pressure, negative = selling pressure")
        .machine_id(BarIndicatorId::QueueImb)
        .role_kind(IndicatorRoleKind::Volume)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        // Note: "QUEUE_IMB" is already the main ID, no need for alias
        .alias("QueueImb")
        .alias("queue_imb")
        .alias("QUEUEIMBALANCE")
        .alias("QueueImbalance")
        .alias("queueimbalance")
        .alias("queue_imbalance")
        .alias("QUEUE_IMBALANCE")
        .alias("Queue_Imbalance")
        .build()
}

/// Wall Detector — percentile-based large size level detector
pub fn signature_wall_detector() -> IndicatorSignature {
    IndicatorSignature::builder("WALL_DETECTOR", CATEGORY)
        .name("Wall Detector")
        .description("Detects anomalously large bid/ask levels (walls) using rolling percentile threshold")
        .add_constraint(ParamConstraint::period(10, 2000, 200))
        .add_constraint(
            ParamConstraint::new("percentile_threshold", ParamType::F64)
                .with_min(ParamValue::F64(50.0))
                .with_max(ParamValue::F64(99.9))
                .with_default(ParamValue::F64(95.0))
        )
        .add_constraint(
            ParamConstraint::new("levels_to_sample", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(50.0))
                .with_default(ParamValue::F64(20.0))
        )
        .metadata("outputs", "bid_wall_price, ask_wall_price, total_wall_size")
        .metadata("requirements", "L2 orderbook snapshots")
        .machine_id(BarIndicatorId::WallDetector)
        .role_kind(IndicatorRoleKind::Level)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Triple)
        .requires_l2()
        .alias("WallDetector")
        .alias("wall_detector")
        .alias("WALLDETECTOR")
        .build()
}

/// Book Depth Change — delta of aggregated bid/ask depth between snapshots
pub fn signature_book_depth_change() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_DEPTH_CHANGE", CATEGORY)
        .name("Book Depth Change")
        .description("Delta of total bid and ask depth between consecutive orderbook snapshots")
        .add_constraint(ParamConstraint::period(1, 100, 10))
        .metadata("outputs", "bid_depth_change, ask_depth_change")
        .metadata("requirements", "L2 orderbook consecutive snapshots")
        .metadata("interpretation", "Positive bid_change = book deepening on bid side")
        .machine_id(BarIndicatorId::BookDepthChange)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Double)
        .requires_l2()
        .alias("BookDepthChange")
        .alias("book_depth_change")
        .alias("BOOKDEPTHCHANGE")
        .build()
}

/// Hidden Liquidity Detector — detects hidden/iceberg liquidity via trade-vs-book mismatch
pub fn signature_hidden_liquidity_detector() -> IndicatorSignature {
    IndicatorSignature::builder("HIDDEN_LIQUIDITY_DETECTOR", CATEGORY)
        .name("Hidden Liquidity Detector")
        .description("Detects hidden (iceberg) liquidity by comparing trade size vs visible book size at the traded price level")
        .add_constraint(
            ParamConstraint::new("price_bucket", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(1000.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("window", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(1000))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("outputs", "side (+1/-1/0), last_hidden_vol, cumulative_hidden_vol")
        .metadata("requirements", "synchronized tick + L2 orderbook")
        .metadata("interpretation", "+1 = buy aggressor hit hidden ask, -1 = sell hit hidden bid")
        .machine_id(BarIndicatorId::HiddenLiquidityDetector)
        .role_kind(IndicatorRoleKind::Pattern)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Triple)
        .requires_l2()
        .alias("HiddenLiquidityDetector")
        .alias("hidden_liquidity_detector")
        .alias("HIDDENLIQUIDITY")
        .build()
}

/// Trade Book Absorption — detects absorption using real top-of-book state
pub fn signature_trade_book_absorption() -> IndicatorSignature {
    IndicatorSignature::builder("TRADE_BOOK_ABSORPTION", CATEGORY)
        .name("Trade Book Absorption")
        .description("Detects absorption at best bid/ask: trade size exceeds visible top-of-book yet price stays at level")
        .add_constraint(ParamConstraint::period(1, 1000, 50))
        .metadata("outputs", "side (+1/-1/0), last_absorbed_vol, cumulative_absorbed_vol")
        .metadata("requirements", "synchronized tick + L2 orderbook")
        .metadata("interpretation", "+1 = buy absorbed at ask, -1 = sell absorbed at bid")
        .machine_id(BarIndicatorId::TradeBookAbsorption)
        .role_kind(IndicatorRoleKind::Pattern)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Triple)
        .requires_l2()
        .alias("TradeBookAbsorption")
        .alias("trade_book_absorption")
        .alias("TRADEBOOKABSORPTION")
        .build()
}

/// Iceberg Detector — detects hidden iceberg orders via level replenishment patterns
pub fn signature_iceberg_detector() -> IndicatorSignature {
    IndicatorSignature::builder("ICEBERG_DETECTOR", CATEGORY)
        .name("Iceberg Detector")
        .description("Detects hidden iceberg orders by tracking level replenishment events in delta updates")
        .add_constraint(
            ParamConstraint::new("price_bucket", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(10000.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("replenishment_threshold", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(1000))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .metadata("outputs", "side (+1/-1/0), price, replenishment_count")
        .metadata("requirements", "L2 orderbook delta stream")
        .machine_id(BarIndicatorId::IcebergDetector)
        .role_kind(IndicatorRoleKind::Pattern)
        .input_stream(StreamKind::OrderbookDelta)
        .output_kind(IndicatorValueKind::Triple)
        .requires_l2()
        .alias("IcebergDetector")
        .alias("iceberg_detector")
        .alias("ICEBERGDETECTOR")
        .build()
}

/// Level Replenish Rate — rolling rate of positive-size orderbook updates per second
pub fn signature_level_replenish_rate() -> IndicatorSignature {
    IndicatorSignature::builder("LEVEL_REPLENISH_RATE", CATEGORY)
        .name("Level Replenish Rate")
        .description("Rolling rate of orderbook level replenishments (positive-size updates) in events/second")
        .add_constraint(
            ParamConstraint::new("rolling_window", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(10000))
                .with_default(ParamValue::USize(200))
                .required()
        )
        .metadata("outputs", "events_per_second")
        .metadata("requirements", "L2 orderbook delta stream")
        .machine_id(BarIndicatorId::LevelReplenishRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .input_stream(StreamKind::OrderbookDelta)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .alias("LevelReplenishRate")
        .alias("level_replenish_rate")
        .alias("LEVELREPLENISHRATE")
        .alias("LevelReplenishmentRate")
        .alias("level_replenishment_rate")
        .build()
}

/// Book Churn Rate — rolling average number of level changes per delta
pub fn signature_book_churn_rate() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_CHURN_RATE", CATEGORY)
        .name("Book Churn Rate")
        .description("Rolling average of total bid/ask level changes per delta update")
        .add_constraint(
            ParamConstraint::new("rolling_window", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(10000))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("outputs", "avg_changes_per_delta")
        .metadata("requirements", "L2 orderbook delta stream")
        .machine_id(BarIndicatorId::BookChurnRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .input_stream(StreamKind::OrderbookDelta)
        .output_kind(IndicatorValueKind::Single)
        .requires_l2()
        .alias("BookChurnRate")
        .alias("book_churn_rate")
        .alias("BOOKCHURNRATE")
        .build()
}

/// Sweep Impact Analyzer — measures how many book levels a trade consumed
pub fn signature_sweep_impact_analyzer() -> IndicatorSignature {
    IndicatorSignature::builder("SWEEP_IMPACT_ANALYZER", CATEGORY)
        .name("Sweep Impact Analyzer")
        .description("Measures how many price levels a trade swept and the resulting slippage (effective price impact)")
        .add_constraint(ParamConstraint::period(1, 1000, 50))
        .metadata("outputs", "side (+1/-1/0), levels_swept, slippage")
        .metadata("requirements", "synchronized tick + L2 orderbook")
        .metadata("interpretation", "levels_swept ≥ 2 = real sweep; slippage = price distance from best to last level")
        .machine_id(BarIndicatorId::SweepImpactAnalyzer)
        .role_kind(IndicatorRoleKind::Pattern)
        .input_stream(StreamKind::OrderBook)
        .output_kind(IndicatorValueKind::Triple)
        .requires_l2()
        .alias("SweepImpactAnalyzer")
        .alias("sweep_impact_analyzer")
        .alias("SWEEPIMPACT")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all book indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("BOOK_IMB", signature_book_imbalance_ratio as fn() -> IndicatorSignature),
    ("BOOK_MICROPRICE", signature_microprice as fn() -> IndicatorSignature),
    ("BOOK_SLOPE", signature_order_book_slope as fn() -> IndicatorSignature),
    ("OFI", signature_order_flow_imbalance as fn() -> IndicatorSignature),
    ("QUEUE_IMB", signature_queue_imbalance as fn() -> IndicatorSignature),
    ("BOOK_LIQUIDITY_SWEEP", signature_liquidity_sweep as fn() -> IndicatorSignature),
    ("BOOK_PRESSURE", signature_book_pressure as fn() -> IndicatorSignature),
    ("BOOK_SPREAD_DIST", signature_spread_distribution as fn() -> IndicatorSignature),
    ("BOOK_OBV", signature_order_book_velocity as fn() -> IndicatorSignature),
    ("WALL_DETECTOR", signature_wall_detector as fn() -> IndicatorSignature),
    ("BOOK_DEPTH_CHANGE", signature_book_depth_change as fn() -> IndicatorSignature),
    ("HIDDEN_LIQUIDITY_DETECTOR", signature_hidden_liquidity_detector as fn() -> IndicatorSignature),
    ("TRADE_BOOK_ABSORPTION", signature_trade_book_absorption as fn() -> IndicatorSignature),
    ("SWEEP_IMPACT_ANALYZER", signature_sweep_impact_analyzer as fn() -> IndicatorSignature),
    ("ICEBERG_DETECTOR", signature_iceberg_detector as fn() -> IndicatorSignature),
    ("LEVEL_REPLENISH_RATE", signature_level_replenish_rate as fn() -> IndicatorSignature),
    ("BOOK_CHURN_RATE", signature_book_churn_rate as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static BOOK_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    BOOK_CATALOG.get(id).map(|f| f())
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
    fn test_get_book_imbalance_signature() {
        let sig = get_signature("BOOK_IMB").unwrap();
        assert_eq!(sig.id, "BOOK_IMB");
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
        assert_eq!(count(), 17);
    }

    #[test]
    fn test_order_flow_imbalance_params() {
        let sig = get_signature("OFI").unwrap();
        assert_eq!(sig.constraints.constraints.len(), 2);
        assert!(sig.constraints.constraints.iter().any(|c| c.name == "period"));
        assert!(sig.constraints.constraints.iter().any(|c| c.name == "tick_size"));
    }
}
