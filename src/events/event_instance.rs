//! EventInstance — enum wrapper around all 18 event/detector primitives.
//!
//! Mirrors `IndicatorInstance` from `bar_indicators/instance_factory.rs`.
//! Provides `create()`, `update_bar()`, `value()`, `is_ready()`, `reset()`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;

use super::bos_event_detector::BosEventDetector;
use super::candle_pattern::{CandlePatternDetector, CandlePatternKind};
use super::confluence::{Confluence, ConfluenceMode};
use super::cross_asset_beta::CrossAssetBeta;
use super::direction_detector::DirectionDetector;
use super::divergence::{Divergence, DivergenceKind};
use super::fvg_event_detector::FvgEventDetector;
use super::pairs_cointegration_proxy::PairsCointegrationProxy;
use super::relative_strength_cross::RelativeStrengthCross;
use super::line_cross::{CrossMode, LineCross, LineSource as LineCrossSource};
use super::oscillator_with_divergence::OscillatorWithDivergence;
use super::oscillator_with_volume_weight::OscillatorWithVolumeWeight;
use super::pivot::Pivot;
use super::price_line_cross::{LineSource as PriceLineSource, PriceLineCross, TouchMode};
use super::regime_gate::{GateDirection, RegimeGate};
use super::relative_position::RelativePosition;
use super::statistical_wick_detector::StatisticalWickDetector;
use super::swing_detection::{SwingDetection, SwingMode};
use super::threshold::{Threshold, ThresholdKind};
use super::volatility_regime::VolatilityRegimeDetector;
use super::volume_event::VolumeEventDetector;
use super::event_config::EventConfig;
use super::event_id::EventId;

/// Enum wrapper around every event/detector primitive.
///
/// All variants are `Box<T>` to keep the enum size small.
#[derive(Clone)]
pub enum EventInstance {
    BosEventDetector(Box<BosEventDetector>),
    CandlePattern(Box<CandlePatternDetector>),
    Confluence(Box<Confluence>),
    /// Rolling cross-asset beta. Primary via `update_bar`; secondary via `update_secondary_bar`.
    CrossAssetBeta(Box<CrossAssetBeta>),
    /// DirectionDetector operates on a scalar; `update_bar` feeds close price.
    DirectionDetector(Box<DirectionDetector>),
    Divergence(Box<Divergence>),
    FvgEventDetector(Box<FvgEventDetector>),
    /// Cointegration proxy (spread z-score). Primary via `update_bar`; secondary via `update_secondary_bar`.
    PairsCointegrationProxy(Box<PairsCointegrationProxy>),
    LineCross(Box<LineCross>),
    OscillatorWithDivergence(Box<OscillatorWithDivergence>),
    OscillatorWithVolumeWeight(Box<OscillatorWithVolumeWeight>),
    /// Pivot operates on a scalar; `update_bar` feeds close price.
    Pivot(Box<Pivot>),
    PriceLineCross(Box<PriceLineCross>),
    /// RegimeGate operates on a scalar; `update_bar` feeds close price.
    RegimeGate(Box<RegimeGate>),
    RelativePosition(Box<RelativePosition>),
    /// Relative strength cross. Primary via `update_bar`; secondary via `update_secondary_bar`.
    RelativeStrengthCross(Box<RelativeStrengthCross>),
    StatisticalWickDetector(Box<StatisticalWickDetector>),
    SwingDetection(Box<SwingDetection>),
    /// Threshold operates on a scalar; `update_bar` feeds close price (or the
    /// inner indicator's main scalar when one is configured).
    Threshold(Box<Threshold>, Option<Box<IndicatorInstance>>),
    /// VolatilityRegimeDetector operates on a scalar; `update_bar` feeds close
    /// (or the inner indicator's main scalar when one is configured — typically
    /// an ATR/StdDev so thresholds match the indicator's natural scale).
    VolatilityRegimeDetector(Box<VolatilityRegimeDetector>, Option<Box<IndicatorInstance>>),
    /// VolumeEventDetector operates on volume; `update_bar` feeds volume parameter.
    VolumeEventDetector(Box<VolumeEventDetector>),
}

impl EventInstance {
    /// Construct an event instance from its config.
    ///
    /// Returns `Err(String)` when a required parameter is missing or a string
    /// parameter contains an unrecognised variant name.
    pub fn create(config: &EventConfig) -> Result<Self, String> {
        match config.id {
            // ── BosEventDetector ─────────────────────────────────────────────
            EventId::BosEventDetector => {
                let lookback = config.period_or(14);
                Ok(Self::BosEventDetector(Box::new(BosEventDetector::new(lookback))))
            }

            // ── CandlePattern ────────────────────────────────────────────────
            EventId::CandlePattern => {
                let kind_str = config.str_param_or("kind", "doji");
                let kind = parse_candle_kind(kind_str)
                    .ok_or_else(|| format!("unknown CandlePatternKind: {kind_str}"))?;
                Ok(Self::CandlePattern(Box::new(CandlePatternDetector::new(kind))))
            }

            // ── Confluence ───────────────────────────────────────────────────
            EventId::Confluence => {
                if config.inner_indicators.is_empty() {
                    return Err("Confluence requires at least 1 inner indicator".into());
                }
                let inners: Result<Vec<IndicatorInstance>, String> = config
                    .inner_indicators
                    .iter()
                    .map(|cfg| IndicatorInstance::create(cfg))
                    .collect();
                let mode = parse_confluence_mode(config)?;
                Ok(Self::Confluence(Box::new(Confluence::new(inners?, mode))))
            }

            // ── DirectionDetector ────────────────────────────────────────────
            EventId::DirectionDetector => {
                Ok(Self::DirectionDetector(Box::new(DirectionDetector::new())))
            }

            // ── Divergence ───────────────────────────────────────────────────
            EventId::Divergence => {
                let inner_cfg = config
                    .inner_indicators
                    .first()
                    .ok_or("Divergence requires 1 inner indicator")?;
                let inner = IndicatorInstance::create(inner_cfg)?;
                let lookback = config.period_or(14);
                let kind = parse_divergence_kind(config.str_param_or("kind", "regular"))
                    .ok_or_else(|| format!("unknown DivergenceKind: {}", config.str_param_or("kind", "regular")))?;
                Ok(Self::Divergence(Box::new(Divergence::new(inner, lookback, kind))))
            }

            // ── FvgEventDetector ─────────────────────────────────────────────
            EventId::FvgEventDetector => {
                Ok(Self::FvgEventDetector(Box::new(FvgEventDetector::new())))
            }

            // ── LineCross ────────────────────────────────────────────────────
            EventId::LineCross => {
                let left = build_line_cross_source(config, "left", 0)?;
                let right = build_line_cross_source(config, "right", 1)?;
                let mode_str = config.str_param_or("mode", "momentary");
                let mode = match mode_str {
                    "sticky" => CrossMode::Sticky,
                    _ => CrossMode::Momentary,
                };
                Ok(Self::LineCross(Box::new(LineCross::new(left, right, mode))))
            }

            // ── OscillatorWithDivergence ─────────────────────────────────────
            EventId::OscillatorWithDivergence => {
                let inner_cfg = config
                    .inner_indicators
                    .first()
                    .ok_or("OscillatorWithDivergence requires 1 inner indicator")?;
                let inner = Box::new(IndicatorInstance::create(inner_cfg)?);
                let swing_lookback = config.period_or(14);
                let detect_regular = config.flag_or("detect_regular", true);
                let detect_hidden = config.flag_or("detect_hidden", true);
                let with_strength = config.flag_or("with_strength", false);
                // Optional ATR at inner[1]
                let atr = config.inner_indicators.get(1).map(|cfg| {
                    IndicatorInstance::create(cfg).map(Box::new)
                }).transpose()?;
                Ok(Self::OscillatorWithDivergence(Box::new(
                    OscillatorWithDivergence::new(inner, swing_lookback, detect_regular, detect_hidden, with_strength, atr),
                )))
            }

            // ── OscillatorWithVolumeWeight ────────────────────────────────────
            EventId::OscillatorWithVolumeWeight => {
                let inner_cfg = config
                    .inner_indicators
                    .first()
                    .ok_or("OscillatorWithVolumeWeight requires 1 inner indicator")?;
                let inner = Box::new(IndicatorInstance::create(inner_cfg)?);
                let baseline_period = config.period_or(14);
                let spike_threshold = config.param_or("spike_threshold", 2.5);
                let with_volume_event = config.flag_or("with_volume_event", true);
                let with_strength = config.flag_or("with_strength", false);
                Ok(Self::OscillatorWithVolumeWeight(Box::new(
                    OscillatorWithVolumeWeight::new(inner, baseline_period, spike_threshold, with_volume_event, with_strength),
                )))
            }

            // ── Pivot ────────────────────────────────────────────────────────
            EventId::Pivot => {
                let left = config.period_or(5);
                let right = config.period_n_or(1, 5);
                Ok(Self::Pivot(Box::new(Pivot::new(left, right))))
            }

            // ── PriceLineCross ───────────────────────────────────────────────
            EventId::PriceLineCross => {
                let line = build_price_line_source(config)?;
                let mode = parse_touch_mode(config)?;
                Ok(Self::PriceLineCross(Box::new(PriceLineCross::new(line, mode))))
            }

            // ── RegimeGate ───────────────────────────────────────────────────
            EventId::RegimeGate => {
                let threshold = config.param_or("regime_threshold", 0.5);
                let dir_str = config.str_param_or("direction", "above");
                let direction = match dir_str {
                    "below" => GateDirection::Below,
                    _ => GateDirection::Above,
                };
                Ok(Self::RegimeGate(Box::new(RegimeGate::new(threshold, direction))))
            }

            // ── RelativePosition ─────────────────────────────────────────────
            EventId::RelativePosition => {
                let subj_cfg = config
                    .inner_indicators
                    .first()
                    .ok_or("RelativePosition requires 2 inner indicators (subject, reference)")?;
                let ref_cfg = config
                    .inner_indicators
                    .get(1)
                    .ok_or("RelativePosition requires 2 inner indicators (subject, reference)")?;
                let subject = IndicatorInstance::create(subj_cfg)?;
                let reference = IndicatorInstance::create(ref_cfg)?;
                Ok(Self::RelativePosition(Box::new(RelativePosition::new(subject, reference))))
            }

            // ── StatisticalWickDetector ──────────────────────────────────────
            EventId::StatisticalWickDetector => {
                let window = config.period_or(50);
                Ok(Self::StatisticalWickDetector(Box::new(StatisticalWickDetector::new(window))))
            }

            // ── SwingDetection ───────────────────────────────────────────────
            EventId::SwingDetection => {
                let mode = parse_swing_mode(config)?;
                Ok(Self::SwingDetection(Box::new(SwingDetection::new(mode))))
            }

            // ── Threshold ────────────────────────────────────────────────────
            EventId::Threshold => {
                let upper = config.param_or("upper", 70.0);
                let lower = config.param_or("lower", 30.0);
                let kind_str = config.str_param_or("kind", "above");
                let kind = parse_threshold_kind(kind_str)
                    .ok_or_else(|| format!("unknown ThresholdKind: {kind_str}"))?;
                let inner = match config.inner_indicators.first() {
                    Some(cfg) => Some(Box::new(IndicatorInstance::create(cfg)?)),
                    None => None,
                };
                Ok(Self::Threshold(Box::new(Threshold::new(kind, upper, lower)), inner))
            }

            // ── VolatilityRegimeDetector ─────────────────────────────────────
            EventId::VolatilityRegimeDetector => {
                let low = config.param_or("low_threshold", 0.5);
                let high = config.param_or("high_threshold", 1.5);
                let inner = match config.inner_indicators.first() {
                    Some(cfg) => Some(Box::new(IndicatorInstance::create(cfg)?)),
                    None => None,
                };
                Ok(Self::VolatilityRegimeDetector(Box::new(VolatilityRegimeDetector::new(low, high)), inner))
            }

            // ── VolumeEventDetector ──────────────────────────────────────────
            EventId::VolumeEventDetector => {
                let period = config.period_or(20);
                let multiplier = config.param_or("multiplier", 2.0);
                Ok(Self::VolumeEventDetector(Box::new(VolumeEventDetector::new(period, multiplier))))
            }

            // ── CrossAssetBeta ────────────────────────────────────────────────
            EventId::CrossAssetBeta => {
                let window = config.period_or(50);
                Ok(Self::CrossAssetBeta(Box::new(CrossAssetBeta::new(window))))
            }

            // ── PairsCointegrationProxy ───────────────────────────────────────
            EventId::PairsCointegrationProxy => {
                let window = config.period_or(50);
                Ok(Self::PairsCointegrationProxy(Box::new(PairsCointegrationProxy::new(window))))
            }

            // ── RelativeStrengthCross ─────────────────────────────────────────
            EventId::RelativeStrengthCross => {
                let window = config.period_or(50);
                Ok(Self::RelativeStrengthCross(Box::new(RelativeStrengthCross::new(window))))
            }
        }
    }

    /// Feed one OHLCV bar and return the event's output as `IndicatorValue`.
    ///
    /// Normalisation rules for primitives that return non-`IndicatorValue` types:
    /// - `f64` → `IndicatorValue::Single(v)`
    /// - `i8`  → `IndicatorValue::Signal(v)`
    /// - `(bool, bool)` (StatisticalWickDetector) → `IndicatorValue::DoubleFlag(a, b)`
    ///
    /// Primitives that operate on a scalar (DirectionDetector, RegimeGate, Threshold,
    /// VolatilityRegimeDetector) receive `close` as their scalar input.
    /// VolumeEventDetector receives `volume` as its scalar input.
    /// Pivot receives `close` as its scalar input.
    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> IndicatorValue {
        match self {
            Self::BosEventDetector(d) => d.update_bar(open, high, low, close, volume),
            Self::CandlePattern(d) => d.update_bar(open, high, low, close, volume),
            Self::CrossAssetBeta(d) => d.update_bar(open, high, low, close, volume),
            Self::PairsCointegrationProxy(d) => d.update_bar(open, high, low, close, volume),
            Self::RelativeStrengthCross(d) => d.update_bar(open, high, low, close, volume),
            Self::Confluence(d) => {
                let v = d.update_bar(open, high, low, close, volume);
                IndicatorValue::Single(v)
            }
            Self::DirectionDetector(d) => {
                // Feed close price as the scalar direction input.
                // Returns Option<...> — map to Signal(+1/-1/0).
                let signal = match d.detect_from_values(close) {
                    Some((_, crate::core::signal::direction::Direction::Up)) => 1i8,
                    Some((_, crate::core::signal::direction::Direction::Down)) => -1i8,
                    _ => 0i8,
                };
                IndicatorValue::Signal(signal)
            }
            Self::Divergence(d) => {
                let v = d.update_bar(open, high, low, close, volume);
                IndicatorValue::Single(v)
            }
            Self::FvgEventDetector(d) => d.update_bar(open, high, low, close, volume),
            Self::LineCross(d) => d.update_bar(open, high, low, close, volume),
            Self::OscillatorWithDivergence(d) => d.update_bar(open, high, low, close, volume),
            Self::OscillatorWithVolumeWeight(d) => d.update_bar(open, high, low, close, volume),
            Self::Pivot(d) => {
                // Pivot operates on a scalar value; use close price.
                match d.detect_from_values(close) {
                    Some((_, crate::core::signal::direction::Direction::Up)) => IndicatorValue::Signal(1),
                    Some((_, crate::core::signal::direction::Direction::Down)) => IndicatorValue::Signal(-1),
                    _ => IndicatorValue::Signal(0),
                }
            }
            Self::PriceLineCross(d) => d.update_bar(open, high, low, close, volume),
            Self::RegimeGate(d) => {
                // RegimeGate operates on a scalar regime value; use close price.
                match d.detect_from_values(close) {
                    Some((_, crate::core::signal::direction::Direction::Up)) => IndicatorValue::Signal(1),
                    Some((_, crate::core::signal::direction::Direction::Down)) => IndicatorValue::Signal(-1),
                    _ => IndicatorValue::Signal(0),
                }
            }
            Self::RelativePosition(d) => {
                let v = d.update_bar(open, high, low, close, volume);
                IndicatorValue::Signal(v)
            }
            Self::StatisticalWickDetector(d) => {
                let (upper, lower) = d.update_bar(open, high, low, close, volume);
                IndicatorValue::DoubleFlag(upper, lower)
            }
            Self::SwingDetection(d) => {
                let v = d.update_bar(open, high, low, close, volume);
                IndicatorValue::Single(v)
            }
            Self::Threshold(d, inner) => {
                // Optional inner indicator (e.g. RSI) normalizes raw close into
                // a scalar whose range matches the configured upper/lower band.
                // Without an inner, fall back to feeding raw close directly.
                let scalar = match inner.as_deref_mut() {
                    Some(ind) => {
                        let v = ind.update_bar(open, high, low, close, volume, None);
                        if !ind.is_ready() { return IndicatorValue::Signal(0); }
                        v.main()
                    }
                    None => close,
                };
                match d.detect_from_values(scalar) {
                    Some((_, crate::core::signal::direction::Direction::Up)) => IndicatorValue::Signal(1),
                    Some((_, crate::core::signal::direction::Direction::Down)) => IndicatorValue::Signal(-1),
                    _ => IndicatorValue::Signal(0),
                }
            }
            Self::VolatilityRegimeDetector(d, inner) => {
                // Optional inner indicator (e.g. ATR) provides the natural
                // volatility scalar; without it we'd compare raw close to
                // tiny thresholds which never calibrate across spot drift.
                let scalar = match inner.as_deref_mut() {
                    Some(ind) => {
                        let v = ind.update_bar(open, high, low, close, volume, None);
                        if !ind.is_ready() { return IndicatorValue::Signal(0); }
                        v.main()
                    }
                    None => close,
                };
                match d.detect_from_values(scalar) {
                    Some((_, crate::core::signal::direction::Direction::Up)) => IndicatorValue::Signal(1),
                    Some((_, crate::core::signal::direction::Direction::Down)) => IndicatorValue::Signal(-1),
                    _ => IndicatorValue::Signal(0),
                }
            }
            Self::VolumeEventDetector(d) => {
                // VolumeEventDetector operates on volume.
                match d.detect_from_values(volume) {
                    Some(_) => IndicatorValue::Signal(1),
                    None => IndicatorValue::Signal(0),
                }
            }
        }
    }

    /// Return the last computed value without advancing state.
    pub fn value(&self) -> IndicatorValue {
        match self {
            Self::BosEventDetector(d) => d.value(),
            Self::CandlePattern(d) => d.value(),
            Self::Confluence(d) => d.value(),
            Self::CrossAssetBeta(d) => d.indicator_value(),
            Self::PairsCointegrationProxy(d) => d.indicator_value(),
            Self::RelativeStrengthCross(d) => d.indicator_value(),
            Self::DirectionDetector(_) => IndicatorValue::Signal(0),
            Self::Divergence(d) => d.value(),
            Self::FvgEventDetector(d) => d.value(),
            Self::LineCross(d) => d.value(),
            Self::OscillatorWithDivergence(d) => d.value(),
            Self::OscillatorWithVolumeWeight(d) => d.value(),
            Self::Pivot(_) => IndicatorValue::Signal(0),
            Self::PriceLineCross(d) => d.value(),
            Self::RegimeGate(_) => IndicatorValue::Signal(0),
            Self::RelativePosition(d) => d.value(),
            Self::StatisticalWickDetector(d) => d.value(),
            Self::SwingDetection(d) => d.value(),
            Self::Threshold(_, _) => IndicatorValue::Signal(0),
            Self::VolatilityRegimeDetector(_, _) => IndicatorValue::Signal(0),
            Self::VolumeEventDetector(_) => IndicatorValue::Signal(0),
        }
    }

    /// True once the event has consumed enough bars to produce meaningful output.
    pub fn is_ready(&self) -> bool {
        match self {
            Self::BosEventDetector(d) => d.is_ready(),
            Self::CandlePattern(d) => d.is_ready(),
            Self::Confluence(d) => d.is_ready(),
            Self::CrossAssetBeta(d) => d.indicator_is_ready(),
            Self::PairsCointegrationProxy(d) => d.indicator_is_ready(),
            Self::RelativeStrengthCross(d) => d.indicator_is_ready(),
            Self::DirectionDetector(_) => true,
            Self::Divergence(d) => d.is_ready(),
            Self::FvgEventDetector(d) => d.is_ready(),
            Self::LineCross(d) => d.is_ready(),
            Self::OscillatorWithDivergence(d) => d.is_ready(),
            Self::OscillatorWithVolumeWeight(d) => d.is_ready(),
            Self::Pivot(_) => true,
            Self::PriceLineCross(d) => d.is_ready(),
            Self::RegimeGate(_) => true,
            Self::RelativePosition(d) => d.is_ready(),
            Self::StatisticalWickDetector(d) => d.is_ready(),
            Self::SwingDetection(d) => d.is_ready(),
            Self::Threshold(_, inner) => inner.as_deref().map(|i| i.is_ready()).unwrap_or(true),
            Self::VolatilityRegimeDetector(_, inner) => inner.as_deref().map(|i| i.is_ready()).unwrap_or(true),
            Self::VolumeEventDetector(_) => true,
        }
    }

    /// Reset all internal state.
    pub fn reset(&mut self) {
        match self {
            Self::BosEventDetector(d) => d.reset(),
            Self::CandlePattern(d) => d.reset(),
            Self::Confluence(d) => d.reset(),
            Self::CrossAssetBeta(d) => d.indicator_reset(),
            Self::PairsCointegrationProxy(d) => d.indicator_reset(),
            Self::RelativeStrengthCross(d) => d.indicator_reset(),
            Self::DirectionDetector(d) => d.reset(),
            Self::Divergence(d) => d.reset(),
            Self::FvgEventDetector(d) => d.reset(),
            Self::LineCross(d) => d.reset(),
            Self::OscillatorWithDivergence(d) => d.reset(),
            Self::OscillatorWithVolumeWeight(d) => d.reset(),
            Self::Pivot(d) => d.reset(),
            Self::PriceLineCross(d) => d.reset(),
            Self::RegimeGate(d) => d.reset(),
            Self::RelativePosition(d) => d.reset(),
            Self::StatisticalWickDetector(d) => d.reset(),
            Self::SwingDetection(d) => d.reset(),
            Self::Threshold(d, inner) => { d.reset(); if let Some(i) = inner.as_deref_mut() { i.reset(); } }
            Self::VolatilityRegimeDetector(d, inner) => { d.reset(); if let Some(i) = inner.as_deref_mut() { i.reset(); } }
            Self::VolumeEventDetector(d) => d.reset(),
        }
    }

    /// Feed a secondary-symbol bar to multi-symbol events.
    ///
    /// Only `CrossAssetBeta`, `PairsCointegrationProxy`, and `RelativeStrengthCross`
    /// consume this call. All other events ignore it (no-op).
    pub fn update_secondary_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
        ts_ms: i64,
    ) {
        match self {
            Self::CrossAssetBeta(x) => { x.update_secondary_price(close, ts_ms); }
            Self::PairsCointegrationProxy(x) => { x.update_secondary_price(close, ts_ms); }
            Self::RelativeStrengthCross(x) => { x.update_secondary_price(close, ts_ms); }
            _ => {}
        }
    }
}

impl std::fmt::Debug for EventInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // IndicatorInstance does not implement Debug, so we elide inner indicator
        // contents and emit only the variant name + a hint when an inner is set.
        let name = match self {
            Self::BosEventDetector(_) => "BosEventDetector",
            Self::CandlePattern(_) => "CandlePattern",
            Self::Confluence(_) => "Confluence",
            Self::CrossAssetBeta(_) => "CrossAssetBeta",
            Self::DirectionDetector(_) => "DirectionDetector",
            Self::Divergence(_) => "Divergence",
            Self::FvgEventDetector(_) => "FvgEventDetector",
            Self::PairsCointegrationProxy(_) => "PairsCointegrationProxy",
            Self::LineCross(_) => "LineCross",
            Self::OscillatorWithDivergence(_) => "OscillatorWithDivergence",
            Self::OscillatorWithVolumeWeight(_) => "OscillatorWithVolumeWeight",
            Self::Pivot(_) => "Pivot",
            Self::PriceLineCross(_) => "PriceLineCross",
            Self::RegimeGate(_) => "RegimeGate",
            Self::RelativePosition(_) => "RelativePosition",
            Self::RelativeStrengthCross(_) => "RelativeStrengthCross",
            Self::StatisticalWickDetector(_) => "StatisticalWickDetector",
            Self::SwingDetection(_) => "SwingDetection",
            Self::Threshold(_, inner) => {
                return write!(f, "Threshold {{ inner: {} }}", if inner.is_some() { "yes" } else { "no" });
            }
            Self::VolatilityRegimeDetector(_, inner) => {
                return write!(f, "VolatilityRegimeDetector {{ inner: {} }}", if inner.is_some() { "yes" } else { "no" });
            }
            Self::VolumeEventDetector(_) => "VolumeEventDetector",
        };
        f.write_str(name)
    }
}

// ── Private parse helpers ─────────────────────────────────────────────────────

fn parse_candle_kind(s: &str) -> Option<CandlePatternKind> {
    match s.to_lowercase().as_str() {
        "doji" => Some(CandlePatternKind::Doji),
        "gravestone_doji" | "gravestonedoji" => Some(CandlePatternKind::GravestoneDoji),
        "dragonfly_doji" | "dragonflydoji" => Some(CandlePatternKind::DragonflyDoji),
        "long_legged_doji" | "longleggeddoji" => Some(CandlePatternKind::LongLeggedDoji),
        "hammer" => Some(CandlePatternKind::Hammer),
        "inverted_hammer" | "invertedhammer" => Some(CandlePatternKind::InvertedHammer),
        "shooting_star" | "shootingstar" => Some(CandlePatternKind::ShootingStar),
        "hanging_man" | "hangingman" => Some(CandlePatternKind::HangingMan),
        "marubozu" => Some(CandlePatternKind::Marubozu),
        "white_marubozu" | "whitemarubozu" => Some(CandlePatternKind::WhiteMarubozu),
        "black_marubozu" | "blackmarubozu" => Some(CandlePatternKind::BlackMarubozu),
        "spinning_top" | "spinningtop" => Some(CandlePatternKind::SpinningTop),
        "bullish_engulfing" | "bullishengulfing" => Some(CandlePatternKind::BullishEngulfing),
        "bearish_engulfing" | "bearishengulfing" => Some(CandlePatternKind::BearishEngulfing),
        "bullish_harami" | "bullishharami" => Some(CandlePatternKind::BullishHarami),
        "bearish_harami" | "bearishharami" => Some(CandlePatternKind::BearishHarami),
        "piercing_pattern" | "piercingpattern" | "piercing" => Some(CandlePatternKind::PiercingPattern),
        "dark_cloud_cover" | "darkcloudcover" => Some(CandlePatternKind::DarkCloudCover),
        "tweezer_top" | "tweezertop" => Some(CandlePatternKind::TweezerTop),
        "tweezer_bottom" | "tweezerbottom" => Some(CandlePatternKind::TweezerBottom),
        "morning_star" | "morningstar" => Some(CandlePatternKind::MorningStar),
        "evening_star" | "eveningstar" => Some(CandlePatternKind::EveningStar),
        "morning_doji_star" | "morningdojistar" => Some(CandlePatternKind::MorningDojiStar),
        "evening_doji_star" | "eveningdojistar" => Some(CandlePatternKind::EveningDojiStar),
        "three_white_soldiers" | "threewhitesoldiers" => Some(CandlePatternKind::ThreeWhiteSoldiers),
        "three_black_crows" | "threeblackcrows" => Some(CandlePatternKind::ThreeBlackCrows),
        "three_inside_up" | "threeinsideup" => Some(CandlePatternKind::ThreeInsideUp),
        "three_inside_down" | "threeinsidedown" => Some(CandlePatternKind::ThreeInsideDown),
        "three_outside_up" | "threeoutsideup" => Some(CandlePatternKind::ThreeOutsideUp),
        "three_outside_down" | "threeoutsidedown" => Some(CandlePatternKind::ThreeOutsideDown),
        "rising_three_methods" | "risingthreemethods" => Some(CandlePatternKind::RisingThreeMethods),
        "falling_three_methods" | "fallingthreemethods" => Some(CandlePatternKind::FallingThreeMethods),
        "upside_gap_two_crows" | "upsidegaptwocrows" => Some(CandlePatternKind::UpsideGapTwoCrows),
        "downside_gap_three_methods" | "downsidegapthreemethods" => Some(CandlePatternKind::DownsideGapThreeMethods),
        _ => None,
    }
}

fn parse_divergence_kind(s: &str) -> Option<DivergenceKind> {
    match s.to_lowercase().as_str() {
        "regular" => Some(DivergenceKind::Regular),
        "hidden" => Some(DivergenceKind::Hidden),
        _ => None,
    }
}

fn parse_confluence_mode(config: &EventConfig) -> Result<ConfluenceMode, String> {
    let mode_str = config.str_param_or("mode", "all");
    match mode_str {
        "all" => Ok(ConfluenceMode::All),
        "any" => Ok(ConfluenceMode::Any),
        "majority" => Ok(ConfluenceMode::Majority),
        "sum" => {
            let threshold = config.param_or("threshold", 1.0) as i32;
            Ok(ConfluenceMode::Sum { threshold })
        }
        other => Err(format!("unknown ConfluenceMode: {other}")),
    }
}

fn parse_swing_mode(config: &EventConfig) -> Result<SwingMode, String> {
    let mode_str = config.str_param_or("mode", "percent");
    match mode_str {
        "percent" => {
            let t = config.param_or("threshold_pct", 1.0);
            Ok(SwingMode::Percent { threshold_pct: t })
        }
        "atr_multiple" | "atrmultiple" => {
            let mult = config.param_or("mult", 1.5);
            Ok(SwingMode::AtrMultiple { mult })
        }
        "n_bar_extreme" | "nbarextreme" => {
            let n = config.period_or(5);
            Ok(SwingMode::NBarExtreme { n })
        }
        "lookahead" => {
            let n = config.period_or(3);
            Ok(SwingMode::Lookahead { n })
        }
        "time" => {
            let n_bars = config.period_or(10);
            Ok(SwingMode::Time { n_bars })
        }
        other => Err(format!("unknown SwingMode: {other}")),
    }
}

fn parse_threshold_kind(s: &str) -> Option<ThresholdKind> {
    match s.to_lowercase().as_str() {
        "above" => Some(ThresholdKind::Above),
        "below" => Some(ThresholdKind::Below),
        "in_range" | "inrange" => Some(ThresholdKind::InRange),
        "out_of_range" | "outofrange" => Some(ThresholdKind::OutOfRange),
        _ => None,
    }
}

fn parse_touch_mode(config: &EventConfig) -> Result<TouchMode, String> {
    let mode_str = config.str_param_or("touch_mode", "close_above");
    match mode_str {
        "close_above" | "closeabove" => Ok(TouchMode::CloseAbove),
        "close_below" | "closebelow" => Ok(TouchMode::CloseBelow),
        "wick_through" | "wickthrough" => Ok(TouchMode::WickThrough),
        "wick_reject" | "wickreject" => Ok(TouchMode::WickReject),
        "touch" => {
            let tolerance = config.param_or("tolerance", 1.0);
            Ok(TouchMode::Touch { tolerance })
        }
        "with_candle" | "withcandle" => {
            let kind_str = config.str_param_or("candle_kind", "doji");
            let kind = parse_candle_kind(kind_str)
                .ok_or_else(|| format!("unknown CandlePatternKind for WithCandle: {kind_str}"))?;
            Ok(TouchMode::WithCandle(kind))
        }
        other => Err(format!("unknown TouchMode: {other}")),
    }
}

fn build_line_cross_source(
    config: &EventConfig,
    side: &str,
    inner_idx: usize,
) -> Result<LineCrossSource, String> {
    // Check if there is an inner indicator at the given index.
    if let Some(inner_cfg) = config.inner_indicators.get(inner_idx) {
        let inst = IndicatorInstance::create(inner_cfg)?;
        return Ok(LineCrossSource::Indicator(Box::new(inst)));
    }
    // Fall back to a constant from additional_params (e.g. "left_value" / "right_value").
    let key = format!("{side}_value");
    let val = config.additional_params.get(&key).copied().unwrap_or(0.0);
    Ok(LineCrossSource::Constant(val))
}

fn build_price_line_source(config: &EventConfig) -> Result<PriceLineSource, String> {
    if let Some(inner_cfg) = config.inner_indicators.first() {
        let inst = IndicatorInstance::create(inner_cfg)?;
        return Ok(PriceLineSource::Indicator(Box::new(inst)));
    }
    let val = config.param_or("line_value", 0.0);
    Ok(PriceLineSource::Constant(val))
}
