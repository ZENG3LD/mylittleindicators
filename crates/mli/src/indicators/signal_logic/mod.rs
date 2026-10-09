// Signal-logic primitives (Phase 1 reorg 2026-06-16).
// Threshold / cross / gate / direction primitives over an inner value. Gathered
// from events/ (line_cross, price_line_cross, threshold, direction_detector) and
// signal_processing/ (threshold_gate, hysteresis_gate, logic_gates).
pub mod line_cross;
pub mod price_line_cross;
pub mod threshold;
pub mod direction_detector;
pub mod threshold_gate;
pub mod hysteresis_gate;
pub mod logic_gates;

pub use line_cross::{CrossMode, LineCross, LineProducer, LineProducerError, LineProducerOrder};
pub use price_line_cross::{PriceLineCross, TouchMode};
pub use threshold::*;
pub use direction_detector::*;
pub use threshold_gate::*;
pub use hysteresis_gate::*;
pub use logic_gates::*;
