//! Stress indicators — consume InsuranceFund and SettlementEvent stream events.

pub mod fund_depletion_rate;
pub mod fund_stress_detector;
pub mod insurance_fund_momentum;

pub use fund_depletion_rate::FundDepletionRate;
pub use fund_stress_detector::FundStressDetector;
pub use insurance_fund_momentum::InsuranceFundMomentum;
