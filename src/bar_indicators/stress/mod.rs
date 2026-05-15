//! Stress indicators — consume InsuranceFund and SettlementEvent stream events.

pub mod fund_depletion_rate;
pub mod fund_stress_detector;
pub mod settlement_price_momentum;
pub mod settlement_vs_mark_spread;

pub use fund_depletion_rate::FundDepletionRate;
pub use fund_stress_detector::FundStressDetector;
pub use settlement_price_momentum::SettlementPriceMomentum;
pub use settlement_vs_mark_spread::SettlementVsMarkSpread;
