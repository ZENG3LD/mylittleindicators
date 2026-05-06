//! `operator_class` → `SignalType` mapping.
//!
//! Provides `signal_type_for` which maps the structural description of
//! an event to the semantic `SignalType` used by the backtester for
//! recording and filtering.

use crate::strategies::events::operator::OperatorClass;
use crate::strategies::shapes::role_kind::RoleKind;

use serde::{Deserialize, Serialize};

/// SignalType — WHY the signal was generated. Used by backtester for recording
/// and filtering.
///
/// Canonical taxonomy ported from `mlq-core::strategy`. mlq-core пока имеет
/// свою копию для backwards-compat — постепенно перейдём на эту.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SignalType {
    Unknown = 0,
    MaCrossover = 1,
    MaCross = 2,
    RsiReversal = 3,
    RsiDivergence = 4,
    MacdCross = 5,
    MacdHistogram = 6,
    StochReversal = 7,
    StochCross = 8,
    CciExtreme = 9,
    BollingerBreak = 10,
    BollingerSqueeze = 11,
    AtrBreakout = 12,
    DonchianBreak = 13,
    KeltnerBreak = 14,
    AroonCross = 15,
    AdxTrend = 16,
    VhfTrend = 17,
    TrendFollow = 18,
    PriceAction = 19,
    SupportResistance = 20,
    SwingTouch = 21,
    ZigzagReversal = 22,
    VolumeSpike = 23,
    VolumeMaCross = 24,
    Divergence = 25,
    MeanReversion = 26,
    Momentum = 27,
    Breakout = 28,
    FiboRetracement = 29,
    FiboExtension = 30,
    MultiTfAlign = 31,
    HtfFilter = 32,
    LtfConfirm = 33,
    ExitTakeProfit = 34,
    ExitStopLoss = 35,
    ExitTrailing = 36,
    ExitSignal = 37,
    ExitForced = 38,
    Combined = 39,
    Filtered = 40,
}

impl SignalType {
    #[inline]
    pub fn as_u8(self) -> u8 { self as u8 }
}

/// Derive the `SignalType` for an event given its operator class and an
/// optional hint about the kind of indicator in the primary role.
///
/// The `role_kind_hint` is optional: when `None`, a generic mapping is used.
pub fn signal_type_for(op: OperatorClass, role_kind_hint: Option<RoleKind>) -> SignalType {
    match op {
        OperatorClass::Cross => match role_kind_hint {
            Some(RoleKind::Smoother) => SignalType::MaCross,
            Some(RoleKind::OscillatorUnbounded) => SignalType::MacdCross,
            Some(RoleKind::OscillatorBounded) => SignalType::StochCross,
            _ => SignalType::MaCross,
        },

        OperatorClass::ThresholdCompare => match role_kind_hint {
            Some(RoleKind::OscillatorBounded) => SignalType::RsiReversal,
            Some(RoleKind::TrendStrength) => SignalType::AdxTrend,
            Some(RoleKind::OscillatorUnbounded) => SignalType::MacdHistogram,
            _ => SignalType::RsiReversal,
        },

        OperatorClass::ZoneEnter | OperatorClass::ZoneExit => match role_kind_hint {
            Some(RoleKind::Channel) => SignalType::BollingerBreak,
            _ => SignalType::BollingerBreak,
        },

        OperatorClass::NBarExtreme => SignalType::Breakout,

        OperatorClass::Pivot => SignalType::ZigzagReversal,

        OperatorClass::Divergence => SignalType::RsiDivergence,

        OperatorClass::CandlePattern => SignalType::PriceAction,

        OperatorClass::RegimeGate => SignalType::HtfFilter,

        OperatorClass::Direction => match role_kind_hint {
            Some(RoleKind::OscillatorBounded) | Some(RoleKind::OscillatorUnbounded) => {
                SignalType::Momentum
            }
            _ => SignalType::TrendFollow,
        },

        OperatorClass::Sequence => SignalType::PriceAction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_smoother_is_ma_cross() {
        assert_eq!(
            signal_type_for(OperatorClass::Cross, Some(RoleKind::Smoother)),
            SignalType::MaCross
        );
    }

    #[test]
    fn threshold_bounded_is_rsi_reversal() {
        assert_eq!(
            signal_type_for(OperatorClass::ThresholdCompare, Some(RoleKind::OscillatorBounded)),
            SignalType::RsiReversal
        );
    }

    #[test]
    fn threshold_trend_strength_is_adx_trend() {
        assert_eq!(
            signal_type_for(OperatorClass::ThresholdCompare, Some(RoleKind::TrendStrength)),
            SignalType::AdxTrend
        );
    }

    #[test]
    fn nbar_extreme_is_breakout() {
        assert_eq!(
            signal_type_for(OperatorClass::NBarExtreme, None),
            SignalType::Breakout
        );
    }

    #[test]
    fn pivot_is_zigzag_reversal() {
        assert_eq!(
            signal_type_for(OperatorClass::Pivot, None),
            SignalType::ZigzagReversal
        );
    }

    #[test]
    fn divergence_is_rsi_divergence() {
        assert_eq!(
            signal_type_for(OperatorClass::Divergence, None),
            SignalType::RsiDivergence
        );
    }

    #[test]
    fn candle_pattern_is_price_action() {
        assert_eq!(
            signal_type_for(OperatorClass::CandlePattern, None),
            SignalType::PriceAction
        );
    }

    #[test]
    fn regime_gate_is_htf_filter() {
        assert_eq!(
            signal_type_for(OperatorClass::RegimeGate, None),
            SignalType::HtfFilter
        );
    }
}
