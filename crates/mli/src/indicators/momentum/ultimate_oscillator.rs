//! Ultimate Oscillator indicator.

use std::collections::VecDeque;
use crate::indicators::utils::true_range::true_range;

/// Ultimate Oscillator - multi-timeframe momentum oscillator by Larry Williams.
///
/// UO = 100 × [(4×Average7) + (2×Average14) + Average28] / (4 + 2 + 1)
/// where Average = Sum(BP) / Sum(TR) over period
/// BP (Buying Pressure) = Close - Min(Low, Previous Close)
/// TR (True Range) = Max(High, Previous Close) - Min(Low, Previous Close)
///
/// Uses three periods (typically 7, 14, 28) to reduce false signals.
#[derive(Debug, Clone)]
pub struct UltimateOscillator {
    period1: usize,
    period2: usize,
    period3: usize,

    bp_values: VecDeque<f64>,
    tr_values: VecDeque<f64>,

    sum_bp_short: f64,
    sum_tr_short: f64,
    sum_bp_medium: f64,
    sum_tr_medium: f64,
    sum_bp_long: f64,
    sum_tr_long: f64,

    prev_close: f64,
    value: f64,
    bars_count: usize,
    is_ready: bool,
}

impl UltimateOscillator {
    pub fn new() -> Self {
        Self::with_periods(7, 14, 28)
    }

    /// Build with explicit raw periods. Caller guarantees `period1 < period2 < period3`.
    /// Prefer [`UltimateOscillator::with_multipliers`] from config-driven paths.
    pub fn with_periods(period1: usize, period2: usize, period3: usize) -> Self {
        assert!(period1 > 0, "Period1 must be > 0");
        assert!(period1 < period2, "Period1 must be less than Period2");
        assert!(period2 < period3, "Period2 must be less than Period3");

        Self {
            period1,
            period2,
            period3,
            bp_values: VecDeque::with_capacity(period3),
            tr_values: VecDeque::with_capacity(period3),
            sum_bp_short: 0.0,
            sum_tr_short: 0.0,
            sum_bp_medium: 0.0,
            sum_tr_medium: 0.0,
            sum_bp_long: 0.0,
            sum_tr_long: 0.0,
            prev_close: 0.0,
            value: 50.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Build from a base period plus two relative multipliers.
    ///
    /// `period2 = (period1 * mult2).round().max(period1 + 1)`
    /// `period3 = (period2 * mult3).round().max(period2 + 1)`
    ///
    /// The `.max(prev + 1)` floor guarantees strict `period1 < period2 < period3`
    /// by construction for any `mult2 ≥ 1` and `mult3 ≥ 1`, even when rounding on
    /// small periods would otherwise collapse the ordering.
    pub fn with_multipliers(period1: usize, mult2: f64, mult3: f64) -> Self {
        assert!(period1 > 0, "period1 must be > 0");
        let period2 = ((period1 as f64 * mult2).round() as usize).max(period1 + 1);
        let period3 = ((period2 as f64 * mult3).round() as usize).max(period2 + 1);
        Self::with_periods(period1, period2, period3)
    }

    /// Feed resolved `[high, low, close]` lanes — contract input (SOURCE = KlineSlice[H, L, C]).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        if self.bars_count == 1 {
            self.prev_close = close;
            return self.value;
        }

        let min_low_prev_close = low.min(self.prev_close);
        let buying_pressure = close - min_low_prev_close;
        let tr = true_range(high, low, self.prev_close);

        if self.bp_values.len() >= self.period3 {
            let old_bp = self.bp_values.pop_front().unwrap_or(0.0);
            let old_tr = self.tr_values.pop_front().unwrap_or(0.0);
            self.sum_bp_long -= old_bp;
            self.sum_tr_long -= old_tr;
            self.sum_bp_medium -= old_bp;
            self.sum_tr_medium -= old_tr;
            self.sum_bp_short -= old_bp;
            self.sum_tr_short -= old_tr;
        }

        self.bp_values.push_back(buying_pressure);
        self.tr_values.push_back(tr);
        self.sum_bp_long += buying_pressure;
        self.sum_tr_long += tr;
        self.sum_bp_medium += buying_pressure;
        self.sum_tr_medium += tr;
        self.sum_bp_short += buying_pressure;
        self.sum_tr_short += tr;

        let len = self.bp_values.len();
        if len > self.period1 {
            let idx = len - self.period1 - 1;
            self.sum_bp_short -= self.bp_values[idx];
            self.sum_tr_short -= self.tr_values[idx];
        }
        if len > self.period2 {
            let idx = len - self.period2 - 1;
            self.sum_bp_medium -= self.bp_values[idx];
            self.sum_tr_medium -= self.tr_values[idx];
        }

        if self.bp_values.len() >= self.period3 {
            self.is_ready = true;
        }

        if self.is_ready {
            let avg1 = if self.sum_tr_short.abs() < 1e-12 { 0.0 } else { self.sum_bp_short / self.sum_tr_short };
            let avg2 = if self.sum_tr_medium.abs() < 1e-12 { 0.0 } else { self.sum_bp_medium / self.sum_tr_medium };
            let avg3 = if self.sum_tr_long.abs() < 1e-12 { 0.0 } else { self.sum_bp_long / self.sum_tr_long };
            self.value = (100.0 * ((4.0 * avg1) + (2.0 * avg2) + avg3) / 7.0).clamp(0.0, 100.0);
        }

        self.prev_close = close;
        self.value
    }


    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn periods(&self) -> (usize, usize, usize) {
        (self.period1, self.period2, self.period3)
    }

    pub fn reset(&mut self) {
        self.bp_values.clear();
        self.tr_values.clear();
        self.sum_bp_short = 0.0;
        self.sum_tr_short = 0.0;
        self.sum_bp_medium = 0.0;
        self.sum_tr_medium = 0.0;
        self.sum_bp_long = 0.0;
        self.sum_tr_long = 0.0;
        self.prev_close = 0.0;
        self.value = 50.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    pub fn market_condition(&self) -> &'static str {
        match self.value {
            v if v >= 70.0 => "Overbought",
            v if v <= 30.0 => "Oversold",
            v if v >= 50.0 => "Bullish",
            _ => "Bearish",
        }
    }

    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        match self.value {
            v if v <= 30.0 => 1,
            v if v >= 70.0 => -1,
            _ => 0,
        }
    }

    pub fn advanced_signal(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 {
            return 0;
        }
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let base_signal = self.trading_signal();
        if base_signal == 1 {
            if current_price < past_price && self.value > 30.0 {
                return 1;
            }
        } else if base_signal == -1 {
            if current_price > past_price && self.value < 70.0 {
                return -1;
            }
        }
        base_signal
    }

    pub fn signal_strength(&self) -> f64 {
        if self.value >= 70.0 {
            (self.value - 70.0) / 30.0
        } else if self.value <= 30.0 {
            (30.0 - self.value) / 30.0
        } else {
            0.0
        }
    }

    pub fn level_crossover(&self, prev_value: f64) -> i8 {
        if prev_value <= 30.0 && self.value > 30.0 {
            return 1;
        }
        if prev_value >= 70.0 && self.value < 70.0 {
            return -1;
        }
        0
    }

    pub fn components(&self) -> (f64, f64, f64) {
        if !self.is_ready() {
            return (0.0, 0.0, 0.0);
        }
        let avg1 = if self.sum_tr_short.abs() < 1e-12 { 0.0 } else { 100.0 * self.sum_bp_short / self.sum_tr_short };
        let avg2 = if self.sum_tr_medium.abs() < 1e-12 { 0.0 } else { 100.0 * self.sum_bp_medium / self.sum_tr_medium };
        let avg3 = if self.sum_tr_long.abs() < 1e-12 { 0.0 } else { 100.0 * self.sum_bp_long / self.sum_tr_long };
        (avg1, avg2, avg3)
    }

    pub fn info(&self) -> String {
        let (avg1, avg2, avg3) = self.components();
        format!(
            "UO: {:.2}, Periods: ({},{},{}), Avg1: {:.2}, Avg2: {:.2}, Avg3: {:.2}",
            self.value, self.period1, self.period2, self.period3, avg1, avg2, avg3
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uo_basic_calculation() {
        let mut uo = UltimateOscillator::new();
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            uo.feed(&[base + 2.0, base - 1.0, base + 1.0]);
        }
        assert!(uo.is_ready());
        assert!(uo.value() >= 0.0 && uo.value() <= 100.0);
    }

    #[test]
    fn test_uo_overbought() {
        let mut uo = UltimateOscillator::new();
        for i in 1..=50 {
            let base = 100.0 + i as f64 * 2.0;
            uo.feed(&[base + 3.0, base - 0.5, base + 3.0]);
        }
        assert!(uo.is_ready());
        assert!(uo.value() > 50.0, "UO in strong uptrend should be > 50");
    }

    #[test]
    fn test_uo_oversold() {
        let mut uo = UltimateOscillator::new();
        for i in 1..=50 {
            let base = 200.0 - i as f64 * 2.0;
            uo.feed(&[base + 0.5, base - 3.0, base - 3.0]);
        }
        assert!(uo.is_ready());
        assert!(uo.value() < 50.0, "UO in strong downtrend should be < 50");
    }

    #[test]
    fn test_uo_range() {
        let mut uo = UltimateOscillator::new();
        for i in 1..=50 {
            let base = 100.0 + (i % 10) as f64;
            uo.feed(&[base + 2.0, base - 2.0, base]);
        }
        assert!(uo.is_ready());
        assert!(uo.value() >= 0.0 && uo.value() <= 100.0);
    }

    #[test]
    fn test_uo_reset() {
        let mut uo = UltimateOscillator::new();
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            uo.feed(&[base + 2.0, base - 1.0, base + 1.0]);
        }
        assert!(uo.is_ready());
        uo.reset();
        assert!(!uo.is_ready());
        assert!((uo.value() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_uo_periods() {
        let uo = UltimateOscillator::new();
        assert_eq!(uo.periods(), (7, 14, 28));
    }

    #[test]
    fn test_uo_market_condition() {
        let mut uo = UltimateOscillator::new();
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            uo.feed(&[base + 2.0, base - 1.0, base + 1.0]);
        }
        assert!(uo.is_ready());
        let condition = uo.market_condition();
        assert!(
            condition == "Overbought"
                || condition == "Oversold"
                || condition == "Bullish"
                || condition == "Bearish"
        );
    }
}

impl Default for UltimateOscillator {
    fn default() -> Self {
        Self::with_periods(7, 14, 28)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for UltimateOscillator.
///
/// The three periods are given directly as absolute period values.
/// `valid_params` enforces `period1 < period2 < period3`.
///
/// Defaults reproduce the standard UO 7 / 14 / 28.
///
/// Dual-mode: every field is a `Param`. No smoother slots (no `#[slot]` fields).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct UltimateOscillatorConfig {
    /// Short (fast) period (default 7).
    pub period1: Param<usize>,
    /// Medium period (default 14). Must satisfy `period1 < period2`.
    pub period2: Param<usize>,
    /// Long (slow) period (default 28). Must satisfy `period2 < period3`.
    pub period3: Param<usize>,
}

impl Indicator for UltimateOscillator {
    const ID: IndicatorId = IndicatorId::Uo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads High, Low, Close — fixed fields (prev_close maintained internally).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Uo)];
    /// O(1): period3-bounded deques with running sums.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Deque), // bp_values
            Store::window(StoreKind::Deque), // tr_values
        ],
    );

    type Config = UltimateOscillatorConfig;
    type Runtime = UltimateOscillator;

    fn create(cfg: UltimateOscillatorConfig) -> UltimateOscillator {
        UltimateOscillator::with_periods(
            cfg.period1.resolved(),
            cfg.period2.resolved(),
            cfg.period3.resolved(),
        )
    }
}

impl crate::contract::Config for UltimateOscillatorConfig {
    fn defaults() -> Self {
        // Standard UO: 7 / 14 / 28.
        UltimateOscillatorConfig {
            period1: Param::Solo(7),
            period2: Param::Solo(14),
            period3: Param::Solo(28),
        }
    }
    fn valid_params(&self) -> Result<(), String> {
        let p1 = self.period1.resolved();
        let p2 = self.period2.resolved();
        let p3 = self.period3.resolved();
        if p1 >= p2 {
            return Err(format!("period1({p1}) >= period2({p2}): must have period1 < period2 < period3"));
        }
        if p2 >= p3 {
            return Err(format!("period2({p2}) >= period3({p3}): must have period1 < period2 < period3"));
        }
        Ok(())
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period1/period2/period3: Class A → auto range(2,10000,1) EACH, but all three then
        // resolve to the same min (2), failing this config's OWN `valid_params`
        // (period1 < period2 < period3) at the min corner (2026-07-03 fix). Split into three
        // disjoint ranges so `resolved()` (each axis's min) stays strictly ordered.
        let mut s = Self::machine_defaults_auto();
        s.period1 = Param::range(1, 50, 1);
        s.period2 = Param::range(51, 200, 1);
        s.period3 = Param::range(201, 10000, 1);
        s
    }
}


impl Render for UltimateOscillator {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::Uo,
                "Ultimate Osc",
                Color::hex(0x3F51B5),
                1.5,
            ))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Uo(<<UltimateOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 2.0,
                low: base - 1.0,
                close: base + 1.0,
                volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 100.0, "UO must be in [0,100], got {v}");
    }
}
