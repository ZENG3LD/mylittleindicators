//! Chandelier Stop — dynamic stop levels based on Chandelier Exit.
//!
//! Long stop:  highest_high over N bars − (ATR × multiplier).
//! Short stop: lowest_low  over N bars + (ATR × multiplier).
//!
//! Unlike ATR Trailing Stop this version "jumps" with each new extreme rather
//! than using trailing ratchet logic. Strategy stop logic lives outside.

use crate::indicators::volatility::atr::Atr;

/// Chandelier Stop — ATR-from-extreme-high/low stop levels.
#[derive(Debug, Clone)]
pub struct ChandelierStop {
    period: usize,
    multiplier: f64,

    /// ATR for volatility calculation.
    atr: Atr,

    /// Rolling extreme-price buffers (capped at `period`).
    highs: Vec<f64>,
    lows: Vec<f64>,

    long_stop: f64,
    short_stop: f64,

    bars_count: usize,
    is_ready: bool,
}

impl ChandelierStop {
    /// Default parameters: period=22, multiplier=3.0 (Wilder ATR).
    pub fn new() -> Self {
        Self::with_params(22, 3.0)
    }

    /// Build with explicit `period` and `multiplier` (Wilder RMA ATR).
    pub fn with_params(period: usize, multiplier: f64) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");

        Self {
            period,
            multiplier,
            atr: Atr::new_wilder(period),
            highs: Vec::with_capacity(512),
            lows: Vec::with_capacity(512),
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Build from a pre-built `Atr` instance (contract creation path).
    fn from_atr(period: usize, multiplier: f64, atr: Atr) -> Self {
        Self {
            period,
            multiplier,
            atr,
            highs: Vec::with_capacity(512),
            lows: Vec::with_capacity(512),
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed one bar's `[high, low, close]` lanes.
    /// Returns `(long_stop, short_stop)`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        // Drive the inner ATR — open/volume are ignored.
        let atr_value = self.atr.feed(&[high, low, close]);

        // Maintain rolling extreme buffers.
        if self.highs.len() >= self.period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);

        let highest_high = self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lowest_low = self.lows.iter().cloned().fold(f64::INFINITY, f64::min);

        // No trailing logic — levels move with every new extreme.
        self.long_stop = highest_high - atr_value * self.multiplier;
        self.short_stop = lowest_low + atr_value * self.multiplier;

        self.is_ready = self.bars_count >= self.period && self.atr.is_ready();

        (self.long_stop, self.short_stop)
    }

    pub fn long_stop(&self) -> f64 {
        self.long_stop
    }

    pub fn short_stop(&self) -> f64 {
        self.short_stop
    }

    pub fn value(&self) -> f64 {
        self.long_stop
    }

    pub fn levels(&self) -> (f64, f64) {
        (self.long_stop, self.short_stop)
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn atr_value(&self) -> f64 {
        self.atr.value()
    }

    pub fn highest_high(&self) -> f64 {
        if self.highs.is_empty() {
            0.0
        } else {
            self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        }
    }

    pub fn lowest_low(&self) -> f64 {
        if self.lows.is_empty() {
            0.0
        } else {
            self.lows.iter().cloned().fold(f64::INFINITY, f64::min)
        }
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.highs.clear();
        self.lows.clear();
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    pub fn params(&self) -> (usize, f64) {
        (self.period, self.multiplier)
    }
}

impl Default for ChandelierStop {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::SmootherSlotOrder;
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
    sweep_f64,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`ChandelierStop`].
/// `period` = extreme-window and ATR period; `multiplier` = ATR scale;
/// `atr_smoother` = inner ATR smoother order (member + its own period).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ChandConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for ChandelierStop {
    const ID: IndicatorId = IndicatorId::Chand;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// H/L/C fixed lanes — outer math uses High+Low for extrema; inner ATR uses H/L/C.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(period) — rescans the extreme-price windows each bar. Two period-deep `Vec`s.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Chand)];

    type Config = ChandConfig;
    type Runtime = ChandelierStop;

    fn create(cfg: ChandConfig) -> ChandelierStop {
        let period = cfg.period.resolved();
        let multiplier = cfg.multiplier.resolved();
        let atr_order = cfg.atr_smoother.resolved();
        let atr = Atr::from_smoother(atr_order.period(), atr_order.id());
        ChandelierStop::from_atr(period, multiplier, atr)
    }
}

impl crate::contract::Config for ChandConfig {
    fn defaults() -> Self {
        ChandConfig {
            period: Param::Solo(22),
            multiplier: Param::Solo(3.0),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 22 })),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for ChandelierStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Chand, "Chandelier Stop", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chandelier_stop_creation() {
        let ind = ChandelierStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_stop(), 0.0);
    }

    #[test]
    fn test_chandelier_stop_with_params() {
        let ind = ChandelierStop::with_params(20, 2.5);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (20, 2.5));
    }

    #[test]
    fn test_chandelier_stop_warmup() {
        let mut ind = ChandelierStop::with_params(22, 3.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_chandelier_stop_values_finite() {
        let mut ind = ChandelierStop::with_params(22, 3.0);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_chandelier_stop_reset() {
        let mut ind = ChandelierStop::with_params(22, 3.0);
        for i in 0..30 {
            ind.feed(&[105.0 + i as f64, 95.0, 101.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_stop(), 0.0);
    }

    /// Factory resolves the fixed H/L/C lanes from `const SOURCE`.
    /// Open and Volume are wild (not consumed by this indicator or by ATR).
    #[test]
    fn factory_feeds_resolved_chand() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Chand(<<ChandelierStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();

        for i in 1..=30usize {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 101.0 + i as f64,
                low: 99.0,
                close: 100.0 + i as f64,
                volume: -1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }
}
