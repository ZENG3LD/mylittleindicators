//! Operator class axis — describes the type of event/comparison.

/// What kind of comparison or event the signal evaluates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorClass {
    /// Two values cross each other (crossover / crossunder).
    Cross,
    /// One value crosses a threshold level.
    ThresholdCompare,
    /// Value enters a zone defined by lo/hi bounds.
    ZoneEnter,
    /// Value exits a zone defined by lo/hi bounds.
    ZoneExit,
    /// Value sets an N-bar extreme (highest/lowest over window).
    NBarExtreme,
    /// Pivot high or pivot low detected via left/right lookback.
    Pivot,
    /// Directional movement signal (slope, above/below).
    Direction,
    /// Divergence between indicator and price action.
    Divergence,
    /// Candle pattern detected (no operands).
    CandlePattern,
    /// Regime gate — filter based on trend/volatility indicator.
    RegimeGate,
    /// Sequential event: A followed by B within N bars.
    ///
    /// Requires stateful arming: slot0 arms the machine, slot1 fires the signal.
    Sequence,
}

/// How the condition must hold over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strictness {
    /// Trigger exactly at the crossing bar (edge).
    OnEdge,
    /// Condition must hold persistently (every bar).
    Persistent,
    /// Trigger only the first bar the condition becomes true.
    FirstTime,
    /// Condition must be confirmed for N consecutive bars.
    NBarsConfirmed(usize),
}

/// Returns the default strictness for an operator class.
///
/// This mapping is canonical: codegen uses it to emit the correct
/// condition-check pattern without the user having to specify strictness
/// explicitly on each event.
pub fn strictness_for(op: OperatorClass) -> Strictness {
    match op {
        OperatorClass::Cross => Strictness::OnEdge,
        OperatorClass::ThresholdCompare => Strictness::Persistent,
        OperatorClass::ZoneEnter => Strictness::FirstTime,
        OperatorClass::ZoneExit => Strictness::FirstTime,
        OperatorClass::NBarExtreme => Strictness::OnEdge,
        OperatorClass::Pivot => Strictness::OnEdge,
        OperatorClass::Direction => Strictness::NBarsConfirmed(1),
        OperatorClass::Divergence => Strictness::OnEdge,
        OperatorClass::CandlePattern => Strictness::OnEdge,
        OperatorClass::RegimeGate => Strictness::Persistent,
        OperatorClass::Sequence => Strictness::OnEdge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_strictness_is_on_edge() {
        assert_eq!(strictness_for(OperatorClass::Cross), Strictness::OnEdge);
    }

    #[test]
    fn threshold_strictness_is_persistent() {
        assert_eq!(strictness_for(OperatorClass::ThresholdCompare), Strictness::Persistent);
    }

    #[test]
    fn zone_enter_strictness_is_first_time() {
        assert_eq!(strictness_for(OperatorClass::ZoneEnter), Strictness::FirstTime);
    }

    #[test]
    fn direction_strictness_is_nbar_confirmed() {
        assert!(matches!(strictness_for(OperatorClass::Direction), Strictness::NBarsConfirmed(1)));
    }
}
