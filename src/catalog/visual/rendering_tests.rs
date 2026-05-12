//! Rendering validation tests
//!
//! Tests to verify that all indicators in the catalog can be rendered correctly.
//! This module checks:
//! 1. All BarIndicatorIds in RENDERING_CATALOG can be created via IndicatorInstance::create()
//! 2. ValueExtractors match the IndicatorValue type returned by each indicator
//! 3. Counts are consistent across enum, factory, and rendering catalog

use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};
use crate::catalog::rendering_catalog::{get_rendering, has_rendering, RENDERING_CATALOG};
use crate::catalog::value_adapter::ValueAdapter;

/// Get all indicator IDs from RENDERING_CATALOG (source of truth)
/// This ensures we test exactly what's in the catalog, no manual sync needed
pub fn all_indicator_ids() -> Vec<BarIndicatorId> {
    RENDERING_CATALOG.keys().copied().collect()
}


/// Result of a single indicator validation
#[derive(Debug)]
pub struct IndicatorValidationResult {
    pub id: BarIndicatorId,
    pub has_rendering: bool,
    pub creates_ok: bool,
    pub create_error: Option<String>,
    pub value_ok: bool,
    pub value_type: Option<String>,
    pub extractor_errors: Vec<String>,
}

impl IndicatorValidationResult {
    pub fn is_fully_valid(&self) -> bool {
        self.has_rendering && self.creates_ok && self.value_ok && self.extractor_errors.is_empty()
    }
}

/// Validate a single indicator
pub fn validate_indicator(id: BarIndicatorId) -> IndicatorValidationResult {
    let mut result = IndicatorValidationResult {
        id,
        has_rendering: has_rendering(id),
        creates_ok: false,
        create_error: None,
        value_ok: false,
        value_type: None,
        extractor_errors: Vec::new(),
    };

    // Try to create the indicator with multiple periods
    let config = IndicatorConfig::new(id, format!("{:?}", id), vec![7, 14, 28]);

    // Catch panics during creation
    let create_result = std::panic::catch_unwind(|| {
        IndicatorInstance::create(&config)
    });

    let instance = match create_result {
        Ok(Ok(inst)) => {
            result.creates_ok = true;
            inst
        }
        Ok(Err(e)) => {
            result.create_error = Some(e);
            return result;
        }
        Err(_) => {
            result.create_error = Some("Panicked during creation".to_string());
            return result;
        }
    };

    // Feed sample data and get value with panic catching
    let mut instance = instance;
    let feed_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for i in 0..100 {
            let price = 100.0 + (i as f64) * 0.1;
            instance.update_bar(price, price + 0.5, price - 0.5, price + 0.2, 1000.0, None);
        }
        instance.value()
    }));

    let value = match feed_result {
        Ok(v) => v,
        Err(_) => {
            result.create_error = Some("Panicked during update/value".to_string());
            return result;
        }
    };

    result.value_ok = true;
    result.value_type = Some(format!("{:?}", std::mem::discriminant(&value)));

    // Check extractor compatibility if rendering exists
    if let Some(rendering) = get_rendering(id) {
        for output in &rendering.outputs {
            let extracted = ValueAdapter::extract(&value, &output.value_extractor);
            if extracted.is_none() {
                result.extractor_errors.push(format!(
                    "Output '{}': extractor {:?} returned None for value type {:?}",
                    output.name, output.value_extractor, result.value_type
                ));
            }
        }
    }

    result
}

/// Validate all indicators and return results
pub fn validate_all_indicators() -> Vec<IndicatorValidationResult> {
    all_indicator_ids().iter()
        .map(|&id| validate_indicator(id))
        .collect()
}

/// Generate a validation report
pub fn generate_validation_report() -> ValidationReport {
    let results = validate_all_indicators();

    let mut report = ValidationReport {
        total: results.len(),
        missing_rendering: Vec::new(),
        create_failures: Vec::new(),
        extractor_mismatches: Vec::new(),
        fully_valid: Vec::new(),
    };

    for result in results {
        if !result.has_rendering {
            report.missing_rendering.push(result.id);
        } else if !result.creates_ok {
            report.create_failures.push((result.id, result.create_error.clone().unwrap_or_default()));
        } else if !result.extractor_errors.is_empty() {
            report.extractor_mismatches.push((result.id, result.extractor_errors.clone()));
        } else {
            report.fully_valid.push(result.id);
        }
    }

    report
}

/// Validation report summary
#[derive(Debug)]
pub struct ValidationReport {
    pub total: usize,
    pub missing_rendering: Vec<BarIndicatorId>,
    pub create_failures: Vec<(BarIndicatorId, String)>,
    pub extractor_mismatches: Vec<(BarIndicatorId, Vec<String>)>,
    pub fully_valid: Vec<BarIndicatorId>,
}

impl ValidationReport {
    pub fn print_summary(&self) {
        println!("=== RENDERING VALIDATION REPORT ===");
        println!("Total indicators: {}", self.total);
        println!("Fully valid: {}", self.fully_valid.len());
        println!("Missing rendering: {}", self.missing_rendering.len());
        println!("Create failures: {}", self.create_failures.len());
        println!("Extractor mismatches: {}", self.extractor_mismatches.len());
        println!();

        if !self.missing_rendering.is_empty() {
            println!("--- MISSING RENDERING ({}) ---", self.missing_rendering.len());
            for id in &self.missing_rendering {
                println!("  {:?}", id);
            }
            println!();
        }

        if !self.create_failures.is_empty() {
            println!("--- CREATE FAILURES ({}) ---", self.create_failures.len());
            for (id, err) in &self.create_failures {
                println!("  {:?}: {}", id, err);
            }
            println!();
        }

        if !self.extractor_mismatches.is_empty() {
            println!("--- EXTRACTOR MISMATCHES ({}) ---", self.extractor_mismatches.len());
            for (id, errors) in &self.extractor_mismatches {
                println!("  {:?}:", id);
                for err in errors {
                    println!("    - {}", err);
                }
            }
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rendering_catalog_count() {
        // Verify rendering catalog has entries
        let ids = all_indicator_ids();
        println!("\nTotal indicators in RENDERING_CATALOG: {}", ids.len());
        assert!(ids.len() >= 400, "Expected at least 400 indicators, got {}", ids.len());
    }

    #[test]
    fn test_all_indicators_create() {
        let mut failures = Vec::new();
        let mut panics = Vec::new();
        let ids = all_indicator_ids();

        for id in &ids {
            // Use multiple periods for indicators that need them (like UltimateOscillator)
            let config = IndicatorConfig::new(*id, format!("{:?}", id), vec![7, 14, 28]);

            // Catch panics during creation
            let result = std::panic::catch_unwind(|| {
                IndicatorInstance::create(&config)
            });

            match result {
                Ok(Ok(_)) => { /* success */ }
                Ok(Err(e)) => failures.push((*id, e)),
                Err(_) => panics.push(*id),
            }
        }

        if !failures.is_empty() {
            println!("\n=== CREATE FAILURES ({}) ===", failures.len());
            for (id, err) in &failures {
                println!("  {:?}: {}", id, err);
            }
        }

        if !panics.is_empty() {
            println!("\n=== PANICS DURING CREATE ({}) ===", panics.len());
            for id in &panics {
                println!("  {:?}", id);
            }
        }

        assert!(failures.is_empty(), "Found {} create failures", failures.len());
        assert!(panics.is_empty(), "Found {} panics during create", panics.len());
    }

    #[test]
    fn test_extractor_compatibility() {
        let mut mismatches = Vec::new();
        let mut skipped_create = Vec::new();
        let mut skipped_panic = Vec::new();
        let ids = all_indicator_ids();

        for id in &ids {
            // Get rendering (should always exist since we iterate catalog keys)
            let rendering = match get_rendering(*id) {
                Some(r) => r,
                None => continue,
            };

            // Try to create with panic catching
            let config = IndicatorConfig::new(*id, format!("{:?}", id), vec![7, 14, 28]);
            let create_result = std::panic::catch_unwind(|| {
                IndicatorInstance::create(&config)
            });

            let mut instance = match create_result {
                Ok(Ok(i)) => i,
                Ok(Err(_)) => {
                    skipped_create.push(*id);
                    continue;
                }
                Err(_) => {
                    skipped_panic.push(*id);
                    continue;
                }
            };

            // Feed data with panic catching
            let feed_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for i in 0..100 {
                    let price = 100.0 + (i as f64) * 0.1;
                    instance.update_bar(price, price + 0.5, price - 0.5, price + 0.2, 1000.0, None);
                }
                instance.value()
            }));

            let value = match feed_result {
                Ok(v) => v,
                Err(_) => {
                    skipped_panic.push(*id);
                    continue;
                }
            };

            // Check each output extractor
            for output in &rendering.outputs {
                if ValueAdapter::extract(&value, &output.value_extractor).is_none() {
                    mismatches.push((*id, output.name.clone(), format!("{:?}", output.value_extractor)));
                }
            }
        }

        println!("\n=== EXTRACTOR COMPATIBILITY REPORT ===");
        println!("Skipped (create error): {}", skipped_create.len());
        println!("Skipped (panic): {}", skipped_panic.len());

        if !mismatches.is_empty() {
            println!("\n=== EXTRACTOR MISMATCHES ({}) ===", mismatches.len());
            for (id, output, extractor) in &mismatches {
                println!("  {:?} -> output '{}' with extractor {}", id, output, extractor);
            }
        }

        assert!(mismatches.is_empty(), "Found {} extractor mismatches - these indicators will NOT render correctly!", mismatches.len());
        println!("\nAll extractors compatible!");
    }

    #[test]
    fn test_full_validation_report() {
        let report = generate_validation_report();
        report.print_summary();

        assert!(report.total > 0, "Should have some indicators to validate");
        assert!(
            report.create_failures.is_empty(),
            "Found {} create failures: {:?}",
            report.create_failures.len(),
            report.create_failures.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );
        assert!(
            report.extractor_mismatches.is_empty(),
            "Found {} extractor mismatches: {:?}",
            report.extractor_mismatches.len(),
            report.extractor_mismatches.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );

        // All indicators should be fully valid
        assert_eq!(
            report.fully_valid.len(),
            report.total,
            "Expected {} fully valid indicators, got {}",
            report.total,
            report.fully_valid.len()
        );
    }

    /// Test to find indicators that return zero/empty values after feeding data
    /// These indicators technically "work" but don't produce useful output
    #[test]
    fn test_indicators_return_meaningful_values() {
        let ids = all_indicator_ids();
        let mut zero_indicators: Vec<(BarIndicatorId, String, Vec<f64>)> = Vec::new();
        let mut not_ready: Vec<BarIndicatorId> = Vec::new();
        let mut skipped: Vec<BarIndicatorId> = Vec::new();

        // Pre-generate bar data once (500 bars for indicators needing longer warmup)
        let start_ts: i64 = 1704067200; // 2024-01-01 00:00:00 UTC
        let bars: Vec<(f64, f64, f64, f64, f64, i64)> = (0..500)
            .map(|i| {
                let base = 100.0 + (i as f64 * 0.1).sin() * 10.0;
                let open = base;
                let high = base + 1.0 + (i as f64 * 0.3).sin().abs() * 2.0;
                let low = base - 1.0 - (i as f64 * 0.2).cos().abs() * 2.0;
                let close = base + (i as f64 * 0.15).cos() * 1.5;
                let volume = 1000.0 + (i as f64 * 50.0);
                let timestamp = start_ts + (i as i64 * 3600);
                (open, high, low, close, volume, timestamp)
            })
            .collect();

        // Skip indicators - currently none (fixed memory issues)
        let skip_indicators: [BarIndicatorId; 0] = [];

        for id in &ids {
            // Skip known memory-heavy indicators
            if skip_indicators.contains(id) {
                skipped.push(*id);
                continue;
            }

            let config = IndicatorConfig::new(*id, format!("{:?}", id), vec![7, 14, 28]);

            let create_result = std::panic::catch_unwind(|| {
                IndicatorInstance::create(&config)
            });

            let mut instance = match create_result {
                Ok(Ok(i)) => i,
                Ok(Err(e)) => {
                    println!("  CREATE ERROR {:?}: {}", id, e);
                    skipped.push(*id);
                    continue;
                }
                Err(_) => {
                    println!("  CREATE PANIC {:?}", id);
                    skipped.push(*id);
                    continue;
                }
            };

            // Use 500 bars for all indicators
            let bar_count = 500;

            let feed_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for &(open, high, low, close, volume, timestamp) in bars.iter().take(bar_count) {
                    instance.update_bar(open, high, low, close, volume, Some(timestamp));
                }
                (instance.is_ready(), instance.value())
            }));

            let (is_ready, value) = match feed_result {
                Ok(v) => v,
                Err(_) => {
                    println!("  UPDATE PANIC {:?}", id);
                    skipped.push(*id);
                    continue;
                }
            };

            if !is_ready {
                not_ready.push(*id);
                continue;
            }

            // Check if ALL values are zero using as_vec()
            let values = value.as_vec();
            let all_zero = values.iter().all(|v| *v == 0.0);

            if all_zero {
                zero_indicators.push((*id, format!("{}", value), values));
            }
        }

        println!("\n=== INDICATORS RETURNING ALL ZEROS ({}) ===", zero_indicators.len());
        for (id, display, values) in &zero_indicators {
            println!("  {:?}: {} -> {:?}", id, display, values);
        }

        println!("\n=== NOT READY AFTER 500 BARS ({}) ===", not_ready.len());
        for id in &not_ready {
            println!("  {:?}", id);
        }

        println!("\n=== SKIPPED (create/update error) ({}) ===", skipped.len());
        for id in &skipped {
            println!("  {:?}", id);
        }

        // This is informational for now - don't fail
        // TODO: Once fixed, enable this assert:
        // assert!(zero_indicators.is_empty(), "Found {} indicators returning zeros", zero_indicators.len());
    }

    /// Test indicators with specialized data generators
    /// Uses appropriate synthetic data for each indicator category
    #[test]
    fn test_indicators_with_specialized_data() {
        use crate::catalog::synthetic_data::{generate_bars, recommended_data_type, DataType};

        let ids = all_indicator_ids();
        let mut results: Vec<(BarIndicatorId, DataType, bool, String)> = Vec::new();
        let mut improved: Vec<BarIndicatorId> = Vec::new();
        let mut still_zero: Vec<(BarIndicatorId, DataType)> = Vec::new();
        let mut skipped: Vec<BarIndicatorId> = Vec::new();

        let start_ts: i64 = 1704067200; // 2024-01-01 00:00:00 UTC (Monday)

        // Pre-generate all data types
        let data_cache: std::collections::HashMap<DataType, Vec<(f64, f64, f64, f64, f64, i64)>> = [
            DataType::Smooth,
            DataType::CandlePatterns,
            DataType::Gaps,
            DataType::Divergence,
            DataType::VolatilityBreakout,
            DataType::VolatilityClustering,
            DataType::StructuralBreaks,
            DataType::Squeeze,
            DataType::Calendar,
            DataType::Correlated,
            DataType::StrongTrend,
            DataType::Ranging,
            DataType::SweepReversion,
            DataType::ZigZagSwings,
            DataType::TickData,
        ]
        .into_iter()
        .map(|dt| {
            let bars = generate_bars(dt, 500, start_ts);
            let tuples: Vec<_> = bars
                .into_iter()
                .map(|b| (b.open, b.high, b.low, b.close, b.volume, b.timestamp))
                .collect();
            (dt, tuples)
        })
        .collect();

        for id in &ids {
            let indicator_name = format!("{:?}", id);
            let data_type = recommended_data_type(&indicator_name);

            let config = IndicatorConfig::new(*id, indicator_name.clone(), vec![7, 14, 28]);

            let create_result = std::panic::catch_unwind(|| {
                IndicatorInstance::create(&config)
            });

            let mut instance = match create_result {
                Ok(Ok(i)) => i,
                Ok(Err(_)) | Err(_) => {
                    skipped.push(*id);
                    continue;
                }
            };

            // Get appropriate data for this indicator
            let bars = data_cache.get(&data_type).unwrap();

            // For pattern-based indicators, we need to track if ANY value was non-zero
            // because patterns only fire occasionally (not on every bar)
            let feed_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut any_nonzero = false;
                let mut last_value = instance.value();

                for &(open, high, low, close, volume, timestamp) in bars.iter() {
                    instance.update_bar(open, high, low, close, volume, Some(timestamp));
                    let val = instance.value();
                    let values = val.as_vec();
                    if values.iter().any(|v| *v != 0.0) {
                        any_nonzero = true;
                    }
                    last_value = val;
                }
                (instance.is_ready(), last_value, any_nonzero)
            }));

            let (is_ready, value, any_nonzero) = match feed_result {
                Ok(v) => v,
                Err(_) => {
                    skipped.push(*id);
                    continue;
                }
            };

            if !is_ready {
                continue;
            }

            // Consider indicator successful if ANY value during the run was non-zero
            let has_value = any_nonzero;

            results.push((*id, data_type, has_value, format!("{}", value)));

            if has_value && data_type != DataType::Smooth {
                improved.push(*id);
            } else if !has_value {
                still_zero.push((*id, data_type));
            }
        }

        println!("\n=== SPECIALIZED DATA TEST RESULTS ===");
        println!("Total tested: {}", results.len());
        println!("Improved with specialized data: {}", improved.len());
        println!("Still zero: {}", still_zero.len());
        println!("Skipped: {}", skipped.len());

        if !improved.is_empty() {
            println!("\n=== IMPROVED INDICATORS ({}) ===", improved.len());
            for id in &improved {
                let data_type = recommended_data_type(&format!("{:?}", id));
                println!("  {:?} (using {:?} data)", id, data_type);
            }
        }

        if !still_zero.is_empty() {
            println!("\n=== STILL RETURNING ZEROS ({}) ===", still_zero.len());
            for (id, data_type) in &still_zero {
                println!("  {:?} (tried {:?} data)", id, data_type);
            }
        }

        // Summary by data type
        println!("\n=== BY DATA TYPE ===");
        let mut by_type: std::collections::HashMap<DataType, (usize, usize)> = std::collections::HashMap::new();
        for (_, dt, has_value, _) in &results {
            let entry = by_type.entry(*dt).or_insert((0, 0));
            entry.0 += 1;
            if *has_value {
                entry.1 += 1;
            }
        }
        for (dt, (total, non_zero)) in &by_type {
            println!("  {:?}: {}/{} non-zero", dt, non_zero, total);
        }
    }
}

/// Debug test to understand why candle patterns aren't being detected
#[test]
fn debug_candle_pattern_detection() {
    use crate::catalog::synthetic_data::{generate_bars, DataType};
    use crate::bar_indicators::candles::patterns::hammer::Hammer;
    use crate::bar_indicators::candles::patterns::doji::Doji;

    let bars = generate_bars(DataType::CandlePatterns, 100, 0);

    println!("\n=== DEBUG: Hammer Detection ===");
    let mut hammer = Hammer::new(2.0, 0.5);
    let mut hammer_detections = 0;

    for (i, bar) in bars.iter().enumerate() {
        let val = hammer.update_bar(bar.open, bar.high, bar.low, bar.close, bar.volume);
        let range = bar.high - bar.low;
        let body = (bar.close - bar.open).abs();
        let body_top = bar.open.max(bar.close);
        let body_bottom = bar.open.min(bar.close);
        let lower_shadow = body_bottom - bar.low;
        let upper_shadow = bar.high - body_top;

        // Check bars that SHOULD be hammers (every 40 bars at position 5)
        if i % 40 == 5 {
            println!("Bar {}: O={:.2} H={:.2} L={:.2} C={:.2}", i, bar.open, bar.high, bar.low, bar.close);
            println!("  range={:.2}, body={:.2}, body_ratio={:.3}", range, body, body/range);
            println!("  lower_shadow={:.2}, upper_shadow={:.2}", lower_shadow, upper_shadow);
            if body > 0.0 {
                println!("  lower_shadow_ratio={:.2}, upper_shadow_ratio={:.2}", lower_shadow/body, upper_shadow/body);
            }
            println!("  DETECTED={}, strength={:.2}", val > 0.0, val);
        }

        if val > 0.0 {
            hammer_detections += 1;
            println!("Bar {}: Hammer detected! val={:.2}", i, val);
        }
    }
    println!("Total hammer detections: {}", hammer_detections);

    println!("\n=== DEBUG: Doji Detection ===");
    let mut doji = Doji::new(0.1);
    let mut doji_detections = 0;

    for (i, bar) in bars.iter().enumerate() {
        let val = doji.update_bar(bar.open, bar.high, bar.low, bar.close, bar.volume);

        // Check bars that SHOULD be dojis (every 40 bars at position 15)
        if i % 40 == 15 {
            let range = bar.high - bar.low;
            let body = (bar.close - bar.open).abs();
            println!("Bar {}: O={:.2} H={:.2} L={:.2} C={:.2}", i, bar.open, bar.high, bar.low, bar.close);
            println!("  range={:.2}, body={:.2}, body_ratio={:.3}", range, body, body/range);
            println!("  DETECTED={}, strength={:.2}", val > 0.0, val);
        }

        if val > 0.0 {
            doji_detections += 1;
        }
    }
    println!("Total doji detections: {}", doji_detections);
}
