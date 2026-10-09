use super::priors::prior_for;
use super::types::EwaCandidate;

#[derive(Debug, Clone, Copy)]
pub struct EwaProbabilityModel {
    pub structural_weight: f64,
    pub fib_weight: f64,
    pub geometry_weight: f64,
    pub subdivision_weight: f64,
    pub technical_weight: f64,
    pub calibration_bias: f64,
    pub calibration_scale: f64,
}

impl Default for EwaProbabilityModel {
    fn default() -> Self {
        Self {
            structural_weight: 1.8,
            fib_weight: 0.8,
            geometry_weight: 0.6,
            subdivision_weight: 0.7,
            technical_weight: 0.4,
            calibration_bias: 0.0,
            calibration_scale: 1.0,
        }
    }
}

impl EwaProbabilityModel {
    pub fn probability(self, candidate: &EwaCandidate) -> f64 {
        sigmoid(self.calibrated_logit(candidate))
    }

    pub fn raw_probability(self, candidate: &EwaCandidate) -> f64 {
        sigmoid(self.raw_logit(candidate))
    }

    pub fn raw_logit(self, candidate: &EwaCandidate) -> f64 {
        if !candidate.is_rule_valid() {
            return f64::NEG_INFINITY;
        }
        let prior = prior_for(candidate.pattern)
            .base_probability
            .clamp(0.001, 0.999);
        (prior / (1.0 - prior)).ln()
            + self.structural_weight * centered(candidate.score)
            + self.fib_weight * centered(candidate.fib_matrix_score)
            + self.geometry_weight * centered(candidate.geometry_score)
            + self.subdivision_weight * centered(candidate.subdivision_score)
            + self.technical_weight * centered(candidate.technical_features.aggregate())
    }

    pub fn calibrated_logit(self, candidate: &EwaCandidate) -> f64 {
        self.calibration_bias + self.calibration_scale.max(0.0) * self.raw_logit(candidate)
    }

    pub fn with_calibration(mut self, bias: f64, scale: f64) -> Self {
        self.calibration_bias = bias;
        self.calibration_scale = scale.max(0.0);
        self
    }

    pub fn fit_calibration(
        mut self,
        observations: &[(f64, bool)],
        iterations: usize,
        learning_rate: f64,
    ) -> Self {
        if observations.is_empty() {
            return self;
        }
        let mut bias = self.calibration_bias;
        let mut scale = self.calibration_scale;
        for _ in 0..iterations {
            let mut bias_gradient = 0.0;
            let mut scale_gradient = 0.0;
            for &(raw_probability, outcome) in observations {
                let raw_logit = probability_logit(raw_probability);
                let prediction = sigmoid(bias + scale * raw_logit);
                let error = prediction - if outcome { 1.0 } else { 0.0 };
                bias_gradient += error;
                scale_gradient += error * raw_logit;
            }
            let normalization = observations.len() as f64;
            bias -= learning_rate * bias_gradient / normalization;
            scale = (scale - learning_rate * scale_gradient / normalization).max(0.0);
        }
        self.calibration_bias = bias;
        self.calibration_scale = scale;
        self
    }
}

fn centered(value: f64) -> f64 {
    2.0 * value.clamp(0.0, 1.0) - 1.0
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

fn probability_logit(probability: f64) -> f64 {
    let probability = probability.clamp(0.000_001, 0.999_999);
    (probability / (1.0 - probability)).ln()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ewa::types::{EwaPatternKind, EwaTechnicalFeatures};

    fn candidate(score: f64, subdivision_score: f64) -> EwaCandidate {
        EwaCandidate {
            source_world: "test".into(),
            pattern: EwaPatternKind::Zigzag,
            pivot_indices: vec![0, 1, 2, 3],
            segment_indices: vec![0, 1, 2],
            ratios: Vec::new(),
            children: Vec::new(),
            subdivision_score,
            technical_features: EwaTechnicalFeatures::default(),
            hard_violations: Vec::new(),
            fib_confluence: score,
            fib_matrix_score: score,
            harmonic_confluence: 0.0,
            geometry_score: score,
            prior_probability: 0.7,
            soft_penalty: 1.0 - score,
            score,
        }
    }

    #[test]
    fn stronger_evidence_increases_probability() {
        let model = EwaProbabilityModel::default();
        assert!(
            model.probability(&candidate(0.9, 1.0))
                > model.probability(&candidate(0.6, 0.0))
        );
    }

    #[test]
    fn calibration_fit_separates_outcomes() {
        let observations = [(0.8, true), (0.7, true), (0.3, false), (0.2, false)];
        let model = EwaProbabilityModel::default().fit_calibration(&observations, 200, 0.1);
        assert!(model.calibration_scale > 1.0);
    }
}
