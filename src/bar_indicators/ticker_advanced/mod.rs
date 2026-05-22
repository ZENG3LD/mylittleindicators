//! Ticker advanced indicators — consume Ticker (24h stats) stream events.

pub mod ticker_spread_ratio;
pub mod volume_24h_z_score;
pub mod ticker_advanced_catalog;

pub use ticker_spread_ratio::TickerSpreadRatio;
pub use volume_24h_z_score::Volume24hZScore;
