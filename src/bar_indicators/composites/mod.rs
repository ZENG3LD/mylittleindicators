//! Cross-stream composite indicators.
//!
//! These indicators consume multiple independent data streams simultaneously
//! (funding rate, open interest, mark price, liquidations, etc.) to produce
//! composite signals that no single-stream indicator can generate.

pub mod adaptive_threshold;
pub mod adaptive_window_selector;
pub mod block_trade_volume_ratio;
pub mod capitulation_detector;
pub mod compound_squeeze_probability;
pub mod cross_asset_beta;
pub mod funding_oi_pressure;
pub mod funding_sentiment_alignment;
pub mod index_tracking_error;
pub mod iv_hv_spread;
pub mod market_stress_composite;
pub mod pairs_cointegration_proxy;
pub mod relative_strength_cross;
pub mod risk_off_detector;
pub mod sentiment_composite;
pub mod squeeze_probability;
pub mod vol_regime_entry;

pub use adaptive_threshold::AdaptiveThreshold;
pub use adaptive_window_selector::AdaptiveWindowSelector;
pub use block_trade_volume_ratio::BlockTradeVolumeRatio;
pub use capitulation_detector::CapitulationDetector;
pub use compound_squeeze_probability::CompoundSqueezeProbability;
pub use cross_asset_beta::CrossAssetBeta;
pub use funding_oi_pressure::FundingOiPressure;
pub use funding_sentiment_alignment::FundingSentimentAlignment;
pub use index_tracking_error::IndexTrackingError;
pub use iv_hv_spread::IvHvSpread;
pub use market_stress_composite::MarketStressComposite;
pub use pairs_cointegration_proxy::PairsCointegrationProxy;
pub use relative_strength_cross::RelativeStrengthCross;
pub use risk_off_detector::RiskOffDetector;
pub use sentiment_composite::SentimentComposite;
pub use squeeze_probability::SqueezeProbability;
pub use vol_regime_entry::VolRegimeEntry;
