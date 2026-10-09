// DSS Bressert — Double Smoothed Stochastic oscillator.
//
// Algorithm:
//   1. Raw %K over k_period bars: (close - min_low) / (max_high - min_low) * 100
//   2. First EMA(smooth_period) of %K  → smoothed_k
//   3. Second EMA(smooth_period) of smoothed_k  → DSS value
//
// Output in [0, 100]. Above 50 = bullish, below 50 = bearish.

use crate::engine::contract_engine::{SmootherId, SmootherSlot};

#[derive(Debug, Clone)]
pub struct DssBressert {
    k_period: usize,
    smooth_period: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    count: usize,
    ema1: SmootherSlot,
    ema2: SmootherSlot,
    value: f64,
}

impl DssBressert {
    /// Default ctor — both smoothing passes use EMA (the classic DSS Bressert kernel).
    pub fn new(k_period: usize, smooth_period: usize) -> Self {
        Self::from_smoother(k_period, smooth_period, SmootherId::Ema)
    }

    /// Build from a narrow `SmootherId` for BOTH double-smoothing passes + the periods.
    /// Legacy bridge; the contract path goes through `DssBressertConfig`.
    pub fn from_smoother(k_period: usize, smooth_period: usize, smoother: SmootherId) -> Self {
        let k = k_period.clamp(2, 512);
        let s = smooth_period.max(1);
        Self {
            k_period: k,
            smooth_period: s,
            highs: Vec::with_capacity(k),
            lows: Vec::with_capacity(k),
            idx: 0,
            count: 0,
            ema1: SmootherSlot::new(smoother, s),
            ema2: SmootherSlot::new(smoother, s),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.idx = 0;
        self.count = 0;
        self.ema1.reset();
        self.ema2.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.k_period && self.ema2.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed the resolved input lanes — `[high, low, close]` (the factory resolves the fixed
    /// HLC slice). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        if self.count < self.k_period {
            self.highs.push(h);
            self.lows.push(l);
            self.count += 1;
        } else {
            self.highs[self.idx] = h;
            self.lows[self.idx] = l;
        }
        self.idx = (self.idx + 1) % self.k_period;

        // raw %K over window
        let len = self.count.min(self.k_period);
        let mut max_h = f64::NEG_INFINITY;
        let mut min_l = f64::INFINITY;
        for i in 0..len {
            max_h = max_h.max(self.highs[i]);
            min_l = min_l.min(self.lows[i]);
        }
        let range = (max_h - min_l).abs().max(1e-12);
        let k = (c - min_l) / range * 100.0;

        let s1 = self.ema1.feed(k);
        let s2 = self.ema2.feed(s1);
        self.value = s2;
        self.value
    }

    pub fn k_period(&self) -> usize {
        self.k_period
    }

    pub fn smooth_period(&self) -> usize {
        self.smooth_period
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Typed dual-mode config for [`DssBressert`] — double-smoothed stochastic (bounded 0-100).
///
/// `k_period` = %K lookback, `smooth_period` = the double-smoothing period (the host period
/// the `ma` slot follows by default). The `ma` slot (default `follow(Ema)`) drives BOTH
/// double-smoothing passes.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct DssBressertConfig {
    /// %K lookback window (clamped 2..512).
    pub k_period: Param<usize>,
    /// Double-smoothing period (the host period the slot follows by default).
    pub smooth_period: Param<usize>,
    /// Smoother for both double-smoothing passes — default `follow(Ema)` at `smooth_period`.
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for DssBressert {
    const ID: IndicatorId = IndicatorId::Dss;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Stochastic over the raw range slices (h/l/c).
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// Per-bar %K rescan over the window -> Linear; two `k_period`-deep heap ring buffers
    /// (highs/lows). The two smoother passes land recursively through `SLOTS`.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const SLOTS: &'static [Slot] = DssBressertConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Dss)];
    type Config = DssBressertConfig;
    type Runtime = DssBressert;

    fn create(cfg: DssBressertConfig) -> DssBressert {
        let k = cfg.k_period.resolved().clamp(2, 512);
        let smooth_period = cfg.smooth_period.resolved();
        let choice = cfg.ma.resolved();
        DssBressert {
            k_period: k,
            smooth_period: smooth_period.max(1),
            highs: Vec::with_capacity(k),
            lows: Vec::with_capacity(k),
            idx: 0,
            count: 0,
            ema1: choice.build(smooth_period),
            ema2: choice.build(smooth_period),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &DssBressertConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for DssBressertConfig {
    fn defaults() -> Self {
        DssBressertConfig {
            k_period: Param::Solo(13),
            smooth_period: Param::Solo(8),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // k_period/smooth_period: Class A → auto range(2,4048,1).
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for DssBressert {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dss, "DSS", Color::hex(0x2196F3))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, Color::hex(0x9E9E9E)))
            .reference_line(ReferenceLine::new(20.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dss_creation() {
        let dss = DssBressert::new(10, 3);
        assert!(!dss.is_ready());
        assert_eq!(dss.value(), 0.0);
        assert_eq!(dss.k_period(), 10);
        assert_eq!(dss.smooth_period(), 3);
    }

    #[test]
    fn test_dss_with_smoother() {
        let mut dss = DssBressert::from_smoother(10, 3, SmootherId::Sma);
        for i in 1..=30 {
            let p = 100.0 + i as f64 * 0.5;
            let v = dss.feed(&[p + 1.0, p - 1.0, p]);
            assert!(v.is_finite());
        }
        assert!(dss.is_ready());
    }

    #[test]
    fn test_dss_uptrend() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            dss.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(dss.is_ready());
        // In uptrend with close near high, DSS should be high
        assert!(dss.value() > 50.0, "DSS should be > 50 in uptrend, got {}", dss.value());
    }

    #[test]
    fn test_dss_downtrend() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            dss.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(dss.is_ready());
        // In downtrend with close near low, DSS should be low
        assert!(dss.value() < 50.0, "DSS should be < 50 in downtrend, got {}", dss.value());
    }

    #[test]
    fn test_dss_range() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = dss.feed(&[price + 2.0, price - 2.0, price]);
            if dss.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "DSS should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_dss_reset() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dss.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(dss.is_ready());
        dss.reset();
        assert!(!dss.is_ready());
        assert_eq!(dss.value(), 0.0);
    }

    #[test]
    fn test_dss_contract_create() {
        let cfg = <<DssBressert as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.smooth_period.resolved(), 8);
        let mut dss = <DssBressert as Indicator>::create(cfg);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            dss.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(dss.is_ready());
    }

    /// The factory resolves the fixed HLC lanes and feeds the three scalars; DSS runs
    /// end-to-end (uptrend close near high -> > 50).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Dss(<<DssBressert as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary() > 50.0, "factory DSS uptrend should be > 50, got {}", f.primary());
    }
}
