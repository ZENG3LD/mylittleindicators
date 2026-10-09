// Composite-index indicators (Phase 1 reorg 2026-06-16).
// Split out of index_basis/ — these consume the CompositeIndex stream
// (component / weight / correlation drift), not basis/spread.
pub mod composite_weight_drift;
pub mod index_component_drift;
pub mod index_correlation_breakdown;

pub use composite_weight_drift::*;
pub use index_component_drift::*;
pub use index_correlation_breakdown::*;
