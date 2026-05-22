//! MasterEventCatalog — unified access to all 18 event/detector primitive signatures.
//!
//! Mirrors `MasterIndicatorCatalog` for the events subsystem.

use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::catalog::data::event_signature::EventSignature;
use crate::events::events_catalog;

// ── Static catalog ─────────────────────────────────────────────────────────────

static CATALOG: Lazy<HashMap<String, EventSignature>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for sig in events_catalog::all_signatures() {
        // Primary id
        map.insert(sig.id.clone(), sig.clone());
        // All aliases
        for alias in &sig.aliases {
            map.insert(alias.clone(), sig.clone());
        }
    }
    map
});

// ── Public API ────────────────────────────────────────────────────────────────

/// Unified catalog for all event/detector primitive signatures.
pub struct MasterEventCatalog;

impl MasterEventCatalog {
    /// Construct (zero-cost — backed by Lazy static).
    pub fn new() -> Self {
        Self
    }

    /// Iterate all 18 distinct event signatures (no alias duplicates).
    pub fn iter_signatures(&self) -> impl Iterator<Item = &EventSignature> {
        // Only emit entries whose key matches the canonical id to avoid duplicates
        // from alias keys.
        CATALOG.iter().filter_map(|(k, v)| {
            if k == &v.id {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Look up by id or alias (case-insensitive).
    pub fn get_signature(&self, id: &str) -> Option<&EventSignature> {
        CATALOG.get(id.to_lowercase().as_str()).or_else(|| {
            CATALOG.get(id)
        })
    }

    /// Total number of distinct event signatures.
    pub fn total_count(&self) -> usize {
        self.iter_signatures().count()
    }

    /// Search signatures by substring (case-insensitive, checks id, name, description).
    pub fn search(&self, query: &str) -> Vec<&EventSignature> {
        let q = query.to_lowercase();
        self.iter_signatures()
            .filter(|s| {
                s.id.to_lowercase().contains(&q)
                    || s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
            })
            .collect()
    }
}

impl Default for MasterEventCatalog {
    fn default() -> Self {
        Self::new()
    }
}
