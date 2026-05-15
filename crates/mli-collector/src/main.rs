//! mli-collector daemon: subscribe to live streams + write to local binary storage.
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

    let config_clone = config.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = subscriber.start(&config_clone).await {
            tracing::error!("Subscriber error: {e}");
        }
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");
    handle.abort();
    Ok(())
}
