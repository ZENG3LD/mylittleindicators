//! stress_catalog.rs: Indicator catalog for stress indicators (insurance fund, settlement)

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::Stress;

static AUX_MARK_PRICE: &[StreamKind] = &[StreamKind::MarkPrice];

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_fund_depletion_rate() -> IndicatorSignature {
    IndicatorSignature::builder("FUND_DEPLETION_RATE", CATEGORY)
        .name("Fund Depletion Rate")
        .description("Rate at which the insurance fund is being depleted")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundDepletionRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::InsuranceFund)
        .alias("fund_depletion_rate")
        .alias("FundDepletionRate")
        .build()
}

pub fn signature_fund_stress_detector() -> IndicatorSignature {
    IndicatorSignature::builder("FUND_STRESS_DETECTOR", CATEGORY)
        .name("Fund Stress Detector")
        .description("Detects stress conditions in the insurance fund balance")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::FundStressDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::InsuranceFund)
        .alias("fund_stress_detector")
        .alias("FundStressDetector")
        .build()
}

pub fn signature_insurance_fund_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("INSURANCE_FUND_MOMENTUM", CATEGORY)
        .name("Insurance Fund Momentum")
        .description("Rate of change of the insurance fund balance over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::InsuranceFundMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::InsuranceFund)
        .alias("insurance_fund_momentum")
        .alias("InsuranceFundMomentum")
        .build()
}

pub fn signature_settlement_approach_signal() -> IndicatorSignature {
    IndicatorSignature::builder("SETTLEMENT_APPROACH_SIGNAL", CATEGORY)
        .name("Settlement Approach Signal")
        .description("Signal strength as settlement date approaches (time-decay driven)")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SettlementApproachSignal)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Settlement)
        .alias("settlement_approach_signal")
        .alias("SettlementApproachSignal")
        .build()
}

pub fn signature_settlement_price_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("SETTLEMENT_PRICE_MOMENTUM", CATEGORY)
        .name("Settlement Price Momentum")
        .description("Rate of change of the settlement price over a rolling window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SettlementPriceMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Settlement)
        .alias("settlement_price_momentum")
        .alias("SettlementPriceMomentum")
        .build()
}

pub fn signature_settlement_vs_mark_spread() -> IndicatorSignature {
    IndicatorSignature::builder("SETTLEMENT_VS_MARK_SPREAD", CATEGORY)
        .name("Settlement vs Mark Spread")
        .description("Spread between settlement price and current mark price")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SettlementVsMarkSpread)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Settlement)
        .aux_streams(AUX_MARK_PRICE)
        .alias("settlement_vs_mark_spread")
        .alias("SettlementVsMarkSpread")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("FUND_DEPLETION_RATE", signature_fund_depletion_rate as fn() -> IndicatorSignature),
    ("FUND_STRESS_DETECTOR", signature_fund_stress_detector as fn() -> IndicatorSignature),
    ("INSURANCE_FUND_MOMENTUM", signature_insurance_fund_momentum as fn() -> IndicatorSignature),
    ("SETTLEMENT_APPROACH_SIGNAL", signature_settlement_approach_signal as fn() -> IndicatorSignature),
    ("SETTLEMENT_PRICE_MOMENTUM", signature_settlement_price_momentum as fn() -> IndicatorSignature),
    ("SETTLEMENT_VS_MARK_SPREAD", signature_settlement_vs_mark_spread as fn() -> IndicatorSignature),
];

pub static STRESS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    STRESS_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
