// Logic Gates for combining indicator signals
//
// Self-contained versions: use two internal RSIs with different periods
// - RSI(7) for short-term momentum
// - RSI(21) for medium-term momentum
// AND: Both overbought/oversold
// OR: Either overbought/oversold
// XOR: Divergence between short and medium term
// SignCombiner: Sum of signal directions

use crate::indicators::momentum::rsi::Rsi;

/// AND Gate: Returns true when both RSIs agree on overbought (>70) or oversold (<30)
#[derive(Debug, Clone)]
pub struct AndGate {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub v: bool,
}
impl Default for AndGate {
    fn default() -> Self {
        Self::new()
    }
}

impl AndGate {
    /// Default: rsi_short_period=7, rsi_long_period=21.
    pub fn new() -> Self {
        Self::with_periods(7, 21)
    }

    /// Configurable RSI periods. `short_period` < `long_period` recommended.
    pub fn with_periods(short_period: usize, long_period: usize) -> Self {
        Self {
            rsi_short: Rsi::new(short_period),
            rsi_long: Rsi::new(long_period),
            v: false,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). Both internal
    /// RSIs consume the scalar; the factory extracts the source field.
    pub fn feed(&mut self, value: f64) -> bool {
        self.rsi_short.feed(value);
        self.rsi_long.feed(value);

        if self.is_ready() {
            let short_val = self.rsi_short.value();
            let long_val = self.rsi_long.value();
            // Both overbought OR both oversold
            self.v = (short_val > 70.0 && long_val > 70.0) || (short_val < 30.0 && long_val < 30.0);
        }
        self.v
    }

    pub fn update(&mut self, a: bool, b: bool) -> bool {
        self.v = a && b;
        self.v
    }

    /// Returns the current gate state.
    pub fn value(&self) -> f64 {
        if self.v { 1.0 } else { 0.0 }
    }

    /// Returns the raw boolean gate state.
    pub fn bool_value(&self) -> bool {
        self.v
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.v = false;
    }
}

/// OR Gate: Returns true when either RSI is overbought (>70) or oversold (<30)
#[derive(Debug, Clone)]
pub struct OrGate {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub v: bool,
}
impl Default for OrGate {
    fn default() -> Self {
        Self::new()
    }
}

impl OrGate {
    /// Default: rsi_short_period=7, rsi_long_period=21.
    pub fn new() -> Self {
        Self::with_periods(7, 21)
    }

    /// Configurable RSI periods. `short_period` < `long_period` recommended.
    pub fn with_periods(short_period: usize, long_period: usize) -> Self {
        Self {
            rsi_short: Rsi::new(short_period),
            rsi_long: Rsi::new(long_period),
            v: false,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). Both internal
    /// RSIs consume the scalar; the factory extracts the source field.
    pub fn feed(&mut self, value: f64) -> bool {
        self.rsi_short.feed(value);
        self.rsi_long.feed(value);

        if self.is_ready() {
            let short_val = self.rsi_short.value();
            let long_val = self.rsi_long.value();
            // Either is extreme
            self.v = !(30.0..=70.0).contains(&short_val) || !(30.0..=70.0).contains(&long_val);
        }
        self.v
    }

    pub fn update(&mut self, a: bool, b: bool) -> bool {
        self.v = a || b;
        self.v
    }

    /// Returns the current gate state.
    pub fn value(&self) -> f64 {
        if self.v { 1.0 } else { 0.0 }
    }

    /// Returns the raw boolean gate state.
    pub fn bool_value(&self) -> bool {
        self.v
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.v = false;
    }
}

/// XOR Gate: Returns true when RSIs disagree (divergence)
/// True when one is overbought and other is not, or one is oversold and other is not
#[derive(Debug, Clone)]
pub struct XorGate {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub v: bool,
}
impl Default for XorGate {
    fn default() -> Self {
        Self::new()
    }
}

impl XorGate {
    /// Default: rsi_short_period=7, rsi_long_period=21.
    pub fn new() -> Self {
        Self::with_periods(7, 21)
    }

    /// Configurable RSI periods. `short_period` < `long_period` recommended.
    pub fn with_periods(short_period: usize, long_period: usize) -> Self {
        Self {
            rsi_short: Rsi::new(short_period),
            rsi_long: Rsi::new(long_period),
            v: false,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). Both internal
    /// RSIs consume the scalar; the factory extracts the source field.
    pub fn feed(&mut self, value: f64) -> bool {
        self.rsi_short.feed(value);
        self.rsi_long.feed(value);

        if self.is_ready() {
            let short_val = self.rsi_short.value();
            let long_val = self.rsi_long.value();
            let short_extreme = !(30.0..=70.0).contains(&short_val);
            let long_extreme = !(30.0..=70.0).contains(&long_val);
            // XOR: exactly one is extreme (divergence)
            self.v = short_extreme ^ long_extreme;
        }
        self.v
    }

    pub fn update(&mut self, a: bool, b: bool) -> bool {
        self.v = a ^ b;
        self.v
    }

    /// Returns the current gate state.
    pub fn value(&self) -> f64 {
        if self.v { 1.0 } else { 0.0 }
    }

    /// Returns the raw boolean gate state.
    pub fn bool_value(&self) -> bool {
        self.v
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.v = false;
    }
}

/// SignCombiner: Combines RSI signals into {-1, 0, 1}
/// +1 if both bullish (RSI < 30), -1 if both bearish (RSI > 70), 0 otherwise
#[derive(Debug, Clone)]
pub struct SignCombiner {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub s: i8,
}
impl Default for SignCombiner {
    fn default() -> Self {
        Self::new()
    }
}

impl SignCombiner {
    /// Default: rsi_short_period=7, rsi_long_period=21.
    pub fn new() -> Self {
        Self::with_periods(7, 21)
    }

    /// Configurable RSI periods. `short_period` < `long_period` recommended.
    pub fn with_periods(short_period: usize, long_period: usize) -> Self {
        Self {
            rsi_short: Rsi::new(short_period),
            rsi_long: Rsi::new(long_period),
            s: 0,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). Both internal
    /// RSIs consume the scalar; the factory extracts the source field.
    pub fn feed(&mut self, value: f64) -> i8 {
        self.rsi_short.feed(value);
        self.rsi_long.feed(value);

        if self.is_ready() {
            let short_val = self.rsi_short.value();
            let long_val = self.rsi_long.value();

            // Convert to signals: +1 bullish (oversold), -1 bearish (overbought), 0 neutral
            let short_sig: i8 = if short_val < 30.0 { 1 } else if short_val > 70.0 { -1 } else { 0 };
            let long_sig: i8 = if long_val < 30.0 { 1 } else if long_val > 70.0 { -1 } else { 0 };

            // Combine signals
            self.s = (short_sig + long_sig).clamp(-1, 1);
        }
        self.s
    }

    pub fn update(&mut self, a: i8, b: i8) -> i8 {
        self.s = a.saturating_add(b).clamp(-1, 1);
        self.s
    }

    pub fn value(&self) -> f64 {
        (self.s) as f64
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.s = 0;
    }
}

// ─── Contracts ───────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`AndGate`], [`OrGate`], [`XorGate`], and [`SignCombiner`].
/// All four have identical configuration: two RSI periods.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LogicGateConfig {
    /// Short-term RSI period (default 7).
    pub short_period: Param<usize>,
    /// Long-term RSI period (default 21).
    pub long_period: Param<usize>,
}

// ── AndGate ──────────────────────────────────────────────────────────────────

impl Indicator for AndGate {
    const ID: IndicatorId = IndicatorId::Logicand;
    /// Signal combiner — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Outer is O(1) boolean AND; two embedded RSIs charged recursively.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Logicand)];

    type Config = LogicGateConfig;
    type Runtime = AndGate;

    fn create(cfg: LogicGateConfig) -> AndGate {
        AndGate::with_periods(cfg.short_period.resolved(), cfg.long_period.resolved())
    }
}

impl crate::contract::Config for LogicGateConfig {
    fn valid_params(&self) -> Result<(), String> {
        let short = self.short_period.resolved();
        let long = self.long_period.resolved();
        if short >= long {
            return Err(format!("short_period({short}) >= long_period({long})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        LogicGateConfig {
            short_period: Param::Solo(7),
            long_period: Param::Solo(21),
        }
    }
    fn machine_defaults() -> Self {
        // short_period / long_period: Class A periods — auto range(2,4048,1) EACH, but both
        // then resolve to the same min (2), failing this config's OWN `valid_params`
        // (short < long) at the min corner (2026-07-03 fix, shared by AndGate/OrGate/XorGate/
        // SignCombiner). Split into disjoint ranges so `resolved()` stays ordered.
        let mut s = Self::machine_defaults_auto();
        s.short_period = Param::range(1, 100, 1);
        s.long_period = Param::range(101, 10000, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AndGate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Logicand, "AND Gate", Color::hex(0x4CAF50))
            .bounds(0.0, 1.0)
            .precision(0)
            .build()
    }
}

// ── OrGate ───────────────────────────────────────────────────────────────────

impl Indicator for OrGate {
    const ID: IndicatorId = IndicatorId::Logicor;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Logicor)];

    type Config = LogicGateConfig;
    type Runtime = OrGate;

    fn create(cfg: LogicGateConfig) -> OrGate {
        OrGate::with_periods(cfg.short_period.resolved(), cfg.long_period.resolved())
    }
}


impl Render for OrGate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Logicor, "OR Gate", Color::hex(0x2196F3))
            .bounds(0.0, 1.0)
            .precision(0)
            .build()
    }
}

// ── XorGate ──────────────────────────────────────────────────────────────────

impl Indicator for XorGate {
    const ID: IndicatorId = IndicatorId::Logicxor;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Logicxor)];

    type Config = LogicGateConfig;
    type Runtime = XorGate;

    fn create(cfg: LogicGateConfig) -> XorGate {
        XorGate::with_periods(cfg.short_period.resolved(), cfg.long_period.resolved())
    }
}


impl Render for XorGate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Logicxor, "XOR Gate", Color::hex(0xFF9800))
            .bounds(0.0, 1.0)
            .precision(0)
            .build()
    }
}

// ── SignCombiner ──────────────────────────────────────────────────────────────

impl Indicator for SignCombiner {
    const ID: IndicatorId = IndicatorId::Logicsign;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
            Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::Logicsign)];

    type Config = LogicGateConfig;
    type Runtime = SignCombiner;

    fn create(cfg: LogicGateConfig) -> SignCombiner {
        SignCombiner::with_periods(cfg.short_period.resolved(), cfg.long_period.resolved())
    }
}


impl Render for SignCombiner {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Logicsign, "Sign Combiner", Color::hex(0x9C27B0))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logic_gates_with_periods() {
        // Verify richer ctor builds, warms up, and produces finite output for all 4 structs
        let mut and = AndGate::with_periods(5, 14);
        let mut or = OrGate::with_periods(5, 14);
        let mut xor = XorGate::with_periods(5, 14);
        let mut sc = SignCombiner::with_periods(5, 14);
        let mut price = 100.0;
        for _ in 0..30 {
            price += 1.5;
            and.feed(price);
            or.feed(price);
            xor.feed(price);
            sc.feed(price);
        }
        assert!(and.is_ready());
        assert!(or.is_ready());
        assert!(xor.is_ready());
        assert!(sc.is_ready());
        assert!((sc.value() as i8).abs() <= 1);
    }

    #[test]
    fn test_and_gate_legacy() {
        let mut gate = AndGate::new();
        // Legacy update method still works
        assert!(!gate.update(true, false));
        assert!(!gate.update(false, true));
        assert!(gate.update(true, true));
        assert!(!gate.update(false, false));

        gate.reset();
        assert!(!gate.bool_value());
    }

    #[test]
    fn test_and_gate_with_data() {
        let mut gate = AndGate::new();
        assert!(!gate.is_ready()); // Needs warmup

        // Feed price data to warm up internal RSIs
        let mut price = 100.0;
        for _ in 0..30 {
            price += 0.5;
            gate.feed(price);
        }
        assert!(gate.is_ready());
    }

    #[test]
    fn test_or_gate_legacy() {
        let mut gate = OrGate::new();
        assert!(gate.update(true, false));
        assert!(gate.update(false, true));
        assert!(gate.update(true, true));
        assert!(!gate.update(false, false));

        gate.reset();
        assert!(!gate.bool_value());
    }

    #[test]
    fn test_or_gate_with_data() {
        let mut gate = OrGate::new();

        let mut price = 100.0;
        for _ in 0..30 {
            price += 0.5;
            gate.feed(price);
        }
        assert!(gate.is_ready());
    }

    #[test]
    fn test_xor_gate_legacy() {
        let mut gate = XorGate::new();
        assert!(gate.update(true, false));
        assert!(gate.update(false, true));
        assert!(!gate.update(true, true));
        assert!(!gate.update(false, false));

        gate.reset();
        assert!(!gate.bool_value());
    }

    #[test]
    fn test_xor_gate_with_divergence() {
        // XOR needs short RSI in extreme but long RSI neutral
        // Key: warmup with oscillating prices to get RSI near 50, then spike

        let mut rsi_short = Rsi::new(7);
        let mut rsi_long = Rsi::new(21);

        // Warmup with oscillating prices (up/down) to get RSI near 50
        let mut price = 100.0;
        for i in 0..30 {
            // Alternate up/down to balance gains and losses
            if i % 2 == 0 {
                price += 1.0;
            } else {
                price -= 1.0;
            }
            rsi_short.feed(price);
            rsi_long.feed(price);
        }

        // Both RSIs should be near 50 now
        eprintln!("After warmup: short={:.1}, long={:.1}",
            rsi_short.value(), rsi_long.value());

        // Sharp upward spike - 7 consecutive gains
        let mut any_divergence = false;
        for i in 0..10 {
            price += 3.0; // Strong up move
            rsi_short.feed(price);
            rsi_long.feed(price);

            let short_val = rsi_short.value();
            let long_val = rsi_long.value();
            let short_extreme = short_val > 70.0 || short_val < 30.0;
            let long_extreme = long_val > 70.0 || long_val < 30.0;

            eprintln!("Bar {}: short={:.1}, long={:.1}, xor={}",
                i, short_val, long_val, short_extreme ^ long_extreme);

            if short_extreme ^ long_extreme {
                any_divergence = true;
            }
        }

        // XOR should fire when short RSI hits extreme before long RSI does
        assert!(any_divergence, "XOR should detect divergence during sharp spike");
    }

    #[test]
    fn test_sign_combiner_legacy() {
        let mut sc = SignCombiner::new();
        // Legacy update method still works
        assert_eq!(sc.update(1, 0), 1);
        assert_eq!(sc.update(-1, 0), -1);
        assert_eq!(sc.update(1, 1), 1);  // clamped to 1
        assert_eq!(sc.update(-1, -1), -1);  // clamped to -1
        assert_eq!(sc.update(1, -1), 0);

        sc.reset();
        assert_eq!(sc.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_logicand() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<AndGate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Logicand(cfg).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..30 {
            price += 1.5;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5,
                high: price + 0.5,
                low: price - 0.5,
                close: price,
                volume: 1000.0,
            });
        }
        // After warmup value should be a Flag
        assert!(f.primary().is_finite());
    }

    #[test]
    fn factory_feeds_resolved_logicor() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<OrGate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Logicor(cfg).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..30 {
            price += 1.5;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5,
                high: price + 0.5,
                low: price - 0.5,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }

    #[test]
    fn factory_feeds_resolved_logicxor() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<XorGate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Logicxor(cfg).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..30 {
            price += 1.5;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5,
                high: price + 0.5,
                low: price - 0.5,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }

    #[test]
    fn factory_feeds_resolved_logicsign() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SignCombiner as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Logicsign(cfg).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..30 {
            price += 1.5;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5,
                high: price + 0.5,
                low: price - 0.5,
                close: price,
                volume: 1000.0,
            });
        }
        let sig = f.primary() as i8;
        assert!(sig >= -1 && sig <= 1);
    }
}
