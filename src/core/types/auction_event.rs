//! Auction event — exchange opening/closing auction state update.

/// Auction event snapshot.
///
/// Published during exchange opening, indicative, and closing auction phases.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone)]
pub struct AuctionEvent {
    /// Exchange-assigned auction identifier.
    pub auction_id: String,
    /// Indicative clearing price at current auction state.
    pub indicative_price: f64,
    /// Indicative clearing quantity at current auction state.
    pub indicative_qty: f64,
    /// Auction phase: `"opening"` | `"indicative"` | `"closing"`
    pub state: String,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
