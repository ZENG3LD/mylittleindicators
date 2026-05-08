//! Validation functions for `Event` and `CompositionSpec`.
//!
//! Call `validate_event` before storing or generating code from an event.
//! Call `validate_composition` on the root of a `buy_when` / `sell_when` tree.

use crate::strategies::composition::spec::CompositionSpec;
use crate::strategies::events::operator::OperatorClass;
use crate::strategies::events::window::Window;
use crate::strategies::events::event::Event;

/// Validate an `Event` against the operand / structure rules for its
/// `operator_class`.
///
/// # Errors
///
/// Returns a human-readable error string if validation fails.
pub fn validate_event(e: &Event) -> Result<(), String> {
    match e.operator_class {
        OperatorClass::Cross => {
            let left = e.left_operand.as_ref()
                .ok_or("cross: left_operand is required")?;
            let right = e.right_operand.as_ref()
                .ok_or("cross: right_operand is required")?;

            // left must be Indicator or BarField
            if !left.is_indicator() && !left.is_bar_field() {
                return Err(format!(
                    "cross: left_operand must be Indicator or BarField, got {:?}",
                    left
                ));
            }
            // right must be Indicator, BarField, or Constant — NOT both Constant
            if !right.is_indicator() && !right.is_bar_field() && !right.is_constant() {
                return Err(format!(
                    "cross: right_operand must be Indicator, BarField, or Constant, got {:?}",
                    right
                ));
            }
            // Reject (Constant, Constant)
            if left.is_constant() && right.is_constant() {
                return Err("cross: both operands are constants — meaningless comparison".into());
            }
        }

        OperatorClass::ThresholdCompare => {
            let left = e.left_operand.as_ref()
                .ok_or("threshold_compare: left_operand is required")?;
            let right = e.right_operand.as_ref()
                .ok_or("threshold_compare: right_operand is required")?;

            if !left.is_indicator() && !left.is_bar_field() {
                return Err(format!(
                    "threshold_compare: left_operand must be Indicator or BarField, got {:?}",
                    left
                ));
            }
            if !right.is_constant() && !right.is_indicator() {
                return Err(format!(
                    "threshold_compare: right_operand must be Constant or Indicator, got {:?}",
                    right
                ));
            }
        }

        OperatorClass::ZoneEnter | OperatorClass::ZoneExit => {
            let left = e.left_operand.as_ref()
                .ok_or("zone: left_operand (indicator) is required")?;
            if !left.is_indicator() {
                return Err(format!(
                    "zone: left_operand must be Indicator, got {:?}",
                    left
                ));
            }
            if e.right_operand.is_some() {
                return Err("zone: right_operand must be None (use zone_bounds instead)".into());
            }
            e.zone_bounds.as_ref()
                .ok_or("zone: zone_bounds is required")?;
        }

        OperatorClass::NBarExtreme => {
            let left = e.left_operand.as_ref()
                .ok_or("nbar_extreme: left_operand (BarField) is required")?;
            if !left.is_bar_field() {
                return Err(format!(
                    "nbar_extreme: left_operand must be BarField, got {:?}",
                    left
                ));
            }
            let right = e.right_operand.as_ref()
                .ok_or("nbar_extreme: right_operand (Aggregate) is required")?;
            if !right.is_bar_field() {
                return Err(format!(
                    "nbar_extreme: right_operand must be Aggregate(BarField, n), got {:?}",
                    right
                ));
            }
            // Window must carry N
            match e.window_n {
                Window::NBars(_) | Window::CurrentBar => {}
                Window::PivotLR { .. } => {
                    return Err("nbar_extreme: window_n should be NBars or CurrentBar".into());
                }
            }
        }

        OperatorClass::Pivot => {
            let left = e.left_operand.as_ref()
                .ok_or("pivot: left_operand (BarField) is required")?;
            if !left.is_bar_field() {
                return Err(format!(
                    "pivot: left_operand must be BarField, got {:?}",
                    left
                ));
            }
            if e.right_operand.is_some() {
                return Err("pivot: right_operand must be None (window carries L/R)".into());
            }
            match e.window_n {
                Window::PivotLR { .. } => {}
                _ => return Err("pivot: window_n must be PivotLR { l, r }".into()),
            }
        }

        OperatorClass::Divergence => {
            let left = e.left_operand.as_ref()
                .ok_or("divergence: left_operand (Indicator/oscillator) is required")?;
            let right = e.right_operand.as_ref()
                .ok_or("divergence: right_operand (BarField) is required")?;
            if !left.is_indicator() {
                return Err(format!(
                    "divergence: left_operand must be Indicator, got {:?}",
                    left
                ));
            }
            if !right.is_bar_field() {
                return Err(format!(
                    "divergence: right_operand must be BarField, got {:?}",
                    right
                ));
            }
        }

        OperatorClass::CandlePattern => {
            if e.left_operand.is_some() {
                return Err("candle_pattern: left_operand must be None".into());
            }
            if e.right_operand.is_some() {
                return Err("candle_pattern: right_operand must be None".into());
            }
            e.pattern_id
                .ok_or("candle_pattern: pattern_id is required")?;
        }

        OperatorClass::RegimeGate => {
            let left = e.left_operand.as_ref()
                .ok_or("regime_gate: left_operand (Indicator) is required")?;
            let right = e.right_operand.as_ref()
                .ok_or("regime_gate: right_operand (Constant) is required")?;
            if !left.is_indicator() {
                return Err(format!(
                    "regime_gate: left_operand must be Indicator, got {:?}",
                    left
                ));
            }
            if !right.is_constant() {
                return Err(format!(
                    "regime_gate: right_operand must be Constant, got {:?}",
                    right
                ));
            }
        }

        OperatorClass::Direction => {
            let left = e.left_operand.as_ref()
                .ok_or("direction: left_operand is required")?;
            if !left.is_indicator() && !left.is_bar_field() && !left.is_constant() {
                return Err(format!(
                    "direction: left_operand must be Indicator, BarField, or Constant, got {:?}",
                    left
                ));
            }
            // right_operand optional (Constant or BarField or Indicator for relative direction)
        }

        OperatorClass::Sequence => {
            // Sequence as a Single event: slot 0 = arm trigger, slot 1 = fire trigger.
            // Both operands required: left = arm, right = fire.
            e.left_operand.as_ref()
                .ok_or("sequence: left_operand (arm trigger) is required")?;
            e.right_operand.as_ref()
                .ok_or("sequence: right_operand (fire trigger) is required")?;
            // Window must carry NBars > 0
            match e.window_n {
                Window::NBars(n) if n > 0 => {}
                _ => return Err("sequence: window_n must be NBars(n) with n > 0".into()),
            }
        }

        OperatorClass::VolatilityRegime => {
            // Vol regime shift: left = vol metric (z-score, ATR), right = optional threshold.
            e.left_operand.as_ref()
                .ok_or("volatility_regime: left_operand (vol metric role) is required")?;
        }

        OperatorClass::VolumeEvent => {
            // Volume event: left = volume (or vol-derived), right = avg/threshold.
            e.left_operand.as_ref()
                .ok_or("volume_event: left_operand (volume role) is required")?;
        }
    }

    Ok(())
}

/// Validate a `CompositionSpec` tree.
///
/// Rules:
/// - `Not` cannot be the root.
/// - `And` and `Or` vectors cannot be empty.
/// - `Sequence` must have `max_bars > 0`.
/// - All nested `Single(event)` are individually validated.
///
/// # Errors
///
/// Returns a human-readable error string if validation fails.
pub fn validate_composition(c: &CompositionSpec) -> Result<(), String> {
    validate_composition_inner(c, /* is_root */ true)
}

fn validate_composition_inner(c: &CompositionSpec, is_root: bool) -> Result<(), String> {
    match c {
        CompositionSpec::Single(event) => validate_event(event),

        CompositionSpec::And(children) => {
            if children.is_empty() {
                return Err("composition And: children vector is empty".into());
            }
            for child in children {
                validate_composition_inner(child, false)?;
            }
            Ok(())
        }

        CompositionSpec::Or(children) => {
            if children.is_empty() {
                return Err("composition Or: children vector is empty".into());
            }
            for child in children {
                validate_composition_inner(child, false)?;
            }
            Ok(())
        }

        CompositionSpec::Not(inner) => {
            if is_root {
                return Err(
                    "composition Not: cannot be the root of buy_when/sell_when".into()
                );
            }
            validate_composition_inner(inner, false)
        }

        CompositionSpec::Sequence { events, max_bars } => {
            if *max_bars == 0 {
                return Err("composition Sequence: max_bars must be > 0".into());
            }
            if events.is_empty() {
                return Err("composition Sequence: events vector is empty".into());
            }
            for event in events {
                validate_composition_inner(event, false)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{Operand, EventDirection, Window};
    use crate::event::Event;

    fn make_cross_event(left: Operand, right: Operand) -> Event {
        Event {
            operator_class: OperatorClass::Cross,
            left_operand: Some(left),
            right_operand: Some(right),
            zone_bounds: None,
            pattern_id: None,
            window_n: Window::CurrentBar,
            direction: EventDirection::Above,
            guards: vec![],
        }
    }

    #[test]
    fn cross_rejects_constant_constant() {
        let e = make_cross_event(Operand::Constant(50.0), Operand::Constant(30.0));
        assert!(validate_event(&e).is_err());
    }

    #[test]
    fn cross_accepts_indicator_constant() {
        let e = make_cross_event(
            Operand::IndicatorValue { role_idx: 0 },
            Operand::Constant(50.0),
        );
        assert!(validate_event(&e).is_ok());
    }

    #[test]
    fn cross_accepts_indicator_bar_field() {
        use crate::axes::BarField;
        let e = make_cross_event(
            Operand::IndicatorValue { role_idx: 0 },
            Operand::BarField(BarField::Close),
        );
        assert!(validate_event(&e).is_ok());
    }

    #[test]
    fn composition_not_as_root_rejected() {
        use crate::axes::BarField;
        let inner_event = make_cross_event(
            Operand::IndicatorValue { role_idx: 0 },
            Operand::BarField(BarField::Close),
        );
        let comp = CompositionSpec::Not(Box::new(CompositionSpec::Single(inner_event)));
        assert!(validate_composition(&comp).is_err());
    }

    #[test]
    fn composition_not_as_sub_ok() {
        use crate::axes::BarField;
        let inner_event = make_cross_event(
            Operand::IndicatorValue { role_idx: 0 },
            Operand::BarField(BarField::Close),
        );
        let inner_event2 = make_cross_event(
            Operand::IndicatorValue { role_idx: 1 },
            Operand::BarField(BarField::Close),
        );
        let comp = CompositionSpec::And(vec![
            CompositionSpec::Single(inner_event),
            CompositionSpec::Not(Box::new(CompositionSpec::Single(inner_event2))),
        ]);
        assert!(validate_composition(&comp).is_ok());
    }

    #[test]
    fn composition_empty_and_rejected() {
        let comp = CompositionSpec::And(vec![]);
        assert!(validate_composition(&comp).is_err());
    }

    #[test]
    fn composition_sequence_zero_max_bars_rejected() {
        use crate::axes::BarField;
        let e = make_cross_event(
            Operand::IndicatorValue { role_idx: 0 },
            Operand::BarField(BarField::Close),
        );
        let comp = CompositionSpec::Sequence {
            events: vec![CompositionSpec::Single(e)],
            max_bars: 0,
        };
        assert!(validate_composition(&comp).is_err());
    }
}
