//! Settlement event — contract settlement or expiry price confirmation.

/// Contract settlement event.
///
/// Published when a futures or options contract settles at expiry.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettlementEvent {
    /// Final settlement price of the contract.
    pub settlement_price: f64,
    /// Scheduled settlement time in milliseconds.
    pub settlement_time: i64,
    /// Event publication timestamp in milliseconds.
    pub timestamp: i64,
}
