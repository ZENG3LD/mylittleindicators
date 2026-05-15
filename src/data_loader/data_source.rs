//! Per-stream data source selector.

use super::StreamKind;
use std::collections::HashMap;
use std::path::PathBuf;

/// Where to load each stream from.
#[derive(Debug, Clone)]
pub enum DataSource {
    /// Local binary log (for streams without public REST history).
    ///
    /// Written by the collector daemon in format:
    /// `[u64 ts LE][u32 len LE][payload bytes (serde_json)]`.
    ///
    /// Path layout: `{storage_root}/{symbol}/{stream_kind}.bin`
    Binary { storage_root: PathBuf },

    /// Local JSON snapshot file (for quick plugin / one-off loading).
    ///
    /// Format: JSON array of `TimedEvent`.
    /// Path: `{storage_root}/{symbol}/{stream_kind}.json`
    ///
    /// Storage write is not supported in this variant — use it for
    /// pre-existing test data or one-off imports.
    Json { storage_root: PathBuf },

    /// Direct REST fetch from exchange (no local storage).
    ///
    /// Supported for streams where public REST history is available
    /// (klines, funding history, OI history, liquidations <7d, aggTrades).
    ///
    /// Requires a `RestFetcher` implementor passed via
    /// `EnrichedDataLoader::with_rest_fetcher(...)`.
    /// Without one, loading returns `ErrorKind::Unsupported`.
    Rest { exchange: String },

    /// Per-stream source selection.
    ///
    /// Each `StreamKind` independently resolves to its own `DataSource`.
    /// Allows mix-and-match: bars via `Rest`, funding via `Binary`, etc.
    ///
    /// If a stream is not present in `per_stream`, loading returns
    /// `ErrorKind::NotFound`.
    Mixed {
        per_stream: HashMap<StreamKind, Box<DataSource>>,
    },
}

#[cfg(test)]
mod tests {
    use super::DataSource;
    use crate::data_loader::StreamKind;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn binary_variant_stores_path() {
        let ds = DataSource::Binary {
            storage_root: PathBuf::from("/tmp/data"),
        };
        match ds {
            DataSource::Binary { storage_root } => {
                assert_eq!(storage_root, PathBuf::from("/tmp/data"));
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn json_variant_stores_path() {
        let ds = DataSource::Json {
            storage_root: PathBuf::from("/tmp/json_data"),
        };
        match ds {
            DataSource::Json { storage_root } => {
                assert_eq!(storage_root, PathBuf::from("/tmp/json_data"));
            }
            _ => panic!("expected Json"),
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

    #[test]
    fn mixed_variant_stores_per_stream() {
        let mut map: HashMap<StreamKind, Box<DataSource>> = HashMap::new();
        map.insert(
            StreamKind::Funding,
            Box::new(DataSource::Binary {
                storage_root: PathBuf::from("/tmp/binary"),
            }),
        );
        map.insert(
            StreamKind::OpenInterest,
            Box::new(DataSource::Json {
                storage_root: PathBuf::from("/tmp/json"),
            }),
        );
        let ds = DataSource::Mixed { per_stream: map };
        match ds {
            DataSource::Mixed { per_stream } => {
                assert!(per_stream.contains_key(&StreamKind::Funding));
                assert!(per_stream.contains_key(&StreamKind::OpenInterest));
            }
            _ => panic!("expected Mixed"),
        }
    }
}
