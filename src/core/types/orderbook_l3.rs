//! Level-3 orderbook event — individual order-level add/modify/delete messages.

/// Side of an L3 orderbook entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderBookSide {
    /// Buy side (bids).
    Bid,
    /// Sell side (asks).
    Ask,
}

/// Action applied to an individual L3 order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum L3Action {
    /// New order placed at this price level.
    Add,
    /// Existing order quantity or price changed.
    Modify,
    /// Order fully cancelled or filled and removed.
    Delete,
}

/// Level-3 orderbook event — individual order-level mutation.
///
/// Carries add, modify, or delete for a single named order in the book.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone)]
pub struct OrderbookL3Event {
    /// Which side of the book this order belongs to.
    pub side: OrderBookSide,
    /// Exchange-assigned order identifier.
    pub order_id: String,
    /// Price of the order.
    pub price: f64,
    /// Remaining quantity of the order (0 on `Delete`).
    pub quantity: f64,
    /// Action applied to this order.
    pub action: L3Action,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
