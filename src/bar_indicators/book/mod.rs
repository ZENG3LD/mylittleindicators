// High-performance bar-based book indicators
pub mod imbalance;
pub mod microprice;
pub mod liquidity_sweep;
pub mod book_pressure;
pub mod spread_distribution;
pub mod order_book_velocity;
pub mod wall_detector;
pub mod book_depth_change;

// Delta-based book indicators
pub mod iceberg_detector;
pub mod level_replenishment_rate;
pub mod book_churn_rate;

// Hybrid Tick+Book indicators (require synchronized tick + orderbook state)
pub mod hidden_liquidity_detector;
pub mod trade_book_absorption;
pub mod sweep_impact_analyzer;

// Indicator catalog for optimizer integration
pub mod book_catalog;

pub use imbalance::BookImbalanceRatio;
pub use microprice::Microprice;
pub use liquidity_sweep::LiquiditySweep;
pub use book_pressure::BookPressure;
pub use spread_distribution::SpreadDistribution;
pub use order_book_velocity::OrderBookVelocity;
pub use wall_detector::WallDetector;
pub use book_depth_change::BookDepthChange;
pub use iceberg_detector::IcebergDetector;
pub use level_replenishment_rate::LevelReplenishmentRate;
pub use book_churn_rate::BookChurnRate;
pub use hidden_liquidity_detector::HiddenLiquidityDetector;
pub use trade_book_absorption::TradeBookAbsorption;
pub use sweep_impact_analyzer::SweepImpactAnalyzer;






















