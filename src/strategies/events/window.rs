//! Window axis — how many bars the event looks at.

/// The bar window over which the event is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Window {
    /// Only the current bar (window = 1).
    CurrentBar,
    /// Last N bars.
    NBars(usize),
    /// Pivot window: L bars to the left, R bars to the right.
    PivotLR { l: usize, r: usize },
}
