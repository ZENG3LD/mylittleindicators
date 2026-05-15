//! Per-stream data source selector.

use super::StreamKind;
use std::collections::HashMap;
use std::path::PathBuf;

/// Where to load each stream from.
#[derive(Debug, Clone)]
pub enum DataSource {
    /// Local binary log (from mli-collector daemon).
    Local { storage_root: PathBuf },
    /// Direct REST fetch from exchange (via digdigdig3 in future — not yet implemented).
    Rest { exchange: String },
    /// Per-stream selection, routing each `StreamKind` to a different source.
    Mixed {
        per_stream: HashMap<StreamKind, Box<DataSource>>,
    },
}

#[cfg(test)]
mod tests {
    use super::DataSource;
    use std::path::PathBuf;

    #[test]
    fn local_variant_stores_path() {
        let ds = DataSource::Local {
            storage_root: PathBuf::from("/tmp/data"),
        };
        match ds {
            DataSource::Local { storage_root } => {
                assert_eq!(storage_root, PathBuf::from("/tmp/data"));
            }
            _ => panic!("expected Local"),
        }
    }

    #[test]
    fn rest_variant_stores_exchange() {
        let ds = DataSource::Rest {
            exchange: "binance".into(),
        };
        match ds {
            DataSource::Rest { exchange } => assert_eq!(exchange, "binance"),
            _ => panic!("expected Rest"),
        }
    }
}
