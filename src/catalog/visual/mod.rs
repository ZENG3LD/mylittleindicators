pub mod rendering;
pub mod rendering_catalog;
pub mod value_adapter;
#[cfg(test)]
mod rendering_tests;

pub use rendering::{
    RenderingMetadata, RenderingMetadataBuilder, OutputSpec, OutputType,
    ReferenceLine, LineStyle, HistogramStyle, ValueExtractor,
    ChannelPart, MacdPart, IchimokuPart, DoublePart, TriplePart,
    CandlePart, AdaptivePart, VolatilityPart, StatTestPart, CandleAnatomyPart,
};
pub use rendering_catalog::{get_rendering, has_rendering, rendering_count};
pub use value_adapter::ValueAdapter;
