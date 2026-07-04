//! Opaque overlay/toggle catalog entries.
//!
//! These are discoverability-only `IndicatorSignature`s for host-driven overlays
//! that are not computed through the standard `BarIndicatorId` factory path
//! (`machine_id: None`). See `overlay_catalog.rs` for details.

pub mod overlay_catalog;
