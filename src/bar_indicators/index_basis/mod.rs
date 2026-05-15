//! Index/Basis indicators — consume IndexPrice, CompositeIndex, and Basis stream events.

pub mod basis_extreme;
pub mod basis_momentum;
pub mod basis_z_score;
pub mod index_component_drift;
pub mod index_correlation_breakdown;
pub mod price_vs_index_spread;

pub use basis_extreme::BasisExtreme;
pub use basis_momentum::BasisMomentum;
pub use basis_z_score::BasisZScore;
pub use index_component_drift::IndexComponentDrift;
pub use index_correlation_breakdown::IndexCorrelationBreakdown;
pub use price_vs_index_spread::PriceVsIndexSpread;
