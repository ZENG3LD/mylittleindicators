//! Order-flow markup: divergence between price and any per-bar series (the
//! chart's use case is cumulative volume delta) measured between
//! CONSECUTIVE swing extremes, not at a fixed bar distance.
//!
//! Structural precedent: `crate::smc` — pure predicate functions over an
//! explicit `&[SmcBar]` window and a config struct, no I/O, no chart type,
//! bar indices out. This module adds no swing definition of its own: it
//! reuses `smc::market_structure::swings` so the crate carries exactly one
//! fractal swing detector, and it reuses `events::divergence`'s
//! `divergence_signal` predicate so the streaming detector and this scan
//! can never disagree about what a divergence is.

pub mod delta_divergence;

pub use delta_divergence::{delta_divergences, DeltaDivergenceConfig, DeltaDivergenceHit};
