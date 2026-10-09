use super::types::EwaPatternKind;

#[derive(Debug, Clone, Copy)]
pub struct EwaPatternPrior {
    pub pattern: EwaPatternKind,
    pub base_probability: f64,
    pub rarity: &'static str,
    pub context: &'static str,
    pub fib_bias: &'static str,
}

pub fn pattern_priors() -> &'static [EwaPatternPrior] {
    PATTERN_PRIORS
}

pub fn prior_for(pattern: EwaPatternKind) -> EwaPatternPrior {
    PATTERN_PRIORS
        .iter()
        .find(|prior| prior.pattern == pattern)
        .copied()
        .unwrap_or(EwaPatternPrior {
            pattern,
            base_probability: 0.45,
            rarity: "unknown",
            context: "not calibrated",
            fib_bias: "neutral",
        })
}

const PATTERN_PRIORS: &[EwaPatternPrior] = &[
    EwaPatternPrior {
        pattern: EwaPatternKind::Impulse,
        base_probability: 0.78,
        rarity: "common",
        context: "motive 1/3/5/A/C positions; strongest when wave 3 is not shortest and wave 4 does not overlap wave 1",
        fib_bias: "wave 2 often 0.382-0.786 of wave 1; wave 3 often 1.618+ of wave 1; wave 4 often 0.236-0.382 of wave 3",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ImpulseExtendedWave1,
        base_probability: 0.48,
        rarity: "situational",
        context: "motive count where wave 1 dominates and later waves do not invalidate impulse rules",
        fib_bias: "extended actionary wave should dominate other actionary waves by 1.618+ when clean",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ImpulseExtendedWave3,
        base_probability: 0.82,
        rarity: "common",
        context: "trend acceleration; wave 3 extension is the default high-confidence impulse variant",
        fib_bias: "wave 3 commonly targets 1.618 or 2.618 of wave 1",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ImpulseExtendedWave5,
        base_probability: 0.58,
        rarity: "situational",
        context: "late-stage trend or commodity-style extension; needs wave 4 context",
        fib_bias: "wave 5 often relates to wave 1 by 1.0, 1.618, or equality with wave 1/3 net distance",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::TruncatedImpulse,
        base_probability: 0.24,
        rarity: "rare",
        context: "terminal exhaustion after a strong third wave; prefer non-truncated counts unless context supports it",
        fib_bias: "wave 5 fails beyond wave 3 and usually has weak extension confluence",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::LeadingDiagonalContracting,
        base_probability: 0.42,
        rarity: "less_common",
        context: "only wave 1 or A position; overlap is expected",
        fib_bias: "contracting actionary waves should shrink; wave 2 and 4 are usually zigzag-like retracements",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::LeadingDiagonalExpanding,
        base_probability: 0.30,
        rarity: "rare",
        context: "only wave 1 or A position; expanding diagonals are less common than contracting",
        fib_bias: "actionary waves expand; tolerate larger later-wave extensions",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::EndingDiagonalContracting,
        base_probability: 0.44,
        rarity: "less_common",
        context: "wave 5 or C position; terminal overlap and converging geometry",
        fib_bias: "contracting actionary waves and deep internal retracements",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::EndingDiagonalExpanding,
        base_probability: 0.28,
        rarity: "rare",
        context: "terminal wave 5 or C position; needs strong evidence",
        fib_bias: "expanding actionary waves with divergent geometry",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Correction,
        base_probability: 0.62,
        rarity: "common",
        context: "generic ABC fallback when subtype evidence is weak",
        fib_bias: "B retraces prior A and C often equals or extends A",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Zigzag,
        base_probability: 0.70,
        rarity: "common",
        context: "sharp corrective move; common in wave 2, less common in wave 4",
        fib_bias: "B usually shallow-to-medium; C often 1.0 or 1.618 of A",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::RunningZigzag,
        base_probability: 0.26,
        rarity: "rare",
        context: "strong larger trend; C fails to exceed A endpoint",
        fib_bias: "B/C ratios can look good while endpoint fails, so require trend context",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::DoubleZigzag,
        base_probability: 0.52,
        rarity: "reasonably_common",
        context: "first zigzag did not correct deeply enough; W-X-Y should move strongly corrective",
        fib_bias: "Y often projects W by 0.618, 1.0, or 1.618; X is usually a corrective retracement",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::TripleZigzag,
        base_probability: 0.18,
        rarity: "rare",
        context: "only after W-X-Y fails to reach correction depth; require subdivisions",
        fib_bias: "Z should materially deepen price, not merely add sideways noise",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Flat,
        base_probability: 0.58,
        rarity: "common",
        context: "sideways 3-3-5 correction",
        fib_bias: "B should retrace at least about 0.9 of A; C often near 1.0 of A",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::RegularFlat,
        base_probability: 0.56,
        rarity: "common",
        context: "B about 0.9-1.05 of A and C near A endpoint",
        fib_bias: "B/A near 1.0; C/A near 1.0",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ExpandedFlat,
        base_probability: 0.50,
        rarity: "common",
        context: "B exceeds A origin and C extends beyond A endpoint",
        fib_bias: "B/A above 1.05; C/A often 1.236-1.618",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::RunningFlat,
        base_probability: 0.22,
        rarity: "rare",
        context: "very strong larger trend; B exceeds origin while C fails beyond A",
        fib_bias: "B/A above 1.05 with weak/truncated C",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Triangle,
        base_probability: 0.50,
        rarity: "common_but_position_limited",
        context: "wave 4, B, or combination position; continuation before final actionary wave",
        fib_bias: "legs commonly retrace each other and compress; 0.618-0.786 is useful but geometry matters",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ContractingTriangle,
        base_probability: 0.56,
        rarity: "common_but_position_limited",
        context: "A-B-C-D-E with converging boundaries",
        fib_bias: "successive same-side legs contract; retracements cluster near common fib levels",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::BarrierTriangle,
        base_probability: 0.36,
        rarity: "less_common",
        context: "triangle with one nearly horizontal boundary",
        fib_bias: "D near B boundary is more important than pure ratio fit",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ExpandingTriangle,
        base_probability: 0.16,
        rarity: "very_rare",
        context: "diverging boundaries; demand strong evidence",
        fib_bias: "later legs expand; observed maximums around 1.25-1.664 of prior opposite leg are useful guards",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::RunningTriangle,
        base_probability: 0.20,
        rarity: "rare",
        context: "strong trend continuation with triangle displacement",
        fib_bias: "endpoint displacement matters more than exact fib match",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::DoubleCombo,
        base_probability: 0.36,
        rarity: "uncommon",
        context: "W-X-Y sideways correction; triangle only as final component",
        fib_bias: "prefer sideways net slope and balanced W/Y duration over strong corrective projection",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::DoubleThree,
        base_probability: 0.38,
        rarity: "uncommon",
        context: "two simple corrections joined by X; purpose is time/sideways movement",
        fib_bias: "X usually retraces; W and Y should not behave like a strong double zigzag",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::TripleCombo,
        base_probability: 0.12,
        rarity: "very_rare",
        context: "three corrective structures; require explicit subdivisions",
        fib_bias: "sideways time consumption dominates ratio fit",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::TripleThree,
        base_probability: 0.14,
        rarity: "very_rare",
        context: "three simple corrections joined by X waves",
        fib_bias: "penalize unless it adds proportion and time, not just extra pivots",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Abcd,
        base_probability: 0.46,
        rarity: "harmonic_extension",
        context: "generic harmonic correction overlay",
        fib_bias: "AB retracement and CD projection/equality drive evidence",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Xabcd,
        base_probability: 0.40,
        rarity: "harmonic_extension",
        context: "harmonic overlay, not core EWA primitive",
        fib_bias: "completion near 0.786/0.886/1.0/1.272/1.618 of XA",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Cypher,
        base_probability: 0.34,
        rarity: "harmonic_extension",
        context: "harmonic overlay useful for PRZ scoring",
        fib_bias: "AB 0.382-0.618, XC 1.272-1.414, CD 0.786 of XC",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Gartley,
        base_probability: 0.34,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "AB 0.618 of XA and XD 0.786 of XA",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Bat,
        base_probability: 0.32,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "AB 0.382-0.5 of XA and XD 0.886 of XA",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Butterfly,
        base_probability: 0.30,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "AB 0.786 of XA and XD 1.272-1.618 of XA",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Crab,
        base_probability: 0.28,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "deep CD and XD 1.618 of XA",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::Shark,
        base_probability: 0.26,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "BC 1.13-1.618 and CD 1.618-2.24",
    },
    EwaPatternPrior {
        pattern: EwaPatternKind::ThreeDrives,
        base_probability: 0.24,
        rarity: "harmonic_extension",
        context: "harmonic overlay requiring three symmetric drives",
        fib_bias: "drive symmetry plus 1.272-1.618 projections",
    },
];
