use crate::indicators::swing::swing_detection::SwingMode;

use super::probability::EwaProbabilityModel;
use super::types::{EwaPatternKind, EwaRuleSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaExecutionProfile {
    Runtime,
    DevExplore,
}

#[derive(Debug, Clone)]
pub struct EwaSwingPassConfig {
    pub label: String,
    pub mode: SwingMode,
}

impl EwaSwingPassConfig {
    pub fn new(label: impl Into<String>, mode: SwingMode) -> Self {
        Self {
            label: label.into(),
            mode,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EwaAlgorithmPassConfig {
    pub label: String,
    pub swing: EwaSwingPassConfig,
    pub min_segment_bars: usize,
    pub min_segment_abs_change: f64,
    pub rule_settings: EwaRuleSettings,
}

impl EwaAlgorithmPassConfig {
    pub fn new(
        label: impl Into<String>,
        swing: EwaSwingPassConfig,
        min_segment_bars: usize,
        min_segment_abs_change: f64,
        rule_settings: EwaRuleSettings,
    ) -> Self {
        Self {
            label: label.into(),
            swing,
            min_segment_bars,
            min_segment_abs_change,
            rule_settings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EwaConfig {
    pub execution_profile: EwaExecutionProfile,
    pub algorithm_passes: Vec<EwaAlgorithmPassConfig>,
    pub swing_passes: Vec<EwaSwingPassConfig>,
    pub min_segment_bars: usize,
    pub min_segment_abs_change: f64,
    pub max_candidate_pivots: usize,
    pub top_n: usize,
    pub selection_overlap_penalty: f64,
    pub selection_recency_weight: f64,
    pub selection_same_pattern_penalty: f64,
    pub selection_same_world_penalty: f64,
    pub confirmed_subdivision_threshold: f64,
    pub provisional_confidence_multiplier: f64,
    pub probability_model: EwaProbabilityModel,
    pub cpu_composition_fallback: bool,
}

impl Default for EwaConfig {
    fn default() -> Self {
        Self {
            execution_profile: EwaExecutionProfile::Runtime,
            algorithm_passes: Vec::new(),
            swing_passes: vec![
                EwaSwingPassConfig::new("percent_1_0", SwingMode::Percent { threshold_pct: 1.0 }),
            ],
            min_segment_bars: 1,
            min_segment_abs_change: 0.0,
            max_candidate_pivots: 26,
            top_n: 4,
            selection_overlap_penalty: 0.15,
            selection_recency_weight: 0.02,
            selection_same_pattern_penalty: 0.03,
            selection_same_world_penalty: 0.02,
            confirmed_subdivision_threshold: 0.8,
            provisional_confidence_multiplier: 0.72,
            probability_model: EwaProbabilityModel::default(),
            cpu_composition_fallback: true,
        }
    }
}

impl EwaConfig {
    pub fn dev_explore() -> Self {
        Self::default()
            .with_execution_profile(EwaExecutionProfile::DevExplore)
            .with_algorithm_passes(Vec::new())
            .with_swing_passes(Vec::new())
            .with_percent_worlds(&[0.25, 0.5, 0.75, 1.0, 1.5, 2.0])
            .with_nbar_worlds(&[3, 5, 8, 13])
            .with_lookahead_worlds(&[3, 5])
    }

    pub fn with_execution_profile(mut self, profile: EwaExecutionProfile) -> Self {
        self.execution_profile = profile;
        self
    }

    pub fn with_probability_model(mut self, model: EwaProbabilityModel) -> Self {
        self.probability_model = model;
        self
    }

    pub fn with_cpu_composition_fallback(mut self, enabled: bool) -> Self {
        self.cpu_composition_fallback = enabled;
        self
    }

    pub fn with_algorithm_passes(mut self, passes: Vec<EwaAlgorithmPassConfig>) -> Self {
        self.algorithm_passes = passes;
        self
    }

    pub fn with_swing_passes(mut self, passes: Vec<EwaSwingPassConfig>) -> Self {
        self.swing_passes = passes;
        self
    }

    pub fn with_swing_pass(mut self, label: impl Into<String>, mode: SwingMode) -> Self {
        self.swing_passes.push(EwaSwingPassConfig::new(label, mode));
        self
    }

    pub fn with_percent_worlds(mut self, thresholds: &[f64]) -> Self {
        for &threshold_pct in thresholds {
            self.swing_passes.push(EwaSwingPassConfig::new(
                format!("percent_{threshold_pct:.3}"),
                SwingMode::Percent { threshold_pct },
            ));
        }
        self
    }

    pub fn with_nbar_worlds(mut self, values: &[usize]) -> Self {
        for &n in values {
            self.swing_passes.push(EwaSwingPassConfig::new(
                format!("nbar_{n}"),
                SwingMode::NBarExtreme { n },
            ));
        }
        self
    }

    pub fn with_lookahead_worlds(mut self, values: &[usize]) -> Self {
        for &n in values {
            self.swing_passes.push(EwaSwingPassConfig::new(
                format!("lookahead_{n}"),
                SwingMode::Lookahead { n },
            ));
        }
        self
    }

    pub fn with_top_n(mut self, top_n: usize) -> Self {
        self.top_n = top_n.clamp(1, 4);
        self
    }

    pub fn with_selection_overlap_penalty(mut self, penalty: f64) -> Self {
        self.selection_overlap_penalty = penalty.max(0.0);
        self
    }

    pub fn with_selection_recency_weight(mut self, weight: f64) -> Self {
        self.selection_recency_weight = weight.max(0.0);
        self
    }

    pub fn with_selection_same_pattern_penalty(mut self, penalty: f64) -> Self {
        self.selection_same_pattern_penalty = penalty.max(0.0);
        self
    }

    pub fn with_selection_same_world_penalty(mut self, penalty: f64) -> Self {
        self.selection_same_world_penalty = penalty.max(0.0);
        self
    }

    pub fn analysis_passes(&self) -> Vec<EwaAlgorithmPassConfig> {
        if !self.algorithm_passes.is_empty() {
            return self.algorithm_passes.clone();
        }

        self.swing_passes
            .iter()
            .map(|swing| {
                EwaAlgorithmPassConfig::new(
                    swing.label.clone(),
                    swing.clone(),
                    self.min_segment_bars,
                    self.min_segment_abs_change,
                    EwaRuleSettings {
                        max_candidate_pivots: self.max_candidate_pivots,
                        compose_corrections_on_cpu: self.cpu_composition_fallback,
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    pub fn with_rule_variants(
        mut self,
        ratio_error_scales: &[f64],
        hard_violation_penalties: &[f64],
    ) -> Self {
        if self.swing_passes.is_empty() {
            return self;
        }

        let mut passes = Vec::new();
        let pattern_sets = [
            all_patterns(),
            motive_and_corrective_patterns(),
            harmonic_patterns(),
        ];

        for swing in &self.swing_passes {
            for &ratio_error_scale in ratio_error_scales {
                for &hard_violation_penalty in hard_violation_penalties {
                    for enabled_patterns in &pattern_sets {
                        let rule_settings = EwaRuleSettings {
                            max_candidate_pivots: self.max_candidate_pivots,
                            ratio_error_scale,
                            hard_violation_penalty,
                            compose_corrections_on_cpu: self.cpu_composition_fallback,
                            enabled_patterns: enabled_patterns.clone(),
                        };
                        let label = format!(
                            "{}|ratio_{ratio_error_scale:.2}|viol_{hard_violation_penalty:.2}|rules_{}",
                            swing.label,
                            rule_family_label(enabled_patterns)
                        );
                        passes.push(EwaAlgorithmPassConfig::new(
                            label,
                            swing.clone(),
                            self.min_segment_bars,
                            self.min_segment_abs_change,
                            rule_settings,
                        ));
                    }
                }
            }
        }

        self.algorithm_passes = passes;
        self
    }
}

fn all_patterns() -> Vec<EwaPatternKind> {
    EwaRuleSettings::default().enabled_patterns
}

fn motive_and_corrective_patterns() -> Vec<EwaPatternKind> {
    vec![
        EwaPatternKind::Impulse,
        EwaPatternKind::ImpulseExtendedWave1,
        EwaPatternKind::ImpulseExtendedWave3,
        EwaPatternKind::ImpulseExtendedWave5,
        EwaPatternKind::TruncatedImpulse,
        EwaPatternKind::LeadingDiagonalContracting,
        EwaPatternKind::LeadingDiagonalExpanding,
        EwaPatternKind::EndingDiagonalContracting,
        EwaPatternKind::EndingDiagonalExpanding,
        EwaPatternKind::Correction,
        EwaPatternKind::Zigzag,
        EwaPatternKind::RunningZigzag,
        EwaPatternKind::DoubleZigzag,
        EwaPatternKind::TripleZigzag,
        EwaPatternKind::Flat,
        EwaPatternKind::RegularFlat,
        EwaPatternKind::ExpandedFlat,
        EwaPatternKind::RunningFlat,
        EwaPatternKind::Triangle,
        EwaPatternKind::ContractingTriangle,
        EwaPatternKind::BarrierTriangle,
        EwaPatternKind::ExpandingTriangle,
        EwaPatternKind::RunningTriangle,
        EwaPatternKind::DoubleThree,
        EwaPatternKind::TripleThree,
    ]
}

fn harmonic_patterns() -> Vec<EwaPatternKind> {
    vec![
        EwaPatternKind::Xabcd,
        EwaPatternKind::Abcd,
        EwaPatternKind::Cypher,
        EwaPatternKind::Gartley,
        EwaPatternKind::Bat,
        EwaPatternKind::Butterfly,
        EwaPatternKind::Crab,
        EwaPatternKind::Shark,
        EwaPatternKind::ThreeDrives,
    ]
}

fn rule_family_label(patterns: &[EwaPatternKind]) -> &'static str {
    let has_impulse = patterns.contains(&EwaPatternKind::Impulse);
    let has_gartley = patterns.contains(&EwaPatternKind::Gartley);

    match (has_impulse, has_gartley) {
        (true, true) => "all",
        (true, false) => "ewa",
        (false, true) => "harmonic",
        (false, false) => "custom",
    }
}
