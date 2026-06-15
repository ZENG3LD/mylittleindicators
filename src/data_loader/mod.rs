//! Stream-kind classification for the backtest data pipeline.
//!
//! `StreamKind` enumerates every historical data stream that can feed indicators
//! (Bar / Tick / OrderBook / Funding / Liquidation / OpenInterest / … ). It is the
//! routing key used by the catalog signatures (`signature.input_stream`) and by
//! downstream consumers (mlq warmup/spec-engine).
//!
//! ## Removed: the live-fetch subsystem
//!
//! This module previously also held a live-fetch subsystem — `ExchangeHubFetcher`
//! (backed by digdigdig3 `ExchangeHub`), `EnrichedDataLoader`, the `RestFetcher`
//! trait, `DataSource`, plus `storage` / `timed_event` / `enriched_history` /
//! `timeline_merger` helpers. It had **zero consumers**: mlc has its own data
//! services (dig3 + station directly), mlq uses only `StreamKind`, and
//! mli-validator fetches via `digdigdig3-station` directly. It was a legacy
//! attempt to keep connector/station logic inside the OSS indicator crate.
//!
//! It was removed when mli switched its only dependency from the full
//! `digdigdig3` (47 connectors + reqwest/tokio/websockets) to the light
//! `digdigdig3-core` (pure data types). Connector-backed fetching belongs in a
//! consumer that actually needs it (mli-validator, or any app over
//! `digdigdig3-station`), NOT in the OSS types/indicators crate.

pub mod stream_kind;

pub use stream_kind::StreamKind;
