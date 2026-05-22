//! Advanced funding rate indicators.

pub mod annualized_funding_rate;
pub mod funding_direction_shift;
pub mod funding_extreme_alert;
pub mod funding_advanced_catalog;

pub use annualized_funding_rate::AnnualizedFundingRate;
pub use funding_direction_shift::FundingDirectionShift;
pub use funding_extreme_alert::FundingExtremeAlert;
