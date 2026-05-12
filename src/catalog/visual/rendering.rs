//! Rendering metadata for indicators
//!
//! This module defines rendering-specific metadata that complements
//! the computation-focused IndicatorSignature. It provides information
//! needed to render indicators in a charting UI.

use std::fmt;

/// How to extract a value from IndicatorValue
#[derive(Debug, Clone, PartialEq)]
pub enum ValueExtractor {
    /// Use .main() - primary value
    Main,
    /// Extract from Channel3/ChannelExtended
    Channel(ChannelPart),
    /// Extract from Macd variant
    Macd(MacdPart),
    /// Extract from Ichimoku variant
    Ichimoku(IchimokuPart),
    /// Extract from Double(a, b)
    Double(DoublePart),
    /// Extract from Triple(a, b, c)
    Triple(TriplePart),
    /// Extract from Candle variant
    Candle(CandlePart),
    /// Extract from Adaptive variant
    Adaptive(AdaptivePart),
    /// Extract from Volatility variant
    Volatility(VolatilityPart),
    /// Extract from StatTest variant
    StatTest(StatTestPart),
    /// Extract from CandleAnatomy variant
    CandleAnatomy(CandleAnatomyPart),
    /// Extract from Hilbert variant
    Hilbert(HilbertPart),
    /// Extract signal value
    Signal,
    /// Extract flag as 0/1
    Flag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelPart {
    Upper,
    Middle,
    Lower,
    Bandwidth,  // For ChannelExtended
    PercentB,   // For ChannelExtended
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacdPart {
    Line,
    Signal,
    Histogram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IchimokuPart {
    Tenkan,
    Kijun,
    SenkouA,
    SenkouB,
    Chikou,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoublePart {
    First,
    Second,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriplePart {
    First,
    Second,
    Third,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandlePart {
    Open,
    High,
    Low,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptivePart {
    Value,
    Period,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityPart {
    Total,
    CloseClose,
    HighLow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatTestPart {
    Statistic,
    PValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleAnatomyPart {
    Body,
    UpperWick,
    LowerWick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HilbertPart {
    Amplitude,
    Phase,
    Frequency,
}

/// Type of visual output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// Single line plot
    Line,
    /// Histogram bars (volume-style or MACD-style)
    Histogram,
    /// Filled area between two values
    Band,
    /// Filled area from line to baseline
    Area,
    /// Dot markers
    Dots,
    /// Background color zones
    Background,
    /// Cloud fill (Ichimoku)
    Cloud,
}

/// Line style for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

/// Histogram rendering style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistogramStyle {
    /// Bars grow from bottom (volume)
    #[default]
    FromBottom,
    /// Bars grow from center/zero line (MACD)
    Centered,
    /// Bars grow from top
    FromTop,
}

/// Single output specification
#[derive(Debug, Clone)]
pub struct OutputSpec {
    /// Internal name (e.g., "macd_line", "signal", "histogram")
    pub name: String,
    /// Display name for UI (e.g., "MACD Line")
    pub display_name: String,
    /// Type of visual output
    pub output_type: OutputType,
    /// Default color (hex string)
    pub default_color: String,
    /// Default line width
    pub default_line_width: f32,
    /// Default line style
    pub default_line_style: LineStyle,
    /// How to extract value from IndicatorValue
    pub value_extractor: ValueExtractor,
    /// Whether this output is visible by default
    pub visible_by_default: bool,
}

impl OutputSpec {
    /// Create a line output
    pub fn line(
        name: impl Into<String>,
        display_name: impl Into<String>,
        color: impl Into<String>,
        line_width: f32,
        extractor: ValueExtractor,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            output_type: OutputType::Line,
            default_color: color.into(),
            default_line_width: line_width,
            default_line_style: LineStyle::Solid,
            value_extractor: extractor,
            visible_by_default: true,
        }
    }

    /// Create a histogram output
    pub fn histogram(
        name: impl Into<String>,
        display_name: impl Into<String>,
        color: impl Into<String>,
        extractor: ValueExtractor,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            output_type: OutputType::Histogram,
            default_color: color.into(),
            default_line_width: 1.0,
            default_line_style: LineStyle::Solid,
            value_extractor: extractor,
            visible_by_default: true,
        }
    }

    /// Create a band output (for channels)
    pub fn band(
        name: impl Into<String>,
        display_name: impl Into<String>,
        fill_color: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            output_type: OutputType::Band,
            default_color: fill_color.into(),
            default_line_width: 1.0,
            default_line_style: LineStyle::Solid,
            value_extractor: ValueExtractor::Main, // Band uses upper/lower from Channel
            visible_by_default: true,
        }
    }

    /// Create an area output
    pub fn area(
        name: impl Into<String>,
        display_name: impl Into<String>,
        fill_color: impl Into<String>,
        extractor: ValueExtractor,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            output_type: OutputType::Area,
            default_color: fill_color.into(),
            default_line_width: 1.0,
            default_line_style: LineStyle::Solid,
            value_extractor: extractor,
            visible_by_default: true,
        }
    }

    /// Create a cloud output (Ichimoku)
    pub fn cloud(
        name: impl Into<String>,
        display_name: impl Into<String>,
        fill_color: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            output_type: OutputType::Cloud,
            default_color: fill_color.into(),
            default_line_width: 1.0,
            default_line_style: LineStyle::Solid,
            value_extractor: ValueExtractor::Main,
            visible_by_default: true,
        }
    }

    /// Set line style
    pub fn with_style(mut self, style: LineStyle) -> Self {
        self.default_line_style = style;
        self
    }

    /// Set visibility
    pub fn hidden(mut self) -> Self {
        self.visible_by_default = false;
        self
    }
}

/// Reference line (horizontal) for oscillators
#[derive(Debug, Clone)]
pub struct ReferenceLine {
    /// Y-axis value
    pub value: f64,
    /// Line color
    pub color: String,
    /// Line style
    pub style: LineStyle,
    /// Optional label
    pub label: Option<String>,
}

impl ReferenceLine {
    /// Create a reference line
    pub fn new(value: f64, color: impl Into<String>) -> Self {
        Self {
            value,
            color: color.into(),
            style: LineStyle::Dashed,
            label: None,
        }
    }

    /// Add a label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set line style
    pub fn with_style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }
}

/// Complete rendering metadata for an indicator
#[derive(Debug, Clone)]
pub struct RenderingMetadata {
    /// Indicator ID (must match IndicatorSignature.id)
    pub indicator_id: String,
    /// Whether to render as overlay on main chart (true) or sub-pane (false)
    pub overlay: bool,
    /// Output specifications
    pub outputs: Vec<OutputSpec>,
    /// Fixed Y-axis bounds (e.g., Some((0.0, 100.0)) for RSI)
    pub bounds: Option<(f64, f64)>,
    /// Whether to extend Y range to include zero
    pub zero_baseline: bool,
    /// Histogram rendering style
    pub histogram_style: HistogramStyle,
    /// Reference lines (e.g., overbought/oversold for RSI)
    pub reference_lines: Vec<ReferenceLine>,
    /// Default pane height ratio (0.0-1.0, relative to main pane)
    pub default_height_ratio: f32,
    /// Precision for value display
    pub precision: u32,
}

impl RenderingMetadata {
    /// Create a builder
    pub fn builder(indicator_id: impl Into<String>) -> RenderingMetadataBuilder {
        RenderingMetadataBuilder::new(indicator_id)
    }

    /// Check if this is an oscillator (bounded 0-100 or similar)
    pub fn is_oscillator(&self) -> bool {
        self.bounds.is_some() && !self.overlay
    }

    /// Check if this uses zero baseline
    pub fn uses_zero_baseline(&self) -> bool {
        self.zero_baseline
    }
}

/// Builder for RenderingMetadata
pub struct RenderingMetadataBuilder {
    indicator_id: String,
    overlay: bool,
    outputs: Vec<OutputSpec>,
    bounds: Option<(f64, f64)>,
    zero_baseline: bool,
    histogram_style: HistogramStyle,
    reference_lines: Vec<ReferenceLine>,
    default_height_ratio: f32,
    precision: u32,
}

impl RenderingMetadataBuilder {
    pub fn new(indicator_id: impl Into<String>) -> Self {
        Self {
            indicator_id: indicator_id.into(),
            overlay: false,
            outputs: Vec::new(),
            bounds: None,
            zero_baseline: false,
            histogram_style: HistogramStyle::FromBottom,
            reference_lines: Vec::new(),
            default_height_ratio: 0.15,
            precision: 4,
        }
    }

    /// Set as overlay indicator (on main price chart)
    pub fn overlay(mut self) -> Self {
        self.overlay = true;
        self
    }

    /// Set as sub-pane indicator
    pub fn sub_pane(mut self) -> Self {
        self.overlay = false;
        self
    }

    /// Add an output
    pub fn output(mut self, output: OutputSpec) -> Self {
        self.outputs.push(output);
        self
    }

    /// Add a simple line output
    pub fn line_output(
        mut self,
        name: impl Into<String>,
        display_name: impl Into<String>,
        color: impl Into<String>,
    ) -> Self {
        self.outputs.push(OutputSpec::line(
            name,
            display_name,
            color,
            2.0,
            ValueExtractor::Main,
        ));
        self
    }

    /// Set fixed Y-axis bounds
    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.bounds = Some((min, max));
        self
    }

    /// Enable zero baseline
    pub fn zero_baseline(mut self) -> Self {
        self.zero_baseline = true;
        self
    }

    /// Set histogram style
    pub fn histogram_style(mut self, style: HistogramStyle) -> Self {
        self.histogram_style = style;
        self
    }

    /// Add a reference line
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.reference_lines.push(line);
        self
    }

    /// Add overbought/oversold lines (common for oscillators)
    pub fn overbought_oversold(mut self, overbought: f64, oversold: f64) -> Self {
        self.reference_lines.push(
            ReferenceLine::new(overbought, "#FF5722")
                .with_label("Overbought")
        );
        self.reference_lines.push(
            ReferenceLine::new(oversold, "#4CAF50")
                .with_label("Oversold")
        );
        self
    }

    /// Set default height ratio for sub-pane
    pub fn height_ratio(mut self, ratio: f32) -> Self {
        self.default_height_ratio = ratio;
        self
    }

    /// Set display precision
    pub fn precision(mut self, precision: u32) -> Self {
        self.precision = precision;
        self
    }

    /// Build the RenderingMetadata
    pub fn build(self) -> RenderingMetadata {
        RenderingMetadata {
            indicator_id: self.indicator_id,
            overlay: self.overlay,
            outputs: self.outputs,
            bounds: self.bounds,
            zero_baseline: self.zero_baseline,
            histogram_style: self.histogram_style,
            reference_lines: self.reference_lines,
            default_height_ratio: self.default_height_ratio,
            precision: self.precision,
        }
    }
}

impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputType::Line => write!(f, "Line"),
            OutputType::Histogram => write!(f, "Histogram"),
            OutputType::Band => write!(f, "Band"),
            OutputType::Area => write!(f, "Area"),
            OutputType::Dots => write!(f, "Dots"),
            OutputType::Background => write!(f, "Background"),
            OutputType::Cloud => write!(f, "Cloud"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rendering_metadata_builder() {
        let meta = RenderingMetadata::builder("RSI")
            .sub_pane()
            .line_output("rsi", "RSI", "#9C27B0")
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build();

        assert_eq!(meta.indicator_id, "RSI");
        assert!(!meta.overlay);
        assert_eq!(meta.outputs.len(), 1);
        assert_eq!(meta.bounds, Some((0.0, 100.0)));
        assert_eq!(meta.reference_lines.len(), 2);
        assert!(meta.is_oscillator());
    }

    #[test]
    fn test_output_spec_line() {
        let output = OutputSpec::line("sma", "SMA", "#2196F3", 2.0, ValueExtractor::Main);

        assert_eq!(output.name, "sma");
        assert_eq!(output.output_type, OutputType::Line);
        assert_eq!(output.default_color, "#2196F3");
        assert!(output.visible_by_default);
    }

    #[test]
    fn test_macd_rendering() {
        let meta = RenderingMetadata::builder("MACD")
            .sub_pane()
            .output(OutputSpec::line("macd", "MACD", "#2196F3", 2.0, ValueExtractor::Macd(MacdPart::Line)))
            .output(OutputSpec::line("signal", "Signal", "#FF5722", 1.0, ValueExtractor::Macd(MacdPart::Signal)))
            .output(OutputSpec::histogram("histogram", "Histogram", "#4CAF50", ValueExtractor::Macd(MacdPart::Histogram)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .build();

        assert_eq!(meta.outputs.len(), 3);
        assert!(meta.uses_zero_baseline());
        assert_eq!(meta.histogram_style, HistogramStyle::Centered);
    }

    #[test]
    fn test_overlay_indicator() {
        let meta = RenderingMetadata::builder("SMA")
            .overlay()
            .line_output("sma", "SMA", "#2196F3")
            .build();

        assert!(meta.overlay);
        assert!(!meta.is_oscillator());
    }
}
