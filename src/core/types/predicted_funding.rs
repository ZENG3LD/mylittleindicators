//! Predicted funding rate — exchange estimate of the next funding period.

/// Predicted funding rate snapshot.
///
/// Published by exchanges ahead of the actual funding settlement to give
/// market participants advance notice.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PredictedFunding {
    /// Exchange's predicted funding rate for the next period (e.g., 0.0001 = 0.01%).
    pub predicted_rate: f64,
    /// Timestamp of the next funding settlement in milliseconds.
    pub next_funding_time: i64,
    /// Event publication timestamp in milliseconds.
    pub timestamp: i64,
}
