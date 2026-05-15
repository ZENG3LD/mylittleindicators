//! WebSocket subscriber stub.
//!
//! Real implementation will use digdigdig3 `ConnectorManager` to subscribe
//! to live streams and dispatch events to `EventWriter`.
//!
//! Enable with feature flag `digdigdig3-integration` when the integration
//! crate is available.

use std::sync::Arc;

use crate::config::CollectorConfig;
use crate::writer::EventWriter;

/// Subscriber wires live WS stream data into the binary storage.
pub struct Subscriber {
    pub writer: Arc<EventWriter>,
}

impl Subscriber {
    pub fn new(writer: Arc<EventWriter>) -> Self {
        Self { writer }
    }

    /// Start subscribing to all configured streams and write events.
    ///
    /// **Not yet implemented** — returns immediately after logging a warning.
    /// Integration with digdigdig3 `ConnectorManager` is a future step
    /// (see feature `digdigdig3-integration`).
    pub async fn start(&self, config: &CollectorConfig) -> anyhow::Result<()> {
        tracing::warn!(
            "Subscriber::start — digdigdig3 integration not yet implemented; \
             enable feature `digdigdig3-integration` and wire ConnectorManager. \
             Configured: {} symbols, {} streams on {}",
            config.symbols.len(),
            config.streams.len(),
            config.exchange,
        );
        // Log each stream config so the fields are exercised.
        for stream in &config.streams {
            tracing::debug!(
                "stream kind={:?}, symbol_override_count={}",
                stream.kind,
                stream.symbols.len(),
            );
        }
        tracing::debug!("event writer storage root: {:?}", self.writer.storage_root());
        // Real usage pattern (when digdigdig3 streams events):
        //   self.writer.write(symbol, &event)?;
        // The write method is part of the public API; stub reference kept here
        // so the compiler sees it as reachable from the binary target.
        let _ = |sym: &str, ev: &mylittleindicators::data_loader::TimedEvent| {
            self.writer.write(sym, ev)
        };
        Ok(())
    }
}
