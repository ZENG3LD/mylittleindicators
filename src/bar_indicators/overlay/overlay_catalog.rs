//! overlay_catalog.rs: Catalog entries for opaque host-driven overlays
//!
//! These six signatures (Volume Profile, TPO Profile, OI Delta, Funding
//! Rate, DOM Heatmap, Liquidation Heatmap) exist **only** for
//! discoverability in the unified indicator catalog (the "+"-add indicator
//! picker). They are NOT computed through `IndicatorInstance` /
//! `BarIndicatorId` — the host (mlc) drives them via bespoke
//! `ChartOutEvent::Toggle{Vp,TpoProfile,OpenInterest,FundingRate,DomHeatmap,LiquidationHeatmap}Overlay`
//! handlers and renders them with dedicated overlay/subpane renderers, not
//! via `IndicatorOutput`. Accordingly every signature here has
//! `machine_id: None` (no factory-backed compute path) and zero parameter
//! constraints — the catalog only needs to carry id/name/category for the
//! picker UI.
//!
//! `volume_profile` collides with a pre-existing REAL compute indicator
//! already registered under the `Volume` category (`VPROFILE` aliases
//! `volume_profile`). Registering a second signature under that exact
//! string would make `MasterIndicatorCatalog::get_signature` return
//! `CatalogError::Ambiguous`, breaking every existing consumer (mlq codegen,
//! live validator) that resolves `"volume_profile"` today.
//!
//! Fix: every id in this module carries an `overlay_` prefix, so it can
//! never collide with a compute-indicator id or alias:
//! `overlay_volume_profile`, `overlay_tpo_profile`, `overlay_oi_delta`,
//! `overlay_funding_rate`, `overlay_dom_heatmap`,
//! `overlay_liquidation_heatmap`. The pre-existing `volume_profile` compute
//! indicator is untouched.
//!
//! `overlay_dom_heatmap` / `overlay_liquidation_heatmap` (chart-type →
//! overlay migration, mirrors Bookmap/ATAS/Coinglass — DOM/liquidation heat
//! layers are overlays on any chart type, not their own chart types) were
//! previously `chart_type` registry entries (`mlc-core`
//! `chart_type::registry`, ids `"dom_heatmap"`/`"liquidation_heatmap"`).
//! The host still owns the same ring-buffer state (`DomHeatmapBuffer`/
//! `LiquidationHeatmapBuffer` on the bubble) and the same draw functions
//! (`draw_dom_heatmap`/`draw_liquidation_heatmap`) — only the trigger moved
//! from a chart-type switch to an overlay toggle.
//!
//! `overlay_tpo_profile` (canon-fix batch, item D) is a NEW overlay (not a
//! chart-type migration) — a block-histogram time-at-price profile computed
//! from the visible bar range, distinct from the letter-based TPO Market
//! Profile chart type (`chart_type` id `"tpo"`, which stays a full chart
//! type driven by session-scoped letter buckets, not this overlay).
//!
//! There is no `overlay_cvd` placeholder — CVD is a real compute indicator
//! (`BarIndicatorId::Cvd`, catalog id `"CVD"`) fed real aggressor-side
//! buy/sell volume via `IndicatorInstance::update_bar_with_delta`, not a
//! host-driven opaque overlay.
//!
//! ## Data streams are DECLARED, not hardcoded by the host (2026-08-02)
//!
//! An overlay that is a real indicator names the stream it consumes
//! through the ordinary `input_stream` / `aux_streams` contract every
//! other indicator uses (`signature.accepts(kind)`), so the bubble hosting
//! it raises that feed from the catalog instead of anything matching on
//! its id. `overlay_liquidation_heatmap` declares `Liquidation`.
//!
//! Overlay INDICATORS are one per data source, deliberately: the reference
//! UI (TapeSurf) folds its heatmap sources into a single widget with a
//! source dropdown; we keep them separate so each carries its own params,
//! its own stream declaration and its own legend row inside the indicator
//! system.

use crate::catalog::{IndicatorSignature, IndicatorCategory};
use crate::catalog::constraints::ParamConstraint;
use crate::data_loader::stream_kind::StreamKind;

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

/// TPO Profile — opaque overlay: catalog discoverability only; the host
/// consumes this via a bespoke Toggle event, not via IndicatorOutput.
///
/// Block-histogram time-at-price profile computed from the visible bar
/// range (canon-fix batch, item D) — no letters, distinct from the
/// letter-based TPO Market Profile chart type (`chart_type` id `"tpo"`).
pub fn signature_tpo_profile() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_tpo_profile", CATEGORY)
        .name("TPO Profile")
        .description("TPO time-at-price block profile overlay — host-rendered, not computed via IndicatorOutput")
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

/// DOM Heatmap — opaque overlay: catalog discoverability only; the host
/// consumes this via a bespoke Toggle event, not via IndicatorOutput.
///
/// Bookmap-style time-by-price depth heatmap. Chart-type → overlay
/// migration (was `chart_type` id `"dom_heatmap"`) — draws under the main
/// series on any chart type, driven by a live OrderBook + Trade
/// subscription instead of a chart-type switch.
pub fn signature_dom_heatmap() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_dom_heatmap", CATEGORY)
        .name("DOM Heatmap")
        .description("Depth-of-market heatmap overlay — host-rendered, not computed via IndicatorOutput")
        .metadata("kind", "opaque_overlay")
        .metadata("icon", "Histogram")
        .build()
}

/// Liquidation Heatmap — opaque overlay: catalog discoverability only; the
/// host consumes this via a bespoke Toggle event, not via IndicatorOutput.
///
/// Hyblock/CoinGlass-style time-by-price liquidation heatmap. Chart-type →
/// overlay migration (was `chart_type` id `"liquidation_heatmap"`) — draws
/// under the main series on any chart type, driven by a live Liquidation
/// subscription instead of a chart-type switch.
///
/// Unlike its 5 opaque-overlay siblings, this one carries ONE parameter
/// constraint (wave 1, 2026-08-01): `sensitivity` (F64, default 1.0,
/// 0.1..=10.0) — a gain applied to the heat normalizer at render time
/// (`draw_liquidation_heatmap` divides `max_volume_seen` by it), mirroring
/// TapeSurf's "gain" slider. It rides the same `IndicatorBridge::
/// extract_params` path every real compute indicator's params use — this
/// signature is the ONLY opaque overlay promoted to a real
/// `IndicatorInstance` on the host side (mlc `overlay_toggle_for_catalog_id`
/// no longer intercepts `overlay_liquidation_heatmap` in the picker
/// add-flow), so its param actually reaches the Indicator Settings modal.
pub fn signature_liquidation_heatmap() -> IndicatorSignature {
    IndicatorSignature::builder("overlay_liquidation_heatmap", CATEGORY)
        .name("Liquidation Heatmap")
        .description("Liquidation heatmap overlay — host-rendered, not computed via IndicatorOutput")
        .metadata("kind", "opaque_overlay")
        .metadata("icon", "Activity")
        .input_stream(StreamKind::Liquidation)
        .add_constraint(ParamConstraint::threshold("sensitivity", 0.1, 10.0, 1.0))
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of opaque overlay indicators.
/// Base catalog with main IDs only (used for initialization).
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("overlay_volume_profile", signature_volume_profile as fn() -> IndicatorSignature),
    ("overlay_tpo_profile", signature_tpo_profile as fn() -> IndicatorSignature),
    ("overlay_oi_delta", signature_oi_delta as fn() -> IndicatorSignature),
    ("overlay_funding_rate", signature_funding_rate as fn() -> IndicatorSignature),
    ("overlay_dom_heatmap", signature_dom_heatmap as fn() -> IndicatorSignature),
    ("overlay_liquidation_heatmap", signature_liquidation_heatmap as fn() -> IndicatorSignature),
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
    fn test_get_tpo_profile_signature() {
        let sig = get_signature("overlay_tpo_profile").unwrap();
        assert_eq!(sig.id, "overlay_tpo_profile");
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
    fn test_get_dom_heatmap_signature() {
        let sig = get_signature("overlay_dom_heatmap").unwrap();
        assert_eq!(sig.id, "overlay_dom_heatmap");
        assert_eq!(sig.category, CATEGORY);
        assert!(sig.machine_id.is_none(), "opaque overlay must not have a machine_id");
    }

    #[test]
    fn test_get_liquidation_heatmap_signature() {
        let sig = get_signature("overlay_liquidation_heatmap").unwrap();
        assert_eq!(sig.id, "overlay_liquidation_heatmap");
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
        assert_eq!(count(), 6);
    }
}
