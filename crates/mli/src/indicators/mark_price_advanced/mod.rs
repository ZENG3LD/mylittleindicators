//! Advanced mark price indicators.

pub mod mark_price_gap_detector;
pub mod mark_price_momentum;
pub mod mark_price_volatility;

pub use mark_price_gap_detector::MarkPriceGapDetector;
pub use mark_price_momentum::MarkPriceMomentum;
pub use mark_price_volatility::MarkPriceVolatility;

// Relocated from volume/ (Phase 1 reorg 2026-06-16): mark/index-price squatters.
pub mod index_price_momentum;
pub mod mark_price_vs_last;
pub use index_price_momentum::*;
pub use mark_price_vs_last::*;
