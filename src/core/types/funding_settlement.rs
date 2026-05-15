//! Funding settlement — confirmed funding rate after a period closes.

/// Funding settlement event.
///
/// Published by the exchange after each funding period closes, confirming
/// the rate that was applied.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FundingSettlement {
    /// Actual settled funding rate (e.g., 0.0001 = 0.01%).
    pub settled_rate: f64,
    /// Timestamp at which the funding was applied in milliseconds.
    pub settlement_time: i64,
    /// Event publication timestamp in milliseconds.
    pub timestamp: i64,
}
