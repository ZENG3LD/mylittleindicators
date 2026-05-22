//! risk_funding_catalog.rs: Indicator catalog for risk, funding, and auction indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::RiskFunding;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_auction_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("AUCTION_IMBALANCE", CATEGORY)
        .name("Auction Imbalance")
        .description("Buy/sell imbalance detected during auction events")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AuctionImbalance)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Auction)
        .alias("auction_imbalance")
        .alias("AuctionImbalance")
        .build()
}

pub fn signature_auction_liquidity_score() -> IndicatorSignature {
    IndicatorSignature::builder("AUCTION_LIQUIDITY_SCORE", CATEGORY)
        .name("Auction Liquidity Score")
        .description("Liquidity quality score derived from auction event data")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AuctionLiquidityScore)
        .role_kind(IndicatorRoleKind::Statistical)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Auction)
        .alias("auction_liquidity_score")
        .alias("AuctionLiquidityScore")
        .build()
}

pub fn signature_auction_price_deviation() -> IndicatorSignature {
    IndicatorSignature::builder("AUCTION_PRICE_DEVIATION", CATEGORY)
        .name("Auction Price Deviation")
        .description("Deviation of auction clearing price from pre-auction mid price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AuctionPriceDeviation)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Auction)
        .alias("auction_price_deviation")
        .alias("AuctionPriceDeviation")
        .build()
}

pub fn signature_funding_drift() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_DRIFT", CATEGORY)
        .name("Funding Drift")
        .description("Cumulative drift of funding rate from zero over time")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingDrift)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("funding_drift")
        .alias("FundingDrift")
        .build()
}

pub fn signature_settled_funding_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("SETTLED_FUNDING_MOMENTUM", CATEGORY)
        .name("Settled Funding Momentum")
        .description("Momentum of the settled funding rate series")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SettledFundingMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("settled_funding_momentum")
        .alias("SettledFundingMomentum")
        .build()
}

pub fn signature_funding_time_decay() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_TIME_DECAY", CATEGORY)
        .name("Funding Time Decay")
        .description("Time-decay weighting applied to historical funding rates")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingTimeDecay)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Funding)
        .alias("funding_time_decay")
        .alias("FundingTimeDecay")
        .build()
}

pub fn signature_funding_settlement_impact() -> IndicatorSignature {
    IndicatorSignature::builder("FUNDING_SETTLEMENT_IMPACT", CATEGORY)
        .name("Funding Settlement Impact")
        .description("Price impact around funding settlement events")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundingSettlementImpact)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::FundingSettlement)
        .alias("funding_settlement_impact")
        .alias("FundingSettlementImpact")
        .build()
}

pub fn signature_leverage_reduction_warning() -> IndicatorSignature {
    IndicatorSignature::builder("LEVERAGE_REDUCTION_WARNING", CATEGORY)
        .name("Leverage Reduction Warning")
        .description("Warning signal when auto-deleveraging or leverage reduction events occur")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LeverageReductionWarning)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::MarketWarning)
        .alias("leverage_reduction_warning")
        .alias("LeverageReductionWarning")
        .build()
}

pub fn signature_warning_frequency_filter() -> IndicatorSignature {
    IndicatorSignature::builder("WARNING_FREQUENCY_FILTER", CATEGORY)
        .name("Warning Frequency Filter")
        .description("Filters and smooths market warning events by frequency")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::WarningFrequencyFilter)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::MarketWarning)
        .alias("warning_frequency_filter")
        .alias("WarningFrequencyFilter")
        .build()
}

pub fn signature_warning_rate() -> IndicatorSignature {
    IndicatorSignature::builder("WARNING_RATE", CATEGORY)
        .name("Warning Rate")
        .description("Rolling rate of market warning events per unit time")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::WarningRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::MarketWarning)
        .alias("warning_rate")
        .alias("WarningRate")
        .build()
}

pub fn signature_mmr_tracker() -> IndicatorSignature {
    IndicatorSignature::builder("MMR_TRACKER", CATEGORY)
        .name("MMR Tracker")
        .description("Tracks maintenance margin ratio changes and proximity to liquidation")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MmrTracker)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::RiskLimit)
        .alias("mmr_tracker")
        .alias("MmrTracker")
        .build()
}

pub fn signature_risk_limit_proximity() -> IndicatorSignature {
    IndicatorSignature::builder("RISK_LIMIT_PROXIMITY", CATEGORY)
        .name("Risk Limit Proximity")
        .description("Distance from current position size to risk limit threshold")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RiskLimitProximity)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::RiskLimit)
        .alias("risk_limit_proximity")
        .alias("RiskLimitProximity")
        .build()
}

pub fn signature_predicted_funding_extreme() -> IndicatorSignature {
    IndicatorSignature::builder("PREDICTED_FUNDING_EXTREME", CATEGORY)
        .name("Predicted Funding Extreme")
        .description("Detects when predicted funding rate reaches extreme levels")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::PredictedFundingExtreme)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::PredictedFunding)
        .alias("predicted_funding_extreme")
        .alias("PredictedFundingExtreme")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("AUCTION_IMBALANCE", signature_auction_imbalance as fn() -> IndicatorSignature),
    ("AUCTION_LIQUIDITY_SCORE", signature_auction_liquidity_score as fn() -> IndicatorSignature),
    ("AUCTION_PRICE_DEVIATION", signature_auction_price_deviation as fn() -> IndicatorSignature),
    ("FUNDING_DRIFT", signature_funding_drift as fn() -> IndicatorSignature),
    ("SETTLED_FUNDING_MOMENTUM", signature_settled_funding_momentum as fn() -> IndicatorSignature),
    ("FUNDING_TIME_DECAY", signature_funding_time_decay as fn() -> IndicatorSignature),
    ("FUNDING_SETTLEMENT_IMPACT", signature_funding_settlement_impact as fn() -> IndicatorSignature),
    ("LEVERAGE_REDUCTION_WARNING", signature_leverage_reduction_warning as fn() -> IndicatorSignature),
    ("WARNING_FREQUENCY_FILTER", signature_warning_frequency_filter as fn() -> IndicatorSignature),
    ("WARNING_RATE", signature_warning_rate as fn() -> IndicatorSignature),
    ("MMR_TRACKER", signature_mmr_tracker as fn() -> IndicatorSignature),
    ("RISK_LIMIT_PROXIMITY", signature_risk_limit_proximity as fn() -> IndicatorSignature),
    ("PREDICTED_FUNDING_EXTREME", signature_predicted_funding_extreme as fn() -> IndicatorSignature),
];

pub static RISK_FUNDING_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    RISK_FUNDING_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
