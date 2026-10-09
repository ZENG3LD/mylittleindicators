//! Stochastic RSI — applies the Stochastic formula to RSI values.
//!
//! Stoch RSI = (RSI - Lowest RSI) / (Highest RSI - Lowest RSI)
//! %K = MA(Stoch RSI, k_period)
//! %D = MA(%K, d_period)

use crate::indicators::momentum::rsi::Rsi;
use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::engine::ohlcv_field::OhlcvField;

/// Stochastic RSI indicator.
///
/// Embeds the contracted `Rsi` core.  The two smoothing MAs (%K and %D)
/// are fully configurable via `SmootherSlot`.
#[derive(Debug, Clone)]
pub struct StochasticRsi {
    stoch_period: usize,
    k_period: usize,

    rsi: Rsi,
    rsi_values: Vec<f64>,
    k_values: Vec<f64>,

    k_ma: SmootherSlot,
    d_ma: SmootherSlot,

    current_k: f64,
    current_d: f64,

    count: usize,
    is_ready: bool,
}

impl StochasticRsi {
    /// Default: RSI(14), Stoch(14), %K SMA(3), %D SMA(3), source = Close.
    pub fn new(rsi_period: usize, stoch_period: usize, k_period: usize, d_period: usize) -> Self {
        Self::from_smoothers(
            rsi_period,
            stoch_period,
            SmootherId::Sma, k_period,
            SmootherId::Sma, d_period,
        )
    }

    /// Build with explicit smoother IDs.
    pub fn from_smoothers(
        rsi_period: usize,
        stoch_period: usize,
        k_id: SmootherId,
        k_period: usize,
        d_id: SmootherId,
        d_period: usize,
    ) -> Self {
        let k_p = k_period.max(1);
        let d_p = d_period.max(1);
        Self {
            stoch_period,
            k_period: k_p,
            rsi: Rsi::from_smoother(rsi_period, SmootherId::Rma),
            rsi_values: Vec::with_capacity(512),
            k_values: Vec::with_capacity(512),
            k_ma: SmootherSlot::new(k_id, k_p),
            d_ma: SmootherSlot::new(d_id, d_p),
            current_k: 50.0,
            current_d: 50.0,
            count: 0,
            is_ready: false,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). The factory
    /// extracts the source field; the inner RSI consumes the scalar directly. Returns (%K, %D).
    pub fn feed(&mut self, value: f64) -> (f64, f64) {
        let rsi_value = self.rsi.feed(value);

        if self.rsi.is_ready() {
            if self.rsi_values.len() >= self.stoch_period {
                self.rsi_values.remove(0);
            }
            self.rsi_values.push(rsi_value);
        }

        if self.rsi_values.len() >= self.stoch_period {
            let len = self.rsi_values.len();
            let start = len - self.stoch_period;
            let slice = &self.rsi_values[start..];

            let highest = slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let lowest  = slice.iter().fold(f64::INFINITY,     |a, &b| a.min(b));

            let raw_k = if (highest - lowest).abs() < 1e-12 {
                50.0
            } else {
                ((rsi_value - lowest) / (highest - lowest)) * 100.0
            };

            if self.k_values.len() >= self.k_period {
                self.k_values.remove(0);
            }
            self.k_values.push(raw_k);

            if self.k_values.len() >= self.k_period {
                let latest = self.k_values[self.k_values.len() - 1];
                self.current_k = self.k_ma.feed(latest);
            }
        }

        if self.k_ma.is_ready() {
            self.current_d = self.d_ma.feed(self.current_k);
        }

        if self.rsi.is_ready() && self.k_ma.is_ready() && self.d_ma.is_ready() {
            self.is_ready = true;
        }

        self.count += 1;
        (self.current_k, self.current_d)
    }


    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn period(&self) -> usize {
        self.rsi.period()
    }

    pub fn reset(&mut self) {
        self.rsi.reset();
        self.rsi_values.clear();
        self.k_values.clear();
        self.k_ma.reset();
        self.d_ma.reset();
        self.current_k = 50.0;
        self.current_d = 50.0;
        self.count = 0;
        self.is_ready = false;
    }

    /// Brace-named getter: `k` output (smoothed Stochastic RSI %K).
    #[inline]
    pub fn k(&self) -> f64 {
        self.current_k
    }

    /// Brace-named getter: `d` output (smoothed Stochastic RSI %D).
    #[inline]
    pub fn d(&self) -> f64 {
        self.current_d
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Typed dual-mode config for [`StochasticRsi`].
///
/// Absorbs RSI: `rsi_period` + `rsi_smoother` (default `follow(Rma)`) replace the previous
/// plain `rsi_period: usize`. The inner `Rsi` is built via `Rsi::from_choice(...)`. `k_period`
/// and `d_period` drive the %K/%D smoothing slots (follow(Sma) defaults).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct StochRsiConfig {
    pub source: Param<OhlcvField>,
    pub rsi_period: Param<usize>,
    pub stoch_period: Param<usize>,
    /// RSI gain/loss smoother — default `follow(Rma)` (Wilder's RMA at `rsi_period`).
    #[slot]
    pub rsi_smoother: Param<SmootherChoice>,
    /// %K smoothing period.
    pub k_period: Param<usize>,
    /// %K smoothing slot — default `follow(Sma)` at `k_period`.
    #[slot]
    pub k_ma: Param<SmootherChoice>,
    /// %D smoothing period.
    pub d_period: Param<usize>,
    /// %D smoothing slot — default `follow(Sma)` at `d_period`.
    #[slot]
    pub d_ma: Param<SmootherChoice>,
}

impl Indicator for StochasticRsi {
    const ID: IndicatorId = IndicatorId::StochRsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(stoch_period) per bar (rsi_values scan). RSI cost charged via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [Slot] = StochRsiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::StochRsiK),
        Output::percent(IndicatorOutputId::StochRsiD),
    ];
    type Config = StochRsiConfig;
    type Runtime = StochasticRsi;

    fn create(cfg: StochRsiConfig) -> StochasticRsi {
        let rsi_period = cfg.rsi_period.resolved();
        let stoch_period = cfg.stoch_period.resolved();
        let rsi_choice = cfg.rsi_smoother.resolved();
        let k_period = cfg.k_period.resolved();
        let d_period = cfg.d_period.resolved();
        let k_choice = cfg.k_ma.resolved();
        let d_choice = cfg.d_ma.resolved();
        let k_p = k_period.max(1);
        let d_p = d_period.max(1);
        StochasticRsi {
            stoch_period,
            k_period: k_p,
            rsi: Rsi::from_choice(rsi_choice, rsi_period),
            rsi_values: Vec::with_capacity(512),
            k_values: Vec::with_capacity(512),
            k_ma: k_choice.build(k_p),
            d_ma: d_choice.build(d_p),
            current_k: 50.0,
            current_d: 50.0,
            count: 0,
            is_ready: false,
        }
    }

    fn source_fields(cfg: &StochRsiConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &StochRsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for StochRsiConfig {
    fn defaults() -> Self {
        StochRsiConfig {
            source: Param::Solo(OhlcvField::Close),
            rsi_period: Param::Solo(14),
            stoch_period: Param::Solo(14),
            rsi_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
            k_period: Param::Solo(3),
            k_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            d_period: Param::Solo(3),
            d_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/stoch_period/k_period/d_period: Class A → auto range(2,4048,1).
        // source: Class O → auto all-8.
        // rsi_smoother/k_ma/d_ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for StochasticRsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::StochRsiK, "%K", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::StochRsiD, "%D", Color::hex(0xFF9800))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(20.0, Color::hex(0x4CAF50)))
            .precision(4)
            .build()
    }
}

impl Default for StochasticRsi {
    fn default() -> Self {
        Self::new(14, 14, 3, 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stoch_rsi_basic_calculation() {
        let mut s = StochasticRsi::new(14, 14, 3, 3);
        for i in 1..=60 {
            s.feed(100.0 + i as f64);
        }
        assert!(s.is_ready());
        let (k, d) = (s.k(), s.d());
        assert!(k >= 0.0 && k <= 100.0, "K={k} out of range");
        assert!(d >= 0.0 && d <= 100.0, "D={d} out of range");
    }

    #[test]
    fn test_stoch_rsi_range() {
        let mut s = StochasticRsi::new(14, 14, 3, 3);
        for i in 1..=80 {
            let price = 100.0 + (i % 20) as f64 * 2.0;
            s.feed(price);
        }
        assert!(s.is_ready());
        let (k, d) = (s.k(), s.d());
        assert!(k >= 0.0 && k <= 100.0);
        assert!(d >= 0.0 && d <= 100.0);
    }

    #[test]
    fn test_stoch_rsi_reset() {
        let mut s = StochasticRsi::new(14, 14, 3, 3);
        for i in 1..=60 {
            s.feed(100.0 + i as f64);
        }
        assert!(s.is_ready());
        s.reset();
        assert!(!s.is_ready());
        assert!((s.k() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_stoch_rsi_period() {
        let s = StochasticRsi::new(14, 14, 3, 3);
        assert_eq!(s.period(), 14);
    }

    #[test]
    fn test_stoch_rsi_with_source() {
        let mut s = StochasticRsi::new(14, 14, 3, 3);
        for i in 1..=60 {
            let high = 110.0 + i as f64;
            let low = 90.0 + i as f64;
            // Source selection is the factory's job now; feed the resolved HL2 scalar.
            s.feed((high + low) / 2.0);
        }
        assert!(s.is_ready());
        let (k, d) = (s.k(), s.d());
        assert!(k >= 0.0 && k <= 100.0);
        assert!(d >= 0.0 && d <= 100.0);
    }

    #[test]
    fn factory_feeds_resolved_stoch_rsi() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StochasticRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::StochRsi(cfg).build_solo().unwrap();
        for i in 1..=60 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
