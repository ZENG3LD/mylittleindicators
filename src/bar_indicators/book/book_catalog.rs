//! book_catalog.rs: Indicator catalog for order book indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 2 order book indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
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
        .machine_id(BarIndicatorId::BookImb) // TODO: Add to enum
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

/// Order Book Slope
pub fn signature_order_book_slope() -> IndicatorSignature {
    IndicatorSignature::builder("BOOK_SLOPE", CATEGORY)
        .name("Order Book Slope")
        .description("Slope proxy using normalized volume vs price spread")
        .metadata("calculation", "ln(volume) / (high - low)")
        .metadata("category", "depth_proxy")
        .metadata("interpretation", "Higher values indicate steeper order book")
        .machine_id(BarIndicatorId::BookSlope) // TODO: Add to enum
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
        .machine_id(BarIndicatorId::Ofi) // TODO: Add to enum
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

/// Queue Imbalance
pub fn signature_queue_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("QUEUE_IMB", CATEGORY)
        .name("Queue Imbalance")
        .description("Level-1 imbalance proxy from OHLCV: (close - mid) / range")
        .metadata("calculation", "(close - (high+low)/2) / (high-low)")
        .metadata("range", "-1 to +1")
        .metadata("category", "level1_proxy")
        .metadata("interpretation", "Positive = buying pressure, negative = selling pressure")
        .machine_id(BarIndicatorId::QueueImb) // TODO: Add to enum
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

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all book indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("BOOK_IMB", signature_book_imbalance_ratio as fn() -> IndicatorSignature),
    ("BOOK_SLOPE", signature_order_book_slope as fn() -> IndicatorSignature),
    ("OFI", signature_order_flow_imbalance as fn() -> IndicatorSignature),
    ("QUEUE_IMB", signature_queue_imbalance as fn() -> IndicatorSignature),
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
        assert_eq!(count(), 4);
    }

    #[test]
    fn test_order_flow_imbalance_params() {
        let sig = get_signature("OFI").unwrap();
        assert_eq!(sig.constraints.constraints.len(), 2);
        assert!(sig.constraints.constraints.iter().any(|c| c.name == "period"));
        assert!(sig.constraints.constraints.iter().any(|c| c.name == "tick_size"));
    }
}
