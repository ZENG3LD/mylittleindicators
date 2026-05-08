//! Composition axis — how multiple events combine into a signal condition.

use crate::core::events::event::Event;

/// Recursive composition of event conditions.
///
/// A `CompositionSpec` is the tree structure evaluated per bar to decide
/// whether to emit Buy / Sell / ForceClose signals.
#[derive(Debug, Clone)]
pub enum CompositionSpec {
    /// Single event — no composition.
    Single(Event),
    /// All sub-expressions must be true simultaneously.
    And(Vec<CompositionSpec>),
    /// At least one sub-expression must be true.
    Or(Vec<CompositionSpec>),
    /// The sub-expression must be false. Only valid as a sub-expression,
    /// never as the root of a `buy_when` / `sell_when` tree.
    Not(Box<CompositionSpec>),
    /// Events must fire in sequence within `max_bars` bars of each other.
    ///
    /// Requires 2 extra state variables: `fired_a` + `bars_since_a`.
    Sequence {
        events: Vec<CompositionSpec>,
        /// Maximum bars allowed between first and last event firing.
        max_bars: usize,
    },
}
