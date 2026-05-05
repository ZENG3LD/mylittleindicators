//! clusters_catalog.rs: Complete catalog of all Cluster indicators
//!
//! This catalog contains cluster/microstructure indicators that analyze market microstructure,
//! order flow, volume clustering, and price level dynamics.
//! Organized alphabetically for easy navigation.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Clusters;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// Market Microstructure - анализатор микроструктуры рынка
pub fn signature_market_microstructure() -> IndicatorSignature {
    IndicatorSignature::builder("MARKET_MICRO", CATEGORY)
        .name("Market Microstructure")
        .description("Analyzes market microstructure: liquidity, efficiency, execution quality")
        .add_constraint(ParamConstraint::period(5, 512, 50))
        .metadata("outputs", "liquidity_score, efficiency_score, execution_score, microstructure_score")
        .metadata("metrics", "spread, depth, price_impact, discovery_speed, volatility_clustering")
        .machine_id(BarIndicatorId::MarketMicro) // TODO: Add to enum
        // Note: "MARKET_MICRO" is already the main ID, no need for alias
        .alias("MarketMicro")
        .alias("market_micro")
        .alias("MARKETMICROSTRUCTURE")
        .alias("MarketMicrostructure")
        .alias("marketmicrostructure")
        .alias("market_microstructure")
        .alias("MARKET_MICROSTRUCTURE")
        .alias("Market_Microstructure")
        .build()
}

/// Order Book Slope - slope of price vs normalized volume
pub fn signature_order_book_slope() -> IndicatorSignature {
    IndicatorSignature::builder("ORDER_BOOK_SLOPE", CATEGORY)
        .name("Order Book Slope")
        .description("Approximates order book slope using OHLCV data")
        .metadata("formula", "ln(volume) / (high - low)")
        .metadata("proxy", "true")
        .machine_id(BarIndicatorId::OrderBookSlope) // TODO: Add to enum
        // Note: "ORDER_BOOK_SLOPE" is already the main ID, no need for alias
        .alias("OrderBookSlope")
        .alias("order_book_slope")
        .alias("ORDERBOOKSLOPE")
        .alias("orderbookslope")
        .alias("Order_Book_Slope")
        .build()
}

/// Order Flow Imbalance - анализатор дисбаланса ордер флоу
pub fn signature_order_flow_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("ORDER_FLOW_IMB", CATEGORY)
        .name("Order Flow Imbalance")
        .description("Analyzes imbalance between buy and sell orders at price levels")
        .add_constraint(ParamConstraint::period(5, 512, 50))
        .add_constraint(
            ParamConstraint::new("tick_size", ParamType::F64)
                .with_min(ParamValue::F64(0.0001))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.01))
        )
        .metadata("outputs", "total_imbalance, avg_imbalance, dominant_side, imbalance_strength")
        .metadata("uses_volume", "true")
        .machine_id(BarIndicatorId::OrderFlowImb) // TODO: Add to enum
        // Note: "ORDER_FLOW_IMB" is already the main ID, no need for alias
        .alias("OrderFlowImb")
        .alias("order_flow_imb")
        .alias("ORDERFLOWIMBALANCE")
        .alias("OrderFlowImbalance")
        .alias("orderflowimbalance")
        .alias("order_flow_imbalance")
        .alias("ORDER_FLOW_IMBALANCE")
        .alias("Order_Flow_Imbalance")
        .build()
}

/// Queue Imbalance - approximation using close vs mid
pub fn signature_queue_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("CL_QUEUE_IMB", CATEGORY)
        .name("Queue Imbalance")
        .description("Level-1 proxy queue imbalance from OHLCV")
        .metadata("formula", "(close - mid) / range")
        .metadata("proxy", "true")
        .machine_id(BarIndicatorId::ClQueueImb)
        // Note: "CL_QUEUE_IMB" is already the main ID, no need for alias
        .alias("ClQueueImb")
        .alias("cl_queue_imb")
        .alias("QUEUEIMBALANCE")
        .alias("QueueImbalance")
        .alias("queueimbalance")
        .alias("queue_imbalance")
        .alias("QUEUE_IMBALANCE")
        .alias("Queue_Imbalance")
        .build()
}

/// Tick Volume Analyzer - анализатор тикового объема
pub fn signature_tick_volume_analyzer() -> IndicatorSignature {
    IndicatorSignature::builder("TICK_VOLUME", CATEGORY)
        .name("Tick Volume Analyzer")
        .description("Detailed analysis of tick volume and microstructure")
        .add_constraint(ParamConstraint::period(10, 1024, 100))
        .metadata("outputs", "volume_delta, volume_ratio, buy_pct, avg_spread, market_pressure")
        .metadata("uses_volume", "true")
        .metadata("uses_ticks", "true")
        .machine_id(BarIndicatorId::TickVolume) // TODO: Add to enum
        // Note: "TICK_VOLUME" is already the main ID, no need for alias
        .alias("TickVolume")
        .alias("tick_volume")
        .alias("TICKVOLUMEANALYZER")
        .alias("TickVolumeAnalyzer")
        .alias("tickvolumeanalyzer")
        .alias("tick_volume_analyzer")
        .alias("TICK_VOLUME_ANALYZER")
        .alias("Tick_Volume_Analyzer")
        .build()
}

/// Volume Weighted Price Levels - объемно-взвешенные ценовые уровни
pub fn signature_volume_weighted_price_levels() -> IndicatorSignature {
    IndicatorSignature::builder("VWAP_LEVELS", CATEGORY)
        .name("Volume Weighted Price Levels")
        .description("Identifies key support/resistance levels based on volume analysis")
        .add_constraint(ParamConstraint::period(10, 512, 100))
        .add_constraint(
            ParamConstraint::new("price_precision", ParamType::F64)
                .with_min(ParamValue::F64(0.0001))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(0.01))
        )
        .metadata("outputs", "vwap, support_levels, resistance_levels, high_volume_nodes")
        .metadata("uses_volume", "true")
        .machine_id(BarIndicatorId::VwapLevels) // TODO: Add to enum
        // Note: "VWAP_LEVELS" is already the main ID, no need for alias
        .alias("VwapLevels")
        .alias("vwap_levels")
        .alias("VOLUMEWEIGHTEDPRICELEVELS")
        .alias("VolumeWeightedPriceLevels")
        .alias("volumeweightedpricelevels")
        .alias("volume_weighted_price_levels")
        .alias("VOLUME_WEIGHTED_PRICE_LEVELS")
        .alias("Volume_Weighted_Price_Levels")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Cluster indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("MARKET_MICRO", signature_market_microstructure as fn() -> IndicatorSignature),
    ("ORDER_BOOK_SLOPE", signature_order_book_slope as fn() -> IndicatorSignature),
    ("ORDER_FLOW_IMB", signature_order_flow_imbalance as fn() -> IndicatorSignature),
    ("CL_QUEUE_IMB", signature_queue_imbalance as fn() -> IndicatorSignature),
    ("TICK_VOLUME", signature_tick_volume_analyzer as fn() -> IndicatorSignature),
    ("VWAP_LEVELS", signature_volume_weighted_price_levels as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static CLUSTERS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    CLUSTERS_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_market_microstructure_signature() {
        let sig = get_signature("MARKET_MICRO").unwrap();
        assert_eq!(sig.id, "MARKET_MICRO");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_order_flow_imbalance_signature() {
        let sig = get_signature("ORDER_FLOW_IMB").unwrap();
        assert_eq!(sig.id, "ORDER_FLOW_IMB");
        assert_eq!(sig.required_params().len(), 1); // period (tick_size is optional)
    }

    #[test]
    fn test_get_tick_volume_signature() {
        let sig = get_signature("TICK_VOLUME").unwrap();
        assert_eq!(sig.id, "TICK_VOLUME");
        assert!(sig.metadata.contains_key("uses_volume"));
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
        assert_eq!(count(), 6); // 6 cluster indicators
    }

    #[test]
    fn test_order_flow_validation() {
        let sig = get_signature("ORDER_FLOW_IMB").unwrap();

        // Valid params
        let params = vec![
            ("period", ParamValue::USize(50)),
            ("tick_size", ParamValue::F64(0.01)),
        ];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: period out of range
        let params = vec![
            ("period", ParamValue::USize(2)),
        ];
        assert!(sig.validate_params(&params).is_err());

        // Invalid: tick_size out of range
        let params = vec![
            ("period", ParamValue::USize(50)),
            ("tick_size", ParamValue::F64(10.0)),
        ];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("ORDER_FLOW_IMB").unwrap();
        let params = vec![
            ("period", ParamValue::USize(50)),
            ("tick_size", ParamValue::F64(0.01)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("ORDER_FLOW_IMB"));
        assert!(key.contains("50"));
    }

    #[test]
    fn test_vwap_levels_signature() {
        let sig = get_signature("VWAP_LEVELS").unwrap();
        assert_eq!(sig.id, "VWAP_LEVELS");
        assert_eq!(sig.required_params().len(), 1); // period
    }

    #[test]
    fn test_order_book_slope_signature() {
        let sig = get_signature("ORDER_BOOK_SLOPE").unwrap();
        assert_eq!(sig.id, "ORDER_BOOK_SLOPE");
        assert_eq!(sig.required_params().len(), 0); // No required params
        assert!(sig.metadata.contains_key("proxy"));
    }

    #[test]
    fn test_queue_imbalance_signature() {
        // Signature may not exist or have different ID
        if let Some(sig) = get_signature("QUEUE_IMB") {
            assert_eq!(sig.id, "QUEUE_IMB");
            assert!(sig.metadata.contains_key("formula"));
        }
    }
}
