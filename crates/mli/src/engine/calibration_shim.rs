//! Measured-cost shim. Under the `calibrated` feature, [`cost_points`](super::contract_catalog::cost_points)
//! and its siblings consult compile-time-embedded measurement tables — produced by
//! `mli-barometer`'s `calibrate` bin (per-DEFAULT-config, [`measured_cost`]) and by
//! `mlq-warmup`'s opt-in warmup-cost harvest baked via `mli-barometer`'s
//! `bake_warmup_table` bin (per-SWEPT-config, [`measured_cost_for_config`]) — instead of
//! the structural `WEIGHTS` rubric. Both tables are generated Rust slices under
//! `src/engine/`: NO serde, NO runtime I/O. Feature OFF (default) → every lookup here
//! returns `None` and every caller keeps the pure structural prior, with no dependency
//! on the generated files at all.
//!
//! `MEASURED_NS` lookup key = `format!("{id:?}")` (the `IndicatorId` Debug name, e.g.
//! `"Sma"`), matching the `IndicatorBarometer::id_str` the table was keyed by — ONE row
//! per indicator, its DEFAULT config only (see `cost_calibration.rs`'s header). Both
//! generated slices are sorted by key (the emitting bins build from a `BTreeMap`/sorted
//! `Vec`) → binary search.
//!
//! `MEASURED_NS_PER_CONFIG` (`cost_calibration_per_config.rs`) is the per-config
//! complement: keyed by `(id, config_hash)` — the exact identity a warmup `ColumnKey`
//! dedups by — harvested from REAL swept configs during an actual warmup build (opt-in,
//! `MLQ_HARVEST_WARMUP_COST=1`; see `mlq_warmup::cache::cpu::warmup_harvest`), not a
//! synthetic default-config bench. This is the table [`measured_cost_for_config`] reads;
//! it supersedes the stale "per-DEFAULT-config + Linux-instruction-baseline" premise for
//! the warmup cost plane (owner override, `barometer-total-system-design-2026-07-01.md`
//! §7 — period-free structural priors are the fallback, not a Linux baseline).

use crate::engine::indicator_id::IndicatorId;

// The generated slices:
//   `pub(crate) static MEASURED_NS: &[(&str, f64)]`             — sorted by id string.
//   `pub(crate) static MEASURED_NS_PER_CONFIG: &[(&str, u64, f64)]` — sorted by (id, config_hash).
//   `pub(crate) static MEASURED_MEM: &[(&str, f64)]`            — sorted by id string (MB, §Ф3a).
// Sibling generated files in `src/engine/` (committed; `data/` is gitignored). Only
// pulled in under the feature, so the files are NOT required for a default build.
#[cfg(feature = "calibrated")]
include!("cost_calibration.rs");
#[cfg(feature = "calibrated")]
include!("cost_calibration_per_config.rs");
#[cfg(feature = "calibrated")]
include!("cost_calibration_mem.rs");

/// Measured flat cost (median ns per `feed` call — already includes nested slot/port cost) for
/// `id`, or `None` when the `calibrated` feature is off or `id` is absent from the table.
#[cfg(feature = "calibrated")]
pub(crate) fn measured_cost(id: IndicatorId) -> Option<f64> {
    let key = format!("{id:?}");
    MEASURED_NS
        .binary_search_by(|(k, _)| (*k).cmp(key.as_str()))
        .ok()
        .map(|i| MEASURED_NS[i].1)
}

/// Feature-off path: always `None` → callers fall back to the structural prior.
#[cfg(not(feature = "calibrated"))]
pub(crate) fn measured_cost(_id: IndicatorId) -> Option<f64> {
    None
}

/// Measured per-SWEPT-config feed cost (median ns per `feed` call at THIS exact
/// `config_hash`, harvested from real warmup builds — not the default-config-only
/// [`measured_cost`]). `None` when the `calibrated` feature is off, or when this exact
/// `(id, config_hash)` was never harvested (an unseen config does NOT interpolate or
/// fall back to a neighboring config's number here — the caller decides the fallback;
/// see [`super::contract_catalog::cost_points_for_config`] for the honest-marker
/// consumption point).
#[cfg(feature = "calibrated")]
pub(crate) fn measured_cost_for_config(id: IndicatorId, config_hash: u64) -> Option<f64> {
    let key = format!("{id:?}");
    MEASURED_NS_PER_CONFIG
        .binary_search_by(|(k, h, _)| (*k).cmp(key.as_str()).then(h.cmp(&config_hash)))
        .ok()
        .map(|i| MEASURED_NS_PER_CONFIG[i].2)
}

/// Feature-off path: always `None`.
#[cfg(not(feature = "calibrated"))]
pub(crate) fn measured_cost_for_config(_id: IndicatorId, _config_hash: u64) -> Option<f64> {
    None
}

/// Measured warmup MEMORY weight for `id`, in MB (heap-live + inline core size at the
/// indicator's DEFAULT config's warmup depth — same per-DEFAULT-config scope as
/// [`measured_cost`], revived from `mli-barometer`'s dead-end `MemWeight`, per 07-01
/// costometer plan Ф3a). `None` when the `calibrated` feature is off or `id` is absent.
#[cfg(feature = "calibrated")]
pub(crate) fn measured_mem(id: IndicatorId) -> Option<f64> {
    let key = format!("{id:?}");
    MEASURED_MEM
        .binary_search_by(|(k, _)| (*k).cmp(key.as_str()))
        .ok()
        .map(|i| MEASURED_MEM[i].1)
}

/// Feature-off path: always `None`.
#[cfg(not(feature = "calibrated"))]
pub(crate) fn measured_mem(_id: IndicatorId) -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "calibrated"))]
    #[test]
    fn feature_off_yields_none() {
        // Default build: no measured table → callers keep the structural prior.
        assert!(measured_cost(IndicatorId::Sma).is_none());
    }

    #[cfg(feature = "calibrated")]
    #[test]
    fn feature_on_table_drives_cost_points() {
        // The embedded slice is non-empty, sorted (binary-search invariant), and covers the
        // canonical MAs.
        assert!(!MEASURED_NS.is_empty());
        assert!(
            MEASURED_NS.windows(2).all(|w| w[0].0 <= w[1].0),
            "cost_calibration.rs must be sorted by id for binary search"
        );
        assert!(measured_cost(IndicatorId::Sma).is_some());
        assert!(measured_cost(IndicatorId::Ema).is_some());
        // Measured REPLACES the structural prior in the budget currency.
        assert_eq!(
            crate::engine::contract_catalog::cost_points(IndicatorId::Sma),
            measured_cost(IndicatorId::Sma),
        );
    }

    #[cfg(feature = "calibrated")]
    #[test]
    fn per_config_table_sorted_deduped_and_round_trips() {
        // Table-agnostic invariants (hold for ANY baked table, incl. empty):
        // strict (id, hash) ordering = the binary-search precondition AND no-dup guarantee.
        assert!(
            MEASURED_NS_PER_CONFIG
                .windows(2)
                .all(|w| (w[0].0, w[0].1) < (w[1].0, w[1].1)),
            "cost_calibration_per_config.rs must be STRICTLY sorted by (id, config_hash)"
        );
        // Positive path: every baked entry must be reachable through the public lookup
        // with the exact ns the table holds (round-trip through the comparator).
        let by_name: std::collections::HashMap<String, IndicatorId> = IndicatorId::all()
            .map(|id| (format!("{id:?}"), id))
            .collect();
        for (key, hash, ns) in MEASURED_NS_PER_CONFIG {
            let id = *by_name
                .get(*key)
                .unwrap_or_else(|| panic!("baked key {key:?} is not a known IndicatorId"));
            assert_eq!(
                measured_cost_for_config(id, *hash),
                Some(*ns),
                "baked entry ({key}, {hash}) must round-trip through measured_cost_for_config"
            );
        }
    }
}
