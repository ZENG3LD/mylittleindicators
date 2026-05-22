//! Tick-advanced indicators — higher-level order-flow analytics on the trade stream.
//!
//! All indicators here implement [`TickConsumer`] and process raw ticks one by one.
//!
//! Indicators:
//! - [`VwapDeviation`]                    — rolling VWAP + price deviation %.
//! - [`TradeRunDetector`]                 — consecutive same-side tick run length.
//! - [`SizeWeightedDirectionalMomentum`]  — volume-weighted directional bias [-1, 1].
//! - [`TickFrequencyAnomaly`]             — tick-rate burst/quiet ratio.
//! - [`AggressorBurstDetector`]           — one-sided burst signal (+1 / -1 / 0).
//! - [`LargeTickMomentum`]                — directional momentum of large ticks only.
//! - [`ValueAreaTracker`]                 — rolling Volume Profile POC / VAH / VAL.
//! - [`VolumeImbalanceZone`]              — buy/sell imbalance zone detector.

pub mod vwap_deviation;
pub mod trade_run_detector;
pub mod size_weighted_directional_momentum;
pub mod tick_frequency_anomaly;
pub mod aggressor_burst_detector;
pub mod large_tick_momentum;
pub mod tpo_session_balance;
pub mod value_area_tracker;
pub mod volume_imbalance_zone;
pub mod tick_advanced_catalog;

pub use vwap_deviation::VwapDeviation;
pub use trade_run_detector::TradeRunDetector;
pub use size_weighted_directional_momentum::SizeWeightedDirectionalMomentum;
pub use tick_frequency_anomaly::TickFrequencyAnomaly;
pub use aggressor_burst_detector::AggressorBurstDetector;
pub use large_tick_momentum::LargeTickMomentum;
pub use tpo_session_balance::TpoSessionBalance;
pub use value_area_tracker::ValueAreaTracker;
pub use volume_imbalance_zone::VolumeImbalanceZone;
