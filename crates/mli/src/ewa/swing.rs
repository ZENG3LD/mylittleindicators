use std::collections::HashMap;

use crate::core::types::Bar;
use crate::indicators::swing::SwingDetection;

use super::config::EwaConfig;
use super::ratios::build_fib_relations;
use super::model::{EwaPivot, EwaPivotKind, EwaWorldAnalysis};
use super::types::{EwaSegment, EwaSegmentDirection};
use crate::fib::FibRelation;

#[derive(Debug, Clone)]
pub struct EwaSwingExtractor {
    config: EwaConfig,
}

impl EwaSwingExtractor {
    pub fn new(config: EwaConfig) -> Self {
        Self { config }
    }

    pub fn extract(&self, bars: &[Bar]) -> (Vec<EwaPivot>, Vec<EwaSegment>) {
        let worlds = self.extract_worlds(bars);
        let pivots = worlds
            .iter()
            .flat_map(|world| world.pivots.iter().cloned())
            .collect::<Vec<_>>();
        let segments = worlds
            .iter()
            .flat_map(|world| world.segments.iter().cloned())
            .collect::<Vec<_>>();

        (pivots, segments)
    }

    pub fn extract_worlds(&self, bars: &[Bar]) -> Vec<EwaWorldAnalysis> {
        let passes = self.config.analysis_passes();
        let mut worlds = Vec::with_capacity(passes.len());
        let mut swing_cache = HashMap::<
            (String, usize, u64),
            (Vec<EwaPivot>, Vec<EwaSegment>, Vec<FibRelation>),
        >::new();

        for pass in &passes {
            let cache_key = (
                pass.swing.label.clone(),
                pass.min_segment_bars,
                pass.min_segment_abs_change.to_bits(),
            );
            let (mut pivots, segments, fib_relations) = swing_cache
                .entry(cache_key)
                .or_insert_with(|| extract_swing_world(pass.swing.mode, bars, pass))
                .clone();
            for pivot in &mut pivots {
                pivot.source_pass = pass.label.clone();
            }

            worlds.push(EwaWorldAnalysis {
                label: pass.label.clone(),
                rule_settings: pass.rule_settings.clone(),
                pivots,
                segments,
                fib_relations,
                candidates: Vec::new(),
            });
        }

        worlds
    }
}

fn extract_swing_world(
    mode: crate::indicators::swing::swing_detection::SwingMode,
    bars: &[Bar],
    pass: &super::config::EwaAlgorithmPassConfig,
) -> (Vec<EwaPivot>, Vec<EwaSegment>, Vec<FibRelation>) {
    let mut pivots = Vec::new();
    let mut detector = SwingDetection::new(mode);
    let mut search_start = 0usize;

    for (index, bar) in bars.iter().enumerate() {
        let signal: i8 = match detector.detect(bar.open, bar.high, bar.low, bar.close, bar.volume) {
            Some((_, crate::Direction::Up)) => 1,
            Some((_, crate::Direction::Down)) => -1,
            _ => 0,
        };
        let Some(kind) = EwaPivotKind::from_signal(signal) else {
            continue;
        };

        let pivot_index = resolve_pivot_index(mode, bars, search_start, index, kind);
        let pivot_bar = &bars[pivot_index];
        pivots.push(EwaPivot {
            index: pivot_index,
            time: pivot_bar.time,
            price: pivot_price(mode, pivot_bar, kind),
            kind,
            confirmed_index: index,
            confirmed_time: bar.time,
            source_pass: pass.label.clone(),
        });
        search_start = pivot_index.saturating_add(1).min(index.saturating_add(1));
    }

    let pivots = normalize_pivots(pivots);
    let segments = build_segments(
        &pivots,
        pass.min_segment_bars,
        pass.min_segment_abs_change,
    );
    let fib_relations = build_fib_relations(&segments);
    (pivots, segments, fib_relations)
}

pub(crate) fn resolve_pivot_index(
    mode: crate::indicators::swing::swing_detection::SwingMode,
    bars: &[Bar],
    search_start: usize,
    confirmed_index: usize,
    kind: EwaPivotKind,
) -> usize {
    match mode {
        crate::indicators::swing::swing_detection::SwingMode::Lookahead { n } => {
            confirmed_index.saturating_sub(n)
        }
        crate::indicators::swing::swing_detection::SwingMode::NBarExtreme { .. }
        | crate::indicators::swing::swing_detection::SwingMode::Time { .. } => confirmed_index,
        crate::indicators::swing::swing_detection::SwingMode::Percent { .. }
        | crate::indicators::swing::swing_detection::SwingMode::AtrMultiple { .. } => {
            let start = search_start.min(confirmed_index);
            let range = &bars[start..=confirmed_index];
            let offset = match kind {
                EwaPivotKind::High => range
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| {
                        a.high
                            .partial_cmp(&b.high)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(offset, _)| offset),
                EwaPivotKind::Low => range
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.low
                            .partial_cmp(&b.low)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(offset, _)| offset),
            };
            start + offset.unwrap_or(confirmed_index.saturating_sub(start))
        }
    }
}

pub(crate) fn pivot_price(
    mode: crate::indicators::swing::swing_detection::SwingMode,
    bar: &Bar,
    kind: EwaPivotKind,
) -> f64 {
    if matches!(mode, crate::indicators::swing::swing_detection::SwingMode::Time { .. }) {
        return bar.close;
    }

    match kind {
        EwaPivotKind::High => bar.high,
        EwaPivotKind::Low => bar.low,
    }
}

pub(crate) fn normalize_pivots(mut pivots: Vec<EwaPivot>) -> Vec<EwaPivot> {
    pivots.sort_by(|a, b| {
        a.index
            .cmp(&b.index)
            .then_with(|| a.confirmed_index.cmp(&b.confirmed_index))
            .then_with(|| a.kind.signal().cmp(&b.kind.signal()))
    });

    let mut out: Vec<EwaPivot> = Vec::with_capacity(pivots.len());

    for pivot in pivots {
        if let Some(last) = out.last_mut() {
            if last.index == pivot.index && last.kind == pivot.kind {
                if is_more_extreme(&pivot, last) {
                    *last = pivot;
                }
                continue;
            }

            if last.kind == pivot.kind {
                if is_more_extreme(&pivot, last) {
                    *last = pivot;
                }
                continue;
            }
        }

        out.push(pivot);
    }

    out
}

fn is_more_extreme(candidate: &EwaPivot, current: &EwaPivot) -> bool {
    match candidate.kind {
        EwaPivotKind::High => candidate.price >= current.price,
        EwaPivotKind::Low => candidate.price <= current.price,
    }
}

pub fn build_segments(
    pivots: &[EwaPivot],
    min_segment_bars: usize,
    min_segment_abs_change: f64,
) -> Vec<EwaSegment> {
    let mut segments = Vec::new();

    for i in 1..pivots.len() {
        let start = &pivots[i - 1];
        let end = &pivots[i];

        if start.kind == end.kind || end.index <= start.index {
            continue;
        }

        let bars = end.index - start.index;
        let price_delta = end.price - start.price;
        let abs_price_delta = price_delta.abs();

        if bars < min_segment_bars || abs_price_delta < min_segment_abs_change {
            continue;
        }

        segments.push(EwaSegment {
            start_pivot: i - 1,
            end_pivot: i,
            direction: EwaSegmentDirection::from_delta(price_delta),
            bars,
            duration_ms: end.time - start.time,
            price_delta,
            abs_price_delta,
            slope_per_bar: price_delta / bars as f64,
        });
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::swing::swing_detection::SwingMode;

    fn bar(i: usize, high: f64, low: f64) -> Bar {
        Bar::new(i as i64, low, high, low, (high + low) * 0.5, 1.0)
    }

    #[test]
    fn extractor_builds_segments_from_swing_signals() {
        let bars = vec![
            bar(0, 10.0, 9.0),
            bar(1, 11.0, 10.0),
            bar(2, 12.0, 11.0),
            bar(3, 11.0, 8.0),
            bar(4, 10.0, 7.0),
            bar(5, 13.0, 12.0),
        ];
        let config = EwaConfig {
            swing_passes: vec![super::super::config::EwaSwingPassConfig::new(
                "nbar_2",
                SwingMode::NBarExtreme { n: 2 },
            )],
            ..Default::default()
        };
        let extractor = EwaSwingExtractor::new(config);
        let (pivots, segments) = extractor.extract(&bars);

        assert!(pivots.len() >= 2);
        assert!(
            segments
                .iter()
                .any(|s| s.direction == EwaSegmentDirection::Down)
        );
        assert!(
            segments
                .iter()
                .any(|s| s.direction == EwaSegmentDirection::Up)
        );
    }

    #[test]
    fn lookahead_resolves_pivot_bar_before_confirmation() {
        let bars = vec![
            bar(0, 10.0, 9.0),
            bar(1, 11.0, 10.0),
            bar(2, 15.0, 11.0),
            bar(3, 12.0, 10.0),
            bar(4, 11.0, 9.0),
        ];
        let index = resolve_pivot_index(
            SwingMode::Lookahead { n: 2 },
            &bars,
            0,
            4,
            EwaPivotKind::High,
        );

        assert_eq!(index, 2);
    }

    #[test]
    fn reversal_resolves_running_extreme_not_confirmation_bar() {
        let bars = vec![
            bar(0, 10.0, 9.0),
            bar(1, 14.0, 10.0),
            bar(2, 12.0, 8.0),
        ];
        let index = resolve_pivot_index(
            SwingMode::Percent { threshold_pct: 2.0 },
            &bars,
            0,
            2,
            EwaPivotKind::High,
        );

        assert_eq!(index, 1);
    }
}
