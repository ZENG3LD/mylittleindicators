//! Indicator Catalog System
//!
//! Three concerns:
//! - data/   — computation metadata (signatures, parameters, constraints, keys)
//! - visual/ — rendering metadata (how to draw on chart)
//! - unified — combined access for code needing both

pub mod data;
pub mod visual;
pub mod unified;

// Re-export sub-modules by name so that paths like
// `catalog::master_catalog::MasterIndicatorCatalog` continue to work
// (required for mlq compatibility).
pub use data::master_catalog;
pub use data::indicator_signature;
pub use data::constraints;
pub use data::param_value;
pub use data::parameter_grid;
pub use data::indicator_key;
pub use data::synthetic_data;

// Visual sub-module re-exports for unified.rs super:: paths
pub use visual::rendering;
pub use visual::rendering_catalog;
pub use visual::value_adapter;

// Flat type re-exports (catalog::MasterIndicatorCatalog etc.)
pub use data::*;
pub use visual::*;
pub use unified::{UnifiedIndicatorCatalog, UnifiedIndicatorInfo, UnifiedCatalogStats};
