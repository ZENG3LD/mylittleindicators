//! Liquidation event — forced position close from public market feeds.
//!
//! `symbol` is omitted: mli indicators are symbol-agnostic.
//! Callers route the event to the correct per-symbol pipeline before calling
//! `LiquidationConsumer::update_liquidation`.

/// Side of the LIQUIDATED position.
///
/// `Long`  — a long position was forced-closed (exchange sold into the market).
/// `Short` — a short position was forced-closed (exchange bought from the market).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LiquidationSide {
    /// Long position liquidated (forced sell).
    Long,
    /// Short position liquidated (forced buy).
    Short,
}

/// Single liquidation event from a public market feed.
///
/// Omits `symbol` — indicators in mli are symbol-agnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Liquidation {
    /// Side of the liquidated position.
    pub side: LiquidationSide,
    /// Fill price of the liquidation order.
    pub price: f64,
    /// Fill quantity in base asset.
    pub quantity: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
    /// Quote value (price × quantity). `None` when caller did not compute it.
    pub value: Option<f64>,
}

impl Liquidation {
    /// Quote value — uses `self.value` when present, otherwise `price * quantity`.
    #[inline]
    pub fn quote_value(&self) -> f64 {
        self.value.unwrap_or(self.price * self.quantity)
    }
}
