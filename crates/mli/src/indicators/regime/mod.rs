// Market regime indicators (trending-vs-ranging / random-walk filters).
// Relocated from momentum/ (2026-06-15 category reorg): these answer
// "is the market trending or ranging?" — a regime question, not momentum.
pub mod rwi;
pub mod vhf;
pub mod vhf_ma;

pub use rwi::*;
pub use vhf::*;
pub use vhf_ma::*;

// Relocated (Phase 1 reorg 2026-06-16): trending-vs-ranging / market-regime classifiers.
pub mod adaptive_volatility_regime;
pub mod dynamic_volatility_regime;
pub mod choppiness_index;
pub mod market_regime_filter;
pub mod regime_composite;
// Selective (not glob): both modules export a `VolatilityRegime` enum — Phase-2
// dedup candidate. Glob re-export here would be ambiguous.
pub use adaptive_volatility_regime::AdaptiveVolatilityRegime;
pub use dynamic_volatility_regime::DynamicVolatilityRegime;
pub use choppiness_index::*;
pub use market_regime_filter::*;
pub use regime_composite::*;

// Decomposed from events/ (Phase 1 reorg 2026-06-16): regime gate + detectors.
pub mod regime_gate;
pub mod volatility_regime;
pub mod relative_position;
pub use regime_gate::*;
pub use volatility_regime::VolatilityRegimeDetector;
pub use relative_position::*;

// EWA (Elliott Wave Analysis) — regime/structure detection via pivot pattern recognition.
pub mod ewa;
