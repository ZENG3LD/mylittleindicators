//! Multi-stream data loading for backtest pipeline.
//!
//! Foundation layer: types and APIs. Integration with cartesian `SliceCache`
//! warmup is a separate step.

pub mod data_source;
pub mod enriched_history;
pub mod enriched_loader;
pub mod rest_fetcher;
pub mod storage;
pub mod stream_kind;
pub mod timed_event;

pub use data_source::DataSource;
pub use enriched_history::EnrichedHistory;
pub use enriched_loader::EnrichedDataLoader;
pub use rest_fetcher::RestFetcher;
pub use storage::StorageRoot;
pub use stream_kind::StreamKind;
pub use timed_event::TimedEvent;
