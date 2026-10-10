//! Smart money concepts — the declarative half of markup engine #2 (see
//! `docs/mlc/plans/markup-engines-2026-08-25.md`, Appendix A, the CONTRACT
//! every detector below implements).
//!
//! Structural precedent: `crate::ewa` — a typed catalogue (`types`), pure
//! predicate functions over an explicit input slice (`market_structure`,
//! `zones`, `liquidity`, `amd`), no I/O, no streaming state where a window
//! suffices. Every detector here takes a plain `&[SmcBar]` window and a
//! `&SmcConfig` and returns bar indices into the caller's slice — never
//! prices-as-identity, and never a second implementation of a fact another
//! detector in this module already produced (`amd::amd_cycles` is the
//! clearest case: it assembles `AmdCycle`s entirely from
//! `liquidity::liquidity_sweeps` and `zones::displacement_legs` output).
//!
//! Module layout:
//! - `types` — the vocabulary: `SmcBar` (the one input type every detector
//!   shares) plus every result type.
//! - `config` — `SmcConfig`, the one parameter struct every detector reads
//!   its thresholds from.
//! - `atr` (private) — a pure window-form Wilder ATR; see its module doc
//!   for why the crate's streaming `Atr` does not fit here.
//! - `market_structure` — swings, BOS/CHoCH, trend, the dealing range.
//! - `zones` — fair value gaps, order blocks (and their breaker
//!   transition), displacement legs.
//! - `liquidity` — liquidity sweeps, equal highs/lows.
//! - `amd` — the accumulation/manipulation/distribution assembly.

mod atr;
pub mod amd;
pub mod config;
pub mod liquidity;
pub mod market_structure;
pub mod types;
pub mod zones;

pub use amd::amd_cycles;
pub use config::{OrderBlockZone, SmcConfig};
pub use liquidity::{equal_levels, liquidity_sweeps};
pub use market_structure::{dealing_range, structure, swings, trend_at};
pub use types::{
    AmdCycle, DealingRange, Direction, DisplacementLeg, EqualLevels, FvgZone, LiquiditySweep,
    OrderBlock, SmcBar, StructureEvent, StructureEventKind, Swing, SwingKind, Trend,
};
pub use zones::{displacement_legs, fair_value_gaps, order_blocks};
