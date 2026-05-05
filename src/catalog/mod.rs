//! Indicator Catalog System
//!
//! Provides metadata and factory methods for all indicators.
//!
//! # Architecture
//!
//! The catalog system has three layers:
//!
//! 1. **Signature Layer** (`indicator_signature`, `master_catalog`)
//!    - Defines indicator parameters, constraints, and computational metadata
//!    - Pure computation, no rendering concerns
//!
//! 2. **Rendering Layer** (`rendering`, `rendering_catalog`, `value_adapter`)
//!    - Defines how indicators are visualized (overlay/sub-pane, colors, bounds)
//!    - Extracts render-ready values from IndicatorValue variants
//!
//! 3. **Unified Layer** (`unified_catalog`)
//!    - Combines signature and rendering metadata
//!    - Single access point for complete indicator information

pub mod master_catalog;
pub mod indicator_signature;
pub mod indicator_key;
pub mod constraints;
pub mod param_value;
pub mod parameter_grid;
pub mod rendering;
pub mod rendering_catalog;
pub mod value_adapter;
pub mod unified_catalog;
pub mod synthetic_data;
pub mod rendering_tests;

// Core catalog exports
pub use master_catalog::MasterIndicatorCatalog;
pub use indicator_signature::{IndicatorSignature, IndicatorCategory, SourceType};
pub use constraints::{ParamConstraint, ConstraintSet};
pub use param_value::{ParamValue, ParamType, ParamError};
pub use parameter_grid::ParameterValue;

// Rendering exports
pub use rendering::{
    RenderingMetadata, RenderingMetadataBuilder, OutputSpec, OutputType,
    ReferenceLine, LineStyle, HistogramStyle, ValueExtractor,
    ChannelPart, MacdPart, IchimokuPart, DoublePart, TriplePart,
    CandlePart, AdaptivePart, VolatilityPart, StatTestPart, CandleAnatomyPart,
};
pub use rendering_catalog::{get_rendering, has_rendering, rendering_count};
pub use value_adapter::ValueAdapter;

// Unified catalog exports
pub use unified_catalog::{UnifiedIndicatorCatalog, UnifiedIndicatorInfo, UnifiedCatalogStats};

// Synthetic data generators
pub use synthetic_data::{Bar, DataType, generate_bars, recommended_data_type};
