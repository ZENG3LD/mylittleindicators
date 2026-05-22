//! indicator_signature.rs: Complete indicator specification for universal factory
//!
//! Provides IndicatorSignature - a complete specification of an indicator including
//! its ID, category, parameters, constraints, and metadata. This is the bridge between
//! TOML metadata and the UniversalIndicatorFactory.

use crate::catalog::{
    ParamValue, ParamConstraint, ConstraintSet, ParamError,
};
use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use std::collections::HashMap;
use std::fmt;

// =============================================================================
// SourceType: Indicator data source requirements
// =============================================================================

/// Defines what data sources an indicator requires
///
/// This enum determines whether an indicator:
/// - Requires source selection (PriceOnly - user chooses OHLC field)
/// - Uses fixed data (VolumeOnly - no source selection)
/// - Uses multiple data types internally (PriceAndVolume - no user selection)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceType {
    /// Uses only price data (OHLC) - supports OhlcvField source selection
    ///
    /// Indicators like SMA, RSI, Bollinger Bands allow users to choose
    /// which price field (Open, High, Low, Close, HL/2, etc.) to use.
    #[default]
    PriceOnly,

    /// Uses only volume data - no source selection needed
    ///
    /// Indicators like Volume Oscillator, OBV, VWAP use volume exclusively.
    /// No source selection is needed as volume is a single field.
    VolumeOnly,

    /// Uses both price and volume - internal logic, no user source selection
    ///
    /// Indicators like Money Flow Index, Volume-Weighted Average Price combine
    /// price and volume internally. User doesn't select source as the indicator
    /// requires specific OHLCV fields for its calculation.
    PriceAndVolume,
}

impl SourceType {
    /// Check if this indicator requires source selection UI
    pub fn requires_source_selection(&self) -> bool {
        matches!(self, SourceType::PriceOnly)
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::PriceOnly => "price_only",
            SourceType::VolumeOnly => "volume_only",
            SourceType::PriceAndVolume => "price_and_volume",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "price_only" | "price" => Some(SourceType::PriceOnly),
            "volume_only" | "volume" => Some(SourceType::VolumeOnly),
            "price_and_volume" | "both" => Some(SourceType::PriceAndVolume),
            _ => None,
        }
    }
}

impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// IndicatorCategory: Major indicator categories
// =============================================================================

/// Major indicator categories (25 total in design doc)
///
/// Represents the top-level organization of ~447 indicators across the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndicatorCategory {
    // Most common categories (by count from design doc)
    Momentum,         // 92 indicators
    SignalProcessing, // 52 indicators
    Channels,         // 42 indicators
    Volatility,       // 40 indicators
    Average,          // 38 indicators
    Levels,           // 30 indicators
    Adaptive,         // 22 indicators
    Entropy,          // 18 indicators
    Volume,           // 17 indicators
    Kalman,           // 16 indicators
    TrendStop,        // 13 indicators
    Accumulation,     // 12 indicators
    Regression,       // 12 indicators
    Chaos,            // 11 indicators
    Ratio,            // 10 indicators
    Candles,          // 8 indicators
    Zigzag,           // 8 indicators
    Trend,            // 7 indicators
    Divergence,       // 6 indicators
    Clusters,         // 5 indicators
    Book,             // 2 indicators
    Position,           // 19 indicators (seasonality, temporal, position-based)
    Statistics,         // 26 indicators (statistical tests, stationarity, etc.)
    StatisticalScoring, // 6 indicators (normalized scalar: probability, density, tanh-strength)

    // Stream-specialized categories
    FundingAdvanced,      // Advanced funding rate indicators
    OpenInterest,         // Open interest indicators
    MarkPriceAdvanced,    // Advanced mark price indicators
    TickerAdvanced,       // Advanced ticker indicators
    Liquidations,         // Liquidation event indicators
    TickAdvanced,         // Advanced tick/trade flow indicators
    BookAdvanced,         // Advanced order book indicators
    Composites,           // Cross-stream composite indicators
    Sentiment,            // Sentiment indicators (long/short ratio, agg trade)
    IndexBasis,           // Index/Basis indicators
    VolatilityAdvanced,   // Advanced volatility indicators (HV, Vol Index)
    Greeks,               // Option Greeks indicators
    Stress,               // Stress indicators (insurance fund, settlement)
    Microstructure,       // Microstructure indicators (block trade, L3)
    RiskFunding,          // Risk, funding, and auction indicators

    // Reserved for future expansion
    Custom,
    Composite,
    Experimental,
    Unknown,
}

impl IndicatorCategory {
    /// Get category from string name (case-insensitive)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "momentum" => Self::Momentum,
            "signal_processing" | "signalprocessing" => Self::SignalProcessing,
            "channels" => Self::Channels,
            "volatility" => Self::Volatility,
            "average" | "averages" => Self::Average,
            "levels" => Self::Levels,
            "adaptive" => Self::Adaptive,
            "entropy" => Self::Entropy,
            "volume" => Self::Volume,
            "kalman" => Self::Kalman,
            "trend_stop" | "trendstop" => Self::TrendStop,
            "accumulation" => Self::Accumulation,
            "regression" => Self::Regression,
            "chaos" => Self::Chaos,
            "ratio" => Self::Ratio,
            "candles" => Self::Candles,
            "zigzag" => Self::Zigzag,
            "trend" => Self::Trend,
            "divergence" => Self::Divergence,
            "clusters" => Self::Clusters,
            "book" => Self::Book,
            "position" => Self::Position,
            "statistics" | "stats" => Self::Statistics,
            "statistical_scoring" | "statisticalscoring" | "scoring" => Self::StatisticalScoring,
            "funding_advanced" | "fundingadvanced" => Self::FundingAdvanced,
            "open_interest" | "openinterest" => Self::OpenInterest,
            "mark_price_advanced" | "markpriceadvanced" => Self::MarkPriceAdvanced,
            "ticker_advanced" | "tickeradvanced" => Self::TickerAdvanced,
            "liquidations" => Self::Liquidations,
            "tick_advanced" | "tickadvanced" => Self::TickAdvanced,
            "book_advanced" | "bookadvanced" => Self::BookAdvanced,
            "composites" => Self::Composites,
            "sentiment" => Self::Sentiment,
            "index_basis" | "indexbasis" => Self::IndexBasis,
            "volatility_advanced" | "volatilityadvanced" => Self::VolatilityAdvanced,
            "greeks" => Self::Greeks,
            "stress" => Self::Stress,
            "microstructure" => Self::Microstructure,
            "risk_funding" | "riskfunding" => Self::RiskFunding,
            "custom" => Self::Custom,
            "composite" => Self::Composite,
            "experimental" => Self::Experimental,
            _ => Self::Unknown,
        }
    }

    /// Get string name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Momentum => "momentum",
            Self::SignalProcessing => "signal_processing",
            Self::Channels => "channels",
            Self::Volatility => "volatility",
            Self::Average => "average",
            Self::Levels => "levels",
            Self::Adaptive => "adaptive",
            Self::Entropy => "entropy",
            Self::Volume => "volume",
            Self::Kalman => "kalman",
            Self::TrendStop => "trend_stop",
            Self::Accumulation => "accumulation",
            Self::Regression => "regression",
            Self::Chaos => "chaos",
            Self::Ratio => "ratio",
            Self::Candles => "candles",
            Self::Zigzag => "zigzag",
            Self::Trend => "trend",
            Self::Divergence => "divergence",
            Self::Clusters => "clusters",
            Self::Book => "book",
            Self::Position => "position",
            Self::Statistics => "statistics",
            Self::StatisticalScoring => "statistical_scoring",
            Self::FundingAdvanced => "funding_advanced",
            Self::OpenInterest => "open_interest",
            Self::MarkPriceAdvanced => "mark_price_advanced",
            Self::TickerAdvanced => "ticker_advanced",
            Self::Liquidations => "liquidations",
            Self::TickAdvanced => "tick_advanced",
            Self::BookAdvanced => "book_advanced",
            Self::Composites => "composites",
            Self::Sentiment => "sentiment",
            Self::IndexBasis => "index_basis",
            Self::VolatilityAdvanced => "volatility_advanced",
            Self::Greeks => "greeks",
            Self::Stress => "stress",
            Self::Microstructure => "microstructure",
            Self::RiskFunding => "risk_funding",
            Self::Custom => "custom",
            Self::Composite => "composite",
            Self::Experimental => "experimental",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for IndicatorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// IndicatorSignature: Complete indicator specification
// =============================================================================

/// Complete specification of an indicator for the universal factory
///
/// Contains all metadata needed to:
/// - Identify the indicator uniquely
/// - Validate parameters
/// - Create the indicator instance
/// - Cache computed values
///
/// ## Example
///
/// ```rust
/// use zengeld_chart_indicators::catalog::indicator_signature::{IndicatorSignature, IndicatorCategory};
/// use zengeld_chart_indicators::catalog::constraints::ParamConstraint;
/// use zengeld_chart_indicators::catalog::param_value::ParamValue;
///
/// let rsi = IndicatorSignature::builder("RSI", IndicatorCategory::Momentum)
///     .description("Relative Strength Index")
///     .add_constraint(ParamConstraint::period(2, 200, 14))
///     .add_constraint(ParamConstraint::flag("use_wilder", true))
///     .build();
///
/// // Validate parameters
/// let params = vec![
///     ("period", ParamValue::USize(14)),
///     ("use_wilder", ParamValue::Bool(true)),
/// ];
/// assert!(rsi.validate_params(&params).is_ok());
/// ```
/// Semantic role an indicator plays — used by codegen to filter slot
/// compatibility (e.g. only `Smoother` indicators fit a moving-average slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndicatorRoleKind {
    /// Smoother / moving average (SMA, EMA, HMA, Kalman, ...).
    Smoother,
    /// Bounded oscillator [0..100] or [-100..100] (RSI, Stoch, CCI bounded, ...).
    OscillatorBounded,
    /// Unbounded oscillator (MACD, ROC, Momentum, ...).
    OscillatorUnbounded,
    /// Channel / band (Bollinger, Keltner, Donchian).
    Channel,
    /// Trend stop / trailing stop (Supertrend, Chandelier, PSAR).
    TrendStop,
    /// Volatility (ATR, NATR, HV).
    Volatility,
    /// Volume / money flow (OBV, CMF, MFI).
    Volume,
    /// Pivot / level (FVG, OrderBlock, Pivot points).
    Level,
    /// Regime classifier (ADX, VHF — outputs strength).
    Regime,
    /// Pattern recognition (candlestick, fractals).
    Pattern,
    /// Statistical test (ADF, KPSS, cointegration).
    Statistical,
    /// Other / catch-all.
    Other,
}

#[derive(Debug, Clone)]
pub struct IndicatorSignature {
    /// Unique identifier (e.g., "SMA", "RSI", "MACD")
    pub id: String,

    /// Human-readable name (e.g., "Simple Moving Average")
    pub name: String,

    /// Indicator category
    pub category: IndicatorCategory,

    /// Short description
    pub description: String,

    /// Parameter constraints
    pub constraints: ConstraintSet,

    /// Data source requirements (price, volume, or both)
    pub source_type: SourceType,

    /// Additional metadata (free-form). Prefer typed fields below; this remains
    /// for legacy / domain-specific tags (e.g. `author`, `formula`).
    pub metadata: HashMap<String, String>,

    /// Machine ID for factory (optional - some indicators may not have implementation yet)
    pub machine_id: Option<BarIndicatorId>,

    /// Aliases for user-facing lookup (e.g., ["rsi", "relative_strength_index"])
    /// Used to generate BAR_INDICATOR_MAP automatically
    pub aliases: Vec<String>,

    // -- Typed metadata for codegen seeding --

    /// Semantic role for slot-compatibility validation in strategy codegen.
    /// `None` → treated as `Other` (no slot constraints).
    pub role_kind: Option<IndicatorRoleKind>,

    /// Output is bounded (e.g. RSI ∈ [0, 100], Williams %R ∈ [-100, 0]).
    /// `Some((lo, hi))` for bounded, `None` for unbounded.
    pub output_bounds: Option<(f64, f64)>,

    /// Default upper / lower thresholds for codegen seeding (RSI 70/30,
    /// CCI ±100, Williams ±20/±80). `None` means caller must supply.
    pub default_thresholds: Option<(f64, f64)>,

    /// Warmup period — number of bars before `is_ready()` returns true.
    /// `None` means warmup equals primary `period` parameter (default).
    pub warmup_bars: Option<usize>,

    /// Indicator validated against real market data (TEST_CHECKLIST DONE).
    /// `false` means inline tests may exist but real-data validation is missing.
    pub validated: bool,

    /// Indicator requires L2 / order-book data (book/clusters/L2-derived).
    /// Codegen will exclude such indicators from bar-only strategies.
    pub requires_l2: bool,

    /// Output shape discriminant. Two indicators with the same `output_kind`
    /// have identical state size in the hot loop and can share a generated
    /// strategy template (mixed via `RoleDef::indicators(&[...])`).
    /// `None` means unknown — codegen falls back to runtime materialisation
    /// via `IndicatorInstance::create` to discover the kind.
    pub output_kind: Option<IndicatorValueKind>,

    /// Primary input stream this indicator consumes. Default `Bar` (OHLCV) for
    /// backward compatibility with classic bar-based indicators. Set explicitly
    /// for non-OHLCV indicators (book, funding, OI, liq, ticker, mark, etc.).
    ///
    /// Used by:
    /// - live validator + collector to dispatch the right `update_*(...)` method
    /// - codegen to filter indicators by data source compatibility
    /// - mlq warmup to know which Station Stream subscription to consume
    pub input_stream: StreamKind,

    /// Auxiliary input streams for hybrid / composite indicators that consume
    /// multiple already-synchronized vectors (e.g. tick + orderbook, bar +
    /// funding + OI). Synchronization is NOT the indicator's responsibility —
    /// the runtime feeds it pre-aligned data through a composite update method.
    ///
    /// Single-input indicators (90%+ of catalog) keep this as `&[]` — zero
    /// allocation, zero cost. The hot path for bar indicators is untouched.
    pub aux_streams: &'static [StreamKind],
}

impl IndicatorSignature {
    /// Create builder for fluent API
    pub fn builder(id: impl Into<String>, category: IndicatorCategory) -> IndicatorSignatureBuilder {
        IndicatorSignatureBuilder::new(id, category)
    }

    /// Validate provided parameters
    ///
    /// ## Errors
    ///
    /// - `ParamError::MissingParam`: Required parameter not provided
    /// - `ParamError::TypeMismatch`: Parameter has wrong type
    /// - `ParamError::OutOfRange`: Parameter value out of bounds
    pub fn validate_params(&self, params: &[(&str, ParamValue)]) -> Result<(), ParamError> {
        self.constraints.validate_all(params)
    }

    /// Effective role kind: explicit `role_kind` field, falling back to a
    /// category-derived heuristic. Used by codegen for slot validation.
    pub fn effective_role_kind(&self) -> IndicatorRoleKind {
        if let Some(rk) = self.role_kind {
            return rk;
        }
        match self.category {
            IndicatorCategory::Average | IndicatorCategory::Adaptive
            | IndicatorCategory::Kalman | IndicatorCategory::SignalProcessing
            | IndicatorCategory::Regression => IndicatorRoleKind::Smoother,
            IndicatorCategory::Channels => IndicatorRoleKind::Channel,
            IndicatorCategory::TrendStop => IndicatorRoleKind::TrendStop,
            IndicatorCategory::Volatility => IndicatorRoleKind::Volatility,
            IndicatorCategory::Volume | IndicatorCategory::Accumulation => IndicatorRoleKind::Volume,
            IndicatorCategory::Levels | IndicatorCategory::Zigzag => IndicatorRoleKind::Level,
            IndicatorCategory::Candles => IndicatorRoleKind::Pattern,
            IndicatorCategory::Statistics => IndicatorRoleKind::Statistical,
            IndicatorCategory::StatisticalScoring => IndicatorRoleKind::Statistical,
            IndicatorCategory::Momentum => IndicatorRoleKind::OscillatorUnbounded,
            IndicatorCategory::Trend => IndicatorRoleKind::Regime,
            _ => IndicatorRoleKind::Other,
        }
    }

    /// Generate cache key from parameters
    ///
    /// Format: "{ID}_{param1}_{param2}_{...}"
    ///
    /// ## Example
    /// ```rust
    /// use zengeld_chart_indicators::catalog::indicator_signature::{IndicatorSignature, IndicatorCategory};
    /// use zengeld_chart_indicators::catalog::param_value::ParamValue;
    ///
    /// let sig = IndicatorSignature::builder("SMA", IndicatorCategory::Average).build();
    /// let params = vec![("period", ParamValue::USize(20))];
    /// let key = sig.cache_key(&params);
    /// assert!(key.starts_with("SMA_"));
    /// ```
    pub fn cache_key(&self, params: &[(&str, ParamValue)]) -> String {
        let mut key = self.id.clone();

        // Sort parameters by name for consistent keys
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by_key(|(name, _)| *name);

        for (_, value) in sorted_params {
            key.push('_');
            key.push_str(&value.to_string());
        }

        key
    }

    /// Get all parameter values with defaults applied
    ///
    /// Returns a HashMap with all parameters (provided + defaults for missing optional params)
    pub fn resolve_params(&self, params: &[(&str, ParamValue)]) -> Result<HashMap<String, ParamValue>, ParamError> {
        // First validate
        self.validate_params(params)?;

        let mut resolved = HashMap::new();

        // Add provided parameters
        for (name, value) in params {
            resolved.insert(name.to_string(), value.clone());
        }

        // Add defaults for missing optional parameters
        for constraint in &self.constraints.constraints {
            if !resolved.contains_key(&constraint.name) {
                if let Some(default) = &constraint.default {
                    resolved.insert(constraint.name.clone(), default.clone());
                }
            }
        }

        Ok(resolved)
    }

    /// Get required parameter names
    pub fn required_params(&self) -> Vec<&str> {
        self.constraints.required_params()
    }

    /// Get all parameter names
    pub fn param_names(&self) -> Vec<&str> {
        self.constraints.param_names()
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

impl fmt::Display for IndicatorSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Indicator: {} ({})", self.name, self.id)?;
        writeln!(f, "Category: {}", self.category)?;
        if !self.description.is_empty() {
            writeln!(f, "Description: {}", self.description)?;
        }
        writeln!(f, "\nParameters:")?;
        for constraint in &self.constraints.constraints {
            writeln!(f, "  {}", constraint)?;
        }
        Ok(())
    }
}

// =============================================================================
// IndicatorSignatureBuilder: Fluent builder API
// =============================================================================

/// Builder for IndicatorSignature with fluent API
pub struct IndicatorSignatureBuilder {
    id: String,
    name: Option<String>,
    category: IndicatorCategory,
    description: Option<String>,
    constraints: Vec<ParamConstraint>,
    source_type: SourceType,
    metadata: HashMap<String, String>,
    machine_id: Option<BarIndicatorId>,
    aliases: Vec<String>,
    role_kind: Option<IndicatorRoleKind>,
    output_bounds: Option<(f64, f64)>,
    default_thresholds: Option<(f64, f64)>,
    warmup_bars: Option<usize>,
    validated: bool,
    requires_l2: bool,
    output_kind: Option<IndicatorValueKind>,
    input_stream: StreamKind,
    aux_streams: &'static [StreamKind],
}

impl IndicatorSignatureBuilder {
    /// Create new builder
    pub fn new(id: impl Into<String>, category: IndicatorCategory) -> Self {
        Self {
            id: id.into(),
            name: None,
            category,
            description: None,
            constraints: Vec::new(),
            source_type: SourceType::default(),
            metadata: HashMap::new(),
            machine_id: None,
            aliases: Vec::new(),
            role_kind: None,
            output_bounds: None,
            default_thresholds: None,
            warmup_bars: None,
            validated: false,
            requires_l2: false,
            output_kind: None,
            input_stream: StreamKind::Bar,
            aux_streams: &[],
        }
    }

    /// Set primary input stream this indicator consumes (default `Bar` OHLCV).
    /// For non-OHLCV indicators (book, funding, OI, liq, ticker, mark, ...).
    pub fn input_stream(mut self, stream: StreamKind) -> Self {
        self.input_stream = stream;
        self
    }

    /// Set auxiliary input streams (for hybrid/composite indicators).
    /// Single-input indicators should leave this empty.
    pub fn aux_streams(mut self, streams: &'static [StreamKind]) -> Self {
        self.aux_streams = streams;
        self
    }

    /// Set output kind (IndicatorValue variant discriminant) — used by codegen
    /// to group indicators with identical state size into one strategy template.
    pub fn output_kind(mut self, kind: IndicatorValueKind) -> Self {
        self.output_kind = Some(kind);
        self
    }

    /// Set semantic role for slot-compatibility validation in codegen.
    pub fn role_kind(mut self, kind: IndicatorRoleKind) -> Self {
        self.role_kind = Some(kind);
        self
    }

    /// Set output bounds (e.g. RSI → (0.0, 100.0)).
    pub fn output_bounds(mut self, lo: f64, hi: f64) -> Self {
        self.output_bounds = Some((lo, hi));
        self
    }

    /// Set default thresholds (lower, upper) for codegen seeding.
    pub fn default_thresholds(mut self, lower: f64, upper: f64) -> Self {
        self.default_thresholds = Some((lower, upper));
        self
    }

    /// Set explicit warmup bars (default = primary period parameter).
    pub fn warmup_bars(mut self, bars: usize) -> Self {
        self.warmup_bars = Some(bars);
        self
    }

    /// Mark indicator as validated against real data.
    pub fn validated(mut self) -> Self {
        self.validated = true;
        self
    }

    /// Mark indicator as requiring L2 / order-book data.
    pub fn requires_l2(mut self) -> Self {
        self.requires_l2 = true;
        self
    }

    /// Set human-readable name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set source type (default: PriceOnly)
    pub fn source_type(mut self, source_type: SourceType) -> Self {
        self.source_type = source_type;
        self
    }

    /// Add parameter constraint
    pub fn add_constraint(mut self, constraint: ParamConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set machine ID for factory
    pub fn machine_id(mut self, id: BarIndicatorId) -> Self {
        self.machine_id = Some(id);
        self
    }

    /// Add an alias for user-facing lookup
    /// Can be called multiple times to add multiple aliases
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Build the signature
    pub fn build(self) -> IndicatorSignature {
        let name = self.name.unwrap_or_else(|| self.id.clone());
        let mut constraints = ConstraintSet::new(&self.id);
        constraints.add_all(self.constraints);

        IndicatorSignature {
            id: self.id,
            name,
            category: self.category,
            description: self.description.unwrap_or_default(),
            constraints,
            source_type: self.source_type,
            metadata: self.metadata,
            machine_id: self.machine_id,
            aliases: self.aliases,
            role_kind: self.role_kind,
            output_bounds: self.output_bounds,
            default_thresholds: self.default_thresholds,
            warmup_bars: self.warmup_bars,
            validated: self.validated,
            requires_l2: self.requires_l2,
            output_kind: self.output_kind,
            input_stream: self.input_stream,
            aux_streams: self.aux_streams,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::average::moving_average::MovingAverageType;

    #[test]
    fn test_indicator_category_from_str() {
        assert_eq!(IndicatorCategory::from_str("momentum"), IndicatorCategory::Momentum);
        assert_eq!(IndicatorCategory::from_str("Momentum"), IndicatorCategory::Momentum);
        assert_eq!(IndicatorCategory::from_str("MOMENTUM"), IndicatorCategory::Momentum);
        assert_eq!(IndicatorCategory::from_str("signal_processing"), IndicatorCategory::SignalProcessing);
        assert_eq!(IndicatorCategory::from_str("signalprocessing"), IndicatorCategory::SignalProcessing);
        assert_eq!(IndicatorCategory::from_str("unknown_category"), IndicatorCategory::Unknown);
    }

    #[test]
    fn test_indicator_category_as_str() {
        assert_eq!(IndicatorCategory::Momentum.as_str(), "momentum");
        assert_eq!(IndicatorCategory::SignalProcessing.as_str(), "signal_processing");
        assert_eq!(IndicatorCategory::Channels.as_str(), "channels");
    }

    #[test]
    fn test_builder_minimal() {
        let sig = IndicatorSignature::builder("SMA", IndicatorCategory::Average)
            .build();

        assert_eq!(sig.id, "SMA");
        assert_eq!(sig.name, "SMA"); // Default to ID
        assert_eq!(sig.category, IndicatorCategory::Average);
    }

    #[test]
    fn test_builder_complete() {
        let sig = IndicatorSignature::builder("RSI", IndicatorCategory::Momentum)
            .name("Relative Strength Index")
            .description("Momentum oscillator measuring speed and magnitude of price changes")
            .add_constraint(ParamConstraint::period(2, 200, 14))
            .add_constraint(ParamConstraint::flag("use_wilder", true))
            .metadata("author", "J. Welles Wilder")
            .metadata("year", "1978")
            .build();

        assert_eq!(sig.id, "RSI");
        assert_eq!(sig.name, "Relative Strength Index");
        assert_eq!(sig.category, IndicatorCategory::Momentum);
        assert!(sig.description.contains("Momentum oscillator"));
        assert_eq!(sig.constraints.constraints.len(), 2);
        assert_eq!(sig.get_metadata("author"), Some("J. Welles Wilder"));
        assert_eq!(sig.get_metadata("year"), Some("1978"));
    }

    #[test]
    fn test_validate_params() {
        let sig = IndicatorSignature::builder("SMA", IndicatorCategory::Average)
            .add_constraint(ParamConstraint::period(2, 200, 20))
            .build();

        // Valid parameters
        let params = vec![("period", ParamValue::USize(20))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_err());

        // Invalid: wrong type
        let params = vec![("period", ParamValue::F64(20.0))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key() {
        let sig = IndicatorSignature::builder("MACD", IndicatorCategory::Momentum)
            .add_constraint(ParamConstraint::period(2, 50, 12))
            .add_constraint(ParamConstraint::period(2, 50, 26))
            .add_constraint(ParamConstraint::period(2, 20, 9))
            .build();

        let params = vec![
            ("fast", ParamValue::USize(12)),
            ("slow", ParamValue::USize(26)),
            ("signal", ParamValue::USize(9)),
        ];

        let key = sig.cache_key(&params);
        assert!(key.starts_with("MACD_"));
        assert!(key.contains("12"));
        assert!(key.contains("26"));
        assert!(key.contains("9"));
    }

    #[test]
    fn test_cache_key_consistency() {
        let sig = IndicatorSignature::builder("TEST", IndicatorCategory::Custom).build();

        // Same parameters in different order should produce same key
        let params1 = vec![
            ("a", ParamValue::USize(1)),
            ("b", ParamValue::USize(2)),
        ];
        let params2 = vec![
            ("b", ParamValue::USize(2)),
            ("a", ParamValue::USize(1)),
        ];

        let key1 = sig.cache_key(&params1);
        let key2 = sig.cache_key(&params2);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_resolve_params_with_defaults() {
        use crate::catalog::param_value::ParamType;

        // Create signature with optional multiplier (not required)
        let sig = IndicatorSignature::builder("BB", IndicatorCategory::Channels)
            .add_constraint(ParamConstraint::period(2, 200, 20))
            .add_constraint(
                ParamConstraint::new("multiplier", ParamType::F64)
                    .with_min(ParamValue::F64(0.5))
                    .with_max(ParamValue::F64(5.0))
                    .with_default(ParamValue::F64(2.0))
                // NOT .required() - so it's optional
            )
            .build();

        // Provide only period, should get default for multiplier
        let params = vec![("period", ParamValue::USize(20))];
        let resolved = sig.resolve_params(&params).unwrap();

        assert_eq!(resolved.get("period"), Some(&ParamValue::USize(20)));
        assert_eq!(resolved.get("multiplier"), Some(&ParamValue::F64(2.0))); // default
    }

    #[test]
    fn test_resolve_params_override_defaults() {
        let sig = IndicatorSignature::builder("BB", IndicatorCategory::Channels)
            .add_constraint(ParamConstraint::period(2, 200, 20))
            .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
            .build();

        // Override default multiplier
        let params = vec![
            ("period", ParamValue::USize(20)),
            ("multiplier", ParamValue::F64(3.0)),
        ];
        let resolved = sig.resolve_params(&params).unwrap();

        assert_eq!(resolved.get("multiplier"), Some(&ParamValue::F64(3.0)));
    }

    #[test]
    fn test_required_params() {
        let sig = IndicatorSignature::builder("RSI", IndicatorCategory::Momentum)
            .add_constraint(ParamConstraint::period(2, 200, 14)) // required
            .add_constraint(ParamConstraint::flag("use_wilder", true)) // optional
            .build();

        let required = sig.required_params();
        assert_eq!(required.len(), 1);
        assert!(required.contains(&"period"));
    }

    #[test]
    fn test_param_names() {
        let sig = IndicatorSignature::builder("MACD", IndicatorCategory::Momentum)
            .add_constraint(ParamConstraint::period(2, 50, 12))
            .add_constraint(ParamConstraint::period(2, 50, 26))
            .add_constraint(ParamConstraint::period(2, 20, 9))
            .build();

        let names = sig.param_names();
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_metadata() {
        let mut sig = IndicatorSignature::builder("RSI", IndicatorCategory::Momentum).build();

        sig.set_metadata("version", "1.0");
        sig.set_metadata("source", "ta-lib");

        assert_eq!(sig.get_metadata("version"), Some("1.0"));
        assert_eq!(sig.get_metadata("source"), Some("ta-lib"));
        assert_eq!(sig.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_display() {
        let sig = IndicatorSignature::builder("RSI", IndicatorCategory::Momentum)
            .name("Relative Strength Index")
            .description("Momentum oscillator")
            .add_constraint(ParamConstraint::period(2, 200, 14))
            .build();

        let display = format!("{}", sig);
        assert!(display.contains("RSI"));
        assert!(display.contains("Relative Strength Index"));
        assert!(display.contains("momentum"));
        assert!(display.contains("period"));
    }

    #[test]
    fn test_ma_type_parameter() {
        let sig = IndicatorSignature::builder("EMA", IndicatorCategory::Average)
            .add_constraint(ParamConstraint::period(2, 200, 20))
            .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
            .build();

        let params = vec![
            ("period", ParamValue::USize(20)),
            ("ma_type", ParamValue::MaType(MovingAverageType::EMA)),
        ];

        assert!(sig.validate_params(&params).is_ok());
    }

    #[test]
    fn test_source_type_default() {
        let sig = IndicatorSignature::builder("SMA", IndicatorCategory::Average)
            .build();

        // Default should be PriceOnly
        assert_eq!(sig.source_type, SourceType::PriceOnly);
        assert!(sig.source_type.requires_source_selection());
    }

    #[test]
    fn test_source_type_explicit() {
        let sig = IndicatorSignature::builder("OBV", IndicatorCategory::Volume)
            .source_type(SourceType::VolumeOnly)
            .build();

        assert_eq!(sig.source_type, SourceType::VolumeOnly);
        assert!(!sig.source_type.requires_source_selection());
    }

    #[test]
    fn test_source_type_price_and_volume() {
        let sig = IndicatorSignature::builder("MFI", IndicatorCategory::Momentum)
            .source_type(SourceType::PriceAndVolume)
            .build();

        assert_eq!(sig.source_type, SourceType::PriceAndVolume);
        assert!(!sig.source_type.requires_source_selection());
    }

    #[test]
    fn test_source_type_from_str() {
        assert_eq!(SourceType::from_str("price_only"), Some(SourceType::PriceOnly));
        assert_eq!(SourceType::from_str("price"), Some(SourceType::PriceOnly));
        assert_eq!(SourceType::from_str("volume_only"), Some(SourceType::VolumeOnly));
        assert_eq!(SourceType::from_str("volume"), Some(SourceType::VolumeOnly));
        assert_eq!(SourceType::from_str("price_and_volume"), Some(SourceType::PriceAndVolume));
        assert_eq!(SourceType::from_str("both"), Some(SourceType::PriceAndVolume));
        assert_eq!(SourceType::from_str("PRICE_ONLY"), Some(SourceType::PriceOnly));
        assert_eq!(SourceType::from_str("invalid"), None);
    }

    #[test]
    fn test_source_type_as_str() {
        assert_eq!(SourceType::PriceOnly.as_str(), "price_only");
        assert_eq!(SourceType::VolumeOnly.as_str(), "volume_only");
        assert_eq!(SourceType::PriceAndVolume.as_str(), "price_and_volume");
    }

    #[test]
    fn test_source_type_display() {
        assert_eq!(format!("{}", SourceType::PriceOnly), "price_only");
        assert_eq!(format!("{}", SourceType::VolumeOnly), "volume_only");
        assert_eq!(format!("{}", SourceType::PriceAndVolume), "price_and_volume");
    }
}
