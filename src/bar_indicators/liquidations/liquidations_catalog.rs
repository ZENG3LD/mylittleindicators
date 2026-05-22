//! liquidations_catalog.rs: Indicator catalog for liquidation event indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, IndicatorRoleKind, ParamConstraint,
};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::data_loader::stream_kind::StreamKind;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const CATEGORY: IndicatorCategory = IndicatorCategory::Liquidations;

// ============================================================================
// Individual indicator signatures
// ============================================================================

pub fn signature_liquidation_cascade() -> IndicatorSignature {
    IndicatorSignature::builder("LIQUIDATION_CASCADE", CATEGORY)
        .name("Liquidation Cascade")
        .description("Detects cascading liquidation events within a short time window")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LiquidationCascade)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("liquidation_cascade")
        .alias("LiquidationCascade")
        .build()
}

pub fn signature_liquidation_cluster_detector() -> IndicatorSignature {
    IndicatorSignature::builder("LIQUIDATION_CLUSTER_DETECTOR", CATEGORY)
        .name("Liquidation Cluster Detector")
        .description("Identifies price zones with high concentration of liquidation events")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LiquidationClusterDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("liquidation_cluster_detector")
        .alias("LiquidationClusterDetector")
        .build()
}

pub fn signature_liquidation_cooldown() -> IndicatorSignature {
    IndicatorSignature::builder("LIQUIDATION_COOLDOWN", CATEGORY)
        .name("Liquidation Cooldown")
        .description("Measures time elapsed since last significant liquidation burst")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LiquidationCooldown)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("liquidation_cooldown")
        .alias("LiquidationCooldown")
        .build()
}

pub fn signature_liquidation_rate() -> IndicatorSignature {
    IndicatorSignature::builder("LIQUIDATION_RATE", CATEGORY)
        .name("Liquidation Rate")
        .description("Rolling rate of liquidation events per unit time")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LiquidationRate)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("liquidation_rate")
        .alias("LiquidationRate")
        .build()
}

pub fn signature_liquidation_volume_imbalance() -> IndicatorSignature {
    IndicatorSignature::builder("LIQUIDATION_VOLUME_IMBALANCE", CATEGORY)
        .name("Liquidation Volume Imbalance")
        .description("Imbalance between long and short liquidation volumes")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LiquidationVolumeImbalance)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("liquidation_volume_imbalance")
        .alias("LiquidationVolumeImbalance")
        .build()
}

pub fn signature_liquidation_volume_velocity() -> IndicatorSignature {
    IndicatorSignature::builder("LIQUIDATION_VOLUME_VELOCITY", CATEGORY)
        .name("Liquidation Volume Velocity")
        .description("Rate of change of total liquidation volume")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::LiquidationVolumeVelocity)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("liquidation_volume_velocity")
        .alias("LiquidationVolumeVelocity")
        .build()
}

pub fn signature_stop_hunt_detector() -> IndicatorSignature {
    IndicatorSignature::builder("STOP_HUNT_DETECTOR", CATEGORY)
        .name("Stop Hunt Detector")
        .description("Detects potential stop-hunting patterns via liquidation clustering near recent highs/lows")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::StopHuntDetector)
        .role_kind(IndicatorRoleKind::OscillatorUnbounded)
        .output_kind(IndicatorValueKind::Single)
        .input_stream(StreamKind::Liquidation)
        .alias("stop_hunt_detector")
        .alias("StopHuntDetector")
        .build()
}

// ============================================================================
// Catalog
// ============================================================================

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("LIQUIDATION_CASCADE", signature_liquidation_cascade as fn() -> IndicatorSignature),
    ("LIQUIDATION_CLUSTER_DETECTOR", signature_liquidation_cluster_detector as fn() -> IndicatorSignature),
    ("LIQUIDATION_COOLDOWN", signature_liquidation_cooldown as fn() -> IndicatorSignature),
    ("LIQUIDATION_RATE", signature_liquidation_rate as fn() -> IndicatorSignature),
    ("LIQUIDATION_VOLUME_IMBALANCE", signature_liquidation_volume_imbalance as fn() -> IndicatorSignature),
    ("LIQUIDATION_VOLUME_VELOCITY", signature_liquidation_volume_velocity as fn() -> IndicatorSignature),
    ("STOP_HUNT_DETECTOR", signature_stop_hunt_detector as fn() -> IndicatorSignature),
];

pub static LIQUIDATIONS_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    LIQUIDATIONS_CATALOG.get(id).map(|f| f())
}

pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

pub fn count() -> usize {
    BASE_CATALOG.len()
}
