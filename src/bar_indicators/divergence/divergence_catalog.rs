//! divergence_catalog.rs — legacy Div variants removed; catalog is now empty.
//!
//! Divergence detection is handled by `events::Divergence` and `events::SwingDetection`.

use crate::catalog::IndicatorSignature;
use crate::catalog::IndicatorCategory;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Divergence;

const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[];

/// Expanded catalog (empty — all Div variants removed)
pub static DIVERGENCE_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> =
    Lazy::new(HashMap::new);

/// Get indicator signature by ID
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    DIVERGENCE_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}
