//! mli-collector daemon: subscribe to live streams + write to local binary storage.
//!
//! Foundation: structure + config loading + storage writer.
//! Integration with digdigdig3 WS subscriber is a separate step (feature flag).
//!
//! Usage:
//!   cargo run -p mli-collector -- collector.toml
//!   (default config path: "collector.toml" in cwd)

mod config;
mod subscriber;
mod writer;

use std::sync::Arc;

use config::CollectorConfig;
use subscriber::Subscriber;
use writer::EventWriter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "collector.toml".to_string());

    let config: CollectorConfig = {
        let s = std::fs::read_to_string(&config_path)?;
        toml::from_str(&s)?
    };

    tracing::info!(
        "Loaded config from {}: {} symbols, {} streams, exchange={}",
        config_path,
        config.symbols.len(),
        config.streams.len(),
        config.exchange,
    );

    let writer = Arc::new(EventWriter::new(config.storage_dir.clone()));

    let subscriber = Subscriber::new(Arc::clone(&writer));

    // digdigdig3 integration not yet implemented — logs a warning and returns.
    subscriber.start(&config).await?;

    tracing::warn!(
        "Storage writer ready at {:?}. Daemon idle — awaiting Ctrl-C.",
        config.storage_dir
    );

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down");
    Ok(())
}
