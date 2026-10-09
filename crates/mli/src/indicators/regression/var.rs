//! Rolling historical Value-at-Risk (VaR) — single-asset, close-based.
//!
//! Computes the **historical VaR** over a rolling window of log-returns:
//!
//! 1. Each bar: log-return r_t = ln(close_t / close_{t-1}).
//! 2. Maintain a `VecDeque<f64>` of the last `window` returns (evict oldest).
//! 3. VaR = negative of the `(1 - confidence)` quantile of the return
//!    distribution, i.e. the `floor((1-confidence) * n)`-th sorted return.
//!    For `confidence=0.95` and `window=100` this is the 5th-percentile loss,
//!    expressed as a positive magnitude.
//!
//! Returns 0.0 until the window is full (at least `window` returns, i.e.
//! `window + 1` closes seen).

use std::collections::VecDeque;


/// Rolling historical Value-at-Risk (VaR) on a single close series.
#[derive(Debug, Clone)]
pub struct Var {
    window: usize,
    confidence: f64,
    returns: VecDeque<f64>,
    prev_close: Option<f64>,
    current_var: f64,
}

impl Var {
    /// Create a new [`Var`] with the given rolling `window` and `confidence` level.
    ///
    /// - `window` — number of log-returns to keep (e.g. 100).
    /// - `confidence` — VaR confidence level, e.g. 0.95 → 5th-percentile loss.
    pub fn new(window: usize, confidence: f64) -> Self {
        let window = window.max(2);
        let confidence = confidence.clamp(0.5, 0.9999);
        Self {
            window,
            confidence,
            returns: VecDeque::with_capacity(window + 1),
            prev_close: None,
            current_var: 0.0,
        }
    }

    /// Feed one close price and return the current rolling historical VaR.
    ///
    /// The returned value is a **positive** loss magnitude (e.g. 0.023 = 2.3%).
    /// Returns 0.0 until `window` log-returns have been accumulated.
    pub fn feed(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.prev_close {
            if prev > 0.0 && close > 0.0 {
                let r = (close / prev).ln();
                self.returns.push_back(r);
                if self.returns.len() > self.window {
                    self.returns.pop_front();
                }
            }
        }
        self.prev_close = Some(close);
        self.current_var = self.compute_var();
        self.current_var
    }

    /// Compute historical VaR from the current return window.
    fn compute_var(&self) -> f64 {
        let n = self.returns.len();
        if n < self.window {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.returns.iter().copied().collect();
        sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Index of the (1 - confidence) quantile (loss tail).
        let idx = ((1.0 - self.confidence) * n as f64).floor() as usize;
        let idx = idx.min(n.saturating_sub(1));
        // VaR = negative of the quantile (a negative return → positive loss).
        -sorted[idx]
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.returns.len() >= self.window
    }

    pub fn reset(&mut self) {
        self.returns.clear();
        self.prev_close = None;
        self.current_var = 0.0;
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.current_var
    }
}

impl Default for Var {
    /// Standard rolling historical VaR: 100-bar window, 95% confidence.
    fn default() -> Self {
        Self::new(100, 0.95)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store, StoreKind,
    UpdateComplexity, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`Var`]: rolling window + confidence level + source field.
///
/// Every field is a [`Param`] — `Solo` = one value, `Many` = a swept set.
/// `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VarConfig {
    /// Number of log-returns in the rolling window (default: 100).
    pub window: Param<usize>,
    /// VaR confidence level, e.g. 0.95 → 5th-percentile loss (default: 0.95).
    pub confidence: Param<f64>,
    /// OHLCV field to use as the price series (default: Close).
    pub source: Param<OhlcvField>,
}

impl Indicator for Var {
    const ID: IndicatorId = IndicatorId::Var;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Var)];
    /// O(n log n) per bar due to the sort of the return window.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);

    type Config = VarConfig;
    type Runtime = Var;

    fn create(cfg: VarConfig) -> Var {
        Var::new(cfg.window.resolved(), cfg.confidence.resolved())
    }

    fn source_fields(cfg: &VarConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for VarConfig {
    fn defaults() -> Self {
        VarConfig {
            window: Param::Solo(100),
            confidence: Param::Solo(0.95),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: A period — auto range(2,4048,1) is correct.
        // confidence: D — ratio/fraction in [0,1] (VaR confidence level).
        // source: OhlcvField — auto sets all 8 variants.
        s.confidence = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Var {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Var, "VaR (95%)", Color::hex(0xE91E63))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_ready_before_window_full() {
        let mut v = Var::new(10, 0.95);
        for i in 0..10 {
            v.feed(100.0 + i as f64);
        }
        // 10 feeds → 9 returns, window=10 → not ready yet
        assert!(!v.is_ready());
    }

    #[test]
    fn ready_after_window_plus_one() {
        let mut v = Var::new(10, 0.95);
        for i in 0..12 {
            v.feed(100.0 + i as f64);
        }
        assert!(v.is_ready());
    }

    #[test]
    fn var_is_positive_for_volatile_series() {
        let mut v = Var::new(20, 0.95);
        let prices = [
            100.0, 98.0, 102.0, 95.0, 105.0, 97.0, 103.0, 90.0, 110.0, 96.0,
            101.0, 93.0, 107.0, 99.0, 104.0, 88.0, 112.0, 95.0, 100.0, 97.0,
            102.0,
        ];
        let mut last = 0.0;
        for &p in &prices {
            last = v.feed(p);
        }
        assert!(v.is_ready());
        assert!(last >= 0.0, "VaR must be non-negative, got {last}");
    }

    #[test]
    fn var_zero_before_window_full() {
        let mut v = Var::new(50, 0.95);
        for i in 0..10 {
            let r = v.feed(100.0 + i as f64);
            assert_eq!(r, 0.0, "should return 0 before window full");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut v = Var::new(10, 0.95);
        for i in 0..12 {
            v.feed(100.0 + i as f64);
        }
        assert!(v.is_ready());
        v.reset();
        assert!(!v.is_ready());
        assert_eq!(v.feed(100.0), 0.0);
    }

    #[test]
    fn higher_confidence_gives_larger_var() {
        let prices: Vec<f64> = (0..105).map(|i| 100.0 + (i as f64 * 0.37).sin() * 3.0).collect();
        let mut v95 = Var::new(100, 0.95);
        let mut v99 = Var::new(100, 0.99);
        for &p in &prices {
            v95.feed(p);
            v99.feed(p);
        }
        assert!(v99.current_var >= v95.current_var,
            "99% VaR ({}) should be >= 95% VaR ({})", v99.current_var, v95.current_var);
    }

    #[test]
    fn factory_feeds_resolved_var() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Var(<<Var as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..110 {
            price *= if price > 100.0 { 0.995 } else { 1.005 };
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: price, volume: 1.0 });
        }
        // After 110 bars the window (100 returns) is full; VaR should be finite and non-negative.
        let val = f.primary();
        assert!(val >= 0.0, "VaR must be non-negative, got {val}");
        assert!(val.is_finite(), "VaR must be finite, got {val}");
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<Var as Indicator>::ID, IndicatorId::Var);
    }
}
