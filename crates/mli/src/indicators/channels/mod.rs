//! channels: High-Performance Channel Indicators
//! Оптимизированные каналы с circular buffer O(1) operations и поддержкой всех MA типов

pub mod bb_period; // relocated from momentum/ (2026-06-15 reorg): classic Bollinger
pub use bb_period::*;
pub mod bollinger_bands;
pub mod donchian_channel;
pub mod keltner_channel;
pub mod ichimoku_cloud;
pub mod atr_channels;
pub mod price_channels;
pub mod vwap_channels;
pub mod regression_channels;
pub mod envelope_channels;
pub mod adaptive_channels;
pub mod standard_deviation_channels;
pub mod median_channels;
pub mod adaptive_bollinger_bands;
pub mod bollinger_metrics;
pub mod darvas_box;
pub mod donchian_channel_metrics;
pub mod donchian_position;
pub mod donchian_width;
pub mod dpo_bands;
pub mod envelope_bandwidth;
pub mod ichimoku_cloud_position;
pub mod ichimoku_cloud_thickness;
pub mod keltner_bandwidth;
pub mod keltner_channel_metrics;
pub mod keltner_distance;
pub mod keltner_position;
pub mod median_channel_position;
pub mod percent_b;
pub mod percentile_channels;
pub mod price_channel_oscillator;
pub mod price_channel_width;
pub mod projection_bands;
pub mod quantile_regression_channels;
pub mod regression_channel_width;
pub mod starc_bands;
pub mod stddev_channel_width;
pub mod theil_sen_channels;
pub mod trima_bands;
pub mod vwap_channel_width;

// pub mod box_channels;

// Re-export main types
pub use donchian_channel::{DonchianChannel, DonchianMode};
pub use keltner_channel::{KeltnerChannel, KeltnerMode};
pub use bollinger_bands::BollingerBands;
pub use atr_channels::{AtrChannels, AtrChannelMode};
pub use price_channels::{PriceChannels, PriceChannelMode};
pub use vwap_channels::{VwapChannels, VwapChannelMode};
pub use regression_channels::{RegressionChannels, RegressionChannelMode, TrendDirection};
pub use envelope_channels::{EnvelopeChannels, EnvelopeMode};
pub use ichimoku_cloud::{IchimokuCloud, CloudState, CloudPosition, IchimokuSignal};
pub use standard_deviation_channels::{StandardDeviationChannels, StandardDeviationMode, RegressionSource, StandardDeviationSignal};
pub use adaptive_channels::{AdaptiveChannels, AdaptationMode, CenterLineType, AdaptiveSignal, MarketRegime};
pub use median_channels::{MedianChannels, MedianMode, MedianSource, MedianSignal, QuantileLevels};
pub use adaptive_bollinger_bands::*; 
// pub use box_channels::{BoxedChannels, BoxChannelsFactory}; 























// Universal Indicator System catalog

// Relocated (Phase 1 reorg): vol percentile-rank BANDS (price bands) + donchian breakout flag.
pub mod volatility_percentile_rank_bands;
pub mod donchian_breakout;
pub use volatility_percentile_rank_bands::*;
pub use donchian_breakout::*;
