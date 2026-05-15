//! Binary storage writer — appends `TimedEvent`s to the local binary log.

use std::path::PathBuf;

use mylittleindicators::data_loader::{StorageRoot, TimedEvent};

/// Thin wrapper around `StorageRoot` for the collector daemon.
pub struct EventWriter {
    storage: StorageRoot,
}

impl EventWriter {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            storage: StorageRoot::new(data_dir),
        }
    }

    /// Append a single event to the appropriate binary stream file.
    pub fn write(&self, symbol: &str, event: &TimedEvent) -> std::io::Result<()> {
        self.storage.append(symbol, event)
    }
}
