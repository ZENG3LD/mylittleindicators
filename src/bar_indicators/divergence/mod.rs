pub mod divergence;
pub mod divergence_catalog;

// Unique divergence algorithms (kept)
pub mod classic_divergence;
pub mod divergence_strength;
pub mod multi_divergence;

pub use divergence_catalog::{
    DIVERGENCE_CATALOG,
    get_signature,
    all_indicator_ids,
    count,
    CATEGORY,
};

// Re-exports for convenience
pub use classic_divergence::ClassicDivergence;
pub use divergence_strength::DivergenceStrength;
pub use multi_divergence::MultiDivergence;
