use super::types::{
    EwaCandidate, EwaPatternKind, EwaReinterpretation, EwaReinterpretationRule, EwaScenario,
    EwaWorldAnalysis,
};

pub fn reinterpretation_matrix() -> &'static [EwaReinterpretationRule] {
    REINTERPRETATION_MATRIX
}

pub fn build_reinterpretations(
    worlds: &[EwaWorldAnalysis],
    scenarios: &[EwaScenario],
    per_scenario_limit: usize,
) -> Vec<EwaReinterpretation> {
    let mut out = Vec::new();

    for scenario in scenarios {
        let Some((start, end)) = candidate_bar_span(&scenario.candidate, worlds) else {
            continue;
        };
        let mut matches = Vec::new();

        for rule in REINTERPRETATION_MATRIX {
            if rule.from != scenario.candidate.pattern {
                continue;
            }

            if let Some((target_world, candidate_idx, confidence_hint)) =
                best_overlapping_candidate(worlds, &scenario.candidate, rule.to, start, end)
            {
                matches.push(EwaReinterpretation {
                    source_world: scenario.candidate.source_world.clone(),
                    target_world,
                    candidate_index: candidate_idx,
                    from: rule.from,
                    to: rule.to,
                    reason: rule.condition.to_string(),
                    granularity_effect: rule.granularity_effect.to_string(),
                    required_context: rule.required_context.to_string(),
                    bar_start: start,
                    bar_end: end,
                    confidence_hint,
                });
            }
        }

        matches.sort_by(|a, b| {
            b.confidence_hint
                .partial_cmp(&a.confidence_hint)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(per_scenario_limit);
        out.extend(matches);
    }

    out
}

fn best_overlapping_candidate(
    worlds: &[EwaWorldAnalysis],
    source: &EwaCandidate,
    target: EwaPatternKind,
    source_start: usize,
    source_end: usize,
) -> Option<(String, usize, f64)> {
    let mut best = None;

    for world in worlds {
        for (idx, candidate) in world.candidates.iter().enumerate() {
            if candidate.pattern != target
                || !candidate.is_rule_valid()
                || !transition_is_compatible(source, candidate)
            {
                continue;
            }
            let Some((start, end)) = candidate_bar_span_in_world(candidate, world) else {
                continue;
            };
            let overlap = overlap_ratio(source_start, source_end, start, end);
            if overlap < 0.60 {
                continue;
            }
            let cross_world_bonus = if world.label == source.source_world { 1.0 } else { 1.08 };
            let confidence = (candidate.score * overlap * cross_world_bonus).clamp(0.0, 1.0);
            if confidence <= f64::EPSILON {
                continue;
            }
            if best
                .as_ref()
                .map(|&(_, _, best_confidence)| confidence > best_confidence)
                .unwrap_or(true)
            {
                best = Some((world.label.clone(), idx, confidence));
            }
        }
    }

    best
}

fn transition_is_compatible(source: &EwaCandidate, target: &EwaCandidate) -> bool {
    match (source.pattern, target.pattern) {
        (EwaPatternKind::Zigzag, EwaPatternKind::Impulse)
        | (EwaPatternKind::RunningZigzag, EwaPatternKind::Impulse) => {
            target.subdivision_score > 0.0
                || target.technical_features.momentum_score >= 0.65
        }
        (EwaPatternKind::Impulse, EwaPatternKind::DoubleZigzag)
        | (EwaPatternKind::ImpulseExtendedWave3, EwaPatternKind::DoubleZigzag)
        | (EwaPatternKind::Zigzag, EwaPatternKind::DoubleZigzag)
        | (EwaPatternKind::DoubleZigzag, EwaPatternKind::TripleZigzag)
        | (EwaPatternKind::DoubleThree, EwaPatternKind::TripleThree) => {
            !target.children.is_empty() && target.subdivision_score >= 0.8
        }
        (EwaPatternKind::Triangle, EwaPatternKind::DoubleThree)
        | (EwaPatternKind::ContractingTriangle, EwaPatternKind::DoubleCombo) => {
            target
                .children
                .last()
                .map(|child| {
                    matches!(
                        child.pattern,
                        EwaPatternKind::Triangle
                            | EwaPatternKind::ContractingTriangle
                            | EwaPatternKind::BarrierTriangle
                            | EwaPatternKind::ExpandingTriangle
                            | EwaPatternKind::RunningTriangle
                    )
                })
                .unwrap_or(false)
        }
        (EwaPatternKind::Impulse, EwaPatternKind::LeadingDiagonalContracting)
        | (EwaPatternKind::Impulse, EwaPatternKind::EndingDiagonalContracting) => {
            target.technical_features.channel_score > 0.0
        }
        (EwaPatternKind::ImpulseExtendedWave5, EwaPatternKind::TruncatedImpulse) => {
            target.technical_features.divergence_score > 0.0
        }
        _ => true,
    }
}

fn candidate_bar_span(
    candidate: &EwaCandidate,
    worlds: &[EwaWorldAnalysis],
) -> Option<(usize, usize)> {
    let world = worlds
        .iter()
        .find(|world| world.label == candidate.source_world)?;
    candidate_bar_span_in_world(candidate, world)
}

fn candidate_bar_span_in_world(
    candidate: &EwaCandidate,
    world: &EwaWorldAnalysis,
) -> Option<(usize, usize)> {
    let first = candidate.pivot_indices.first().copied()?;
    let last = candidate.pivot_indices.last().copied()?;
    let start = world.pivots.get(first)?.index;
    let end = world.pivots.get(last)?.index;
    Some((start.min(end), start.max(end)))
}

fn overlap_ratio(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> f64 {
    let overlap_start = a_start.max(b_start);
    let overlap_end = a_end.min(b_end);
    if overlap_end < overlap_start {
        return 0.0;
    }
    let overlap = overlap_end - overlap_start + 1;
    let a_len = a_end - a_start + 1;
    let b_len = b_end - b_start + 1;
    overlap as f64 / a_len.min(b_len) as f64
}

const REINTERPRETATION_MATRIX: &[EwaReinterpretationRule] = &[
    EwaReinterpretationRule {
        from: EwaPatternKind::Zigzag,
        to: EwaPatternKind::Impulse,
        condition: "A-B-C 5-3-5 can be re-read as 1-2-(1) when lower granularity reveals a second 1-2 and price breaks in motive direction",
        granularity_effect: "finer swing pass splits C into nested motive start; coarser pass keeps it as zigzag",
        required_context: "needs higher-degree trend bias and breakout beyond the suspected wave-1 extreme",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::RunningZigzag,
        to: EwaPatternKind::Impulse,
        condition: "running C failure can be re-read as nested 1-2 if the next swing accelerates in the motive direction",
        granularity_effect: "finer granularity should show two retracements instead of one corrective C",
        required_context: "requires confirmation from following impulse extension or higher timeframe count",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Impulse,
        to: EwaPatternKind::DoubleZigzag,
        condition: "weak or overlapping motive labeling can collapse into W-X-Y when actionary waves do not extend cleanly",
        granularity_effect: "coarser pass may join motive subdivisions into corrective W/Y legs",
        required_context: "invalidated if wave 3 extends and wave 4 respects impulse territory rules",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::ImpulseExtendedWave3,
        to: EwaPatternKind::DoubleZigzag,
        condition: "suspected wave-3 extension can be W-Y projection when the middle retracement behaves like X",
        granularity_effect: "lower granularity may split the extension into W-X-Y instead of 1-2-3",
        required_context: "needs larger-degree position: wave C or wave Y favors correction; wave 3 favors impulse",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Flat,
        to: EwaPatternKind::Triangle,
        condition: "sideways A-B-C can become A-B-C-D-E if two more overlapping threes appear",
        granularity_effect: "finer pass appends D/E legs; coarser pass keeps only flat",
        required_context: "typical in wave 4, B, or final X positions",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Triangle,
        to: EwaPatternKind::DoubleThree,
        condition: "triangle-like compression can be final component of W-X-Y instead of standalone triangle",
        granularity_effect: "coarser pass sees W-X-Y, finer pass sees the terminal triangle internals",
        required_context: "depends on whether preceding corrective structure is counted as W-X",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::RegularFlat,
        to: EwaPatternKind::ExpandedFlat,
        condition: "B near origin vs B beyond origin is threshold-sensitive around 1.0 retracement",
        granularity_effect: "small pivot shifts around A origin move flat subtype",
        required_context: "resolve using exact high/low pivot and next C extension",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::ExpandedFlat,
        to: EwaPatternKind::RunningFlat,
        condition: "both have B beyond origin; C either exceeds A or fails before A extreme",
        granularity_effect: "C endpoint granularity decides expanded vs running",
        required_context: "requires completed C and larger trend strength estimate",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::LeadingDiagonalContracting,
        to: EwaPatternKind::Impulse,
        condition: "diagonal becomes impulse if wave 4 overlap disappears under stricter pivots",
        granularity_effect: "pivot normalization can remove or create wave-1/wave-4 overlap",
        required_context: "leading position permits diagonal; middle wave position favors impulse",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::DoubleZigzag,
        to: EwaPatternKind::TripleZigzag,
        condition: "W-X-Y can extend into W-X-Y-X-Z if another corrective connector and zigzag appear",
        granularity_effect: "runtime pass should not predict Z until enough new pivots exist",
        required_context: "rare; prefer double unless added correction materially deepens price",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Correction,
        to: EwaPatternKind::Zigzag,
        condition: "generic A-B-C becomes zigzag when A and C are motive-like and B stays inside A territory",
        granularity_effect: "finer pass should reveal 5-3-5 subdivisions inside A-B-C",
        required_context: "sharp correction, often wave 2 or A of larger correction",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Correction,
        to: EwaPatternKind::Flat,
        condition: "generic A-B-C becomes flat when B retraces about 90% or more of A and C is motive-like",
        granularity_effect: "endpoint precision around B/A 0.90 is decisive",
        required_context: "sideways correction, often wave 4, B, or combination component",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Correction,
        to: EwaPatternKind::Triangle,
        condition: "unfinished A-B-C may be only the first three legs of A-B-C-D-E",
        granularity_effect: "additional pivots at similar degree convert correction into triangle",
        required_context: "requires wave 4, B, or combination context; avoid in wave 2 position",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Zigzag,
        to: EwaPatternKind::DoubleZigzag,
        condition: "single zigzag becomes W of W-X-Y if it does not correct deeply enough and an X connector appears",
        granularity_effect: "later pivots append X-Y without invalidating the first zigzag",
        required_context: "Y should deepen price; otherwise prefer combination/double three",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Zigzag,
        to: EwaPatternKind::RunningZigzag,
        condition: "zigzag subtype flips if C fails to exceed A endpoint while ratios still fit",
        granularity_effect: "C endpoint high/low precision decides regular vs running zigzag",
        required_context: "strong larger-degree trend must explain C failure",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::DoubleZigzag,
        to: EwaPatternKind::DoubleThree,
        condition: "W-X-Y with weak directional progress is better read as double three than double zigzag",
        granularity_effect: "coarser pass may hide internal flat/triangle pieces inside W or Y",
        required_context: "sideways net movement and time consumption dominate correction depth",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::DoubleThree,
        to: EwaPatternKind::DoubleZigzag,
        condition: "double three becomes double zigzag when W and Y are both sharp and directional",
        granularity_effect: "finer pass should show each W/Y component as zigzag-like",
        required_context: "X should be corrective and not dominate W/Y",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::DoubleThree,
        to: EwaPatternKind::TripleThree,
        condition: "W-X-Y can extend into W-X-Y-X-Z when a third sideways corrective structure appears",
        granularity_effect: "new pivots append the terminal Z component",
        required_context: "very rare; require proportion gain, not just extra noise",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::DoubleCombo,
        to: EwaPatternKind::TripleCombo,
        condition: "W-X-Y combination extends into W-X-Y-X-Z if the market keeps moving sideways",
        granularity_effect: "later same-degree structure adds Z; runtime should wait for confirmed pivots",
        required_context: "triangle, if present, should be terminal component",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Flat,
        to: EwaPatternKind::RegularFlat,
        condition: "flat becomes regular when B retraces roughly 90%-105% of A and C ends near A endpoint",
        granularity_effect: "small endpoint shifts around A origin and A endpoint decide subtype",
        required_context: "sideways correction; C should not strongly extend beyond A",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Flat,
        to: EwaPatternKind::ExpandedFlat,
        condition: "flat becomes expanded when B exceeds A origin and C extends beyond A endpoint",
        granularity_effect: "B/A above about 1.05 and C/A extension should survive stricter pivots",
        required_context: "common flat variant when B makes a false breakout",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Flat,
        to: EwaPatternKind::RunningFlat,
        condition: "flat becomes running when B exceeds A origin but C fails beyond A endpoint",
        granularity_effect: "C endpoint high/low precision is decisive",
        required_context: "rare; requires very strong larger-degree trend",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::RegularFlat,
        to: EwaPatternKind::RunningFlat,
        condition: "regular flat becomes running if B is remeasured beyond A origin and C remains short",
        granularity_effect: "higher precision pivot can push B over the origin threshold",
        required_context: "larger-degree trend strength should support C truncation",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Triangle,
        to: EwaPatternKind::ContractingTriangle,
        condition: "generic triangle becomes contracting when A-C-E and B-D boundaries converge",
        granularity_effect: "finer pivots should preserve lower highs/higher lows",
        required_context: "continuation context before final actionary wave",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Triangle,
        to: EwaPatternKind::BarrierTriangle,
        condition: "generic triangle becomes barrier when one boundary is approximately horizontal",
        granularity_effect: "endpoint tolerance around B-D boundary decides subtype",
        required_context: "wave 4 position often implies short/brief or extended fifth after breakout",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Triangle,
        to: EwaPatternKind::ExpandingTriangle,
        condition: "generic triangle becomes expanding when later legs exceed earlier opposite legs and boundaries diverge",
        granularity_effect: "requires same-degree pivots; lower degree noise can fake expansion",
        required_context: "very rare; demand strong subdivision and boundary evidence",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::ContractingTriangle,
        to: EwaPatternKind::BarrierTriangle,
        condition: "contracting triangle becomes barrier when B-D boundary is effectively flat",
        granularity_effect: "boundary tolerance changes subtype without changing A-B-C-D-E count",
        required_context: "resolve by exact highs/lows and volatility scale",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::ContractingTriangle,
        to: EwaPatternKind::DoubleCombo,
        condition: "contracting triangle can be terminal Y of W-X-Y rather than standalone correction",
        granularity_effect: "coarser pass sees W-X before the terminal triangle",
        required_context: "requires prior W-X structure; triangle should be last component",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Impulse,
        to: EwaPatternKind::LeadingDiagonalContracting,
        condition: "early impulse becomes leading diagonal if wave 4 overlap appears and actionary waves contract",
        granularity_effect: "pivot precision can create wave 1/4 overlap",
        required_context: "only valid in wave 1 or A position",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Impulse,
        to: EwaPatternKind::EndingDiagonalContracting,
        condition: "late impulse becomes ending diagonal if overlap and terminal contraction appear",
        granularity_effect: "lower degree pivots expose diagonal subdivision inside wave 5 or C",
        required_context: "only valid in wave 5 or C position",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::LeadingDiagonalContracting,
        to: EwaPatternKind::LeadingDiagonalExpanding,
        condition: "diagonal subtype flips when actionary waves expand instead of contract",
        granularity_effect: "strict same-degree pivots are required to compare wave 1/3/5 lengths",
        required_context: "leading position remains mandatory",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::EndingDiagonalContracting,
        to: EwaPatternKind::EndingDiagonalExpanding,
        condition: "ending diagonal subtype flips when later actionary waves expand and boundaries diverge",
        granularity_effect: "same-degree pivot filtering decides contraction vs expansion",
        required_context: "ending position remains mandatory",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::ImpulseExtendedWave3,
        to: EwaPatternKind::Impulse,
        condition: "wave-3 extension label collapses to generic impulse if extension dominance is marginal",
        granularity_effect: "ratio tolerance around 1.236-1.618 decides extension subtype",
        required_context: "generic impulse should win unless wave 3 extension is materially clear",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::ImpulseExtendedWave5,
        to: EwaPatternKind::TruncatedImpulse,
        condition: "suspected fifth extension becomes truncation if the final endpoint fails beyond wave 3",
        granularity_effect: "endpoint high/low precision decides extension vs truncation",
        required_context: "requires strong prior wave 3 and terminal exhaustion context",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::TruncatedImpulse,
        to: EwaPatternKind::EndingDiagonalContracting,
        condition: "truncated fifth can be terminal diagonal if overlap and converging geometry appear inside the final wave",
        granularity_effect: "finer pass should reveal overlapping five-wave diagonal internals",
        required_context: "only wave 5/C terminal context",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Abcd,
        to: EwaPatternKind::Zigzag,
        condition: "ABCD harmonic overlay maps to A-B-C zigzag when CD is the C leg projection",
        granularity_effect: "coarser pass may see ABCD symmetry; EWA pass labels A-B-C",
        required_context: "requires corrective context, not motive continuation",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Xabcd,
        to: EwaPatternKind::Flat,
        condition: "XABCD completion near the origin can be a flat subtype rather than standalone harmonic pattern",
        granularity_effect: "endpoint relation to A origin and A endpoint decides flat subtype",
        required_context: "harmonics are PRZ evidence, not replacement for EWA position rules",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Gartley,
        to: EwaPatternKind::Zigzag,
        condition: "Gartley ratios can describe a zigzag correction with harmonic PRZ confluence",
        granularity_effect: "EWA count should own the label; harmonic pattern boosts ratio evidence",
        required_context: "use as confluence when A-B-C rules are also valid",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Bat,
        to: EwaPatternKind::Flat,
        condition: "Bat completion can coincide with flat C termination zone",
        granularity_effect: "nearby endpoints can flip harmonic overlay while EWA structure stays corrective",
        required_context: "prefer EWA label; keep harmonic as probability boost",
    },
    EwaReinterpretationRule {
        from: EwaPatternKind::Crab,
        to: EwaPatternKind::ExpandedFlat,
        condition: "deep crab completion can coincide with expanded flat C extension",
        granularity_effect: "C extension magnitude and false-break B decide expanded flat reading",
        required_context: "requires B beyond origin and C beyond A endpoint",
    },
];
