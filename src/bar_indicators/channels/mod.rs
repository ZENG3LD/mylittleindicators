//! channels: High-Performance Channel Indicators
//! Оптимизированные каналы с circular buffer O(1) operations и поддержкой всех MA типов

pub mod bollinger_bands;
pub mod donchian_channel;
pub mod keltner_channel;
pub mod ichimoku_cloud;
pub mod atr_channels;
pub mod price_channels;
pub mod vwap_channels;
pub mod regression_channels;
pub mod envelope_channels;
pub mod fibonacci_channels;
pub mod pivot_channels;
pub mod volume_profile_channels;
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
pub use bollinger_bands::{BollingerBands, BollingerMode};
pub use atr_channels::{AtrChannels, AtrChannelMode};
pub use price_channels::{PriceChannels, PriceChannelMode};
pub use vwap_channels::{VwapChannels, VwapChannelMode};
pub use regression_channels::{RegressionChannels, RegressionChannelMode, TrendDirection};
pub use envelope_channels::{EnvelopeChannels, EnvelopeMode};
pub use fibonacci_channels::{FibonacciChannels, FibonacciChannelMode, FibonacciChannel};
pub use ichimoku_cloud::{IchimokuCloud, CloudState, CloudPosition, IchimokuSignal};
pub use standard_deviation_channels::{StandardDeviationChannels, StandardDeviationMode, RegressionSource, StandardDeviationSignal};
pub use pivot_channels::{PivotChannels, PivotType, PivotPeriod, PivotSignal, PivotChannelLevels};
pub use volume_profile_channels::{VolumeProfileChannels, VolumeProfileMode, VolumeProfilePeriod, VolumeProfileSignal};
pub use adaptive_channels::{AdaptiveChannels, AdaptationMode, CenterLineType, AdaptiveSignal, MarketRegime};
pub use median_channels::{MedianChannels, MedianMode, MedianSource, MedianSignal, QuantileLevels};
pub use adaptive_bollinger_bands::*; 
// pub use box_channels::{BoxedChannels, BoxChannelsFactory}; 























// Universal Indicator System catalog
pub mod channels_catalog;
