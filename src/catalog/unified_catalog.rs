//! Unified Indicator Catalog
//!
//! Combines IndicatorSignature (computation metadata) with RenderingMetadata
//! (visualization metadata) into a single unified view.

use super::indicator_signature::{IndicatorSignature, IndicatorCategory};
use super::rendering::RenderingMetadata;
use super::master_catalog::{MasterIndicatorCatalog, CatalogError, MASTER_CATALOG};
use super::rendering_catalog::{RENDERING_CATALOG, get_rendering};

/// Unified indicator information combining computation and rendering metadata
#[derive(Debug, Clone)]
pub struct UnifiedIndicatorInfo {
    /// Computation metadata (parameters, constraints, etc.)
    pub signature: IndicatorSignature,
    /// Rendering metadata (overlay, outputs, bounds, etc.)
    pub rendering: Option<RenderingMetadata>,
}

impl UnifiedIndicatorInfo {
    /// Create unified info from signature and optional rendering
    pub fn new(signature: IndicatorSignature, rendering: Option<RenderingMetadata>) -> Self {
        Self { signature, rendering }
    }

    /// Get indicator ID
    pub fn id(&self) -> &str {
        &self.signature.id
    }

    /// Get indicator name
    pub fn name(&self) -> &str {
        &self.signature.name
    }

    /// Get indicator category
    pub fn category(&self) -> IndicatorCategory {
        self.signature.category
    }

    /// Check if this indicator should render as overlay
    pub fn is_overlay(&self) -> bool {
        self.rendering.as_ref().map(|r| r.overlay).unwrap_or(false)
    }

    /// Check if rendering metadata is available
    pub fn has_rendering(&self) -> bool {
        self.rendering.is_some()
    }

    /// Get Y-axis bounds if defined
    pub fn bounds(&self) -> Option<(f64, f64)> {
        self.rendering.as_ref().and_then(|r| r.bounds)
    }

    /// Get number of outputs
    pub fn output_count(&self) -> usize {
        self.rendering
            .as_ref()
            .map(|r| r.outputs.len())
            .unwrap_or(1)
    }
}

/// Unified catalog providing access to both signature and rendering metadata
pub struct UnifiedIndicatorCatalog {
    /// Reference to master catalog for signatures
    master: &'static MasterIndicatorCatalog,
}

impl UnifiedIndicatorCatalog {
    /// Create a new unified catalog
    pub fn new() -> Self {
        Self {
            master: &MASTER_CATALOG,
        }
    }

    /// Get unified indicator info by ID
    pub fn get(&self, id: &str) -> Result<UnifiedIndicatorInfo, CatalogError> {
        let signature = self.master.get_signature(id)?;
        let rendering = signature.machine_id.and_then(|mid| get_rendering(mid).cloned());
        Ok(UnifiedIndicatorInfo::new(signature, rendering))
    }

    /// Get unified indicator info by category and ID
    pub fn get_by_category(
        &self,
        category: IndicatorCategory,
        id: &str,
    ) -> Result<UnifiedIndicatorInfo, CatalogError> {
        let signature = self.master.get_by_category(category, id)?;
        let rendering = signature.machine_id.and_then(|mid| get_rendering(mid).cloned());
        Ok(UnifiedIndicatorInfo::new(signature, rendering))
    }

    /// Get all indicators in a category
    pub fn get_category_indicators(
        &self,
        category: IndicatorCategory,
    ) -> Result<Vec<UnifiedIndicatorInfo>, CatalogError> {
        let signatures = self.master.get_category_indicators(category)?;
        Ok(signatures
            .into_iter()
            .map(|sig| {
                let rendering = sig.machine_id.and_then(|mid| get_rendering(mid).cloned());
                UnifiedIndicatorInfo::new(sig, rendering)
            })
            .collect())
    }

    /// Get all overlay indicators
    pub fn get_overlay_indicators(&self) -> Vec<UnifiedIndicatorInfo> {
        self.get_all()
            .into_iter()
            .filter(|info| info.is_overlay())
            .collect()
    }

    /// Get all sub-pane indicators (non-overlay)
    pub fn get_subpane_indicators(&self) -> Vec<UnifiedIndicatorInfo> {
        self.get_all()
            .into_iter()
            .filter(|info| !info.is_overlay())
            .collect()
    }

    /// Get all indicators with rendering metadata
    pub fn get_all_with_rendering(&self) -> Vec<UnifiedIndicatorInfo> {
        self.get_all()
            .into_iter()
            .filter(|info| info.has_rendering())
            .collect()
    }

    /// Get all indicators
    pub fn get_all(&self) -> Vec<UnifiedIndicatorInfo> {
        let mut result = Vec::new();

        for category in [
            IndicatorCategory::Average,
            IndicatorCategory::Momentum,
            IndicatorCategory::Channels,
            IndicatorCategory::Volatility,
            IndicatorCategory::Volume,
            IndicatorCategory::Trend,
            IndicatorCategory::Levels,
            IndicatorCategory::Entropy,
            IndicatorCategory::Kalman,
            IndicatorCategory::SignalProcessing,
            IndicatorCategory::Chaos,
            IndicatorCategory::Regression,
            IndicatorCategory::Adaptive,
            IndicatorCategory::Accumulation,
            IndicatorCategory::Book,
            IndicatorCategory::Candles,
            IndicatorCategory::Clusters,
            IndicatorCategory::Divergence,
            IndicatorCategory::Ratio,
            IndicatorCategory::TrendStop,
            IndicatorCategory::Position,
            IndicatorCategory::Statistics,
        ] {
            if let Ok(indicators) = self.get_category_indicators(category) {
                result.extend(indicators);
            }
        }

        result
    }

    /// Search for indicators by name/description
    pub fn search(&self, query: &str) -> Vec<UnifiedIndicatorInfo> {
        self.master
            .search(query)
            .into_iter()
            .map(|sig| {
                let rendering = sig.machine_id.and_then(|mid| get_rendering(mid).cloned());
                UnifiedIndicatorInfo::new(sig, rendering)
            })
            .collect()
    }

    /// Check if indicator exists
    pub fn contains(&self, id: &str) -> bool {
        self.master.contains(id)
    }

    /// Get total count of indicators
    pub fn total_count(&self) -> usize {
        self.master.total_count()
    }

    /// Get count of indicators with rendering metadata
    pub fn rendering_count(&self) -> usize {
        RENDERING_CATALOG.len()
    }

    /// Get statistics about the unified catalog
    pub fn stats(&self) -> UnifiedCatalogStats {
        let all = self.get_all();
        let with_rendering = all.iter().filter(|i| i.has_rendering()).count();
        let overlay_count = all.iter().filter(|i| i.is_overlay()).count();

        UnifiedCatalogStats {
            total_indicators: all.len(),
            with_rendering,
            without_rendering: all.len() - with_rendering,
            overlay_count,
            subpane_count: all.len() - overlay_count,
        }
    }
}

impl Default for UnifiedIndicatorCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the unified catalog
#[derive(Debug, Clone)]
pub struct UnifiedCatalogStats {
    pub total_indicators: usize,
    pub with_rendering: usize,
    pub without_rendering: usize,
    pub overlay_count: usize,
    pub subpane_count: usize,
}

impl UnifiedCatalogStats {
    /// Generate a human-readable report
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("═══════════════════════════════════════════════════════════\n");
        report.push_str("     UNIFIED INDICATOR CATALOG STATISTICS\n");
        report.push_str("═══════════════════════════════════════════════════════════\n\n");

        report.push_str(&format!("Total Indicators:      {}\n", self.total_indicators));
        report.push_str(&format!("With Rendering:        {}\n", self.with_rendering));
        report.push_str(&format!("Without Rendering:     {}\n", self.without_rendering));
        report.push_str(&format!("Overlay Indicators:    {}\n", self.overlay_count));
        report.push_str(&format!("Sub-pane Indicators:   {}\n", self.subpane_count));

        let coverage = if self.total_indicators > 0 {
            (self.with_rendering as f64 / self.total_indicators as f64) * 100.0
        } else {
            0.0
        };
        report.push_str(&format!("\nRendering Coverage:    {:.1}%\n", coverage));

        report.push_str("═══════════════════════════════════════════════════════════\n");

        report
    }
}

/// Global unified catalog instance
use once_cell::sync::Lazy;
pub static UNIFIED_CATALOG: Lazy<UnifiedIndicatorCatalog> = Lazy::new(UnifiedIndicatorCatalog::new);

/// Convenience function to get the global unified catalog
pub fn catalog() -> &'static UnifiedIndicatorCatalog {
    &UNIFIED_CATALOG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_catalog_creation() {
        let catalog = UnifiedIndicatorCatalog::new();
        assert!(catalog.total_count() > 400, "Should have 400+ indicators");
    }

    #[test]
    fn test_get_unified_info() {
        let catalog = UnifiedIndicatorCatalog::new();

        let rsi = catalog.get("RSI").unwrap();
        assert_eq!(rsi.id(), "RSI");
        assert!(rsi.has_rendering());
        assert!(!rsi.is_overlay());
    }

    #[test]
    fn test_overlay_detection() {
        let catalog = UnifiedIndicatorCatalog::new();

        // SMA should be overlay
        let sma = catalog.get("SMA").unwrap();
        assert!(sma.is_overlay());

        // RSI should be sub-pane
        let rsi = catalog.get("RSI").unwrap();
        assert!(!rsi.is_overlay());
    }

    #[test]
    fn test_get_overlay_indicators() {
        let catalog = UnifiedIndicatorCatalog::new();
        let overlays = catalog.get_overlay_indicators();

        assert!(!overlays.is_empty());
        for ind in &overlays {
            assert!(ind.is_overlay(), "{} should be overlay", ind.id());
        }
    }

    #[test]
    fn test_get_subpane_indicators() {
        let catalog = UnifiedIndicatorCatalog::new();
        let subpanes = catalog.get_subpane_indicators();

        assert!(!subpanes.is_empty());
        for ind in &subpanes {
            assert!(!ind.is_overlay(), "{} should be sub-pane", ind.id());
        }
    }

    #[test]
    fn test_stats() {
        let catalog = UnifiedIndicatorCatalog::new();
        let stats = catalog.stats();

        assert!(stats.total_indicators > 400);
        assert!(stats.with_rendering > 0);
        assert!(stats.overlay_count > 0);
        assert!(stats.subpane_count > 0);
    }

    #[test]
    fn test_search() {
        let catalog = UnifiedIndicatorCatalog::new();

        let results = catalog.search("moving average");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_global_catalog() {
        let catalog = catalog();
        assert!(catalog.total_count() > 400);
    }

    #[test]
    fn test_unified_info_bounds() {
        let catalog = UnifiedIndicatorCatalog::new();

        // RSI should have bounds
        let rsi = catalog.get("RSI").unwrap();
        assert_eq!(rsi.bounds(), Some((0.0, 100.0)));

        // SMA should not have bounds
        let sma = catalog.get("SMA").unwrap();
        assert_eq!(sma.bounds(), None);
    }
}
