//! Detection thresholds for every `smc` detector.
//!
//! One struct threads through every function in this module rather than one
//! struct per detector family: several thresholds are read by more than one
//! detector (`swing_n` feeds both `structure::swings` and
//! `structure::structure`; `atr_period`/`displacement_atr_mult` feed both
//! `zones::displacement_legs` and, through it, `zones::order_blocks` and
//! `amd::amd_cycles`), and a threshold declared once per consumer is a
//! threshold that can drift between consumers. Every field's default
//! reproduces exactly the value Appendix A of
//! `docs/mlc/plans/markup-engines-2026-08-25.md` states.

/// Which candle range an order block's zone is measured over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderBlockZone {
    /// `[low, high]` of the candle — Appendix A's primary definition.
    FullRange,
    /// `[min(open, close), max(open, close)]` — the configurable
    /// alternative Appendix A names as "also in use", not universal.
    BodyOnly,
}

/// Every configurable threshold Appendix A names for the `smc` detectors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmcConfig {
    /// Fractal half-width for `structure::swings`: a swing high/low must
    /// exceed `swing_n` bars on both sides. Appendix A default: 2.
    pub swing_n: usize,
    /// ATR lookback backing every ATR-scaled threshold below. Appendix A
    /// default: 14.
    pub atr_period: usize,
    /// A displacement leg's total range must exceed this multiple of
    /// `ATR(atr_period)` measured at the leg's start bar. Appendix A
    /// default: 2.0 — "a fixed price threshold cannot survive a change of
    /// instrument."
    pub displacement_atr_mult: f64,
    /// Two swing extremes of the same kind are "equal" when within this
    /// multiple of `ATR(atr_period)` of each other. Appendix A default:
    /// 0.1.
    pub equal_level_atr_tol: f64,
    /// A liquidity sweep's close must recover back inside the swept level
    /// within this many bars of the wick bar (0 = the wick bar itself).
    /// Appendix A default: 1.
    pub sweep_recovery_bars: usize,
    /// Order block zone convention. Appendix A states the full-range
    /// definition first and names body-only as the configurable
    /// alternative, so full-range is the default.
    pub order_block_zone: OrderBlockZone,
    /// How many bars after a manipulation sweep an AMD cycle will still
    /// accept a displacement leg as that cycle's distribution phase.
    ///
    /// Appendix A names no number here because it describes AMD as an
    /// assembly rather than a detector — but an assembly with no bound
    /// will happily marry a sweep to a leg hundreds of bars later and call
    /// the pair one cycle. The premise of the pattern is that distribution
    /// is the REACTION to the raid: it comes promptly or it is not the
    /// reaction. Default 10 bars, on top of the harder structural bound
    /// `amd::amd_cycles` applies anyway — a leg may never be claimed by a
    /// sweep older than the most recent one preceding it.
    pub amd_max_distribution_lag: usize,
}

impl Default for SmcConfig {
    fn default() -> Self {
        Self {
            swing_n: 2,
            atr_period: 14,
            displacement_atr_mult: 2.0,
            equal_level_atr_tol: 0.1,
            sweep_recovery_bars: 1,
            order_block_zone: OrderBlockZone::FullRange,
            amd_max_distribution_lag: 10,
        }
    }
}
