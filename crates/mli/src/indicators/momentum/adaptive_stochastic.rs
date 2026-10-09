//! Adaptive Stochastic — ATR-driven period adaptation over classic Stochastic.
//!
//! High ATR → shorter period (more responsive); low ATR → longer period (smoother).
//! Reuses the contracted `Atr` core for volatility measurement.

use crate::engine::contract_engine::SmootherSlot;
use crate::indicators::volatility::atr::Atr;

/// Full result tuple returned by [`AdaptiveStochastic::feed`].
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveStochasticResult {
    pub k_percent: f64,
    pub d_percent: f64,
    pub adaptive_period: f64,
    pub volatility_factor: f64,
    pub momentum_strength: f64,
    pub overbought_level: f64,
    pub oversold_level: f64,
    pub signal_strength: f64,
    pub trend_bias: i8,
}

impl AdaptiveStochasticResult {
    pub fn empty() -> Self {
        Self {
            k_percent: 50.0,
            d_percent: 50.0,
            adaptive_period: 14.0,
            volatility_factor: 1.0,
            momentum_strength: 0.5,
            overbought_level: 80.0,
            oversold_level: 20.0,
            signal_strength: 0.0,
            trend_bias: 0,
        }
    }

    pub fn stochastic_state(&self) -> &'static str {
        if self.k_percent >= self.overbought_level {
            "Overbought"
        } else if self.k_percent <= self.oversold_level {
            "Oversold"
        } else if self.k_percent > 50.0 {
            "Above Midline"
        } else {
            "Below Midline"
        }
    }
}

/// Adaptive Stochastic indicator.
///
/// The `d_ma` slot controls %D smoothing and is the only configurable MA.
/// Internal `volatility_ma` (EMA 10), `momentum_ma` (EMA 5) and the ATR
/// smoother (Wilder/RMA) are hardcoded per the canonical algorithm.
#[derive(Debug, Clone)]
pub struct AdaptiveStochastic {
    /// Contracted ATR: provides volatility signal.
    atr: Atr,
    /// ATR smoothing (hardcoded EMA 10).
    volatility_ma: SmootherSlot,
    /// Configurable %D smoothing MA.
    d_ma: SmootherSlot,
    /// Momentum analysis MA (hardcoded EMA 5).
    momentum_ma: SmootherSlot,

    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    k_values: Vec<f64>,
    periods: Vec<f64>,

    base_period: usize,
    min_period: usize,
    max_period: usize,
    volatility_sensitivity: f64,

    current_period: f64,
    current_result: AdaptiveStochasticResult,
    is_ready: bool,
    update_count: usize,
}

impl AdaptiveStochastic {
    /// Default parameters: base=14, band ×0.5..×2.0 (→ min 7, max 28), ATR(14), %D SMA(3).
    pub fn new() -> Self {
        Self::from_smoother(14, 0.5, 2.0, 14, 1.5, SmootherId::Sma, 3)
    }

    /// Build with configurable %D MA slot.
    ///
    /// The adaptation band is given as MULTIPLES of `base_period` (`min_mult ≤ 1 ≤
    /// max_mult`) and derived internally: `min = base·min_mult`, `max = base·max_mult`.
    /// Expressing the band relatively makes `min ≤ base ≤ max` hold by construction for
    /// any base — there are no free absolute min/max periods to put out of order.
    pub fn from_smoother(
        base_period: usize,
        min_mult: f64,
        max_mult: f64,
        atr_period: usize,
        volatility_sensitivity: f64,
        d_id: SmootherId,
        d_period: usize,
    ) -> Self {
        assert!(base_period > 0, "base_period must be > 0");
        assert!(atr_period > 0, "atr_period must be > 0");
        assert!(volatility_sensitivity > 0.0, "volatility_sensitivity must be > 0");
        // Adaptation guardrails as multiples of base — `min ≤ base ≤ max` by construction.
        let min_period = ((base_period as f64 * min_mult).round() as usize).clamp(1, base_period);
        let max_period = ((base_period as f64 * max_mult).round() as usize).max(base_period);
        Self {
            atr: Atr::new_wilder(atr_period),
            volatility_ma: SmootherSlot::new(SmootherId::Ema, 10),
            d_ma: SmootherSlot::new(d_id, d_period.max(1)),
            momentum_ma: SmootherSlot::new(SmootherId::Ema, 5),
            highs: Vec::with_capacity(64),
            lows: Vec::with_capacity(64),
            closes: Vec::with_capacity(64),
            k_values: Vec::with_capacity(32),
            periods: Vec::with_capacity(16),
            base_period,
            min_period,
            max_period,
            volatility_sensitivity,
            current_period: base_period as f64,
            current_result: AdaptiveStochasticResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Build with explicit min/max periods (natural parameterization).
    ///
    /// Identical to [`from_smoother`] except `min_period` and `max_period` are taken
    /// directly — the `base_period × mult` derivation is skipped. The runtime stores
    /// `base_period = min_period` (the adaptation floor is treated as the nominal base
    /// for `adapt_period`'s `base_period` scaling).
    pub fn from_periods(
        min_period: usize,
        max_period: usize,
        atr_period: usize,
        volatility_sensitivity: f64,
        d_id: SmootherId,
        d_period: usize,
    ) -> Self {
        assert!(min_period > 0, "min_period must be > 0");
        assert!(max_period > min_period, "max_period must be > min_period");
        assert!(atr_period > 0, "atr_period must be > 0");
        assert!(volatility_sensitivity > 0.0, "volatility_sensitivity must be > 0");
        Self {
            atr: Atr::new_wilder(atr_period),
            volatility_ma: SmootherSlot::new(SmootherId::Ema, 10),
            d_ma: SmootherSlot::new(d_id, d_period.max(1)),
            momentum_ma: SmootherSlot::new(SmootherId::Ema, 5),
            highs: Vec::with_capacity(64),
            lows: Vec::with_capacity(64),
            closes: Vec::with_capacity(64),
            k_values: Vec::with_capacity(32),
            periods: Vec::with_capacity(16),
            base_period: min_period,
            min_period,
            max_period,
            volatility_sensitivity,
            current_period: min_period as f64,
            current_result: AdaptiveStochasticResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). ATR + the
    /// adaptive %K window need H/L/C; volume is unused.
    pub fn feed(&mut self, lanes: &[f64]) -> AdaptiveStochasticResult {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        if self.highs.len() >= 64 {
            self.highs.remove(0);
        }
        self.highs.push(high);
        if self.lows.len() >= 64 {
            self.lows.remove(0);
        }
        self.lows.push(low);
        if self.closes.len() >= 64 {
            self.closes.remove(0);
        }
        self.closes.push(close);

        let atr_value = self.atr.feed(&[high, low, close]);
        self.adapt_period(atr_value, close);

        let k_percent = self.calculate_adaptive_k(high, low, close);
        let d_percent = self.d_ma.feed(k_percent);

        self.analyze_momentum(k_percent, d_percent);
        self.adapt_levels();

        self.current_result.k_percent = k_percent;
        self.current_result.d_percent = d_percent;
        self.current_result.adaptive_period = self.current_period;

        if self.atr.is_ready() && self.highs.len() >= self.base_period {
            self.is_ready = true;
        }

        self.update_count += 1;
        self.current_result
    }

    fn adapt_period(&mut self, atr_value: f64, current_price: f64) {
        if self.update_count < 15 {
            return;
        }
        let smoothed = self.volatility_ma.feed(atr_value);
        let normalized = if current_price > 0.0 {
            smoothed / current_price
        } else {
            0.01
        };
        let vf = (normalized * 100.0 * self.volatility_sensitivity).max(0.1);
        self.current_result.volatility_factor = vf;
        let adj = 1.0 / (1.0 + vf);
        self.current_period = (self.base_period as f64 * adj)
            .max(self.min_period as f64)
            .min(self.max_period as f64);
        if self.periods.len() >= 16 {
            self.periods.remove(0);
        }
        self.periods.push(self.current_period);
    }

    fn calculate_adaptive_k(&mut self, cur_high: f64, cur_low: f64, cur_close: f64) -> f64 {
        let period = (self.current_period as usize).max(2);
        let avail = self.highs.len().min(period);
        if avail < 2 {
            return 50.0;
        }
        let start = self.highs.len() - avail;
        let hh = self.highs[start..].iter().fold(cur_high, |a, &b| a.max(b));
        let ll = self.lows[start..].iter().fold(cur_low, |a, &b| a.min(b));
        let k = if (hh - ll).abs() > 1e-12 {
            ((cur_close - ll) / (hh - ll)) * 100.0
        } else {
            50.0
        };
        if self.k_values.len() >= 32 {
            self.k_values.remove(0);
        }
        self.k_values.push(k);
        k
    }

    fn analyze_momentum(&mut self, k: f64, d: f64) {
        if self.k_values.len() < 3 {
            return;
        }
        let len = self.k_values.len();
        let cur = self.k_values[len - 1];
        let p1 = self.k_values[len - 2];
        let p2 = self.k_values[len - 3];

        let mom = cur - p1;
        let sm = self.momentum_ma.feed(mom);
        self.current_result.momentum_strength = (sm.abs() / 10.0).min(1.0);

        let trend = cur - p2;
        self.current_result.trend_bias = if trend > 2.0 { 1 } else if trend < -2.0 { -1 } else { 0 };

        // Signal strength
        let extremity = if k <= 20.0 || k >= 80.0 { 1.0 } else if k <= 30.0 || k >= 70.0 { 0.7 } else { 0.3 };
        let div = ((k - d).abs() / 20.0).min(1.0);
        let period_stability = if self.periods.len() >= 3 {
            let recent = &self.periods[self.periods.len() - 3..];
            let var = recent.iter().map(|&p| (p - self.current_period).abs()).sum::<f64>()
                / recent.len() as f64;
            (1.0 - (var / self.current_period).min(1.0)).max(0.0)
        } else {
            0.5
        };
        self.current_result.signal_strength =
            (extremity * 0.3 + div * 0.2 + self.current_result.momentum_strength * 0.3 + period_stability * 0.2)
                .min(1.0);
    }

    fn adapt_levels(&mut self) {
        let vf = self.current_result.volatility_factor;
        if vf > 2.0 {
            self.current_result.overbought_level = 85.0;
            self.current_result.oversold_level = 15.0;
        } else if vf > 1.5 {
            self.current_result.overbought_level = 82.0;
            self.current_result.oversold_level = 18.0;
        } else if vf < 0.5 {
            self.current_result.overbought_level = 75.0;
            self.current_result.oversold_level = 25.0;
        } else {
            self.current_result.overbought_level = 80.0;
            self.current_result.oversold_level = 20.0;
        }
    }


    pub fn result(&self) -> AdaptiveStochasticResult {
        self.current_result
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.volatility_ma.reset();
        self.d_ma.reset();
        self.momentum_ma.reset();
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.k_values.clear();
        self.periods.clear();
        self.current_period = self.base_period as f64;
        self.current_result = AdaptiveStochasticResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    pub fn period(&self) -> usize {
        self.current_period as usize
    }

    pub fn parameters(&self) -> (usize, usize, usize, f64) {
        (self.base_period, self.min_period, self.max_period, self.volatility_sensitivity)
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Brace-named getter: `k` output (adaptive %K).
    #[inline]
    pub fn k(&self) -> f64 {
        self.current_result.k_percent
    }

    /// Brace-named getter: `d` output (adaptive %D).
    #[inline]
    pub fn d(&self) -> f64 {
        self.current_result.d_percent
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Dual-mode contract config for [`AdaptiveStochastic`].
///
/// All scalar fields are `Param<T>`. The %D smoother is a `Param<SmootherChoice>` slot
/// (default: follow(Sma)); `d_period` is the dedicated period field it follows.
///
/// `valid_params` enforces `min_period < max_period`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AdaptiveStochConfig {
    /// Absolute lower adaptation guardrail (default 7). Must be < max_period.
    pub min_period: Param<usize>,
    /// Absolute upper adaptation guardrail (default 28). Must be > min_period.
    pub max_period: Param<usize>,
    pub atr_period: Param<usize>,
    pub volatility_sensitivity: Param<f64>,
    /// %D smoothing period (the host period the slot follows by default).
    pub d_period: Param<usize>,
    /// Configurable %D smoother choice. Default: follow(Sma).
    #[slot]
    pub d_smoother: Param<SmootherChoice>,
}

impl Indicator for AdaptiveStochastic {
    const ID: IndicatorId = IndicatorId::AdaptiveStoch;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed multi-field consumer: needs H/L/C for ATR and the %K window (volume unused).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    /// O(1) own update; ATR cost charged via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = AdaptiveStochConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::AdaptiveStochK),
        Output::percent(IndicatorOutputId::AdaptiveStochD),
    ];
    type Config = AdaptiveStochConfig;
    type Runtime = AdaptiveStochastic;

    fn create(cfg: AdaptiveStochConfig) -> AdaptiveStochastic {
        let d_period = cfg.d_period.resolved();
        let choice = cfg.d_smoother.resolved();
        AdaptiveStochastic::from_periods(
            cfg.min_period.resolved(),
            cfg.max_period.resolved(),
            cfg.atr_period.resolved(),
            cfg.volatility_sensitivity.resolved(),
            choice.kind,
            choice.period.resolve(d_period),  // Follow → d_period; Own(p) → p
        )
    }

    fn slot_members(cfg: &AdaptiveStochConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AdaptiveStochConfig {
    fn defaults() -> Self {
        // Old defaults: base_period=14, min_mult=0.5, max_mult=2.0.
        // Derived absolutes:
        //   min_period = (14*0.5).round()=7, clamp(1,14)=7
        //   max_period = (14*2.0).round()=28, max(14)=28
        AdaptiveStochConfig {
            min_period: Param::Solo(7),
            max_period: Param::Solo(28),
            atr_period: Param::Solo(14),
            volatility_sensitivity: Param::Solo(1.5),
            d_period: Param::Solo(3),
            d_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn valid_params(&self) -> Result<(), String> {
        let min = self.min_period.resolved();
        let max = self.max_period.resolved();
        if min >= max {
            return Err(format!("min_period({min}) >= max_period({max})"));
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
        // min_period/max_period/atr_period/d_period: Class A → auto range(2,4048,1) EACH, but
        // min_period and max_period then both resolve to the same min (2), failing this
        // config's OWN `valid_params` (min < max) at the min corner (2026-07-03 fix). Split
        // min/max into disjoint ranges so `resolved()` stays ordered; atr_period/d_period
        // (independent lanes) keep the full auto range.
        // volatility_sensitivity: Class C (multiplier, scales ATR sensitivity) — sweep_f64(0.1,10.0,0.1).
        // d_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.min_period = Param::range(1, 100, 1);
        s.max_period = Param::range(101, 10000, 1);
        s.volatility_sensitivity = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
}


impl Render for AdaptiveStochastic {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AdaptiveStochK, "%K", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::AdaptiveStochD, "%D", Color::hex(0xFF9800))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(20.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

impl Default for AdaptiveStochastic {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_stoch_creation() {
        let s = AdaptiveStochastic::new();
        assert!(!s.is_ready());
        // min_period=7, max_period=28 (derived from old defaults base=14, mult 0.5..2.0)
        assert_eq!(s.parameters().1, 7);
        assert_eq!(s.parameters().2, 28);
    }

    #[test]
    fn test_adaptive_stoch_basic() {
        let mut s = AdaptiveStochastic::new();
        for i in 0..25 {
            let p = 100.0 + i as f64 * 0.5;
            let r = s.feed(&[p + 1.0, p - 1.0, p]);
            assert!(r.k_percent.is_finite());
            assert!(r.d_percent.is_finite());
        }
        assert!(s.is_ready());
    }

    #[test]
    fn test_adaptive_stoch_range() {
        let mut s = AdaptiveStochastic::new();
        for i in 0..20 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let r = s.feed(&[p + 1.0, p - 1.0, p]);
            assert!(r.k_percent >= 0.0 && r.k_percent <= 100.0);
            assert!(r.d_percent >= 0.0 && r.d_percent <= 100.0);
        }
    }

    #[test]
    fn test_adaptive_stoch_reset() {
        let mut s = AdaptiveStochastic::new();
        for i in 0..25 {
            let p = 100.0 + i as f64;
            s.feed(&[p + 1.0, p - 1.0, p]);
        }
        assert!(s.is_ready());
        s.reset();
        assert!(!s.is_ready());
        assert!((s.k() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn factory_feeds_resolved_adaptive_stoch() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<AdaptiveStochastic as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::AdaptiveStoch(cfg).build_solo().unwrap();
        for i in 0..25 {
            let p = 100.0 + i as f64 * 0.5;
            f.feed(0, MarketSample::Bar {
                open: p,
                high: p + 1.0,
                low: p - 1.0,
                close: p,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }

    #[test]
    fn machine_defaults_wide_usize_solo_f64() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::engine::indicator_id::IndicatorId;

        // usize axes (min_period, max_period, atr_period, d_period) must be widened to Many.
        let mc = AdaptiveStochConfig::machine_defaults_auto();
        assert!(mc.min_period.cardinality() > 1, "min_period should be Many (usize sweep)");
        assert!(mc.max_period.cardinality() > 1, "max_period should be Many (usize sweep)");
        assert!(mc.atr_period.cardinality() > 1, "atr_period should be Many (usize sweep)");
        assert!(mc.d_period.cardinality() > 1, "d_period should be Many (usize sweep)");

        // f64 axis (volatility_sensitivity) must stay Solo in auto defaults.
        assert_eq!(mc.volatility_sensitivity.cardinality(), 1, "volatility_sensitivity is f64 — should stay Solo");

        // slot (d_smoother) now SWEEPS its family members (SmootherChoice::machine_sweep).
        assert!(mc.d_smoother.cardinality() > 1, "d_smoother is a slot — should sweep its members");

        // from_machine_defaults produces a wide order.
        let order = IndicatorOrder::from_machine_defaults(IndicatorId::AdaptiveStoch)
            .expect("AdaptiveStoch is a contract member");
        assert!(order.cube_size() > 1, "cube_size should be > 1 with multiple Many usize axes");
    }
}
