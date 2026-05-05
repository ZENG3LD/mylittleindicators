pub mod divergence;
pub mod divergence_catalog;

// Divergence wrapper indicators
pub mod rsi_divergence;
pub mod cci_divergence;
pub mod macd_divergence;
pub mod macd_histogram_divergence;
pub mod stochastic_divergence;
pub mod williams_divergence;
pub mod obv_divergence;
pub mod volume_divergence;
pub mod classic_divergence;
pub mod hidden_divergence;
pub mod divergence_strength;
pub mod multi_divergence;
pub mod momentum_divergence;

pub use divergence_catalog::{
    DIVERGENCE_CATALOG,
    get_signature,
    all_indicator_ids,
    count,
    CATEGORY,
};

// Re-exports for convenience
pub use rsi_divergence::RsiDivergence;
pub use cci_divergence::CciDivergence;
pub use macd_divergence::MacdDivergence;
pub use macd_histogram_divergence::MacdHistogramDivergence;
pub use stochastic_divergence::StochasticDivergence;
pub use williams_divergence::WilliamsDivergence;
pub use obv_divergence::ObvDivergence;
pub use volume_divergence::VolumeDivergence;
pub use classic_divergence::ClassicDivergence;
pub use hidden_divergence::HiddenDivergence;
pub use divergence_strength::DivergenceStrength;
pub use multi_divergence::MultiDivergence;
pub use momentum_divergence::MomentumDivergence;






















