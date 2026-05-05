// High-performance bar-based volatility indicators
pub mod atr;
pub mod atr_channels;

pub mod dc;
pub mod kc;
pub mod rvi;
pub mod vr;
pub mod kp;
pub mod fuzzy;
pub mod mass_index;
pub mod ulcer_index;
pub mod choppiness_index;
pub mod squeeze_momentum;
pub mod volatility_breakout_detector;
pub mod adaptive_bollinger_bands;
pub mod adaptive_volatility_regime;
pub mod atr_bandwidth;
pub mod atr_percentile;
pub mod atr_percentile_trend;
pub mod atr_zscore;
pub mod bipower_variance;
pub mod chaikin_volatility;
pub mod close_to_close_vol_percentile;
pub mod dynamic_volatility_regime;
pub mod har_rv;
pub mod hv_c2c;
pub mod natr;
pub mod nr_range;
pub mod park_gk_rs_yz;
pub mod range_compression_burst;
pub mod range_percentile;
pub mod rbv_jump_test;
pub mod realized_quarticity;
pub mod realized_vol;
pub mod realized_vol_zscore;
pub mod true_range;
pub mod vol_of_vol;
pub mod vol_of_vol_percentile;
pub mod vol_of_vol_percentile_trend;
pub mod volatility_break_exp;
pub mod volatility_percentile_rank_bands;
pub mod wvf;
// pub mod box_volatility;

pub use volatility_breakout_detector::*;
// pub use box_volatility::{BoxedVolatility, BoxVolatilityFactory};























// Universal Indicator System catalog
pub mod volatility_catalog;
