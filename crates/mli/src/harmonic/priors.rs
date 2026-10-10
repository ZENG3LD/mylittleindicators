use super::types::HarmonicKind;

#[derive(Debug, Clone, Copy)]
pub struct HarmonicPrior {
    pub kind: HarmonicKind,
    pub base_probability: f64,
    pub rarity: &'static str,
    pub context: &'static str,
    pub fib_bias: &'static str,
}

pub fn harmonic_priors() -> &'static [HarmonicPrior] {
    HARMONIC_PRIORS
}

pub fn prior_for(kind: HarmonicKind) -> HarmonicPrior {
    HARMONIC_PRIORS
        .iter()
        .find(|prior| prior.kind == kind)
        .copied()
        .unwrap_or(HarmonicPrior {
            kind,
            base_probability: 0.45,
            rarity: "unknown",
            context: "not calibrated",
            fib_bias: "neutral",
        })
}

const HARMONIC_PRIORS: &[HarmonicPrior] = &[
    HarmonicPrior {
        kind: HarmonicKind::Abcd,
        base_probability: 0.46,
        rarity: "harmonic_extension",
        context: "generic harmonic correction overlay",
        fib_bias: "AB retracement and CD projection/equality drive evidence",
    },
    HarmonicPrior {
        kind: HarmonicKind::Xabcd,
        base_probability: 0.40,
        rarity: "harmonic_extension",
        context: "harmonic overlay, not core EWA primitive",
        fib_bias: "completion near 0.786/0.886/1.0/1.272/1.618 of XA",
    },
    HarmonicPrior {
        kind: HarmonicKind::Cypher,
        base_probability: 0.34,
        rarity: "harmonic_extension",
        context: "harmonic overlay useful for PRZ scoring",
        fib_bias: "AB 0.382-0.618, XC 1.272-1.414, CD 0.786 of XC",
    },
    HarmonicPrior {
        kind: HarmonicKind::Gartley,
        base_probability: 0.34,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "AB 0.618 of XA and XD 0.786 of XA",
    },
    HarmonicPrior {
        kind: HarmonicKind::Bat,
        base_probability: 0.32,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "AB 0.382-0.5 of XA and XD 0.886 of XA",
    },
    HarmonicPrior {
        kind: HarmonicKind::Butterfly,
        base_probability: 0.30,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "AB 0.786 of XA and XD 1.272-1.618 of XA",
    },
    HarmonicPrior {
        kind: HarmonicKind::Crab,
        base_probability: 0.28,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "deep CD and XD 1.618 of XA",
    },
    HarmonicPrior {
        kind: HarmonicKind::Shark,
        base_probability: 0.26,
        rarity: "harmonic_extension",
        context: "harmonic overlay",
        fib_bias: "BC 1.13-1.618 and CD 1.618-2.24",
    },
    HarmonicPrior {
        kind: HarmonicKind::ThreeDrives,
        base_probability: 0.24,
        rarity: "harmonic_extension",
        context: "harmonic overlay requiring three symmetric drives",
        fib_bias: "drive symmetry plus 1.272-1.618 projections",
    },
];
