//! Stochastic Oscillator (%K, %D) indicator.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Stochastic Oscillator - momentum indicator comparing close to high-low range.
///
/// %K = 100 × (Close - Lowest Low) / (Highest High - Lowest Low)
/// %D = MA(%K)
///
/// Oscillates between 0 and 100. Traditional interpretation:
/// - Above 80: Overbought
/// - Below 20: Oversold
/// - %K crossing %D: Trading signal
///
/// # Parameters
/// - `period_k`: Lookback for %K calculation (typically 14)
/// - `period_d`: Smoothing period for %D (typically 3)
/// - `ma_type`: Moving average type for %D smoothing
///
/// # Implementation
///
/// Uses ring buffer for high/low tracking. O(period_k) per update for min/max scan.
/// Maximum period is 512.
#[derive(Clone)]
pub struct Stochastics {
    period_k: usize,
    period_d: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    d_ma: SmootherSlot,
    index: usize,
    filled: bool,
    value_k: f64,
    value_d: f64,
}

impl Stochastics {
    /// Creates a new Stochastic Oscillator with SMA smoothing for %D.
    ///
    /// # Arguments
    /// * `period_k` - Lookback period for %K (1..=512)
    /// * `period_d` - Smoothing period for %D (1..=512)
    /// Default ctor — %D smoothed with SMA.
    pub fn new(period_k: usize, period_d: usize) -> Self {
        Self::from_smoother(SmootherId::Sma, period_k, period_d)
    }

    /// Build the %D smoother from one narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `StochasticsConfig`.
    pub fn from_smoother(ma: SmootherId, period_k: usize, period_d: usize) -> Self {
        assert!(period_k > 0, "period_k must be > 0");
        assert!(period_d > 0, "period_d must be > 0");
        Self {
            period_k,
            period_d,
            highs: Vec::with_capacity(period_k),
            lows: Vec::with_capacity(period_k),
            d_ma: SmootherSlot::new(ma, period_d),
            index: 0,
            filled: false,
            value_k: 0.0,
            value_d: 0.0,
        }
    }

    /// Updates the Stochastic with a new bar and returns (%K, %D).
    ///
    /// Uses `high`, `low`, and `close` prices. Volume is ignored.
    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        if self.highs.len() < self.period_k {
            self.highs.push(high);
            self.lows.push(low);
        } else {
            self.highs[self.index] = high;
            self.lows[self.index] = low;
        }

        self.index = (self.index + 1) % self.period_k;
        let ready_k = self.highs.len() == self.period_k;

        if !ready_k {
            self.value_k = 0.0;
            self.value_d = 0.0;
            return (self.value_k, self.value_d);
        }

        // Calculate %K
        let (k_min_low, k_max_high) = self.highs.iter()
            .zip(self.lows.iter())
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), (&h, &l)| (min.min(l), max.max(h)));

        if (k_max_high - k_min_low).abs() < 1e-12 {
            self.value_k = 0.0;
        } else {
            self.value_k = 100.0 * ((close - k_min_low) / (k_max_high - k_min_low));
        }

        // Calculate %D = MA(%K)
        self.d_ma.feed(self.value_k);
        self.value_d = self.d_ma.value();

        if self.d_ma.is_ready() {
            self.filled = true;
        }

        (self.value_k, self.value_d)
    }

    /// Returns the current %K value.
    #[inline]
    pub fn value_k(&self) -> f64 {
        self.value_k
    }

    /// Returns the current %D value.
    #[inline]
    pub fn value_d(&self) -> f64 {
        self.value_d
    }

    /// Returns `true` if the indicator has enough data for valid output.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.d_ma.reset();
        self.index = 0;
        self.filled = false;
        self.value_k = 0.0;
        self.value_d = 0.0;
    }

    /// Returns the %D smoothing period.
    #[inline]
    pub fn get_period_d(&self) -> usize {
        self.period_d
    }

    /// Returns the %K lookback period.
    #[inline]
    pub fn get_period_k(&self) -> usize {
        self.period_k
    }

    /// Brace-named getter: `k` output (%K).
    #[inline]
    pub fn k(&self) -> f64 {
        self.value_k
    }

    /// Brace-named getter: `d` output (%D).
    #[inline]
    pub fn d(&self) -> f64 {
        self.value_d
    }

}

/// Dual-mode config for the Stochastic Oscillator.
///
/// `period_k` is the %K highest/lowest lookback window; `period_d` is the %D smoother
/// period; `d_smoother` is the `Param<SmootherChoice>` slot (kind + follow/own). By
/// default the slot follows `period_d` (SMA).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct StochasticsConfig {
    /// %K lookback period (the window for highest-high / lowest-low).
    pub period_k: Param<usize>,
    /// %D smoothing period (the host period the slot follows by default).
    pub period_d: Param<usize>,
    /// %D smoother choice (kind + follow/own). Default: follow(Sma).
    #[slot]
    pub d_smoother: Param<SmootherChoice>,
}

impl std::fmt::Debug for Stochastics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stochastics")
            .field("period_k", &self.period_k)
            .field("period_d", &self.period_d)
            .field("value_k", &self.value_k)
            .field("value_d", &self.value_d)
            .field("filled", &self.filled)
            .finish()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Slot, Indicator, Output, Param, SourceAxis, Store, StoreKind,
    UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderOutput, RenderSpec};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

impl Indicator for Stochastics {
    const ID: IndicatorId = IndicatorId::Stoch;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices — %K is close vs the window high/low.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// Two outputs: %K and %D.
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::StochK),
        Output::percent(IndicatorOutputId::StochD),
    ];
    /// Own state is two period_k-deep `Vec`s (highs, lows), rescanned for the window
    /// min/max each bar -> O(period). The %D smoother is a MovingAverage slot, charged
    /// recursively by the barometer.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const SLOTS: &'static [Slot] = StochasticsConfig::SLOTS;

    type Config = StochasticsConfig;
    type Runtime = Stochastics;

    fn create(cfg: StochasticsConfig) -> Stochastics {
        let period_k = cfg.period_k.resolved();
        let period_d = cfg.period_d.resolved();
        Stochastics {
            period_k,
            period_d,
            highs: Vec::with_capacity(period_k),
            lows: Vec::with_capacity(period_k),
            d_ma: cfg.d_smoother.resolved().build(period_d),
            index: 0,
            filled: false,
            value_k: 0.0,
            value_d: 0.0,
        }
    }

    fn slot_members(cfg: &StochasticsConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for StochasticsConfig {
    fn defaults() -> Self {
        StochasticsConfig {
            period_k: Param::Solo(14),
            period_d: Param::Solo(3),
            d_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period_k/period_d: Class A → auto range(2,4048,1).
        // d_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Stochastics {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::StochK, "%K", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::StochD, "%D", Color::hex(0xFF9800), 1.0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(20.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_stochastics_basic_calculation() {
        let mut stoch = Stochastics::new(14, 3);

        // Feed uptrend data with increasing highs and lows
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            stoch.feed(&[base + 2.0, base - 2.0, base]);
        }

        // In strong uptrend, %K should be high
        assert!(stoch.value_k() > 50.0, "Stoch %K in uptrend should be above 50, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastics_range() {
        let mut stoch = Stochastics::new(14, 3);

        // Feed data
        for i in 1..=20 {
            let base = 100.0 + (i % 10) as f64;
            stoch.feed(&[base + 5.0, base - 5.0, base]);
        }

        // %K and %D should be in [0, 100]
        let k = stoch.value_k();
        let d = stoch.value_d();
        assert!(k >= 0.0 && k <= 100.0, "%K should be in [0, 100], got {}", k);
        assert!(d >= 0.0 && d <= 100.0, "%D should be in [0, 100], got {}", d);
    }

    #[test]
    fn test_stochastics_reset() {
        let mut stoch = Stochastics::new(14, 3);

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            stoch.feed(&[base + 2.0, base - 2.0, base]);
        }

        stoch.reset();
        assert!(!stoch.is_ready());
        assert!((stoch.value_k()).abs() < 1e-10);
        assert!((stoch.value_d()).abs() < 1e-10);
    }

    #[test]
    fn test_stochastics_at_high() {
        let mut stoch = Stochastics::new(5, 3);

        // Fill with constant price, then close at the high
        for _ in 0..5 {
            stoch.feed(&[110.0, 90.0, 100.0]);
        }

        // Close at the highest high
        stoch.feed(&[110.0, 90.0, 110.0]);
        assert!((stoch.value_k() - 100.0).abs() < 1.0, "%K at high should be ~100, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastics_at_low() {
        let mut stoch = Stochastics::new(5, 3);

        // Fill with constant price, then close at the low
        for _ in 0..5 {
            stoch.feed(&[110.0, 90.0, 100.0]);
        }

        // Close at the lowest low
        stoch.feed(&[110.0, 90.0, 90.0]);
        assert!(stoch.value_k() < 1.0, "%K at low should be ~0, got {}", stoch.value_k());
    }
}
