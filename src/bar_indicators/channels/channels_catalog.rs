//! channels_catalog.rs: Catalog of all Channel/Band indicators
//!
//! Contains IndicatorSignature definitions for all 42 channel indicators in this category.
//! Channels are price bands/envelopes that help identify overbought/oversold conditions and volatility.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Channels;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Adaptive Bollinger Bands
///
/// Supports two modes:
/// - Auto mode (auto_mode=true, default): min/max ranges are computed automatically from base values
/// - Manual mode (auto_mode=false): all 6 parameters can be configured explicitly
pub fn signature_adaptive_bollinger_bands() -> IndicatorSignature {
    IndicatorSignature::builder("ADAPTIVEBB", CATEGORY)
        .name("Adaptive Bollinger Bands")
        .description("Bollinger Bands with adaptive period based on volatility. Supports auto and manual modes for min/max range configuration.")
        // Base parameters (always used)
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 10.0, 2.0))
        // Mode switch: true = auto (compute min/max), false = manual (use explicit values)
        .add_constraint(ParamConstraint::flag("auto_mode", true))
        // Manual mode parameters (only used when auto_mode=false)
        .add_constraint(
            ParamConstraint::new("min_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
        )
        .add_constraint(
            ParamConstraint::new("max_period", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(40))
        )
        .add_constraint(
            ParamConstraint::new("min_multiplier", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(1.0))
        )
        .add_constraint(
            ParamConstraint::new("max_multiplier", ParamType::F64)
                .with_min(ParamValue::F64(0.5))
                .with_max(ParamValue::F64(15.0))
                .with_default(ParamValue::F64(3.0))
        )
        .metadata("adaptive", "true")
        .metadata("outputs", "upper, middle, lower")
        .metadata("auto_mode_desc", "Auto mode: compute min/max ranges automatically")
        .metadata("min_period_desc", "Minimum adaptive period (manual mode)")
        .metadata("max_period_desc", "Maximum adaptive period (manual mode)")
        .metadata("min_multiplier_desc", "Minimum adaptive multiplier (manual mode)")
        .metadata("max_multiplier_desc", "Maximum adaptive multiplier (manual mode)")
        .machine_id(BarIndicatorId::Adaptivebb)
        .alias("Adaptivebb")
        .alias("adaptivebb")
        .alias("ADAPTIVEBOLLINGERBANDS")
        .alias("AdaptiveBollingerBands")
        .alias("adaptivebollingerbands")
        .alias("adaptive_bollinger_bands")
        .alias("ADAPTIVE_BOLLINGER_BANDS")
        .alias("Adaptive_Bollinger_Bands")
        .build()
}

/// Adaptive Channels
pub fn signature_adaptive_channels() -> IndicatorSignature {
    IndicatorSignature::builder("ADAPTIVECHAN", CATEGORY)
        .name("Adaptive Channels")
        .description("Price channels that adapt to market conditions")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .metadata("adaptive", "true")
        .machine_id(BarIndicatorId::Adaptivechan) // TODO: Add to enum
        // Note: "ADAPTIVECHAN" is already the main ID, no need for alias
        .alias("Adaptivechan")
        .alias("adaptivechan")
        .alias("ADAPTIVECHANNELS")
        .alias("AdaptiveChannels")
        .alias("adaptivechannels")
        .alias("adaptive_channels")
        .alias("ADAPTIVE_CHANNELS")
        .alias("Adaptive_Channels")
        .build()
}

/// ATR Channels
pub fn signature_atr_channels() -> IndicatorSignature {
    IndicatorSignature::builder("ATRCHAN", CATEGORY)
        .name("ATR Channels")
        .description("Volatility-based channels using Average True Range")
        .add_constraint(ParamConstraint::period(5, 100, 14))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .add_constraint(ParamConstraint::ma_type_named("center_ma", MovingAverageType::SMA))
        .add_constraint(ParamConstraint::ma_type_named("atr_ma", MovingAverageType::EMA))
        .metadata("volatility_based", "true")
        .metadata("outputs", "upper, middle, lower")
        .metadata("center_ma_desc", "Center line MA type")
        .metadata("atr_ma_desc", "ATR smoothing MA type")
        .machine_id(BarIndicatorId::Atrchan) // TODO: Add to enum
        // Note: "ATRCHAN" is already the main ID, no need for alias
        .alias("Atrchan")
        .alias("atrchan")
        .alias("ATRCHANNELS")
        .alias("ATRChannels")
        .alias("atrchannels")
        .alias("atr_channels")
        .alias("ATR_CHANNELS")
        .alias("Atr_Channels")
        .build()
}

/// Bollinger Bands
pub fn signature_bollinger_bands() -> IndicatorSignature {
    IndicatorSignature::builder("BB", CATEGORY)
        .name("Bollinger Bands")
        .description("Standard deviation bands around moving average")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .add_constraint(
            ParamConstraint::new("ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::SMA))
        )
        .metadata("author", "John Bollinger")
        .metadata("outputs", "upper, middle, lower, bandwidth, percent_b")
        .machine_id(BarIndicatorId::Bb) // TODO: Add to enum
        // Note: "BB" is already the main ID, no need for alias
        .alias("Bb")
        .alias("bb")
        .alias("BOLLINGERBANDS")
        .alias("BollingerBands")
        .alias("bollingerbands")
        .alias("bollinger_bands")
        .alias("BOLLINGER_BANDS")
        .alias("Bollinger_Bands")
        .build()
}

/// Bollinger Bands Metrics
pub fn signature_bollinger_metrics() -> IndicatorSignature {
    IndicatorSignature::builder("BBMETRICS", CATEGORY)
        .name("Bollinger Bands Metrics")
        .description("Additional metrics for Bollinger Bands analysis")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("outputs", "bandwidth, percent_b, squeeze")
        .machine_id(BarIndicatorId::Bbmetrics) // TODO: Add to enum
        // Note: "BBMETRICS" is already the main ID, no need for alias
        .alias("Bbmetrics")
        .alias("bbmetrics")
        .alias("BOLLINGERBANDSMETRICS")
        .alias("BollingerBandsMetrics")
        .alias("bollingerbandsmetrics")
        .alias("bollinger_bands_metrics")
        .alias("BOLLINGER_BANDS_METRICS")
        .alias("Bollinger_Bands_Metrics")
        .build()
}

/// Darvas Box
pub fn signature_darvas_box() -> IndicatorSignature {
    IndicatorSignature::builder("DARVAS", CATEGORY)
        .name("Darvas Box")
        .description("Box theory channel indicator for trend identification")
        .add_constraint(ParamConstraint::period(2, 50, 5))
        .metadata("author", "Nicolas Darvas")
        .metadata("trend_following", "true")
        .machine_id(BarIndicatorId::Darvas) // TODO: Add to enum
        // Note: "DARVAS" is already the main ID, no need for alias
        .alias("Darvas")
        .alias("darvas")
        .alias("DARVASBOX")
        .alias("DarvasBox")
        .alias("darvasbox")
        .alias("darvas_box")
        .alias("DARVAS_BOX")
        .alias("Darvas_Box")
        .build()
}

/// Donchian Channel
pub fn signature_donchian_channel() -> IndicatorSignature {
    IndicatorSignature::builder("DC", CATEGORY)
        .name("Donchian Channel")
        .description("Highest high and lowest low over N periods")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("author", "Richard Donchian")
        .metadata("outputs", "upper, middle, lower")
        .machine_id(BarIndicatorId::Dc) // TODO: Add to enum
        // Note: "DC" is already the main ID, no need for alias
        .alias("Dc")
        .alias("dc")
        .alias("DONCHIANCHANNEL")
        .alias("DonchianChannel")
        .alias("donchianchannel")
        .alias("donchian_channel")
        .alias("DONCHIAN_CHANNEL")
        .alias("Donchian_Channel")
        .build()
}

/// Donchian Channel Metrics
pub fn signature_donchian_channel_metrics() -> IndicatorSignature {
    IndicatorSignature::builder("DCMETRICS", CATEGORY)
        .name("Donchian Channel Metrics")
        .description("Additional metrics for Donchian Channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("outputs", "width, position")
        .machine_id(BarIndicatorId::Dcmetrics) // TODO: Add to enum
        // Note: "DCMETRICS" is already the main ID, no need for alias
        .alias("Dcmetrics")
        .alias("dcmetrics")
        .alias("DONCHIANCHANNELMETRICS")
        .alias("DonchianChannelMetrics")
        .alias("donchianchannelmetrics")
        .alias("donchian_channel_metrics")
        .alias("DONCHIAN_CHANNEL_METRICS")
        .alias("Donchian_Channel_Metrics")
        .build()
}

/// Donchian Position
pub fn signature_donchian_position() -> IndicatorSignature {
    IndicatorSignature::builder("DCPOS", CATEGORY)
        .name("Donchian Position")
        .description("Position of price within Donchian Channel (0-1)")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("range", "0-1")
        .machine_id(BarIndicatorId::Dcpos) // TODO: Add to enum
        // Note: "DCPOS" is already the main ID, no need for alias
        .alias("Dcpos")
        .alias("dcpos")
        .alias("DONCHIANPOSITION")
        .alias("DonchianPosition")
        .alias("donchianposition")
        .alias("donchian_position")
        .alias("DONCHIAN_POSITION")
        .alias("Donchian_Position")
        .build()
}

/// Donchian Width
pub fn signature_donchian_width() -> IndicatorSignature {
    IndicatorSignature::builder("DCWIDTH", CATEGORY)
        .name("Donchian Width")
        .description("Width of Donchian Channel as percentage")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .machine_id(BarIndicatorId::Dcwidth) // TODO: Add to enum
        // Note: "DCWIDTH" is already the main ID, no need for alias
        .alias("Dcwidth")
        .alias("dcwidth")
        .alias("DONCHIANWIDTH")
        .alias("DonchianWidth")
        .alias("donchianwidth")
        .alias("donchian_width")
        .alias("DONCHIAN_WIDTH")
        .alias("Donchian_Width")
        .build()
}

/// DPO Bands
pub fn signature_dpo_bands() -> IndicatorSignature {
    IndicatorSignature::builder("DPOBANDS", CATEGORY)
        .name("Detrended Price Oscillator Bands")
        .description("Bands based on detrended price oscillator")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Dpobands) // TODO: Add to enum
        // Note: "DPOBANDS" is already the main ID, no need for alias
        .alias("Dpobands")
        .alias("dpobands")
        .alias("DETRENDEDPRICEOSCILLATORBANDS")
        .alias("DetrendedPriceOscillatorBands")
        .alias("detrendedpriceoscillatorbands")
        .alias("detrended_price_oscillator_bands")
        .alias("DETRENDED_PRICE_OSCILLATOR_BANDS")
        .alias("Detrended_Price_Oscillator_Bands")
        .build()
}

/// Envelope Bandwidth
pub fn signature_envelope_bandwidth() -> IndicatorSignature {
    IndicatorSignature::builder("ENVBW", CATEGORY)
        .name("Envelope Bandwidth")
        .description("Bandwidth metric for envelope channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("percent", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(20.0))
                .with_default(ParamValue::F64(2.5))
        )
        .machine_id(BarIndicatorId::Envbw) // TODO: Add to enum
        // Note: "ENVBW" is already the main ID, no need for alias
        .alias("Envbw")
        .alias("envbw")
        .alias("ENVELOPEBANDWIDTH")
        .alias("EnvelopeBandwidth")
        .alias("envelopebandwidth")
        .alias("envelope_bandwidth")
        .alias("ENVELOPE_BANDWIDTH")
        .alias("Envelope_Bandwidth")
        .build()
}

/// Envelope Channels
pub fn signature_envelope_channels() -> IndicatorSignature {
    IndicatorSignature::builder("ENVELOPE", CATEGORY)
        .name("Envelope Channels")
        .description("Percentage-based channels around moving average")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("percent", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(20.0))
                .with_default(ParamValue::F64(2.5))
        )
        .metadata("outputs", "upper, middle, lower")
        .machine_id(BarIndicatorId::Envelope) // TODO: Add to enum
        // Note: "ENVELOPE" is already the main ID, no need for alias
        .alias("Envelope")
        .alias("envelope")
        .alias("ENVELOPECHANNELS")
        .alias("EnvelopeChannels")
        .alias("envelopechannels")
        .alias("envelope_channels")
        .alias("ENVELOPE_CHANNELS")
        .alias("Envelope_Channels")
        .build()
}

/// Fibonacci Channels
pub fn signature_fibonacci_channels() -> IndicatorSignature {
    IndicatorSignature::builder("FIBOCHAN", CATEGORY)
        .name("Fibonacci Channels")
        .description("Channels based on Fibonacci retracement levels")
        .add_constraint(ParamConstraint::period(5, 200, 50))
        .metadata("fibonacci", "true")
        .machine_id(BarIndicatorId::Fibochan) // TODO: Add to enum
        // Note: "FIBOCHAN" is already the main ID, no need for alias
        .alias("Fibochan")
        .alias("fibochan")
        .alias("FIBONACCICHANNELS")
        .alias("FibonacciChannels")
        .alias("fibonaccichannels")
        .alias("fibonacci_channels")
        .alias("FIBONACCI_CHANNELS")
        .alias("Fibonacci_Channels")
        .build()
}

/// Ichimoku Cloud
pub fn signature_ichimoku_cloud() -> IndicatorSignature {
    IndicatorSignature::builder("ICHIMOKU", CATEGORY)
        .name("Ichimoku Cloud")
        .description("Comprehensive trend-following indicator with multiple lines")
        .add_constraint(
            ParamConstraint::new("tenkan_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(9))
        )
        .add_constraint(
            ParamConstraint::new("kijun_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(100))
                .with_default(ParamValue::USize(26))
        )
        .add_constraint(
            ParamConstraint::new("senkou_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(52))
        )
        .metadata("author", "Goichi Hosoda")
        .metadata("outputs", "tenkan, kijun, senkou_a, senkou_b, chikou")
        .machine_id(BarIndicatorId::Ichimoku) // TODO: Add to enum
        // Note: "ICHIMOKU" is already the main ID, no need for alias
        .alias("Ichimoku")
        .alias("ichimoku")
        .alias("ICHIMOKUCLOUD")
        .alias("IchimokuCloud")
        .alias("ichimokucloud")
        .alias("ichimoku_cloud")
        .alias("ICHIMOKU_CLOUD")
        .alias("Ichimoku_Cloud")
        .build()
}

/// Ichimoku Cloud Position
pub fn signature_ichimoku_cloud_position() -> IndicatorSignature {
    IndicatorSignature::builder("ICHIMOKUPOS", CATEGORY)
        .name("Ichimoku Cloud Position")
        .description("Position of price relative to Ichimoku cloud")
        .add_constraint(ParamConstraint::period(2, 100, 9))
        .metadata("range", "above_cloud, in_cloud, below_cloud")
        .machine_id(BarIndicatorId::Ichimokupos) // TODO: Add to enum
        // Note: "ICHIMOKUPOS" is already the main ID, no need for alias
        .alias("Ichimokupos")
        .alias("ichimokupos")
        .alias("ICHIMOKUCLOUDPOSITION")
        .alias("IchimokuCloudPosition")
        .alias("ichimokucloudposition")
        .alias("ichimoku_cloud_position")
        .alias("ICHIMOKU_CLOUD_POSITION")
        .alias("Ichimoku_Cloud_Position")
        .build()
}

/// Ichimoku Cloud Thickness
pub fn signature_ichimoku_cloud_thickness() -> IndicatorSignature {
    IndicatorSignature::builder("ICHIMOKUTHICK", CATEGORY)
        .name("Ichimoku Cloud Thickness")
        .description("Thickness of Ichimoku cloud as volatility measure")
        .add_constraint(ParamConstraint::period(2, 100, 9))
        .machine_id(BarIndicatorId::Ichimokuthick) // TODO: Add to enum
        // Note: "ICHIMOKUTHICK" is already the main ID, no need for alias
        .alias("Ichimokuthick")
        .alias("ichimokuthick")
        .alias("ICHIMOKUCLOUDTHICKNESS")
        .alias("IchimokuCloudThickness")
        .alias("ichimokucloudthickness")
        .alias("ichimoku_cloud_thickness")
        .alias("ICHIMOKU_CLOUD_THICKNESS")
        .alias("Ichimoku_Cloud_Thickness")
        .build()
}

/// Keltner Bandwidth
pub fn signature_keltner_bandwidth() -> IndicatorSignature {
    IndicatorSignature::builder("KELTBW", CATEGORY)
        .name("Keltner Bandwidth")
        .description("Bandwidth of Keltner Channel")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Keltbw) // TODO: Add to enum
        // Note: "KELTBW" is already the main ID, no need for alias
        .alias("Keltbw")
        .alias("keltbw")
        .alias("KELTNERBANDWIDTH")
        .alias("KeltnerBandwidth")
        .alias("keltnerbandwidth")
        .alias("keltner_bandwidth")
        .alias("KELTNER_BANDWIDTH")
        .alias("Keltner_Bandwidth")
        .build()
}

/// Keltner Channel
pub fn signature_keltner_channel() -> IndicatorSignature {
    IndicatorSignature::builder("KC", CATEGORY)
        .name("Keltner Channel")
        .description("ATR-based volatility channels")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .add_constraint(
            ParamConstraint::new("ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("atr_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::RMA))
        )
        .metadata("author", "Chester Keltner")
        .metadata("outputs", "upper, middle, lower")
        .machine_id(BarIndicatorId::Kc) // TODO: Add to enum
        // Note: "KC" is already the main ID, no need for alias
        .alias("Kc")
        .alias("kc")
        .alias("KELTNERCHANNEL")
        .alias("KeltnerChannel")
        .alias("keltnerchannel")
        .alias("keltner_channel")
        .alias("KELTNER_CHANNEL")
        .alias("Keltner_Channel")
        .build()
}

/// Keltner Channel Metrics
pub fn signature_keltner_channel_metrics() -> IndicatorSignature {
    IndicatorSignature::builder("KCMETRICS", CATEGORY)
        .name("Keltner Channel Metrics")
        .description("Additional metrics for Keltner Channels")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("outputs", "bandwidth, distance, position")
        .machine_id(BarIndicatorId::Kcmetrics) // TODO: Add to enum
        // Note: "KCMETRICS" is already the main ID, no need for alias
        .alias("Kcmetrics")
        .alias("kcmetrics")
        .alias("KELTNERCHANNELMETRICS")
        .alias("KeltnerChannelMetrics")
        .alias("keltnerchannelmetrics")
        .alias("keltner_channel_metrics")
        .alias("KELTNER_CHANNEL_METRICS")
        .alias("Keltner_Channel_Metrics")
        .build()
}

/// Keltner Distance
pub fn signature_keltner_distance() -> IndicatorSignature {
    IndicatorSignature::builder("KELTDIST", CATEGORY)
        .name("Keltner Distance")
        .description("Distance of price from Keltner middle line")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Keltdist) // TODO: Add to enum
        // Note: "KELTDIST" is already the main ID, no need for alias
        .alias("Keltdist")
        .alias("keltdist")
        .alias("KELTNERDISTANCE")
        .alias("KeltnerDistance")
        .alias("keltnerdistance")
        .alias("keltner_distance")
        .alias("KELTNER_DISTANCE")
        .alias("Keltner_Distance")
        .build()
}

/// Keltner Position
pub fn signature_keltner_position() -> IndicatorSignature {
    IndicatorSignature::builder("KELTPOS", CATEGORY)
        .name("Keltner Position")
        .description("Position of price within Keltner Channel (0-1)")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("range", "0-1")
        .machine_id(BarIndicatorId::Keltpos) // TODO: Add to enum
        // Note: "KELTPOS" is already the main ID, no need for alias
        .alias("Keltpos")
        .alias("keltpos")
        .alias("KELTNERPOSITION")
        .alias("KeltnerPosition")
        .alias("keltnerposition")
        .alias("keltner_position")
        .alias("KELTNER_POSITION")
        .alias("Keltner_Position")
        .build()
}

/// Median Channel Position
pub fn signature_median_channel_position() -> IndicatorSignature {
    IndicatorSignature::builder("MEDCHANPOS", CATEGORY)
        .name("Median Channel Position")
        .description("Position within median-based channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("range", "0-1")
        .machine_id(BarIndicatorId::Medchanpos) // TODO: Add to enum
        // Note: "MEDCHANPOS" is already the main ID, no need for alias
        .alias("Medchanpos")
        .alias("medchanpos")
        .alias("MEDIANCHANNELPOSITION")
        .alias("MedianChannelPosition")
        .alias("medianchannelposition")
        .alias("median_channel_position")
        .alias("MEDIAN_CHANNEL_POSITION")
        .alias("Median_Channel_Position")
        .build()
}

/// Median Channels
pub fn signature_median_channels() -> IndicatorSignature {
    IndicatorSignature::builder("MEDCHAN", CATEGORY)
        .name("Median Channels")
        .description("Channels based on median prices")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Medchan) // TODO: Add to enum
        // Note: "MEDCHAN" is already the main ID, no need for alias
        .alias("Medchan")
        .alias("medchan")
        .alias("MEDIANCHANNELS")
        .alias("MedianChannels")
        .alias("medianchannels")
        .alias("median_channels")
        .alias("MEDIAN_CHANNELS")
        .alias("Median_Channels")
        .build()
}

/// Percent B
pub fn signature_percent_b() -> IndicatorSignature {
    IndicatorSignature::builder("PERCENTB", CATEGORY)
        .name("Percent B")
        .description("Position of price within Bollinger Bands (0-1)")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .metadata("range", "0-1")
        .metadata("overbought", ">1.0")
        .metadata("oversold", "<0.0")
        .machine_id(BarIndicatorId::Percentb) // TODO: Add to enum
        // Note: "PERCENTB" is already the main ID, no need for alias
        .alias("Percentb")
        .alias("percentb")
        .alias("PercentB")
        .alias("percent_b")
        .alias("PERCENT_B")
        .alias("Percent_B")
        .build()
}

/// Percentile Channels
pub fn signature_percentile_channels() -> IndicatorSignature {
    IndicatorSignature::builder("PERCENTILECH", CATEGORY)
        .name("Percentile Channels")
        .description("Channels based on percentile calculations")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(
            ParamConstraint::new("upper_percentile", ParamType::F64)
                .with_min(ParamValue::F64(50.0))
                .with_max(ParamValue::F64(100.0))
                .with_default(ParamValue::F64(90.0))
        )
        .add_constraint(
            ParamConstraint::new("lower_percentile", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(50.0))
                .with_default(ParamValue::F64(10.0))
        )
        .machine_id(BarIndicatorId::Percentilech) // TODO: Add to enum
        // Note: "PERCENTILECH" is already the main ID, no need for alias
        .alias("Percentilech")
        .alias("percentilech")
        .alias("PERCENTILECHANNELS")
        .alias("PercentileChannels")
        .alias("percentilechannels")
        .alias("percentile_channels")
        .alias("PERCENTILE_CHANNELS")
        .alias("Percentile_Channels")
        .build()
}

/// Pivot Channels
pub fn signature_pivot_channels() -> IndicatorSignature {
    IndicatorSignature::builder("PIVOTCHAN", CATEGORY)
        .name("Pivot Channels")
        .description("Channels based on pivot points")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("pivot_based", "true")
        .machine_id(BarIndicatorId::Pivotchan) // TODO: Add to enum
        // Note: "PIVOTCHAN" is already the main ID, no need for alias
        .alias("Pivotchan")
        .alias("pivotchan")
        .alias("PIVOTCHANNELS")
        .alias("PivotChannels")
        .alias("pivotchannels")
        .alias("pivot_channels")
        .alias("PIVOT_CHANNELS")
        .alias("Pivot_Channels")
        .build()
}

/// Price Channel Oscillator
pub fn signature_price_channel_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("PCHOSC", CATEGORY)
        .name("Price Channel Oscillator")
        .description("Oscillator showing position within price channel")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("range", "-100 to +100")
        .machine_id(BarIndicatorId::Pchosc) // TODO: Add to enum
        // Note: "PCHOSC" is already the main ID, no need for alias
        .alias("Pchosc")
        .alias("pchosc")
        .alias("PRICECHANNELOSCILLATOR")
        .alias("PriceChannelOscillator")
        .alias("pricechanneloscillator")
        .alias("price_channel_oscillator")
        .alias("PRICE_CHANNEL_OSCILLATOR")
        .alias("Price_Channel_Oscillator")
        .build()
}

/// Price Channel Width
pub fn signature_price_channel_width() -> IndicatorSignature {
    IndicatorSignature::builder("PCHWIDTH", CATEGORY)
        .name("Price Channel Width")
        .description("Width of price channel as volatility measure")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .machine_id(BarIndicatorId::Pchwidth) // TODO: Add to enum
        // Note: "PCHWIDTH" is already the main ID, no need for alias
        .alias("Pchwidth")
        .alias("pchwidth")
        .alias("PRICECHANNELWIDTH")
        .alias("PriceChannelWidth")
        .alias("pricechannelwidth")
        .alias("price_channel_width")
        .alias("PRICE_CHANNEL_WIDTH")
        .alias("Price_Channel_Width")
        .build()
}

/// Price Channels
pub fn signature_price_channels() -> IndicatorSignature {
    IndicatorSignature::builder("PRICECHAN", CATEGORY)
        .name("Price Channels")
        .description("Simple highest/lowest price channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .metadata("outputs", "upper, middle, lower")
        .machine_id(BarIndicatorId::Pricechan) // TODO: Add to enum
        // Note: "PRICECHAN" is already the main ID, no need for alias
        .alias("Pricechan")
        .alias("pricechan")
        .alias("PRICECHANNELS")
        .alias("PriceChannels")
        .alias("pricechannels")
        .alias("price_channels")
        .alias("PRICE_CHANNELS")
        .alias("Price_Channels")
        .build()
}

/// Projection Bands
pub fn signature_projection_bands() -> IndicatorSignature {
    IndicatorSignature::builder("PROJBANDS", CATEGORY)
        .name("Projection Bands")
        .description("Forward-looking projection bands")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Projbands) // TODO: Add to enum
        // Note: "PROJBANDS" is already the main ID, no need for alias
        .alias("Projbands")
        .alias("projbands")
        .alias("PROJECTIONBANDS")
        .alias("ProjectionBands")
        .alias("projectionbands")
        .alias("projection_bands")
        .alias("PROJECTION_BANDS")
        .alias("Projection_Bands")
        .build()
}

/// Quantile Regression Channels
pub fn signature_quantile_regression_channels() -> IndicatorSignature {
    IndicatorSignature::builder("QRCHAN", CATEGORY)
        .name("Quantile Regression Channels")
        .description("Regression-based channels using quantiles")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .add_constraint(
            ParamConstraint::new("quantile", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(0.5))
                .with_default(ParamValue::F64(0.25))
        )
        .metadata("regression_based", "true")
        .machine_id(BarIndicatorId::Qrchan) // TODO: Add to enum
        // Note: "QRCHAN" is already the main ID, no need for alias
        .alias("Qrchan")
        .alias("qrchan")
        .alias("QUANTILEREGRESSIONCHANNELS")
        .alias("QuantileRegressionChannels")
        .alias("quantileregressionchannels")
        .alias("quantile_regression_channels")
        .alias("QUANTILE_REGRESSION_CHANNELS")
        .alias("Quantile_Regression_Channels")
        .build()
}

/// Regression Channel Width
pub fn signature_regression_channel_width() -> IndicatorSignature {
    IndicatorSignature::builder("REGCHANWIDTH", CATEGORY)
        .name("Regression Channel Width")
        .description("Width of regression-based channels")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .machine_id(BarIndicatorId::Regchanwidth) // TODO: Add to enum
        // Note: "REGCHANWIDTH" is already the main ID, no need for alias
        .alias("Regchanwidth")
        .alias("regchanwidth")
        .alias("REGRESSIONCHANNELWIDTH")
        .alias("RegressionChannelWidth")
        .alias("regressionchannelwidth")
        .alias("regression_channel_width")
        .alias("REGRESSION_CHANNEL_WIDTH")
        .alias("Regression_Channel_Width")
        .build()
}

/// Regression Channels
pub fn signature_regression_channels() -> IndicatorSignature {
    IndicatorSignature::builder("REGCHAN", CATEGORY)
        .name("Regression Channels")
        .description("Linear regression-based price channels")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .add_constraint(ParamConstraint::multiplier(1.0, 5.0, 2.0))
        .metadata("regression_based", "true")
        .metadata("outputs", "upper, middle, lower")
        .machine_id(BarIndicatorId::Regchan) // TODO: Add to enum
        // Note: "REGCHAN" is already the main ID, no need for alias
        .alias("Regchan")
        .alias("regchan")
        .alias("REGRESSIONCHANNELS")
        .alias("RegressionChannels")
        .alias("regressionchannels")
        .alias("regression_channels")
        .alias("REGRESSION_CHANNELS")
        .alias("Regression_Channels")
        .build()
}

/// Standard Deviation Channels
pub fn signature_standard_deviation_channels() -> IndicatorSignature {
    IndicatorSignature::builder("STDDEVCHAN", CATEGORY)
        .name("Standard Deviation Channels")
        .description("Channels based on standard deviation calculation")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .metadata("outputs", "upper, middle, lower")
        .machine_id(BarIndicatorId::Stddevchan) // TODO: Add to enum
        // Note: "STDDEVCHAN" is already the main ID, no need for alias
        .alias("Stddevchan")
        .alias("stddevchan")
        .alias("STANDARDDEVIATIONCHANNELS")
        .alias("StandardDeviationChannels")
        .alias("standarddeviationchannels")
        .alias("standard_deviation_channels")
        .alias("STANDARD_DEVIATION_CHANNELS")
        .alias("Standard_Deviation_Channels")
        .build()
}

/// STARC Bands
pub fn signature_starc_bands() -> IndicatorSignature {
    IndicatorSignature::builder("STARC", CATEGORY)
        .name("STARC Bands")
        .description("Stoller Average Range Channels")
        .add_constraint(ParamConstraint::period(5, 100, 15))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("author", "Manning Stoller")
        .metadata("atr_based", "true")
        .machine_id(BarIndicatorId::Starc) // TODO: Add to enum
        // Note: "STARC" is already the main ID, no need for alias
        .alias("Starc")
        .alias("starc")
        .alias("STARCBANDS")
        .alias("STARCBands")
        .alias("starcbands")
        .alias("starc_bands")
        .alias("STARC_BANDS")
        .alias("Starc_Bands")
        .build()
}

/// StdDev Channel Width
pub fn signature_stddev_channel_width() -> IndicatorSignature {
    IndicatorSignature::builder("STDDEVWIDTH", CATEGORY)
        .name("Standard Deviation Channel Width")
        .description("Width of standard deviation channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .machine_id(BarIndicatorId::Stddevwidth) // TODO: Add to enum
        // Note: "STDDEVWIDTH" is already the main ID, no need for alias
        .alias("Stddevwidth")
        .alias("stddevwidth")
        .alias("STANDARDDEVIATIONCHANNELWIDTH")
        .alias("StandardDeviationChannelWidth")
        .alias("standarddeviationchannelwidth")
        .alias("standard_deviation_channel_width")
        .alias("STANDARD_DEVIATION_CHANNEL_WIDTH")
        .alias("Standard_Deviation_Channel_Width")
        .build()
}

/// Theil-Sen Channels
pub fn signature_theil_sen_channels() -> IndicatorSignature {
    IndicatorSignature::builder("THEILSENCHAN", CATEGORY)
        .name("Theil-Sen Channels")
        .description("Robust regression channels using Theil-Sen estimator")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .add_constraint(ParamConstraint::multiplier(1.0, 5.0, 2.0))
        .metadata("robust_regression", "true")
        .machine_id(BarIndicatorId::Theilsenchan) // TODO: Add to enum
        // Note: "THEILSENCHAN" is already the main ID, no need for alias
        .alias("Theilsenchan")
        .alias("theilsenchan")
        .alias("THEILSENCHANNELS")
        .alias("TheilSenChannels")
        .alias("theilsenchannels")
        .alias("theil_sen_channels")
        .alias("THEIL_SEN_CHANNELS")
        .alias("Theil_Sen_Channels")
        .build()
}

/// TRIMA Bands
pub fn signature_trima_bands() -> IndicatorSignature {
    IndicatorSignature::builder("TRIMABANDS", CATEGORY)
        .name("TRIMA Bands")
        .description("Bands based on Triangular Moving Average")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .machine_id(BarIndicatorId::Trimabands) // TODO: Add to enum
        // Note: "TRIMABANDS" is already the main ID, no need for alias
        .alias("Trimabands")
        .alias("trimabands")
        .alias("TRIMABands")
        .alias("trima_bands")
        .alias("TRIMA_BANDS")
        .alias("Trima_Bands")
        .build()
}

/// Volume Profile Channels
pub fn signature_volume_profile_channels() -> IndicatorSignature {
    IndicatorSignature::builder("VOLPROFCHAN", CATEGORY)
        .name("Volume Profile Channels")
        .description("Channels based on volume profile analysis")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .metadata("volume_based", "true")
        .metadata("outputs", "value_area_high, value_area_low, poc")
        .machine_id(BarIndicatorId::Volprofchan) // TODO: Add to enum
        // Note: "VOLPROFCHAN" is already the main ID, no need for alias
        .alias("Volprofchan")
        .alias("volprofchan")
        .alias("VOLUMEPROFILECHANNELS")
        .alias("VolumeProfileChannels")
        .alias("volumeprofilechannels")
        .alias("volume_profile_channels")
        .alias("VOLUME_PROFILE_CHANNELS")
        .alias("Volume_Profile_Channels")
        .build()
}

/// VWAP Channel Width
pub fn signature_vwap_channel_width() -> IndicatorSignature {
    IndicatorSignature::builder("VWAPCHANWIDTH", CATEGORY)
        .name("VWAP Channel Width")
        .description("Width of VWAP-based channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .machine_id(BarIndicatorId::Vwapchanwidth) // TODO: Add to enum
        // Note: "VWAPCHANWIDTH" is already the main ID, no need for alias
        .alias("Vwapchanwidth")
        .alias("vwapchanwidth")
        .alias("VWAPCHANNELWIDTH")
        .alias("VWAPChannelWidth")
        .alias("vwapchannelwidth")
        .alias("vwap_channel_width")
        .alias("VWAP_CHANNEL_WIDTH")
        .alias("Vwap_Channel_Width")
        .build()
}

/// VWAP Channels
pub fn signature_vwap_channels() -> IndicatorSignature {
    IndicatorSignature::builder("VWAPCHAN", CATEGORY)
        .name("VWAP Channels")
        .description("Volume-weighted average price channels")
        .add_constraint(ParamConstraint::period(5, 200, 20))
        .add_constraint(ParamConstraint::multiplier(0.5, 5.0, 2.0))
        .metadata("volume_based", "true")
        .metadata("outputs", "upper, vwap, lower")
        .machine_id(BarIndicatorId::Vwapchan) // TODO: Add to enum
        // Note: "VWAPCHAN" is already the main ID, no need for alias
        .alias("Vwapchan")
        .alias("vwapchan")
        .alias("VWAPCHANNELS")
        .alias("VWAPChannels")
        .alias("vwapchannels")
        .alias("vwap_channels")
        .alias("VWAP_CHANNELS")
        .alias("Vwap_Channels")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Channel indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ADAPTIVEBB", signature_adaptive_bollinger_bands as fn() -> IndicatorSignature),
    ("ADAPTIVECHAN", signature_adaptive_channels as fn() -> IndicatorSignature),
    ("ATRCHAN", signature_atr_channels as fn() -> IndicatorSignature),
    ("BB", signature_bollinger_bands as fn() -> IndicatorSignature),
    ("BBMETRICS", signature_bollinger_metrics as fn() -> IndicatorSignature),
    ("DARVAS", signature_darvas_box as fn() -> IndicatorSignature),
    ("DC", signature_donchian_channel as fn() -> IndicatorSignature),
    ("DCMETRICS", signature_donchian_channel_metrics as fn() -> IndicatorSignature),
    ("DCPOS", signature_donchian_position as fn() -> IndicatorSignature),
    ("DCWIDTH", signature_donchian_width as fn() -> IndicatorSignature),
    ("DPOBANDS", signature_dpo_bands as fn() -> IndicatorSignature),
    ("ENVBW", signature_envelope_bandwidth as fn() -> IndicatorSignature),
    ("ENVELOPE", signature_envelope_channels as fn() -> IndicatorSignature),
    ("FIBOCHAN", signature_fibonacci_channels as fn() -> IndicatorSignature),
    ("ICHIMOKU", signature_ichimoku_cloud as fn() -> IndicatorSignature),
    ("ICHIMOKUPOS", signature_ichimoku_cloud_position as fn() -> IndicatorSignature),
    ("ICHIMOKUTHICK", signature_ichimoku_cloud_thickness as fn() -> IndicatorSignature),
    ("KELTBW", signature_keltner_bandwidth as fn() -> IndicatorSignature),
    ("KC", signature_keltner_channel as fn() -> IndicatorSignature),
    ("KCMETRICS", signature_keltner_channel_metrics as fn() -> IndicatorSignature),
    ("KELTDIST", signature_keltner_distance as fn() -> IndicatorSignature),
    ("KELTPOS", signature_keltner_position as fn() -> IndicatorSignature),
    ("MEDCHANPOS", signature_median_channel_position as fn() -> IndicatorSignature),
    ("MEDCHAN", signature_median_channels as fn() -> IndicatorSignature),
    ("PERCENTB", signature_percent_b as fn() -> IndicatorSignature),
    ("PERCENTILECH", signature_percentile_channels as fn() -> IndicatorSignature),
    ("PIVOTCHAN", signature_pivot_channels as fn() -> IndicatorSignature),
    ("PCHOSC", signature_price_channel_oscillator as fn() -> IndicatorSignature),
    ("PCHWIDTH", signature_price_channel_width as fn() -> IndicatorSignature),
    ("PRICECHAN", signature_price_channels as fn() -> IndicatorSignature),
    ("PROJBANDS", signature_projection_bands as fn() -> IndicatorSignature),
    ("QRCHAN", signature_quantile_regression_channels as fn() -> IndicatorSignature),
    ("REGCHANWIDTH", signature_regression_channel_width as fn() -> IndicatorSignature),
    ("REGCHAN", signature_regression_channels as fn() -> IndicatorSignature),
    ("STDDEVCHAN", signature_standard_deviation_channels as fn() -> IndicatorSignature),
    ("STARC", signature_starc_bands as fn() -> IndicatorSignature),
    ("STDDEVWIDTH", signature_stddev_channel_width as fn() -> IndicatorSignature),
    ("THEILSENCHAN", signature_theil_sen_channels as fn() -> IndicatorSignature),
    ("TRIMABANDS", signature_trima_bands as fn() -> IndicatorSignature),
    ("VOLPROFCHAN", signature_volume_profile_channels as fn() -> IndicatorSignature),
    ("VWAPCHANWIDTH", signature_vwap_channel_width as fn() -> IndicatorSignature),
    ("VWAPCHAN", signature_vwap_channels as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static CHANNELS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();

    for &(main_id, func) in BASE_CATALOG {
        // Call function once to get signature with aliases
        let sig = func();

        // Insert main ID
        m.insert(main_id.to_string(), func);

        // Auto-insert all aliases from signature
        for alias in &sig.aliases {
            m.insert(alias.clone(), func);
        }
    }

    m
});

// ============================================================================
// Public API
// ============================================================================

/// Get indicator signature by ID
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    CHANNELS_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bollinger_bands() {
        let sig = get_signature("BB").unwrap();
        assert_eq!(sig.id, "BB");
        assert_eq!(sig.category, CATEGORY);
        assert!(sig.required_params().len() >= 1);
    }

    #[test]
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }

    #[test]
    fn test_count() {
        assert_eq!(count(), 42); // All channel indicators
    }
}
