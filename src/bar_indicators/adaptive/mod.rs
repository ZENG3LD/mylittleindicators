pub mod adaptive_moving_average;
pub mod frama;
pub mod vidya;
pub mod mesa_adaptive_ma;
pub mod kaufman_adaptive_ma;
pub mod adaptive_catalog;
// pub mod box_adaptive;

pub use adaptive_moving_average::AdaptiveMovingAverage;
pub use frama::FractalAdaptiveMovingAverage;
pub use vidya::VariableIndexDynamicAverage;
pub use mesa_adaptive_ma::MesaAdaptiveMA;
pub use kaufman_adaptive_ma::KaufmanAdaptiveMA;
// pub use box_adaptive::{BoxedAdaptive, BoxAdaptiveFactory}; 






















