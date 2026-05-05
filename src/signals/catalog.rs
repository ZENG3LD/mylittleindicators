//! Signal Catalog - hierarchical catalog of all signal types
//!
//! Signals are organized into ~13 kinds, each with a subtype for specificity.
//! Direction is a separate axis on `Signal` — NOT part of `SignalKind`.

use serde::{Deserialize, Serialize};

// ============================================================================
// SUBTYPES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThresholdSub {
    Enter,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistogramSub {
    SignChange,
    MomentumShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelSub {
    Touch,
    Break,
    Reenter,
    MidCross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DivergenceSub {
    Regular,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrendSub {
    MaCross,
    PriceCross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolatilitySub {
    Shift,
    Extreme,
    Breakout,
    Squeeze,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolumeSub {
    Spike,
    Climax,
    Level,
    DeltaShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureSub {
    BOS,
    CHoCH,
    LiqSweep,
    FVG,
    FVGFilled,
    OrderBlock,
    Imbalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternSub {
    Candle,
    Fractal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositeSub {
    Confirmed,
    Conflict,
    Strong,
}

// ============================================================================
// SIGNAL KIND
// ============================================================================

/// Hierarchical catalog of signal types.
///
/// Direction (Up/Down/Neutral) is NOT part of SignalKind — it lives on `Signal.direction`.
/// Consumer combines: `"{kind.description()} {direction}"` for a full label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalKind {
    Crossover,
    Threshold(ThresholdSub),
    ZeroCross,
    Histogram(HistogramSub),
    Channel(ChannelSub),
    Divergence(DivergenceSub),
    Trend(TrendSub),
    Volatility(VolatilitySub),
    Volume(VolumeSub),
    Swing,
    Structure(StructureSub),
    Pattern(PatternSub),
    Composite(CompositeSub),
    Custom(u32),
}

// ============================================================================
// SIGNAL CATEGORY
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalCategory {
    Basic,
    Oscillator,
    Channel,
    Divergence,
    Trend,
    Volatility,
    Volume,
    Pattern,
    Structure,
    Composite,
    Custom,
}

impl SignalKind {
    pub fn category(&self) -> SignalCategory {
        match self {
            Self::Crossover | Self::ZeroCross => SignalCategory::Basic,
            Self::Threshold(_) | Self::Histogram(_) => SignalCategory::Oscillator,
            Self::Channel(_) => SignalCategory::Channel,
            Self::Divergence(_) => SignalCategory::Divergence,
            Self::Trend(_) => SignalCategory::Trend,
            Self::Volatility(_) => SignalCategory::Volatility,
            Self::Volume(_) => SignalCategory::Volume,
            Self::Swing => SignalCategory::Pattern,
            Self::Structure(_) => SignalCategory::Structure,
            Self::Pattern(_) => SignalCategory::Pattern,
            Self::Composite(_) => SignalCategory::Composite,
            Self::Custom(_) => SignalCategory::Custom,
        }
    }

    /// Human-readable description for UI display and alert matching.
    ///
    /// Direction is NOT included — it lives on `Signal.direction`.
    /// Consumer combines: `"{description} {direction}"` for full label.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Crossover => "Crossover",
            Self::Threshold(ThresholdSub::Enter) => "Threshold Enter",
            Self::Threshold(ThresholdSub::Exit) => "Threshold Exit",
            Self::ZeroCross => "Zero Cross",
            Self::Histogram(HistogramSub::SignChange) => "Histogram Sign Change",
            Self::Histogram(HistogramSub::MomentumShift) => "Histogram Momentum",
            Self::Channel(ChannelSub::Touch) => "Channel Touch",
            Self::Channel(ChannelSub::Break) => "Channel Break",
            Self::Channel(ChannelSub::Reenter) => "Channel Reenter",
            Self::Channel(ChannelSub::MidCross) => "Channel Mid Cross",
            Self::Divergence(DivergenceSub::Regular) => "Divergence",
            Self::Divergence(DivergenceSub::Hidden) => "Hidden Divergence",
            Self::Trend(TrendSub::MaCross) => "MA Cross",
            Self::Trend(TrendSub::PriceCross) => "Price Cross Trend",
            Self::Volatility(VolatilitySub::Shift) => "Volatility Shift",
            Self::Volatility(VolatilitySub::Extreme) => "Volatility Extreme",
            Self::Volatility(VolatilitySub::Breakout) => "Volatility Breakout",
            Self::Volatility(VolatilitySub::Squeeze) => "Volatility Squeeze",
            Self::Volume(VolumeSub::Spike) => "Volume Spike",
            Self::Volume(VolumeSub::Climax) => "Volume Climax",
            Self::Volume(VolumeSub::Level) => "Volume Level",
            Self::Volume(VolumeSub::DeltaShift) => "Volume Delta",
            Self::Swing => "Swing",
            Self::Structure(StructureSub::BOS) => "Break of Structure",
            Self::Structure(StructureSub::CHoCH) => "Change of Character",
            Self::Structure(StructureSub::LiqSweep) => "Liquidity Sweep",
            Self::Structure(StructureSub::FVG) => "Fair Value Gap",
            Self::Structure(StructureSub::FVGFilled) => "FVG Filled",
            Self::Structure(StructureSub::OrderBlock) => "Order Block",
            Self::Structure(StructureSub::Imbalance) => "Imbalance",
            Self::Pattern(PatternSub::Candle) => "Candle Pattern",
            Self::Pattern(PatternSub::Fractal) => "Fractal",
            Self::Composite(CompositeSub::Confirmed) => "Confirmed",
            Self::Composite(CompositeSub::Conflict) => "Conflict",
            Self::Composite(CompositeSub::Strong) => "Strong Signal",
            Self::Custom(_) => "Custom",
        }
    }
}

impl SignalCategory {
    pub fn all() -> &'static [SignalCategory] {
        &[
            Self::Basic,
            Self::Oscillator,
            Self::Channel,
            Self::Divergence,
            Self::Trend,
            Self::Volatility,
            Self::Volume,
            Self::Pattern,
            Self::Structure,
            Self::Composite,
            Self::Custom,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_category() {
        assert_eq!(SignalKind::Crossover.category(), SignalCategory::Basic);
        assert_eq!(
            SignalKind::Threshold(ThresholdSub::Enter).category(),
            SignalCategory::Oscillator
        );
        assert_eq!(
            SignalKind::Channel(ChannelSub::Break).category(),
            SignalCategory::Channel
        );
        assert_eq!(
            SignalKind::Trend(TrendSub::MaCross).category(),
            SignalCategory::Trend
        );
        assert_eq!(
            SignalKind::Volume(VolumeSub::Spike).category(),
            SignalCategory::Volume
        );
    }

    #[test]
    fn test_description() {
        assert_eq!(SignalKind::Crossover.description(), "Crossover");
        assert_eq!(
            SignalKind::Threshold(ThresholdSub::Enter).description(),
            "Threshold Enter"
        );
        assert_eq!(
            SignalKind::Channel(ChannelSub::Touch).description(),
            "Channel Touch"
        );
        assert_eq!(
            SignalKind::Trend(TrendSub::MaCross).description(),
            "MA Cross"
        );
    }
}
