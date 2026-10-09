// High-performance trend indicators
// Relocated from momentum/ (2026-06-15 category reorg): genuine trend engines.
pub mod adx;
pub mod dm;
pub mod di_plus_minus;
pub mod vortex_indicator;
pub mod amat;
pub mod supertrend;
pub mod ssl_channel;
pub mod ehlers_instantaneous_trendline;
pub mod adx_slope;
pub mod didi_index;
pub mod efficiency_ratio;
pub mod gann_hilo_activator;
pub mod gmma_compression;
pub mod heikin_ashi_trend;
pub mod kama_slope;
pub mod ravi;
pub mod slope_direction_line;
pub mod trend_intensity_index;

pub use supertrend::Supertrend;
pub use adx::*;
pub use dm::*;
pub use di_plus_minus::*;
pub use vortex_indicator::*;
pub use amat::*;
pub use ehlers_instantaneous_trendline::EhlersInstantaneousTrendline; 






















