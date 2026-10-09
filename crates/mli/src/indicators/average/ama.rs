//! Kaufman Adaptive Moving Average (AMA) indicator.

use crate::indicators::ratio::efficiency_ratio_ring::EfficiencyRatioRingWindow;
use crate::engine::ohlcv_field::OhlcvField;

/// Kaufman Adaptive Moving Average (AMA) - adapts smoothing based on market efficiency.
///
/// AMA = AMA_prev + SC² × (Price - AMA_prev)
///
/// where SC = ER × (fast_α - slow_α) + slow_α
/// and ER = Efficiency Ratio (direction/volatility)
///
/// Created by Perry Kaufman. Speeds up in trending markets,
/// slows down in choppy markets.
///
/// # Implementation
///
/// Uses Efficiency Ratio with rolling window. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Ama {
    period_efficiency_ratio: usize,
    alpha_fast: f64,
    alpha_slow: f64,
    value: f64,
    count: usize,
    prior_value: Option<f64>,
    efficiency_ratio: EfficiencyRatioRingWindow,
    initialized: bool,
}

impl Ama {
    /// Returns the period of this AMA (efficiency ratio period).
    pub fn period(&self) -> usize {
        self.period_efficiency_ratio
    }

    /// Creates a new AMA with the specified parameters.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period_efficiency_ratio` - Period for efficiency ratio calculation
    /// * `fast_period` - Fast EMA period (default: 2)
    /// * `slow_period` - Slow EMA period (default: 30)
    pub fn new(period_efficiency_ratio: usize, fast_period: usize, slow_period: usize) -> Self {
        let alpha_fast = 2.0 / (fast_period as f64 + 1.0);
        let alpha_slow = 2.0 / (slow_period as f64 + 1.0);
        Self {
            period_efficiency_ratio,
            alpha_fast,
            alpha_slow,
            value: 0.0,
            count: 0,
            prior_value: None,
            efficiency_ratio: EfficiencyRatioRingWindow::new(period_efficiency_ratio),
            initialized: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The factory
    /// (or host composite) resolves the source field and supplies the value.
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.count == 0 {
            self.value = value;
            self.prior_value = Some(value);
            self.efficiency_ratio.update_raw(value);
            self.count += 1;
            return self.value;
        }
        self.efficiency_ratio.update_raw(value);
        let er = self.efficiency_ratio.value();
        let smoothing_constant = (er * (self.alpha_fast - self.alpha_slow) + self.alpha_slow).powi(2);
        let prior = self.prior_value.unwrap_or(self.value);
        self.value = prior + smoothing_constant * (value - prior);
        self.prior_value = Some(self.value);
        self.count += 1;
        if self.efficiency_ratio.is_initialized() {
            self.initialized = true;
        }
        self.value
    }

    /// Returns the current AMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the AMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the AMA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
        self.prior_value = None;
        self.efficiency_ratio.reset();
        self.initialized = false;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Ama`] (Kaufman Adaptive MA): the efficiency-ratio `period`
/// plus the `fast`/`slow` EMA periods that bound the adaptive smoothing.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AmaConfig {
    pub period: Param<usize>,
    pub fast: Param<usize>,
    pub slow: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for AmaConfig {
    fn defaults() -> Self {
        AmaConfig {
            period: Param::Solo(14),
            fast: Param::Solo(2),
            slow: Param::Solo(30),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast.resolved();
        let slow = self.slow.resolved();
        if fast >= slow {
            return Err(format!("fast({fast}) >= slow({slow})"));
        }
        Ok(())
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period/fast/slow: Class A → auto range(1,10000,1) EACH — but `fast` and `slow` then
        // both resolve to the same min (1), failing this config's OWN `valid_params`
        // (fast < slow) at the min corner (2026-07-03 fix). Split fast/slow into disjoint
        // ranges so `resolved()` (each axis's first/min value) stays ordered.
        // source: Class O → auto all-8.
        // A1 2026-07-04: floor from valid_params (runtime assert `period > 1` in
        // `EfficiencyRatioRingWindow::new`, which `period` feeds via `Ama::new`).
        let mut s = Self::machine_defaults_auto();
        s.period = Param::range(2, 10000, 1);
        s.fast = Param::range(1, 100, 1);
        s.slow = Param::range(101, 10000, 1);
        s
    }
}

impl Indicator for Ama {
    const ID: IndicatorId = IndicatorId::Ama;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// The lean Kaufman impl: O(1) update over a single efficiency-ratio ring
    /// window (the heavy `adaptive/KaufmanAdaptiveMA` twin carries ~5 buffers, so
    /// the barometer weighs this one well below it).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Ama)];
    type Config = AmaConfig;
    type Runtime = Ama;

    fn create(cfg: AmaConfig) -> Ama {
        Ama::new(cfg.period.resolved(), cfg.fast.resolved(), cfg.slow.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the
    /// resolved scalar — the core ingests via `feed(f64)`, knowing no OHLCV fields.
    fn source_fields(cfg: &AmaConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Ama {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Ama, "AMA", Color::hex(0xFFC107))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ama_basic_calculation() {
        let mut ama = Ama::new(10, 2, 30);

        // Feed trending data - AMA should follow closely
        for i in 1..=20 {
            ama.feed(i as f64 * 10.0);
        }

        assert!(ama.is_ready());
        // In a perfect trend, AMA should be close to recent prices
        assert!(ama.value() > 100.0);
    }

    #[test]
    fn test_ama_adapts_to_choppy_market() {
        let mut ama = Ama::new(10, 2, 30);

        // Feed choppy data - AMA should be slow
        for i in 0..30 {
            let price = if i % 2 == 0 { 100.0 } else { 110.0 };
            ama.feed(price);
        }

        assert!(ama.is_ready());
        // In choppy market, AMA should be somewhere in the middle
        let v = ama.value();
        assert!(v > 100.0 && v < 110.0);
    }

    #[test]
    fn test_ama_reset() {
        let mut ama = Ama::new(10, 2, 30);
        for i in 1..=15 {
            ama.feed(i as f64 * 10.0);
        }
        assert!(ama.is_ready());

        ama.reset();
        assert!(!ama.is_ready());
    }
}
