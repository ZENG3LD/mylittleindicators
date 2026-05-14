//! Public trade with full metadata (id, symbol).
//!
//! Differs from [`Tick`](crate::core::types::Tick): includes a trade `id` for
//! deduplication and a `symbol` for multi-asset workflows.
//!
//! # Usage
//! For symbol-agnostic indicator pipelines use [`Tick`](crate::core::types::Tick)
//! directly. `PublicTrade` is intended for audited or multi-asset ingestion layers
//! that need to track trade identity before dispatching.
//!
//! # Conversion
//! ```
//! use zengeld_chart_indicators::core::types::{PublicTrade, Tick};
//!
//! let trade = PublicTrade {
//!     id: "12345".to_string(),
//!     symbol: "BTCUSDT".to_string(),
//!     price: 30_000.0,
//!     quantity: 0.1,
//!     is_buy: true,
//!     timestamp: 1_700_000_000_000,
//! };
//!
//! let tick = Tick::new(trade.timestamp, trade.price, trade.quantity, trade.is_buy);
//! ```

/// Public trade as received from an exchange feed, with full metadata.
#[derive(Debug, Clone)]
pub struct PublicTrade {
    /// Exchange-assigned trade id (used for deduplication).
    pub id: String,
    /// Trading pair symbol (e.g. `"BTCUSDT"`).
    pub symbol: String,
    /// Fill price.
    pub price: f64,
    /// Fill quantity in base asset.
    pub quantity: f64,
    /// `true` = buy aggressor (taker bought); `false` = sell aggressor (taker sold).
    pub is_buy: bool,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
