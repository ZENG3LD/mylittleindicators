//! Operand axis — what value is on each side of a comparison.

/// OHLCV bar field selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarField {
    Open,
    High,
    Low,
    Close,
    Volume,
    /// (High + Low) / 2
    HlMid,
    /// (High + Low + Close) / 3
    Hlc3,
    /// (Open + High + Low + Close) / 4
    Ohlc4,
}

/// Aggregation operation over a bar field window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregateOp {
    Highest,
    Lowest,
    Mean,
    Sum,
}

/// Derived operation on a role's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DerivedOp {
    /// Previous bar value (lag 1).
    Prev,
    /// Slope (current - prev).
    Slope,
    /// Percentage change.
    PctChange,
    /// Z-score over the last `n` bars (mean & std-dev).
    ZScore { n: usize },
}

/// Arithmetic binary operation for `Operand::Arithmetic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticOp {
    /// left + right
    Add,
    /// left - right
    Sub,
    /// left * right
    Mul,
    /// left / right. Codegen MUST emit a zero-guard: `if right.abs() < f64::EPSILON
    /// { return false; }` before the division. Hot loops never panic-divide.
    Div,
}

/// One side of a comparison or event condition.
///
/// Used for both `left_operand` and `right_operand` on an `Event`.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Value from an indicator role at the current bar.
    IndicatorValue {
        /// Index into `StrategySpec::roles` (slot_idx = role_idx).
        role_idx: usize,
    },
    /// Raw OHLCV bar field.
    BarField(BarField),
    /// Aggregate of a bar field over the last `n` bars.
    Aggregate {
        field: BarField,
        op: AggregateOp,
        n: usize,
    },
    /// Look back `n` bars into a role's value.
    Lookback {
        /// Index into `StrategySpec::roles`.
        role_idx: usize,
        n: usize,
    },
    /// Derived (transformed) value from a role.
    Derived {
        /// Index into `StrategySpec::roles`.
        role_idx: usize,
        op: DerivedOp,
    },
    /// Literal constant.
    Constant(f64),
    /// The literal value zero (shorthand for `Constant(0.0)`).
    Zero,
    /// Arithmetic combination of two sub-operands: `left op right`.
    ///
    /// Enables expressions like `MA[i] + ATR[i] * multiplier`.
    Arithmetic {
        op: ArithmeticOp,
        left: Box<Operand>,
        right: Box<Operand>,
    },
}

impl Operand {
    /// Returns true if this operand references no runtime data (pure constant).
    pub fn is_constant(&self) -> bool {
        matches!(self, Operand::Constant(_) | Operand::Zero)
    }

    /// Returns true if this operand references an indicator role output.
    pub fn is_indicator(&self) -> bool {
        matches!(
            self,
            Operand::IndicatorValue { .. } | Operand::Lookback { .. } | Operand::Derived { .. }
        )
    }

    /// Returns true if this operand is a bar field or aggregate thereof.
    pub fn is_bar_field(&self) -> bool {
        matches!(self, Operand::BarField(_) | Operand::Aggregate { .. })
    }

    /// Convenience: `left + right`.
    pub fn add(left: Operand, right: Operand) -> Self {
        Self::Arithmetic { op: ArithmeticOp::Add, left: Box::new(left), right: Box::new(right) }
    }

    /// Convenience: `left * right`.
    pub fn mul(left: Operand, right: Operand) -> Self {
        Self::Arithmetic { op: ArithmeticOp::Mul, left: Box::new(left), right: Box::new(right) }
    }
}
