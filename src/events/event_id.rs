//! EventId — unique identifier for each event/detector primitive.
//!
//! Mirrors `BarIndicatorId` for the event subsystem.

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Unique identifier for each event/detector primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventId {
    BosEventDetector,
    /// CandlePattern with `kind` from `EventConfig.string_params["kind"]`
    CandlePattern,
    Confluence,
    /// Rolling beta of primary asset returns vs secondary asset returns.
    CrossAssetBeta,
    DirectionDetector,
    Divergence,
    FvgEventDetector,
    LineCross,
    OscillatorWithDivergence,
    OscillatorWithVolumeWeight,
    /// Rolling cointegration proxy (spread z-score) between two price series.
    PairsCointegrationProxy,
    Pivot,
    PriceLineCross,
    RegimeGate,
    RelativePosition,
    /// Relative strength of primary vs secondary symbol over a rolling window.
    RelativeStrengthCross,
    StatisticalWickDetector,
    SwingDetection,
    Threshold,
    VolatilityRegimeDetector,
    VolumeEventDetector,
}

impl EventId {
    /// Human-readable name.
    pub fn as_str(self) -> &'static str {
        match self {
            EventId::BosEventDetector => "BosEventDetector",
            EventId::CandlePattern => "CandlePattern",
            EventId::Confluence => "Confluence",
            EventId::CrossAssetBeta => "CrossAssetBeta",
            EventId::DirectionDetector => "DirectionDetector",
            EventId::Divergence => "Divergence",
            EventId::FvgEventDetector => "FvgEventDetector",
            EventId::LineCross => "LineCross",
            EventId::OscillatorWithDivergence => "OscillatorWithDivergence",
            EventId::OscillatorWithVolumeWeight => "OscillatorWithVolumeWeight",
            EventId::PairsCointegrationProxy => "PairsCointegrationProxy",
            EventId::Pivot => "Pivot",
            EventId::PriceLineCross => "PriceLineCross",
            EventId::RegimeGate => "RegimeGate",
            EventId::RelativePosition => "RelativePosition",
            EventId::RelativeStrengthCross => "RelativeStrengthCross",
            EventId::StatisticalWickDetector => "StatisticalWickDetector",
            EventId::SwingDetection => "SwingDetection",
            EventId::Threshold => "Threshold",
            EventId::VolatilityRegimeDetector => "VolatilityRegimeDetector",
            EventId::VolumeEventDetector => "VolumeEventDetector",
        }
    }

    /// Look up by alias (case-insensitive snake_case or PascalCase).
    pub fn from_str(s: &str) -> Option<Self> {
        EVENT_ID_MAP.get(s.to_lowercase().as_str()).copied()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Static alias map: lowercase string → EventId.
pub static EVENT_ID_MAP: Lazy<HashMap<&'static str, EventId>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // BosEventDetector
    m.insert("boseventdetector", EventId::BosEventDetector);
    m.insert("bos_event_detector", EventId::BosEventDetector);
    m.insert("bos", EventId::BosEventDetector);
    // CandlePattern
    m.insert("candlepattern", EventId::CandlePattern);
    m.insert("candle_pattern", EventId::CandlePattern);
    m.insert("candlepatterndetector", EventId::CandlePattern);
    m.insert("candle_pattern_detector", EventId::CandlePattern);
    // Confluence
    m.insert("confluence", EventId::Confluence);
    // CrossAssetBeta
    m.insert("crossassetbeta", EventId::CrossAssetBeta);
    m.insert("cross_asset_beta", EventId::CrossAssetBeta);
    // DirectionDetector
    m.insert("directiondetector", EventId::DirectionDetector);
    m.insert("direction_detector", EventId::DirectionDetector);
    m.insert("direction", EventId::DirectionDetector);
    // Divergence
    m.insert("divergence", EventId::Divergence);
    // FvgEventDetector
    m.insert("fvgeventdetector", EventId::FvgEventDetector);
    m.insert("fvg_event_detector", EventId::FvgEventDetector);
    m.insert("fvg", EventId::FvgEventDetector);
    // LineCross
    m.insert("linecross", EventId::LineCross);
    m.insert("line_cross", EventId::LineCross);
    // OscillatorWithDivergence
    m.insert("oscillatorwithdivergence", EventId::OscillatorWithDivergence);
    m.insert("oscillator_with_divergence", EventId::OscillatorWithDivergence);
    // OscillatorWithVolumeWeight
    m.insert("oscillatorwithvolumeweight", EventId::OscillatorWithVolumeWeight);
    m.insert("oscillator_with_volume_weight", EventId::OscillatorWithVolumeWeight);
    // PairsCointegrationProxy
    m.insert("pairscointegrationproxy", EventId::PairsCointegrationProxy);
    m.insert("pairs_cointegration_proxy", EventId::PairsCointegrationProxy);
    // Pivot
    m.insert("pivot", EventId::Pivot);
    // PriceLineCross
    m.insert("pricelinecross", EventId::PriceLineCross);
    m.insert("price_line_cross", EventId::PriceLineCross);
    // RegimeGate
    m.insert("regimegate", EventId::RegimeGate);
    m.insert("regime_gate", EventId::RegimeGate);
    // RelativePosition
    m.insert("relativeposition", EventId::RelativePosition);
    m.insert("relative_position", EventId::RelativePosition);
    // RelativeStrengthCross
    m.insert("relativestrengthcross", EventId::RelativeStrengthCross);
    m.insert("relative_strength_cross", EventId::RelativeStrengthCross);
    // StatisticalWickDetector
    m.insert("statisticalwickdetector", EventId::StatisticalWickDetector);
    m.insert("statistical_wick_detector", EventId::StatisticalWickDetector);
    m.insert("wick_spike", EventId::StatisticalWickDetector);
    // SwingDetection
    m.insert("swingdetection", EventId::SwingDetection);
    m.insert("swing_detection", EventId::SwingDetection);
    m.insert("swing", EventId::SwingDetection);
    // Threshold
    m.insert("threshold", EventId::Threshold);
    // VolatilityRegimeDetector
    m.insert("volatilityregimedetector", EventId::VolatilityRegimeDetector);
    m.insert("volatility_regime_detector", EventId::VolatilityRegimeDetector);
    m.insert("volatility_regime", EventId::VolatilityRegimeDetector);
    // VolumeEventDetector
    m.insert("volumeeventdetector", EventId::VolumeEventDetector);
    m.insert("volume_event_detector", EventId::VolumeEventDetector);
    m.insert("volume_event", EventId::VolumeEventDetector);
    m
});
