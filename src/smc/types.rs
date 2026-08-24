//! Smart-money-concepts declarative types — the vocabulary the detectors in
//! `structure`, `zones`, `liquidity`, and `amd` read and write.
//!
//! Every detector reports BAR INDICES into the caller's slice, never
//! prices-as-identity, so a caller can always walk back to the exact bar a
//! structure came from.

use std::ops::Range;

/// One OHLC bar as every `smc` detector reads it.
///
/// No timestamp and no volume: none of the smart-money definitions in
/// Appendix A of `docs/mlc/plans/markup-engines-2026-08-25.md` read either
/// field, and carrying them here would invite a rule that starts reading
/// them by accident.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmcBar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Which extreme a swing point marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwingKind {
    High,
    Low,
}

/// A confirmed fractal swing point (Appendix A, "Swing point (fractal)").
///
/// Only ever produced once the swing's right-side confirmation window has
/// fully passed — see `structure::swings`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swing {
    pub index: usize,
    pub price: f64,
    pub kind: SwingKind,
}

/// Directional bias a structure event, zone, or leg carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
}

/// BOS is continuation, CHoCH is reversal. Appendix A: "It is not a
/// stronger BOS; it is the event that ends a trend, and the two must be
/// separate rule ids" — kept as two variants rather than one kind with a
/// strength field for exactly that reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructureEventKind {
    Bos,
    Choch,
}

/// A break of structure or change of character, as decided by
/// `structure::structure`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructureEvent {
    pub index: usize,
    pub kind: StructureEventKind,
    pub direction: Direction,
    pub broken_swing_index: usize,
    pub level: f64,
}

/// Prevailing market structure at a bar.
///
/// `Unresolved` is a real, reported state — Appendix A requires it never be
/// defaulted to a direction: "anything else is unresolved and must be
/// reported as unresolved, not defaulted to a direction."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trend {
    Up,
    Down,
    Unresolved,
}

/// A fair value gap / imbalance zone (Appendix A, "FVG").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FvgZone {
    pub start_index: usize,
    pub end_index: usize,
    pub low: f64,
    pub high: f64,
    pub direction: Direction,
    /// First bar (after the gap) whose range trades into the zone.
    pub mitigated_at: Option<usize>,
    /// First bar (after the gap) whose close crosses the zone's far edge.
    pub filled_at: Option<usize>,
}

/// The last opposing-close candle before a displacement leg that broke
/// structure (Appendix A, "Order block"). Never emitted without the
/// structure break behind it — `broke_structure_at` always points at a
/// real `StructureEvent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderBlock {
    pub index: usize,
    pub low: f64,
    pub high: f64,
    pub direction: Direction,
    pub broke_structure_at: usize,
    /// Set once price closes decisively through the block, the state
    /// transition Appendix A calls a "breaker block".
    pub breaker_at: Option<usize>,
}

/// An impulsive run whose range exceeded `k × ATR` and left an FVG behind
/// it (Appendix A, "Displacement leg").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplacementLeg {
    pub start_index: usize,
    pub end_index: usize,
    pub direction: Direction,
    pub range: f64,
    pub atr_at_start: f64,
}

/// A wick beyond a prior swing extreme with a close back inside it
/// (Appendix A, "Liquidity sweep / stop hunt") — the wick counterpart of
/// BOS on the same level, with the opposite verdict: BOS says the level
/// held broken, a sweep says price came back inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquiditySweep {
    pub index: usize,
    pub swept_swing_index: usize,
    pub level: f64,
    pub direction: Direction,
    pub recovered_within: usize,
}

/// Two or more swing extremes of the same kind within `tol × ATR` of each
/// other (Appendix A, "Equal highs / equal lows") — the liquidity pool a
/// sweep targets.
#[derive(Debug, Clone, PartialEq)]
pub struct EqualLevels {
    pub indices: Vec<usize>,
    pub level: f64,
    pub kind: SwingKind,
}

/// Pure geometry over the last confirmed swing pair (Appendix A, "Dealing
/// range / premium-discount") — no detection thresholds of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DealingRange {
    pub low: f64,
    pub high: f64,
    pub low_index: usize,
    pub high_index: usize,
}

impl DealingRange {
    /// The midpoint of the range.
    pub fn equilibrium(&self) -> f64 {
        (self.low + self.high) / 2.0
    }

    /// Strictly above equilibrium.
    pub fn is_premium(&self, price: f64) -> bool {
        price > self.equilibrium()
    }

    /// Strictly below equilibrium.
    pub fn is_discount(&self, price: f64) -> bool {
        price < self.equilibrium()
    }
}

/// An accumulation / manipulation / distribution cycle, ASSEMBLED from the
/// outputs of `liquidity::liquidity_sweeps` and `zones::displacement_legs`
/// (Appendix A, "AMD"). `amd::amd_cycles` is the only place that builds
/// this type — never re-detected.
#[derive(Debug, Clone, PartialEq)]
pub struct AmdCycle {
    pub accumulation: Range<usize>,
    pub manipulation: LiquiditySweep,
    pub distribution: DisplacementLeg,
}
