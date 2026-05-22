//! composites_catalog.rs: Indicator catalog for cross-stream composite indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::Composites;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_adaptive_threshold() -> IndicatorSignature {
    IndicatorSignature::builder("ADAPTIVE_THRESHOLD", CATEGORY)
        .name("Adaptive Threshold")
        .description("Dynamically adjusts signal threshold based on recent volatility regime")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AdaptiveThreshold)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("adaptive_threshold")
        .alias("AdaptiveThreshold")
        .build()
}

pub fn signature_adaptive_window_selector() -> IndicatorSignature {
    IndicatorSignature::builder("ADAPTIVE_WINDOW_SELECTOR", CATEGORY)
        .name("Adaptive Window Selector")
        .description("Selects optimal lookback window based on market microstructure conditions")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AdaptiveWindowSelector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("adaptive_window_selector")
        .alias("AdaptiveWindowSelector")
        .build()
}

pub fn signature_block_trade_volume_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("BLOCK_TRADE_VOLUME_RATIO", CATEGORY)
        .name("Block Trade Volume Ratio")
        .description("Ratio of block trade volume to total aggregated trade volume")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::BlockTradeVolumeRatio)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("block_trade_volume_ratio")
        .alias("BlockTradeVolumeRatio")
        .build()
}

pub fn signature_capitulation_detector() -> IndicatorSignature {
    IndicatorSignature::builder("CAPITULATION_DETECTOR", CATEGORY)
        .name("Capitulation Detector")
        .description("Detects capitulation events from combined liquidation, trade flow, and price signals")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::CapitulationDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("capitulation_detector")
        .alias("CapitulationDetector")
        .build()
}

pub fn signature_compound_squeeze_probability() -> IndicatorSignature {
    IndicatorSignature::builder("COMPOUND_SQUEEZE_PROBABILITY", CATEGORY)
        .name("Compound Squeeze Probability")
        .description("Probability of a squeeze from OI, liquidations, mark price, and funding combined")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::CompoundSqueezeProbability)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("compound_squeeze_probability")
        .alias("CompoundSqueezeProbability")
        .build()
}

pub fn signature_cross_asset_beta() -> IndicatorSignature {
    IndicatorSignature::builder("CROSS_ASSET_BETA", CATEGORY)
        .name("Cross-Asset Beta")
        .description("Rolling beta coefficient between primary and secondary asset price series")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::CrossAssetBeta)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("cross_asset_beta")
        .alias("CrossAssetBeta")
        .build()
}

pub fn signature_funding_oi_pressure() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_OI_PRESSURE", CATEGORY)
        .name("Funding OI Pressure")
        .description("Combined pressure signal from funding rate and open interest changes")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingOiPressure)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("funding_oi_pressure")
        .alias("FundingOiPressure")
        .build()
}

pub fn signature_funding_sentiment_alignment() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_SENTIMENT_ALIGNMENT", CATEGORY)
        .name("Funding Sentiment Alignment")
        .description("Alignment score between funding rate direction and long/short ratio sentiment")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingSentimentAlignment)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("funding_sentiment_alignment")
        .alias("FundingSentimentAlignment")
        .build()
}

pub fn signature_index_tracking_error() -> IndicatorSignature {
    IndicatorSignature::builder("INDEX_TRACKING_ERROR", CATEGORY)
        .name("Index Tracking Error")
        .description("Rolling tracking error between perpetual price and underlying index")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::IndexTrackingError)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("index_tracking_error")
        .alias("IndexTrackingError")
        .build()
}

pub fn signature_iv_hv_spread() -> IndicatorSignature {
    IndicatorSignature::builder("IV_HV_SPREAD", CATEGORY)
        .name("IV-HV Spread")
        .description("Spread between implied volatility index and realized historical volatility")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::IvHvSpread)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("iv_hv_spread")
        .alias("IvHvSpread")
        .build()
}

pub fn signature_market_stress_composite() -> IndicatorSignature {
    IndicatorSignature::builder("MARKET_STRESS_COMPOSITE", CATEGORY)
        .name("Market Stress Composite")
        .description("Composite stress score from vol index, liquidations, funding, and insurance fund")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MarketStressComposite)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("market_stress_composite")
        .alias("MarketStressComposite")
        .build()
}

pub fn signature_pairs_cointegration_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("PAIRS_COINTEGRATION_PROXY", CATEGORY)
        .name("Pairs Cointegration Proxy")
        .description("Rolling cointegration proxy between two price series for pairs trading")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::PairsCointegrationProxy)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("pairs_cointegration_proxy")
        .alias("PairsCointegrationProxy")
        .build()
}

pub fn signature_relative_strength_cross() -> IndicatorSignature {
    IndicatorSignature::builder("RELATIVE_STRENGTH_CROSS", CATEGORY)
        .name("Relative Strength Cross")
        .description("Relative strength comparison between two assets using rolling returns")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RelativeStrengthCross)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("relative_strength_cross")
        .alias("RelativeStrengthCross")
        .build()
}

pub fn signature_risk_off_detector() -> IndicatorSignature {
    IndicatorSignature::builder("RISK_OFF_DETECTOR", CATEGORY)
        .name("Risk-Off Detector")
        .description("Detects risk-off regimes from vol index, liquidations, funding, and insurance fund signals")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RiskOffDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("risk_off_detector")
        .alias("RiskOffDetector")
        .build()
}

pub fn signature_sentiment_composite() -> IndicatorSignature {
    IndicatorSignature::builder("SENTIMENT_COMPOSITE", CATEGORY)
        .name("Sentiment Composite")
        .description("Composite sentiment from long/short ratio, aggregated trade flow, and funding rate")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SentimentComposite)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("sentiment_composite")
        .alias("SentimentComposite")
        .build()
}

pub fn signature_squeeze_probability() -> IndicatorSignature {
    IndicatorSignature::builder("SQUEEZE_PROBABILITY", CATEGORY)
        .name("Squeeze Probability")
        .description("Probability of a short/long squeeze from OI, mark price, and liquidation inputs")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SqueezeProbability)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("squeeze_probability")
        .alias("SqueezeProbability")
        .build()
}

pub fn signature_vol_regime_entry() -> IndicatorSignature {
    IndicatorSignature::builder("VOL_REGIME_ENTRY", CATEGORY)
        .name("Vol Regime Entry")
        .description("Entry signal based on volatility regime transition from vol index and mark price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::VolRegimeEntry)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Bar)
        .alias("vol_regime_entry")
        .alias("VolRegimeEntry")
        .build()
}

static AUX_FUNDING: &[StreamKind] = &[StreamKind::Funding];

pub fn signature_funding_price_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_PRICE_DIVERGENCE", CATEGORY)
        .name("Funding Price Divergence")
        .description("Composite: funding momentum × price momentum divergence — detects short-squeeze or long-liquidation setups")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingPriceDivergence)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Triple)
        .input_stream(StreamKind::Bar)
        .aux_streams(AUX_FUNDING)
        .alias("funding_price_divergence")
        .alias("FundingPriceDivergence")
        .alias("FUNDINGPRICEDIVERGENCE")
        .alias("FundingPriceMomentumDivergence")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ADAPTIVE_THRESHOLD", signature_adaptive_threshold as fn() -> IndicatorSignature),
    ("ADAPTIVE_WINDOW_SELECTOR", signature_adaptive_window_selector as fn() -> IndicatorSignature),
    ("BLOCK_TRADE_VOLUME_RATIO", signature_block_trade_volume_ratio as fn() -> IndicatorSignature),
    ("CAPITULATION_DETECTOR", signature_capitulation_detector as fn() -> IndicatorSignature),
    ("COMPOUND_SQUEEZE_PROBABILITY", signature_compound_squeeze_probability as fn() -> IndicatorSignature),
    ("CROSS_ASSET_BETA", signature_cross_asset_beta as fn() -> IndicatorSignature),
    ("FUNDING_OI_PRESSURE", signature_funding_oi_pressure as fn() -> IndicatorSignature),
    ("FUNDING_SENTIMENT_ALIGNMENT", signature_funding_sentiment_alignment as fn() -> IndicatorSignature),
    ("INDEX_TRACKING_ERROR", signature_index_tracking_error as fn() -> IndicatorSignature),
    ("IV_HV_SPREAD", signature_iv_hv_spread as fn() -> IndicatorSignature),
    ("MARKET_STRESS_COMPOSITE", signature_market_stress_composite as fn() -> IndicatorSignature),
    ("PAIRS_COINTEGRATION_PROXY", signature_pairs_cointegration_proxy as fn() -> IndicatorSignature),
    ("RELATIVE_STRENGTH_CROSS", signature_relative_strength_cross as fn() -> IndicatorSignature),
    ("RISK_OFF_DETECTOR", signature_risk_off_detector as fn() -> IndicatorSignature),
    ("SENTIMENT_COMPOSITE", signature_sentiment_composite as fn() -> IndicatorSignature),
    ("SQUEEZE_PROBABILITY", signature_squeeze_probability as fn() -> IndicatorSignature),
    ("VOL_REGIME_ENTRY", signature_vol_regime_entry as fn() -> IndicatorSignature),
    ("FUNDING_PRICE_DIVERGENCE", signature_funding_price_divergence as fn() -> IndicatorSignature),
];

pub static COMPOSITES_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for &(main_id, func) in BASE_CATALOG {
        let sig = func();
        m.insert(main_id.to_string(), func);
        for alias in &sig.aliases {
            m.insert(alias.clone(), func);
        }
    }
    m
});

pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    COMPOSITES_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
