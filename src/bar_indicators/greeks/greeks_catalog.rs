//! greeks_catalog.rs: Indicator catalog for option Greeks indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::Greeks;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_charm_tracker() -> IndicatorSignature {
    IndicatorSignature::builder("CHARM_TRACKER", CATEGORY)
        .name("Charm Tracker")
        .description("Tracks charm (delta decay) — rate of change of delta with respect to time")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::CharmTracker)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("charm_tracker")
        .alias("CharmTracker")
        .build()
}

pub fn signature_delta_exposure_flow() -> IndicatorSignature {
    IndicatorSignature::builder("DELTA_EXPOSURE_FLOW", CATEGORY)
        .name("Delta Exposure Flow")
        .description("Rolling net delta exposure flow from options market activity")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::DeltaExposureFlow)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("delta_exposure_flow")
        .alias("DeltaExposureFlow")
        .build()
}

pub fn signature_gamma_squeeze_detector() -> IndicatorSignature {
    IndicatorSignature::builder("GAMMA_SQUEEZE_DETECTOR", CATEGORY)
        .name("Gamma Squeeze Detector")
        .description("Detects conditions for a gamma squeeze from options market maker exposure")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::GammaSqueezeDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("gamma_squeeze_detector")
        .alias("GammaSqueezeDetector")
        .build()
}

pub fn signature_iv_skew() -> IndicatorSignature {
    IndicatorSignature::builder("IV_SKEW", CATEGORY)
        .name("IV Skew")
        .description("Implied volatility skew between puts and calls at same expiry")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::IvSkew)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("iv_skew")
        .alias("IvSkew")
        .build()
}

pub fn signature_pin_risk_detector() -> IndicatorSignature {
    IndicatorSignature::builder("PIN_RISK_DETECTOR", CATEGORY)
        .name("Pin Risk Detector")
        .description("Detects option pin risk as expiry approaches at a specific strike")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::PinRiskDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("pin_risk_detector")
        .alias("PinRiskDetector")
        .build()
}

pub fn signature_theta_decay_tracker() -> IndicatorSignature {
    IndicatorSignature::builder("THETA_DECAY_TRACKER", CATEGORY)
        .name("Theta Decay Tracker")
        .description("Tracks theta decay rate and cumulative time value erosion")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::ThetaDecayTracker)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("theta_decay_tracker")
        .alias("ThetaDecayTracker")
        .build()
}

pub fn signature_vega_exposure_flow() -> IndicatorSignature {
    IndicatorSignature::builder("VEGA_EXPOSURE_FLOW", CATEGORY)
        .name("Vega Exposure Flow")
        .description("Rolling net vega exposure from options positioning changes")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::VegaExposureFlow)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::OptionGreeks)
        .alias("vega_exposure_flow")
        .alias("VegaExposureFlow")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("CHARM_TRACKER", signature_charm_tracker as fn() -> IndicatorSignature),
    ("DELTA_EXPOSURE_FLOW", signature_delta_exposure_flow as fn() -> IndicatorSignature),
    ("GAMMA_SQUEEZE_DETECTOR", signature_gamma_squeeze_detector as fn() -> IndicatorSignature),
    ("IV_SKEW", signature_iv_skew as fn() -> IndicatorSignature),
    ("PIN_RISK_DETECTOR", signature_pin_risk_detector as fn() -> IndicatorSignature),
    ("THETA_DECAY_TRACKER", signature_theta_decay_tracker as fn() -> IndicatorSignature),
    ("VEGA_EXPOSURE_FLOW", signature_vega_exposure_flow as fn() -> IndicatorSignature),
];

pub static GREEKS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    GREEKS_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
