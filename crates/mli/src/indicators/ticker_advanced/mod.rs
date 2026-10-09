//! Ticker advanced indicators — consume Ticker (24h stats) stream events.

pub mod ticker_spread_ratio;
pub mod volume_24h_z_score;

pub use ticker_spread_ratio::TickerSpreadRatio;
pub use volume_24h_z_score::Volume24hZScore;

// Relocated from volume/ (Phase 1 reorg 2026-06-16): 24h ticker-field squatters.
pub mod high_low_range_ratio;
pub mod volume_24h_momentum;
pub use high_low_range_ratio::*;
pub use volume_24h_momentum::*;

// Relocated from statistical_scoring/ (Phase 1 reorg): zscore of ticker 24h field.
pub mod price_change_24h_z_score;
pub use price_change_24h_z_score::*;
