//! Collector configuration deserialized from TOML.

use std::path::PathBuf;

use mylittleindicators::data_loader::StreamKind;
use serde::Deserialize;

/// Top-level collector config.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectorConfig {
    /// Root directory where binary stream files are written.
    pub storage_dir: PathBuf,
    /// Default list of symbols to subscribe to.
    pub symbols: Vec<String>,
    /// Per-stream subscription configs.
    pub streams: Vec<StreamConfig>,
    /// Exchange identifier ("binance", "bybit", etc.).
    pub exchange: String,
}

/// Config for a single stream subscription.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamConfig {
    /// Stream kind to subscribe to.
    pub kind: StreamKind,
    /// Symbol overrides for this stream. Empty = use `CollectorConfig.symbols`.
    #[serde(default)]
    pub symbols: Vec<String>,
}
