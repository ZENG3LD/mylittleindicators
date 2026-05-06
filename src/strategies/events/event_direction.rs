//! Direction axis — which side of the comparison triggers the event.

/// Which directional relationship triggers the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventDirection {
    /// Left crosses above right (bullish cross).
    Above,
    /// Left crosses below right (bearish cross).
    Below,
    /// Either direction triggers.
    Either,
    /// Bullish bias (rising / up-sloping).
    Bullish,
    /// Bearish bias (falling / down-sloping).
    Bearish,
}
