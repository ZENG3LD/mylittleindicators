// Gann HiLo Activator — stateful trailing stop that switches between HI and LO channels.
//
// Algorithm:
//   MA_High[i] = MA(high, period)
//   MA_Low[i]  = MA(low,  period)
//
//   State starts as Long (activator = MA_Low).
//   While Long:
//     activator = MA_Low
//     if close < MA_Low  → switch to Short, activator = MA_High
//   While Short:
//     activator = MA_High
//     if close > MA_High → switch to Long,  activator = MA_Low
//
// Output: Double(activator, side)  where side = +1 (long) / -1 (short)

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Side {
    Long,
    Short,
}

#[derive(Debug, Clone)]
pub struct GannHiLoActivator {
    ma_high: SmootherSlot,
    ma_low: SmootherSlot,
    side: Side,
    activator: f64,
    upper: f64,
    lower: f64,
}

impl GannHiLoActivator {
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for both the high and low MAs.
    /// Legacy bridge; the contract path goes through `GannHiloConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        let p = period.max(1);
        Self {
            ma_high: SmootherSlot::new(smoother, p),
            ma_low: SmootherSlot::new(smoother, p),
            side: Side::Long,
            activator: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma_high.reset();
        self.ma_low.reset();
        self.side = Side::Long;
        self.activator = 0.0;
        self.upper = 0.0;
        self.lower = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma_high.is_ready() && self.ma_low.is_ready()
    }


    /// Returns `(upper_sma, lower_sma)` — raw SMA lines regardless of state.
    #[inline]
    pub fn bands(&self) -> (f64, f64) {
        (self.upper, self.lower)
    }

    /// Brace-named getter for the `activator` output.
    #[inline]
    pub fn activator(&self) -> f64 {
        self.activator
    }

    /// Brace-named getter for the `side` output (+1.0 = long, -1.0 = short).
    #[inline]
    pub fn side(&self) -> f64 {
        if self.side == Side::Long { 1.0 } else { -1.0 }
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let uh = self.ma_high.feed(h);
        let dl = self.ma_low.feed(l);
        self.upper = uh;
        self.lower = dl;

        if self.is_ready() {
            // State machine: switch side on close crossing activator
            match self.side {
                Side::Long => {
                    if c < dl {
                        self.side = Side::Short;
                        self.activator = uh;
                    } else {
                        self.activator = dl;
                    }
                }
                Side::Short => {
                    if c > uh {
                        self.side = Side::Long;
                        self.activator = dl;
                    } else {
                        self.activator = uh;
                    }
                }
            }
        }

        let side_val = if self.side == Side::Long { 1.0 } else { -1.0 };
        (self.activator, side_val)
    }

}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderOutput, RenderSpec};

/// Typed contract config for [`GannHiLoActivator`]. ONE smoother choice applied to BOTH the
/// high and low MA series — a single configurable slot, two instances. Same pattern as
/// [`crate::indicators::trend::ssl_channel::SslConfig`].
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct GannHiloConfig {
    pub period: Param<usize>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl GannHiLoActivator {
    /// Build from a smoother CHOICE (kind + follow/own period) at the given host period.
    pub fn from_choice(choice: SmootherChoice, host_period: usize) -> Self {
        let p = choice.period.resolve(host_period).max(1);
        Self {
            ma_high: SmootherSlot::new(choice.id(), p),
            ma_low: SmootherSlot::new(choice.id(), p),
            side: Side::Long,
            activator: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }
}

impl Indicator for GannHiLoActivator {
    const ID: IndicatorId = IndicatorId::GannHilo;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads high (MA), low (MA), and close (level select) — fixed slices, not a swept field.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1): level-selection state machine over two smoothers; both smoother buffers land
    /// recursively through the single `SLOTS` choice (realized into high + low MAs).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = GannHiloConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::GannHiloActivator),
        Output::discrete(IndicatorOutputId::GannHiloSide),
    ];
    type Config = GannHiloConfig;
    type Runtime = GannHiLoActivator;

    fn create(cfg: GannHiloConfig) -> GannHiLoActivator {
        GannHiLoActivator::from_choice(cfg.ma.resolved(), cfg.period.resolved())
    }

    fn slot_members(cfg: &GannHiloConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for GannHiloConfig {
    fn defaults() -> Self {
        GannHiloConfig {
            period: Param::Solo(10),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1).
        // ma: #[slot] SmootherChoice — left Solo (deferred wave). [FLAG: slot]
        Self::machine_defaults_auto()
    }
}


impl Render for GannHiLoActivator {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::GannHiloActivator, "Gann HiLo", Color::hex(0xFF9800), 2.0))
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

impl Default for GannHiLoActivator {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gann_hilo_creation() {
        let gann = GannHiLoActivator::new(10);
        assert!(!gann.is_ready());
        assert_eq!(gann.activator(), 0.0);
        assert_eq!(gann.side(), 1.0); // starts Long
    }

    #[test]
    fn test_gann_hilo_warmup() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            gann.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(gann.is_ready());
    }

    #[test]
    fn test_gann_hilo_uptrend_stays_long() {
        let mut gann = GannHiLoActivator::new(5);
        // Strong uptrend — close always above sma_low → stays Long
        for i in 0..20 {
            let price = 100.0 + i as f64 * 3.0;
            gann.feed(&[price + 2.0, price - 0.5, price]);
        }
        assert!(gann.is_ready());
        assert_eq!(gann.side(), 1.0, "Strong uptrend should keep side = Long (+1)");
    }

    #[test]
    fn test_gann_hilo_downtrend_goes_short() {
        let mut gann = GannHiLoActivator::new(5);
        // Strong downtrend — close always below sma_low → switches to Short
        for i in 0..20 {
            let price = 200.0 - i as f64 * 3.0;
            gann.feed(&[price + 0.5, price - 2.0, price]);
        }
        assert!(gann.is_ready());
        assert_eq!(gann.side(), -1.0, "Strong downtrend should flip side = Short (-1)");
    }

    #[test]
    fn test_gann_hilo_reset() {
        let mut gann = GannHiLoActivator::new(10);
        for _i in 0..15 {
            gann.feed(&[105.0, 95.0, 101.0]);
        }
        gann.reset();
        assert!(!gann.is_ready());
        assert_eq!(gann.activator(), 0.0);
        assert_eq!(gann.side(), 1.0);
    }

    #[test]
    fn test_gann_hilo_finite() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (act, side) = gann.feed(&[price + 1.0, price - 1.0, price]);
            assert!(act.is_finite());
            assert!(side == 1.0 || side == -1.0);
        }
    }

    /// The factory resolves fixed H/L/C lanes and drives the Fields flavor via feed.
    /// A wild volume value (9999.0) in an unused field proves H/L/C are the only inputs.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::GannHilo(<<GannHiLoActivator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price, high: price + 1.0, low: price - 0.5, close: price,
                volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite() && v > 0.0, "GannHilo activator should be finite > 0, got {v}");
    }
}
