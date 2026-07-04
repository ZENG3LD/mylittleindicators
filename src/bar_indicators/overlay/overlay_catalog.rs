//! overlay_catalog.rs: Catalog entries for opaque host-driven overlays
//!
//! These four signatures (Volume Profile, OI Delta, Funding Rate, CVD) exist
//! **only** for discoverability in the unified indicator catalog (the "+"-add
//! indicator picker). They are NOT computed through `IndicatorInstance` /
//! `BarIndicatorId` — the host (mlc) drives them via bespoke
//! `ChartOutEvent::Toggle{Vp,OpenInterest,FundingRate,Cvd}Overlay` handlers and
//! renders them with dedicated subpane/overlay renderers, not via
//! `IndicatorOutput`. Accordingly every signature here has `machine_id: None`
//! (no factory-backed compute path) and zero parameter constraints — the
//! catalog only needs to carry id/name/category for the picker UI.
//!
//! Two of the four requested names — `volume_profile` and `cvd` — collide
//! with pre-existing REAL compute indicators already registered under the
//! `Volume` category (`VPROFILE` aliases `volume_profile`; `CVD`'s alias list
//! includes `"cvd"`). Registering a second signature under either exact
//! string would make `MasterIndicatorCatalog::get_signature` return
//! `CatalogError::Ambiguous`, breaking every existing consumer (mlq codegen,
//! live validator) that resolves `"volume_profile"` / `"cvd"` today.
//!
//! Fix: every id in this module carries an `overlay_` prefix, so none of the
//! four can ever collide with a compute-indicator id or alias:
//! `overlay_volume_profile`, `overlay_oi_delta`, `overlay_funding_rate`,
//! `overlay_cvd`. The pre-existing `volume_profile` / `cvd` compute
//! indicators are untouched.

use crate::catalog::{IndicatorSignature, IndicatorCategory};

/// Category for all indicators in this module.
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Overlay;

// ============================================================================
// Individual indicator signatures — opaque overlay: catalog discoverability
// only; the host consumes this via a bespoke Toggle event, not via
// IndicatorOutput. `machine_id` is intentionally `None`.
// ============================================================================

/// Volume Profile — opaque overlay: catalog discoverability only; the host
/// consumes this via a bespoke Toggle event, not via IndicatorOutput.
pub fn signature_volume_profile() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_volume_profile", CATEGORY)
        .name("Volume Profile")
        .description("Volume profile overlay — host-rendered, not computed via IndicatorOutput")
        .metadata("kind", "opaque_overlay")
        .metadata("icon", "Histogram")
        .build()
}

/// OI Delta — opaque overlay: catalog discoverability only; the host consumes
/// this via a bespoke Toggle event, not via IndicatorOutput.
pub fn signature_oi_delta() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_oi_delta", CATEGORY)
        .name("OI Delta")
        .description("Open interest delta overlay — host-rendered, not computed via IndicatorOutput")
        .metadata("kind", "opaque_overlay")
        .metadata("icon", "Activity")
        .build()
}

/// Funding Rate — opaque overlay: catalog discoverability only; the host
/// consumes this via a bespoke Toggle event, not via IndicatorOutput.
pub fn signature_funding_rate() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_funding_rate", CATEGORY)
        .name("Funding Rate")
        .description("Funding rate overlay — host-rendered, not computed via IndicatorOutput")
        .metadata("kind", "opaque_overlay")
        .metadata("icon", "LineChart")
        .build()
}

/// CVD — opaque overlay: catalog discoverability only; the host consumes
/// this via a bespoke Toggle event, not via IndicatorOutput.
pub fn signature_cvd() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_cvd", CATEGORY)
        .name("CVD")
        .description("Cumulative volume delta overlay — host-rendered, not computed via IndicatorOutput")
        .metadata("kind", "opaque_overlay")
        .metadata("icon", "LineChart")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of opaque overlay indicators.
/// Base catalog with main IDs only (used for initialization).
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("overlay_volume_profile", signature_volume_profile as fn() -> IndicatorSignature),
    ("overlay_oi_delta", signature_oi_delta as fn() -> IndicatorSignature),
    ("overlay_funding_rate", signature_funding_rate as fn() -> IndicatorSignature),
    ("overlay_cvd", signature_cvd as fn() -> IndicatorSignature),
];

// ============================================================================
// Public API
// ============================================================================

/// Get indicator signature by ID.
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    BASE_CATALOG.iter().find(|(base_id, _)| *base_id == id).map(|(_, f)| f())
}

/// Get all indicator IDs in this category.
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators.
pub fn count() -> usize {
    BASE_CATALOG.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_volume_profile_signature() {
        let sig = get_signature("overlay_volume_profile").unwrap();
        assert_eq!(sig.id, "overlay_volume_profile");
        assert_eq!(sig.category, CATEGORY);
        assert!(sig.machine_id.is_none(), "opaque overlay must not have a machine_id");
    }

    #[test]
    fn test_get_oi_delta_signature() {
        let sig = get_signature("overlay_oi_delta").unwrap();
        assert_eq!(sig.id, "overlay_oi_delta");
        assert_eq!(sig.category, CATEGORY);
        assert!(sig.machine_id.is_none(), "opaque overlay must not have a machine_id");
    }

    #[test]
    fn test_get_funding_rate_signature() {
        let sig = get_signature("overlay_funding_rate").unwrap();
        assert_eq!(sig.id, "overlay_funding_rate");
        assert_eq!(sig.category, CATEGORY);
        assert!(sig.machine_id.is_none(), "opaque overlay must not have a machine_id");
    }

    #[test]
    fn test_get_cvd_signature() {
        let sig = get_signature("overlay_cvd").unwrap();
        assert_eq!(sig.id, "overlay_cvd");
        assert_eq!(sig.category, CATEGORY);
        assert!(sig.machine_id.is_none(), "opaque overlay must not have a machine_id");
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
        assert_eq!(count(), 4);
    }
}
