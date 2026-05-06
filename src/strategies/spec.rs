//! Strategy specification — semantic types describing a complete strategy.
//!
//! Pure semantics: `RoleSpec`, `StateSpec`, `StateVar`, `Transition`,
//! `ActionMap`, `ValidationRuleSpec`, `TfArity`, `StrategySpec`.
//!
//! Codegen-specific extensions (e.g. `extra_decls: Vec<TokenStream>` for
//! injecting custom enum declarations into generated Rust files) live in
//! `mlq-strategies-codegen::strategy::CodegenStrategySpec`, which wraps
//! `StrategySpec` from here.

use crate::BarIndicatorId;
use crate::types::ResearchTimeframe;
use crate::strategies::events::operator::OperatorClass;
use crate::strategies::shapes::role_kind::RoleKind;
use crate::strategies::composition::spec::CompositionSpec;
use crate::catalog::indicator_key::OutputSelector;
use crate::bar_indicators::average::MovingAverageType;

// =============================================================================
// RoleSpec — one indicator role in the strategy
// =============================================================================

/// One indicator slot in a strategy.
///
/// Each role maps to a single index in `IndicatorSlices<N>` and contributes
/// one column of pre-computed indicator values to the hot loop.
#[derive(Debug, Clone)]
pub struct RoleSpec {
    /// Human-readable role name (e.g. `"fast_ma"`, `"slow_ma"`, `"rsi"`).
    pub name: &'static str,
    /// Semantic kind of indicator — used for operator-class validation.
    pub kind: RoleKind,
    /// Allowed indicator types for this role.
    pub indicators: Vec<BarIndicatorId>,
    /// Which output of a multi-output indicator to use.
    pub output: OutputSelector,
    /// Timeframe for this role.
    /// `None` — same timeframe as the strategy. `Some(tf)` — multi-TF role.
    pub tf: Option<ResearchTimeframe>,
    /// Optional integer period range `(min, max, step)`.
    pub period_range: Option<(usize, usize, usize)>,
    /// Optional MA type list.
    pub ma_types: Option<Vec<MovingAverageType>>,
}

impl RoleSpec {
    pub fn simple(name: &'static str, kind: RoleKind, indicator: BarIndicatorId) -> Self {
        Self {
            name,
            kind,
            indicators: vec![indicator],
            output: OutputSelector::Main,
            tf: None,
            period_range: None,
            ma_types: None,
        }
    }

    pub fn with_period(
        name: &'static str,
        kind: RoleKind,
        indicators: Vec<BarIndicatorId>,
        period_range: (usize, usize, usize),
    ) -> Self {
        Self {
            name,
            kind,
            indicators,
            output: OutputSelector::Main,
            tf: None,
            period_range: Some(period_range),
            ma_types: None,
        }
    }
}

// =============================================================================
// StateVar — typed per-combo mutable state variables
// =============================================================================

/// A single named state variable in the generated strategy struct.
///
/// Each variant maps to a Rust field type and a default/reset value.
#[derive(Debug, Clone)]
pub enum StateVar {
    /// Boolean flag field. Default: `false`.
    BoolFlag { name: &'static str },
    /// `f64` field initialised to `f64::NAN` (NaN-sentinel pattern).
    F64NanSentinel { name: &'static str },
    /// `u32` counter field. Default: `0`.
    Counter { name: &'static str },
    /// `mlq_core::strategy::Signal` field. Default: `Signal::None`.
    SignalState { name: &'static str },
    /// `i32` integer state (kept for simple regime flags).
    I32State {
        name: &'static str,
        default_value: i32,
    },
    /// Custom enum state field (enum type itself injected via codegen extra_decls).
    EnumState {
        name: &'static str,
        type_name: &'static str,
        default_variant: &'static str,
    },
}

impl StateVar {
    pub fn field_name(&self) -> &'static str {
        match self {
            Self::BoolFlag { name }
            | Self::F64NanSentinel { name }
            | Self::Counter { name }
            | Self::SignalState { name }
            | Self::I32State { name, .. }
            | Self::EnumState { name, .. } => name,
        }
    }
}

/// State machine transitions: when a condition fires, set a state var.
#[derive(Debug, Clone)]
pub struct Transition {
    pub when: CompositionSpec,
    pub var_idx: usize,
    pub value: i32,
}

/// State machine spec for a strategy.
#[derive(Debug, Clone, Default)]
pub struct StateSpec {
    pub vars: Vec<StateVar>,
    pub transitions: Vec<Transition>,
}

// =============================================================================
// ValidationRuleSpec
// =============================================================================

/// Validation rule to emit in `requirements()`.
///
/// Mirrors `mlq_core::strategies::ValidationRule` variants.
#[derive(Debug, Clone, Copy)]
pub enum ValidationRuleSpec {
    RolePeriodLess { role_a: usize, role_b: usize },
    RolePeriodGreater { role_a: usize, role_b: usize },
    RolePeriodNotEqual { role_a: usize, role_b: usize },
}

// =============================================================================
// ActionMap
// =============================================================================

/// Maps signal conditions to trade actions.
#[derive(Debug, Clone)]
pub struct ActionMap {
    /// Condition tree for entering a long / closing a short.
    pub buy_when: CompositionSpec,
    /// Condition tree for entering a short / closing a long.
    pub sell_when: CompositionSpec,
    /// Optional condition tree for unconditionally closing any position.
    pub force_close_when: Option<CompositionSpec>,
}

// =============================================================================
// TfArity
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TfArity {
    SingleTf,
    MultiTf,
}

impl TfArity {
    pub fn from_roles(roles: &[RoleSpec]) -> Self {
        let first = roles.first().map(|r| r.tf);
        let all_same = roles.iter().all(|r| r.tf == first.unwrap_or(None));
        if all_same { TfArity::SingleTf } else { TfArity::MultiTf }
    }
}

// =============================================================================
// StrategySpec — top-level (pure semantics)
// =============================================================================

/// Complete semantic specification of a strategy.
///
/// `extra_decls` is codegen-specific (TokenStream's of custom enum
/// declarations injected into generated Rust files). Always present here;
/// non-codegen consumers (chart/alerts) just leave it empty.
#[derive(Debug, Clone)]
pub struct StrategySpec {
    /// Snake-case strategy name (e.g. `"dual_ma_cross"`).
    pub name: String,
    /// Ordered list of indicator roles. Index = `role_idx` in operands.
    pub roles: Vec<RoleSpec>,
    /// State machine specification.
    pub state: StateSpec,
    /// Signal action mapping.
    pub actions: ActionMap,
    /// Single-TF or multi-TF arity (derived from `roles`).
    pub tf_arity: TfArity,
    /// Validation rules emitted into `requirements()`.
    pub validation_rules: Vec<ValidationRuleSpec>,
    /// Codegen-only: extra top-level declarations prepended to the generated
    /// `.rs` file (typically custom enum definitions referenced by
    /// `StateVar::EnumState`). Empty for non-codegen consumers.
    pub extra_decls: Vec<proc_macro2::TokenStream>,
}

impl StrategySpec {
    pub fn new(
        name: impl Into<String>,
        roles: Vec<RoleSpec>,
        state: StateSpec,
        actions: ActionMap,
    ) -> Self {
        let tf_arity = TfArity::from_roles(&roles);
        Self {
            name: name.into(),
            roles,
            state,
            actions,
            tf_arity,
            validation_rules: Vec::new(),
            extra_decls: Vec::new(),
        }
    }

    pub fn with_validation(
        name: impl Into<String>,
        roles: Vec<RoleSpec>,
        state: StateSpec,
        actions: ActionMap,
        validation_rules: Vec<ValidationRuleSpec>,
    ) -> Self {
        let tf_arity = TfArity::from_roles(&roles);
        Self {
            name: name.into(),
            roles,
            state,
            actions,
            tf_arity,
            validation_rules,
            extra_decls: Vec::new(),
        }
    }

    /// Number of roles — matches the const generic `N` in `StStaticLite<N>`.
    pub fn n(&self) -> usize {
        self.roles.len()
    }
}

// `OperatorClass` re-export for convenience (used in tests/specs).
#[doc(hidden)]
pub type _OperatorClassRef = OperatorClass;
