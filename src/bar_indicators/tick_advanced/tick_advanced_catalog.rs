//! tick_advanced_catalog.rs: Indicator catalog for advanced tick/trade flow indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::TickAdvanced;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_aggressor_burst_detector() -> IndicatorSignature {
    IndicatorSignature::builder("AGGRESSOR_BURST_DETECTOR", CATEGORY)
        .name("Aggressor Burst Detector")
        .description("Detects one-sided aggressor bursts in the trade stream (+1/-1/0)")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AggressorBurstDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("aggressor_burst_detector")
        .alias("AggressorBurstDetector")
        .build()
}

pub fn signature_large_tick_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("LARGE_TICK_MOMENTUM", CATEGORY)
        .name("Large Tick Momentum")
        .description("Directional momentum of large-size ticks only")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LargeTickMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("large_tick_momentum")
        .alias("LargeTickMomentum")
        .build()
}

pub fn signature_size_weighted_directional_momentum() -> IndicatorSignature {
    IndicatorSignature::builder("SIZE_WEIGHTED_DIRECTIONAL_MOMENTUM", CATEGORY)
        .name("Size-Weighted Directional Momentum")
        .description("Volume-weighted directional bias in range [-1, 1]")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::SizeWeightedDirectionalMomentum)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("size_weighted_directional_momentum")
        .alias("SizeWeightedDirectionalMomentum")
        .build()
}

pub fn signature_tick_frequency_anomaly() -> IndicatorSignature {
    IndicatorSignature::builder("TICK_FREQUENCY_ANOMALY", CATEGORY)
        .name("Tick Frequency Anomaly")
        .description("Tick-rate burst/quiet ratio anomaly detector")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::TickFrequencyAnomaly)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("tick_frequency_anomaly")
        .alias("TickFrequencyAnomaly")
        .build()
}

pub fn signature_tpo_session_balance() -> IndicatorSignature {
    IndicatorSignature::builder("TPO_SESSION_BALANCE", CATEGORY)
        .name("TPO Session Balance")
        .description("Time Price Opportunity session balance: POC position relative to session range")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::TpoSessionBalance)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("tpo_session_balance")
        .alias("TpoSessionBalance")
        .build()
}

pub fn signature_trade_run_detector() -> IndicatorSignature {
    IndicatorSignature::builder("TRADE_RUN_DETECTOR", CATEGORY)
        .name("Trade Run Detector")
        .description("Consecutive same-side tick run length detector")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::TradeRunDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("trade_run_detector")
        .alias("TradeRunDetector")
        .build()
}

pub fn signature_value_area_tracker() -> IndicatorSignature {
    IndicatorSignature::builder("VALUE_AREA_TRACKER", CATEGORY)
        .name("Value Area Tracker")
        .description("Rolling Volume Profile POC / VAH / VAL tracker")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::ValueAreaTracker)
        .role_kind(IndicatorRoleKind::Level)
        .output_kind(IndicatorValueKind::Triple)
        .input_stream(StreamKind::Tick)
        .alias("value_area_tracker")
        .alias("ValueAreaTracker")
        .build()
}

pub fn signature_volume_imbalance_zone() -> IndicatorSignature {
    IndicatorSignature::builder("VOLUME_IMBALANCE_ZONE", CATEGORY)
        .name("Volume Imbalance Zone")
        .description("Buy/sell volume imbalance zone detector")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::VolumeImbalanceZone)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Tick)
        .alias("volume_imbalance_zone")
        .alias("VolumeImbalanceZone")
        .build()
}

pub fn signature_vwap_deviation() -> IndicatorSignature {
    IndicatorSignature::builder("VWAP_DEVIATION", CATEGORY)
        .name("VWAP Deviation")
        .description("Rolling VWAP and price deviation percentage from it")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::VwapDeviation)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Double)
        .input_stream(StreamKind::Tick)
        .alias("vwap_deviation")
        .alias("VwapDeviation")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("AGGRESSOR_BURST_DETECTOR", signature_aggressor_burst_detector as fn() -> IndicatorSignature),
    ("LARGE_TICK_MOMENTUM", signature_large_tick_momentum as fn() -> IndicatorSignature),
    ("SIZE_WEIGHTED_DIRECTIONAL_MOMENTUM", signature_size_weighted_directional_momentum as fn() -> IndicatorSignature),
    ("TICK_FREQUENCY_ANOMALY", signature_tick_frequency_anomaly as fn() -> IndicatorSignature),
    ("TPO_SESSION_BALANCE", signature_tpo_session_balance as fn() -> IndicatorSignature),
    ("TRADE_RUN_DETECTOR", signature_trade_run_detector as fn() -> IndicatorSignature),
    ("VALUE_AREA_TRACKER", signature_value_area_tracker as fn() -> IndicatorSignature),
    ("VOLUME_IMBALANCE_ZONE", signature_volume_imbalance_zone as fn() -> IndicatorSignature),
    ("VWAP_DEVIATION", signature_vwap_deviation as fn() -> IndicatorSignature),
];

pub static TICK_ADVANCED_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    TICK_ADVANCED_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
