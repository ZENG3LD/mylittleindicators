//! Price/series divergence classification shared by chart detectors.

use crate::core::signal::direction::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceKind {
    /// Opposing slopes between price and the compared series.
    Regular,
    /// Hidden divergence: price continues, the series disagrees.
    Hidden,
}

impl Default for DivergenceKind {
    fn default() -> Self {
        Self::Regular
    }
}

/// Four-way price/series comparison.
///
/// Regular bullish: price lower low, series higher low.
/// Regular bearish: price higher high, series lower high.
/// Hidden bullish: price higher low, series lower low.
/// Hidden bearish: price lower high, series higher high.
///
/// `None` when the two slopes agree or either leg is flat.
pub fn divergence_signal(
    price_now: f64,
    price_then: f64,
    series_now: f64,
    series_then: f64,
    kind: DivergenceKind,
) -> Option<Direction> {
    let price_up = price_now > price_then;
    let price_down = price_now < price_then;
    let series_up = series_now > series_then;
    let series_down = series_now < series_then;

    match kind {
        DivergenceKind::Regular => {
            if price_down && series_up {
                Some(Direction::Up)
            } else if price_up && series_down {
                Some(Direction::Down)
            } else {
                None
            }
        }
        DivergenceKind::Hidden => {
            if price_up && series_down {
                Some(Direction::Up)
            } else if price_down && series_up {
                Some(Direction::Down)
            } else {
                None
            }
        }
    }
}
