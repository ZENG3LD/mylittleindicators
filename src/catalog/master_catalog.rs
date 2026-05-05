//! master_catalog.rs - Master Indicator Catalog Aggregator
//!
//! Provides unified access to all 23 indicator category catalogs through a single interface.
//! This is the central registry that connects the Universal Indicator System with optimizers.
//!
//! Architecture:
//! - Master Catalog unifies all 23 category catalogs (22 active + zigzag pending)
//! - Type-erased trait for catalog access
//! - O(1) lookups with HashMap
//! - Support for search and discovery
//!
//! Usage:
//! ```rust
//! use zengeld_chart_indicators::catalog::master_catalog::MasterIndicatorCatalog;
//! use zengeld_chart_indicators::catalog::indicator_signature::IndicatorCategory;
//!
//! let master = MasterIndicatorCatalog::new();
//!
//! // Check indicator count
//! assert!(master.total_count() > 450);
//!
//! // Get any indicator by ID
//! let sma_sig = master.get_signature("SMA").unwrap();
//! assert_eq!(sma_sig.id, "SMA");
//!
//! // Search indicators
//! let ma_indicators = master.search("moving average");
//! assert!(!ma_indicators.is_empty());
//! ```

use crate::catalog::{IndicatorSignature, IndicatorCategory};
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Import all 21 category catalogs
use crate::bar_indicators::average::average_catalog;
use crate::bar_indicators::momentum::momentum_catalog;
use crate::bar_indicators::channels::channels_catalog;
use crate::bar_indicators::volatility::volatility_catalog;
use crate::bar_indicators::volume::volume_catalog;
use crate::bar_indicators::trend::trend_catalog;
use crate::bar_indicators::levels::levels_catalog;
use crate::bar_indicators::entropy::entropy_catalog;
use crate::bar_indicators::kalman::kalman_catalog;
use crate::bar_indicators::signal_processing::signal_processing_catalog;
use crate::bar_indicators::chaos::chaos_catalog;
use crate::bar_indicators::regression::regression_catalog;
use crate::bar_indicators::adaptive::adaptive_catalog;
use crate::bar_indicators::accumulation::accumulation_catalog;
use crate::bar_indicators::book::book_catalog;
use crate::bar_indicators::candles::candles_catalog;
use crate::bar_indicators::clusters::clusters_catalog;
use crate::bar_indicators::divergence::divergence_catalog;
use crate::bar_indicators::ratio::ratio_catalog;
use crate::bar_indicators::trend_stop::trend_stop_catalog;
use crate::bar_indicators::position::position_catalog;
use crate::bar_indicators::statistics::statistics_catalog;
// Note: zigzag_catalog not exported from zigzag module yet

/// Error type for catalog operations
#[derive(Debug, Clone)]
pub enum CatalogError {
    /// Indicator not found
    NotFound(String),
    /// Category not registered
    CategoryNotFound(IndicatorCategory),
    /// Ambiguous indicator ID (exists in multiple categories)
    Ambiguous(String, Vec<IndicatorCategory>),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::NotFound(id) => write!(f, "Indicator '{}' not found in any catalog", id),
            CatalogError::CategoryNotFound(cat) => write!(f, "Category '{:?}' not registered", cat),
            CatalogError::Ambiguous(id, cats) => write!(f, "Indicator '{}' found in multiple categories: {:?}", id, cats),
        }
    }
}

impl std::error::Error for CatalogError {}

/// Trait for type-erased catalog access
pub trait CatalogProvider: Send + Sync {
    /// Get indicator signature by ID
    fn get_signature(&self, id: &str) -> Option<IndicatorSignature>;

    /// Get all indicator IDs in this catalog
    fn get_all_ids(&self) -> Vec<&'static str>;

    /// Get the category this catalog represents
    fn category(&self) -> IndicatorCategory;

    /// Get total indicator count
    fn count(&self) -> usize;
}

/// Wrapper for category catalogs that implement CatalogProvider
struct CategoryCatalogWrapper {
    category: IndicatorCategory,
    get_fn: fn(&str) -> Option<IndicatorSignature>,
    all_ids: Vec<&'static str>,
}

impl CategoryCatalogWrapper {
    fn new(
        category: IndicatorCategory,
        get_fn: fn(&str) -> Option<IndicatorSignature>,
        all_ids: Vec<&'static str>,
    ) -> Self {
        Self { category, get_fn, all_ids }
    }
}

impl CatalogProvider for CategoryCatalogWrapper {
    fn get_signature(&self, id: &str) -> Option<IndicatorSignature> {
        (self.get_fn)(id)
    }

    fn get_all_ids(&self) -> Vec<&'static str> {
        self.all_ids.clone()
    }

    fn category(&self) -> IndicatorCategory {
        self.category
    }

    fn count(&self) -> usize {
        self.all_ids.len()
    }
}

/// Master Indicator Catalog - unified access to all 480+ indicators
pub struct MasterIndicatorCatalog {
    /// Category catalogs
    catalogs: HashMap<IndicatorCategory, Box<dyn CatalogProvider>>,

    /// Fast lookup: ID -> Category (for ambiguity detection)
    id_to_categories: HashMap<String, Vec<IndicatorCategory>>,

    /// Total indicator count
    total_count: usize,
}

impl MasterIndicatorCatalog {
    /// Create a new master catalog with all 23 categories registered (22 active + zigzag pending)
    pub fn new() -> Self {
        let mut catalogs: HashMap<IndicatorCategory, Box<dyn CatalogProvider>> = HashMap::new();
        let mut id_to_categories: HashMap<String, Vec<IndicatorCategory>> = HashMap::new();

        // Register all 22 category catalogs (zigzag catalog not yet exported)
        let catalog_configs = vec![
            (IndicatorCategory::Average, average_catalog::get_signature as fn(&str) -> Option<IndicatorSignature>, average_catalog::all_indicator_ids()),
            (IndicatorCategory::Momentum, momentum_catalog::get_signature, momentum_catalog::all_indicator_ids()),
            (IndicatorCategory::Channels, channels_catalog::get_signature, channels_catalog::all_indicator_ids()),
            (IndicatorCategory::Volatility, volatility_catalog::get_signature, volatility_catalog::all_indicator_ids()),
            (IndicatorCategory::Volume, volume_catalog::get_signature, volume_catalog::all_indicator_ids()),
            (IndicatorCategory::Trend, trend_catalog::get_signature, trend_catalog::all_indicator_ids()),
            (IndicatorCategory::Levels, levels_catalog::get_signature, levels_catalog::all_indicator_ids()),
            (IndicatorCategory::Entropy, entropy_catalog::get_signature, entropy_catalog::all_indicator_ids()),
            (IndicatorCategory::Kalman, kalman_catalog::get_signature, kalman_catalog::all_indicator_ids()),
            (IndicatorCategory::SignalProcessing, signal_processing_catalog::get_signature, signal_processing_catalog::all_indicator_ids()),
            (IndicatorCategory::Chaos, chaos_catalog::get_signature, chaos_catalog::all_indicator_ids()),
            (IndicatorCategory::Regression, regression_catalog::get_signature, regression_catalog::all_indicator_ids()),
            (IndicatorCategory::Adaptive, adaptive_catalog::get_signature, adaptive_catalog::all_indicator_ids()),
            (IndicatorCategory::Accumulation, accumulation_catalog::get_signature, accumulation_catalog::all_indicator_ids()),
            (IndicatorCategory::Book, book_catalog::get_signature, book_catalog::all_indicator_ids()),
            (IndicatorCategory::Candles, candles_catalog::get_signature, candles_catalog::all_indicator_ids()),
            (IndicatorCategory::Clusters, clusters_catalog::get_signature, clusters_catalog::all_indicator_ids()),
            (IndicatorCategory::Divergence, divergence_catalog::get_signature, divergence_catalog::all_indicator_ids()),
            (IndicatorCategory::Ratio, ratio_catalog::get_signature, ratio_catalog::all_indicator_ids()),
            (IndicatorCategory::TrendStop, trend_stop_catalog::get_signature, trend_stop_catalog::all_indicator_ids()),
            (IndicatorCategory::Position, position_catalog::get_signature, position_catalog::all_indicator_ids()),
            (IndicatorCategory::Statistics, statistics_catalog::get_signature, statistics_catalog::all_indicator_ids()),
            // Note: Zigzag catalog will be added when exported from zigzag module
        ];

        let mut total_count = 0;

        for (category, get_fn, ids) in catalog_configs {
            total_count += ids.len();

            // Build ID -> Category mapping for ambiguity detection
            // Register both main IDs and all aliases
            for &id in &ids {
                // Register main ID
                id_to_categories
                    .entry(id.to_string())
                    .or_default()
                    .push(category);

                // Register all aliases from signature
                if let Some(sig) = get_fn(id) {
                    for alias in &sig.aliases {
                        id_to_categories
                            .entry(alias.clone())
                            .or_default()
                            .push(category);
                    }
                }
            }

            let wrapper = CategoryCatalogWrapper::new(category, get_fn, ids);
            catalogs.insert(category, Box::new(wrapper));
        }

        Self {
            catalogs,
            id_to_categories,
            total_count,
        }
    }

    /// Get indicator signature by ID (searches all categories)
    ///
    /// Returns error if indicator is not found or exists in multiple categories.
    /// Use `get_by_category()` if you know the category or for ambiguous indicators.
    pub fn get_signature(&self, id: &str) -> Result<IndicatorSignature, CatalogError> {
        // Check if ID exists and is unambiguous
        match self.id_to_categories.get(id) {
            None => Err(CatalogError::NotFound(id.to_string())),
            Some(categories) if categories.len() > 1 => {
                Err(CatalogError::Ambiguous(id.to_string(), categories.clone()))
            }
            Some(categories) => {
                let category = categories[0];
                self.get_by_category(category, id)
            }
        }
    }

    /// Get indicator signature by category and ID
    pub fn get_by_category(&self, category: IndicatorCategory, id: &str) -> Result<IndicatorSignature, CatalogError> {
        let catalog = self.catalogs
            .get(&category)
            .ok_or(CatalogError::CategoryNotFound(category))?;

        catalog.get_signature(id)
            .ok_or_else(|| CatalogError::NotFound(format!("{}::{}", category.as_str(), id)))
    }

    /// Get all indicators in a specific category
    pub fn get_category_indicators(&self, category: IndicatorCategory) -> Result<Vec<IndicatorSignature>, CatalogError> {
        let catalog = self.catalogs
            .get(&category)
            .ok_or(CatalogError::CategoryNotFound(category))?;

        Ok(catalog.get_all_ids()
            .into_iter()
            .filter_map(|id| catalog.get_signature(id))
            .collect())
    }

    /// Search indicators by name or description (case-insensitive)
    pub fn search(&self, query: &str) -> Vec<IndicatorSignature> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for catalog in self.catalogs.values() {
            for id in catalog.get_all_ids() {
                if let Some(sig) = catalog.get_signature(id) {
                    // Search in ID, name, and description
                    let matches = sig.id.to_lowercase().contains(&query_lower)
                        || sig.name.to_lowercase().contains(&query_lower)
                        || sig.metadata.get("description")
                            .map(|d| d.to_lowercase().contains(&query_lower))
                            .unwrap_or(false);

                    if matches {
                        results.push(sig);
                    }
                }
            }
        }

        results
    }

    /// Check if an indicator exists
    pub fn contains(&self, id: &str) -> bool {
        self.id_to_categories.contains_key(id)
    }

    /// Get total number of indicators across all categories
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Get count of indicators in a specific category
    pub fn category_count(&self, category: IndicatorCategory) -> usize {
        self.catalogs
            .get(&category)
            .map(|c| c.count())
            .unwrap_or(0)
    }

    /// Get statistics about the catalog
    pub fn stats(&self) -> CatalogStats {
        CatalogStats {
            total_indicators: self.total_count,
            total_categories: self.catalogs.len(),
            category_counts: self.catalogs
                .iter()
                .map(|(cat, catalog)| (*cat, catalog.count()))
                .collect(),
        }
    }
}

impl Default for MasterIndicatorCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Catalog statistics
#[derive(Debug, Clone)]
pub struct CatalogStats {
    pub total_indicators: usize,
    pub total_categories: usize,
    pub category_counts: HashMap<IndicatorCategory, usize>,
}

impl CatalogStats {
    /// Generate a human-readable report
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("═══════════════════════════════════════════════════════════\n");
        report.push_str("     MASTER INDICATOR CATALOG STATISTICS\n");
        report.push_str("═══════════════════════════════════════════════════════════\n\n");

        report.push_str(&format!("Total Categories:  {}\n", self.total_categories));
        report.push_str(&format!("Total Indicators:  {}\n\n", self.total_indicators));

        report.push_str("Category Breakdown:\n");
        report.push_str("───────────────────────────────────────────────────────────\n");

        let mut sorted: Vec<_> = self.category_counts.iter().collect();
        sorted.sort_by_key(|(cat, _)| cat.as_str());

        for (category, count) in sorted {
            report.push_str(&format!("  {:20} {:3} indicators\n", category.as_str(), count));
        }

        report.push_str("═══════════════════════════════════════════════════════════\n");

        report
    }
}

/// Global master catalog instance (lazy initialized)
pub static MASTER_CATALOG: Lazy<MasterIndicatorCatalog> = Lazy::new(MasterIndicatorCatalog::new);

/// Convenience function to get the global master catalog
pub fn catalog() -> &'static MasterIndicatorCatalog {
    &MASTER_CATALOG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_catalog_creation() {
        let master = MasterIndicatorCatalog::new();
        assert!(master.total_count() > 450, "Should have 450+ indicators");
    }

    #[test]
    fn test_get_signature_unambiguous() {
        let master = MasterIndicatorCatalog::new();

        // SMA should be unambiguous (only in Average category)
        let sma = master.get_signature("SMA");
        assert!(sma.is_ok(), "SMA should be found");
        assert_eq!(sma.unwrap().id, "SMA");
    }

    #[test]
    fn test_get_by_category() {
        let master = MasterIndicatorCatalog::new();

        // Get RSI from Momentum category
        let rsi = master.get_by_category(IndicatorCategory::Momentum, "RSI");
        assert!(rsi.is_ok(), "RSI should be found in Momentum");
        assert_eq!(rsi.unwrap().category, IndicatorCategory::Momentum);
    }

    #[test]
    fn test_get_category_indicators() {
        let master = MasterIndicatorCatalog::new();

        // Get all average indicators
        let avg_indicators = master.get_category_indicators(IndicatorCategory::Average);
        assert!(avg_indicators.is_ok());
        let indicators = avg_indicators.unwrap();
        assert!(indicators.len() >= 20, "Average category should have 20+ indicators");

        // All should be in Average category
        for ind in indicators {
            assert_eq!(ind.category, IndicatorCategory::Average);
        }
    }

    #[test]
    fn test_search() {
        let master = MasterIndicatorCatalog::new();

        // Search for "moving average"
        let results = master.search("moving average");
        assert!(!results.is_empty(), "Should find moving average indicators");

        // Search for "RSI"
        let rsi_results = master.search("rsi");
        assert!(!rsi_results.is_empty(), "Should find RSI-related indicators");
    }

    #[test]
    fn test_contains() {
        let master = MasterIndicatorCatalog::new();

        assert!(master.contains("SMA"), "Should contain SMA");
        assert!(master.contains("RSI"), "Should contain RSI");
        assert!(!master.contains("NONEXISTENT"), "Should not contain fake indicator");
    }

    #[test]
    fn test_category_count() {
        let master = MasterIndicatorCatalog::new();

        let avg_count = master.category_count(IndicatorCategory::Average);
        assert!(avg_count >= 20, "Average category should have 20+ indicators");

        let momentum_count = master.category_count(IndicatorCategory::Momentum);
        assert!(momentum_count >= 50, "Momentum category should have 50+ indicators");
    }

    #[test]
    fn test_stats() {
        let master = MasterIndicatorCatalog::new();
        let stats = master.stats();

        assert_eq!(stats.total_categories, 22);
        assert!(stats.total_indicators > 450);
        assert_eq!(stats.category_counts.len(), 22);
    }

    #[test]
    fn test_global_catalog() {
        // Access global catalog
        let master = catalog();

        assert!(master.total_count() > 450);
        assert!(master.contains("SMA"));
        assert!(master.contains("RSI"));
    }

    #[test]
    fn test_not_found_error() {
        let master = MasterIndicatorCatalog::new();

        let result = master.get_signature("FAKE_INDICATOR");
        assert!(result.is_err());

        match result {
            Err(CatalogError::NotFound(id)) => {
                assert_eq!(id, "FAKE_INDICATOR");
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
