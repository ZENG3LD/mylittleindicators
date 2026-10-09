// Market-structure event indicators (Phase 1 reorg 2026-06-16).
// Decomposed out of events/ — break-of-structure, fair-value-gap, pivot detectors.
pub mod bos_event_detector;
pub mod fvg_event_detector;
pub mod pivot;

pub use bos_event_detector::*;
pub use fvg_event_detector::*;
pub use pivot::*;
