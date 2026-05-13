//! Statistical scoring indicators.
//!
//! Indicators that output normalized scalar measurements (probability, density,
//! tanh-normalized strength, EMA magnitude). Not events, not lines.
//!
//! Output: `IndicatorValue::Single(f64)`.

pub mod fvg_reversion_probability;
pub mod fvg_duration_intensity_score;
pub mod fvg_intensity_alt_score;
pub mod liquidity_gap_density;
pub mod swing_strength_score;
pub mod swing_age;
pub mod statistical_scoring_catalog;

pub use fvg_reversion_probability::FvgReversionProbability;
pub use fvg_duration_intensity_score::FvgDurationIntensityScore;
pub use fvg_intensity_alt_score::FvgIntensityAltScore;
pub use liquidity_gap_density::LiquidityGapDensity;
pub use swing_strength_score::SwingStrengthScore;
pub use swing_age::SwingAge;
