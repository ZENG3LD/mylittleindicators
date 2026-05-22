//! mli-collector daemon: subscribe to live streams via `digdigdig3-station`,
//! which handles WS multiplex, REST warm-start, persistence to local binary
//! storage, and auto-heal on disconnect.
//!
//! Usage:
//!   cargo run -p mli-collector -- collector.toml
//!   (default config path: "collector.toml" in cwd)

mod config;

use config::CollectorConfig;
use digdigdig3_station::{PersistenceConfig, Station, SubscriptionSet};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "collector.toml".to_string());

    let cfg: CollectorConfig = {
        let s = std::fs::read_to_string(&config_path)?;
        toml::from_str(&s)?
    };

    tracing::info!(
        "Loaded config from {}: {} exchange(s), storage={:?}, warm_start={}",
        config_path,
        cfg.exchanges.len(),
        cfg.storage_dir,
        cfg.warm_start,
    );

    let station = Station::builder()
        .storage_root(cfg.storage_dir.clone())
        .persistence(PersistenceConfig::on())
        .warm_start(cfg.warm_start)
        .build()
        .await?;

    let mut set = SubscriptionSet::new();
    let mut total_subs = 0usize;
    for ex in &cfg.exchanges {
        let Some(exchange) = ex.exchange_id() else {
            tracing::warn!("unknown exchange id: {}", ex.id.0);
            continue;
        };
        for sub in &ex.subscriptions {
            let Some(account) = sub.parsed_account_type() else {
                tracing::warn!("unknown account_type: {} (sub {}/{})", sub.account_type.0, ex.id.0, sub.symbol);
                continue;
            };
            let Some(stream) = sub.parsed_stream() else {
                tracing::warn!("unsupported stream_type: {} (sub {}/{})", sub.stream_type.0, ex.id.0, sub.symbol);
                continue;
            };
            set = set.add(exchange, sub.symbol.clone(), account, [stream]);
            total_subs += 1;
        }
    }
    tracing::info!("Built SubscriptionSet: {} subscriptions across {} exchanges", total_subs, cfg.exchanges.len());

    if total_subs == 0 {
        anyhow::bail!("no valid subscriptions in config");
    }

    let mut handle = station.subscribe(set).await?;
    tracing::info!("Station subscribed; active streams = {}", station.active_streams());

    let recv_task = tokio::spawn(async move {
        let mut count: u64 = 0;
        while let Some(ev) = handle.recv().await {
            count += 1;
            if count % 1000 == 0 {
                tracing::info!(events = count, last_exchange = ?ev.exchange(), last_symbol = ev.symbol(), "events written");
            }
        }
        tracing::info!(events = count, "subscription stream closed");
    });

    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");
    recv_task.abort();
    Ok(())
}
