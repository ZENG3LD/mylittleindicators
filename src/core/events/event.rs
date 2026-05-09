//! Event — the atomic unit of a strategy condition.
//!
//! An `Event` describes a single market observation (crossover, threshold
//! breach, candle pattern, etc.) that can be composed into buy/sell logic
//! via `CompositionSpec`.

use crate::core::events::operator::OperatorClass;
use crate::core::events::operand::Operand;
use crate::core::events::event_direction::EventDirection;
use crate::core::events::window::Window;
use crate::core::composition::guard::Guard;

/// Zone bounds for `ZoneEnter` / `ZoneExit` operator classes.
///
/// Used instead of the normal `right_operand` — the zone is defined
/// by two constants, not a single comparator.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneBounds {
    /// Lower bound of the zone.
    pub lo: f64,
    /// Upper bound of the zone.
    pub hi: f64,
}

impl ZoneBounds {
    /// Create new zone bounds.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `lo >= hi`.
    pub fn new(lo: f64, hi: f64) -> Self {
        debug_assert!(lo < hi, "ZoneBounds: lo must be strictly less than hi");
        Self { lo, hi }
    }
}

/// A single atomic event condition in a strategy.
///
/// The set of valid fields depends on `operator_class`:
/// - `ZoneEnter`/`ZoneExit`: `zone_bounds` is set, `right_operand` is `None`.
/// - `CandlePattern`: `pattern_id` is set, `left_operand` / `right_operand` are both `None`.
/// - `Pivot`: `right_operand` is `None`, `window_n` carries `PivotLR`.
/// - All others: `left_operand` and `right_operand` follow standard rules.
///
/// Use `validation::validate_event` to enforce these invariants.
#[derive(Debug, Clone)]
pub struct Event {
    /// What kind of comparison or event this is.
    pub operator_class: OperatorClass,
    /// Left-hand operand. `None` only for `CandlePattern`.
    pub left_operand: Option<Operand>,
    /// Right-hand operand. `None` for `ZoneEnter`/`ZoneExit`/`Pivot`/`CandlePattern`.
    pub right_operand: Option<Operand>,
    /// Zone bounds (only for `ZoneEnter`/`ZoneExit`).
    pub zone_bounds: Option<ZoneBounds>,
    /// Pattern identifier (only for `CandlePattern`).
    pub pattern_id: Option<u32>,
    /// Bar window the event is evaluated over.
    pub window_n: Window,
    /// Directional constraint on the event.
    pub direction: EventDirection,
    /// Additional guard conditions that must also be true.
    pub guards: Vec<Guard>,
}

impl Event {
    /// Construct a minimal cross event between two operands.
    pub fn cross(
        left: Operand,
        right: Operand,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::Cross,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction,
            guards,
        }
    }

    /// Construct a threshold-compare event.
    pub fn threshold(
        left: Operand,
        right: Operand,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::ThresholdCompare,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction,
            guards,
        }
    }

    /// Construct a zone-enter event.
    pub fn zone_enter(left: Operand, bounds: ZoneBounds, guards: Vec<Guard>) -> Self {
        Self {
            operator_class: OperatorClass::ZoneEnter,
            left_operand: Some(left),
            right_operand: None,
            zone_bounds: Some(bounds),
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction: EventDirection::Either,
            guards,
        }
    }

    /// Construct a candle pattern event.
    pub fn candle_pattern(pattern_id: u32, guards: Vec<Guard>) -> Self {
        Self {
            operator_class: OperatorClass::CandlePattern,
            left_operand: None,
            right_operand: None,
            zone_bounds: None,
            pattern_id: Some(pattern_id),
            window_n: Window::CurrentBar,
            direction: EventDirection::Either,
            guards,
        }
    }

    /// Zone-exit event (mirror of zone_enter).
    pub fn zone_exit(left: Operand, bounds: ZoneBounds, guards: Vec<Guard>) -> Self {
        Self {
            operator_class: OperatorClass::ZoneExit,
            left_operand: Some(left),
            right_operand: None,
            zone_bounds: Some(bounds),
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction: EventDirection::Either,
            guards,
        }
    }

    /// N-bar extreme event (highest / lowest of `field` over last `n` bars).
    pub fn nbar_extreme(
        left: Operand,
        n: usize,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::NBarExtreme,
            left_operand: Some(left),
            right_operand: None,
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::NBars(n),
            direction,
            guards,
        }
    }

    /// Pivot high/low event (l bars left, r bars right).
    pub fn pivot(
        left: Operand,
        l: usize,
        r: usize,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::Pivot,
            left_operand: Some(left),
            right_operand: None,
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::PivotLR { l, r },
            direction,
            guards,
        }
    }

    /// Direction event (slope / above-or-below over a window).
    pub fn direction(
        left: Operand,
        right: Option<Operand>,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::Direction,
            left_operand: Some(left),
            right_operand: right,
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction,
            guards,
        }
    }

    /// Divergence event between two operands (typically price vs oscillator).
    pub fn divergence(
        left: Operand,
        right: Operand,
        n: usize,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::Divergence,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::NBars(n),
            direction,
            guards,
        }
    }

    /// Regime gate (filter that must hold persistently — e.g. ADX > 25).
    pub fn regime_gate(
        left: Operand,
        right: Operand,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::RegimeGate,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction,
            guards,
        }
    }

    /// Sequence event: arm-trigger (left) then fire-trigger (right) within `n` bars.
    pub fn sequence(
        arm: Operand,
        fire: Operand,
        n: usize,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::Sequence,
            left_operand: Some(arm),
            right_operand: Some(fire),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::NBars(n),
            direction: EventDirection::Either,
            guards,
        }
    }

    /// Volatility regime shift (z-score / percentile-based regime transition).
    pub fn volatility_regime(
        left: Operand,
        right: Operand,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::VolatilityRegime,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction,
            guards,
        }
    }

    /// Volume event (Spike / Climax / Delta shift).
    pub fn volume_event(
        left: Operand,
        right: Operand,
        direction: EventDirection,
        guards: Vec<Guard>,
    ) -> Self {
        Self {
            operator_class: OperatorClass::VolumeEvent,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction,
            guards,
        }
    }
}
