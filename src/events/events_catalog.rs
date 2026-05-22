//! Catalog of all 18 event/detector primitives.
//!
//! Each `signature_xxx()` function returns an `EventSignature` with sensible
//! defaults. `get_signature(id)` resolves by id or alias.

use crate::catalog::data::event_signature::EventSignature;
use crate::catalog::data::indicator_signature::{IndicatorRoleKind, SourceType};
use crate::catalog::{ParamConstraint};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use super::event_id::EventId;

// ── Individual signature constructors ─────────────────────────────────────────

pub fn signature_bos_event_detector() -> EventSignature {
    EventSignature::builder("bos_event_detector")
        .name("Break of Structure")
        .description("Detects rolling-extremum breakouts (BOS Up/Down) over a lookback window.")
        .machine_id(EventId::BosEventDetector)
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .role_kind(IndicatorRoleKind::Level)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .alias("bos")
        .alias("boseventdetector")
        .build()
}

pub fn signature_candle_pattern() -> EventSignature {
    EventSignature::builder("candle_pattern")
        .name("Candle Pattern Detector")
        .description("Detects any of 34 candlestick patterns. Set string_param 'kind' to the pattern name (default: 'doji').")
        .machine_id(EventId::CandlePattern)
        .role_kind(IndicatorRoleKind::Pattern)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .metadata("default_kind", "doji")
        .alias("candlepattern")
        .alias("candlepatterndetector")
        .build()
}

pub fn signature_confluence() -> EventSignature {
    EventSignature::builder("confluence")
        .name("Confluence")
        .description("Combines N inner indicators into one composite signal. Mode: all | any | majority | sum. Requires inner_indicators.")
        .machine_id(EventId::Confluence)
        .role_kind(IndicatorRoleKind::Other)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceAndVolume)
        .metadata("default_mode", "all")
        .build()
}

pub fn signature_direction_detector() -> EventSignature {
    EventSignature::builder("direction_detector")
        .name("Direction Detector")
        .description("Fires on each bar where close changes direction vs previous bar.")
        .machine_id(EventId::DirectionDetector)
        .role_kind(IndicatorRoleKind::Other)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .alias("direction")
        .alias("directiondetector")
        .build()
}

pub fn signature_divergence() -> EventSignature {
    EventSignature::builder("divergence")
        .name("Divergence")
        .description("Price/oscillator divergence detector. Requires 1 inner indicator (oscillator). string_param 'kind': regular | hidden.")
        .machine_id(EventId::Divergence)
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .metadata("default_kind", "regular")
        .metadata("inner_count", "1")
        .build()
}

pub fn signature_fvg_event_detector() -> EventSignature {
    EventSignature::builder("fvg_event_detector")
        .name("Fair Value Gap")
        .description("Detects 3-bar imbalance (bullish/bearish FVG).")
        .machine_id(EventId::FvgEventDetector)
        .role_kind(IndicatorRoleKind::Level)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .alias("fvg")
        .alias("fvgeventdetector")
        .build()
}

pub fn signature_line_cross() -> EventSignature {
    EventSignature::builder("line_cross")
        .name("Line Cross")
        .description("Two-line crossover detector. inner_indicators[0]=left, [1]=right (or use 'left_value'/'right_value' params). string_param 'mode': momentary | sticky.")
        .machine_id(EventId::LineCross)
        .role_kind(IndicatorRoleKind::Other)
        .output_kind(IndicatorValueKind::Triple)
        .source_type(SourceType::PriceAndVolume)
        .metadata("default_mode", "momentary")
        .alias("linecross")
        .build()
}

pub fn signature_oscillator_with_divergence() -> EventSignature {
    EventSignature::builder("oscillator_with_divergence")
        .name("Oscillator With Divergence")
        .description("Wraps any oscillator with swing-point divergence detection. Requires 1 inner indicator. Optional ATR at inner[1].")
        .machine_id(EventId::OscillatorWithDivergence)
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Double)
        .source_type(SourceType::PriceAndVolume)
        .metadata("inner_count", "1")
        .alias("oscillatorwithdivergence")
        .build()
}

pub fn signature_oscillator_with_volume_weight() -> EventSignature {
    EventSignature::builder("oscillator_with_volume_weight")
        .name("Oscillator With Volume Weight")
        .description("Wraps any oscillator with volume-event classification. Requires 1 inner indicator. Params: 'spike_threshold' (default 2.5).")
        .machine_id(EventId::OscillatorWithVolumeWeight)
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Double)
        .source_type(SourceType::PriceAndVolume)
        .metadata("inner_count", "1")
        .alias("oscillatorwithvolumeweight")
        .build()
}

pub fn signature_pivot() -> EventSignature {
    EventSignature::builder("pivot")
        .name("Pivot")
        .description("N-bar pivot high/low detector. periods[0]=left (default 5), periods[1]=right (default 5).")
        .machine_id(EventId::Pivot)
        .add_constraint(ParamConstraint::period(1, 100, 5))
        .add_constraint(ParamConstraint::period(1, 100, 5))
        .role_kind(IndicatorRoleKind::Level)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .build()
}

pub fn signature_price_line_cross() -> EventSignature {
    EventSignature::builder("price_line_cross")
        .name("Price Line Cross")
        .description("Price × Line crossover with touch mode. inner_indicators[0]=line or 'line_value' param. string_param 'touch_mode': close_above | close_below | wick_through | wick_reject | touch | with_candle.")
        .machine_id(EventId::PriceLineCross)
        .role_kind(IndicatorRoleKind::Other)
        .output_kind(IndicatorValueKind::Triple)
        .source_type(SourceType::PriceAndVolume)
        .metadata("default_touch_mode", "close_above")
        .alias("pricelinecross")
        .build()
}

pub fn signature_regime_gate() -> EventSignature {
    EventSignature::builder("regime_gate")
        .name("Regime Gate")
        .description("Fires on regime entry/exit transitions. Params: 'regime_threshold' (default 0.5), string_param 'direction': above | below.")
        .machine_id(EventId::RegimeGate)
        .role_kind(IndicatorRoleKind::Regime)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .metadata("default_direction", "above")
        .alias("regimegate")
        .build()
}

pub fn signature_relative_position() -> EventSignature {
    EventSignature::builder("relative_position")
        .name("Relative Position")
        .description("Persistent ±1 trend label based on subject vs reference indicator. Requires 2 inner indicators: [0]=subject, [1]=reference.")
        .machine_id(EventId::RelativePosition)
        .role_kind(IndicatorRoleKind::Regime)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceAndVolume)
        .metadata("inner_count", "2")
        .alias("relativeposition")
        .build()
}

pub fn signature_statistical_wick_detector() -> EventSignature {
    EventSignature::builder("statistical_wick_detector")
        .name("Statistical Wick Detector")
        .description("Flags upper/lower wick spikes at 95th percentile vs rolling window.")
        .machine_id(EventId::StatisticalWickDetector)
        .add_constraint(ParamConstraint::period(5, 500, 50))
        .role_kind(IndicatorRoleKind::Pattern)
        .output_kind(IndicatorValueKind::DoubleFlag)
        .source_type(SourceType::PriceOnly)
        .alias("statisticalwickdetector")
        .alias("wick_spike")
        .build()
}

pub fn signature_swing_detection() -> EventSignature {
    EventSignature::builder("swing_detection")
        .name("Swing Detection")
        .description("Detects swing high/low via configurable mode. string_param 'mode': percent | atr_multiple | n_bar_extreme | lookahead | time. Param 'threshold_pct' (default 1.0) for percent mode.")
        .machine_id(EventId::SwingDetection)
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .role_kind(IndicatorRoleKind::Level)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .metadata("default_mode", "percent")
        .metadata("default_threshold_pct", "1.0")
        .alias("swing")
        .alias("swingdetection")
        .build()
}

pub fn signature_threshold() -> EventSignature {
    EventSignature::builder("threshold")
        .name("Threshold")
        .description("Detects threshold crossings. string_param 'kind': above | below | in_range | out_of_range. Params 'upper' (default 70) and 'lower' (default 30).")
        .machine_id(EventId::Threshold)
        .role_kind(IndicatorRoleKind::OscillatorBounded)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .metadata("default_kind", "above")
        .build()
}

pub fn signature_volatility_regime_detector() -> EventSignature {
    EventSignature::builder("volatility_regime_detector")
        .name("Volatility Regime Detector")
        .description("Classifies ATR/StdDev into Low/Normal/High regimes and fires on transitions. Params: 'low_threshold' (default 0.5), 'high_threshold' (default 1.5).")
        .machine_id(EventId::VolatilityRegimeDetector)
        .role_kind(IndicatorRoleKind::Volatility)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::PriceOnly)
        .alias("volatilityregimedetector")
        .alias("volatility_regime")
        .build()
}

pub fn signature_volume_event_detector() -> EventSignature {
    EventSignature::builder("volume_event_detector")
        .name("Volume Event Detector")
        .description("Detects volume spikes as multiples of rolling mean. periods[0]=period (default 20), param 'multiplier' (default 2.0).")
        .machine_id(EventId::VolumeEventDetector)
        .add_constraint(ParamConstraint::period(2, 500, 20))
        .role_kind(IndicatorRoleKind::Volume)
        .output_kind(IndicatorValueKind::Signal)
        .source_type(SourceType::VolumeOnly)
        .alias("volumeeventdetector")
        .alias("volume_event")
        .build()
}

// ── Lookup ────────────────────────────────────────────────────────────────────

/// Get the event signature by id string or alias (case-insensitive).
///
/// Returns `None` if not found.
pub fn get_signature(id: &str) -> Option<EventSignature> {
    let lower = id.to_lowercase();
    let lower = lower.as_str();
    match lower {
        "bos_event_detector" | "bos" | "boseventdetector" => Some(signature_bos_event_detector()),
        "candle_pattern" | "candlepattern" | "candlepatterndetector" | "candle_pattern_detector" => {
            Some(signature_candle_pattern())
        }
        "confluence" => Some(signature_confluence()),
        "direction_detector" | "direction" | "directiondetector" => {
            Some(signature_direction_detector())
        }
        "divergence" => Some(signature_divergence()),
        "fvg_event_detector" | "fvg" | "fvgeventdetector" => Some(signature_fvg_event_detector()),
        "line_cross" | "linecross" => Some(signature_line_cross()),
        "oscillator_with_divergence" | "oscillatorwithdivergence" => {
            Some(signature_oscillator_with_divergence())
        }
        "oscillator_with_volume_weight" | "oscillatorwithvolumeweight" => {
            Some(signature_oscillator_with_volume_weight())
        }
        "pivot" => Some(signature_pivot()),
        "price_line_cross" | "pricelinecross" => Some(signature_price_line_cross()),
        "regime_gate" | "regimegate" => Some(signature_regime_gate()),
        "relative_position" | "relativeposition" => Some(signature_relative_position()),
        "statistical_wick_detector" | "statisticalwickdetector" | "wick_spike" => {
            Some(signature_statistical_wick_detector())
        }
        "swing_detection" | "swing" | "swingdetection" => Some(signature_swing_detection()),
        "threshold" => Some(signature_threshold()),
        "volatility_regime_detector" | "volatilityregimedetector" | "volatility_regime" => {
            Some(signature_volatility_regime_detector())
        }
        "volume_event_detector" | "volumeeventdetector" | "volume_event" => {
            Some(signature_volume_event_detector())
        }
        _ => None,
    }
}

/// Iterate all 18 event signatures.
pub fn all_signatures() -> impl Iterator<Item = EventSignature> {
    [
        signature_bos_event_detector(),
        signature_candle_pattern(),
        signature_confluence(),
        signature_direction_detector(),
        signature_divergence(),
        signature_fvg_event_detector(),
        signature_line_cross(),
        signature_oscillator_with_divergence(),
        signature_oscillator_with_volume_weight(),
        signature_pivot(),
        signature_price_line_cross(),
        signature_regime_gate(),
        signature_relative_position(),
        signature_statistical_wick_detector(),
        signature_swing_detection(),
        signature_threshold(),
        signature_volatility_regime_detector(),
        signature_volume_event_detector(),
    ]
    .into_iter()
}
