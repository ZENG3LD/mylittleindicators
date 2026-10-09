//! ATR Trailing Stop - simple dynamic trailing levels based on ATR.
//!
//! Long level: highest_high - (ATR × multiplier) — only rises.
//! Short level: lowest_low  + (ATR × multiplier) — only falls.
//!
//! The indicator returns the two trailing levels; stop logic lives in strategies.

use crate::indicators::volatility::atr::Atr;

/// ATR Trailing Stop — simple trailing levels based on ATR from price extremes.
#[derive(Debug, Clone)]
pub struct ATRTrailingStop {
    period: usize,
    multiplier: f64,

    /// ATR for volatility calculation.
    atr: Atr,

    /// Rolling extreme-price buffers (capped at `period`).
    highs: Vec<f64>,
    lows: Vec<f64>,

    /// Current trailing levels.
    long_level: f64,
    short_level: f64,

    bars_count: usize,
    is_ready: bool,
}

impl ATRTrailingStop {
    /// Default parameters: period=14, multiplier=2.0 (Wilder ATR).
    pub fn new() -> Self {
        Self::with_params(14, 2.0)
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
            long_level: 0.0,
            short_level: f64::MAX,
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
            long_level: 0.0,
            short_level: f64::MAX,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed one bar's `[high, low, close]` lanes (resolved by the factory from `const SOURCE`).
    /// Returns `(long_level, short_level)`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        // Drive the inner ATR — it ignores open and volume.
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

        let new_long = highest_high - atr_value * self.multiplier;
        let new_short = lowest_low + atr_value * self.multiplier;

        if self.bars_count == 1 {
            self.long_level = new_long;
            self.short_level = new_short;
        } else {
            // Long level only rises (trailing up).
            if new_long > self.long_level || close < self.long_level {
                self.long_level = new_long;
            }
            // Short level only falls (trailing down).
            if new_short < self.short_level || close > self.short_level {
                self.short_level = new_short;
            }
        }

        self.is_ready = self.bars_count >= self.period && self.atr.is_ready();

        (self.long_level, self.short_level)
    }

    pub fn long_level(&self) -> f64 {
        self.long_level
    }

    pub fn short_level(&self) -> f64 {
        self.short_level
    }

    pub fn value(&self) -> f64 {
        self.long_level
    }

    pub fn levels(&self) -> (f64, f64) {
        (self.long_level, self.short_level)
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn atr_value(&self) -> f64 {
        self.atr.value()
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.highs.clear();
        self.lows.clear();
        self.long_level = 0.0;
        self.short_level = f64::MAX;
        self.bars_count = 0;
        self.is_ready = false;
    }

    pub fn params(&self) -> (usize, f64) {
        (self.period, self.multiplier)
    }
}

impl Default for ATRTrailingStop {
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

/// Typed dual-mode config for [`ATRTrailingStop`].
/// `period` = trailing-window and ATR period; `multiplier` = ATR scale;
/// `atr_smoother` = inner ATR smoother order (member + its own period).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrtsConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for ATRTrailingStop {
    const ID: IndicatorId = IndicatorId::Atrts;
    /// Not a pluggable family member — a trailing-stop level producer consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// H/L/C fixed lanes — outer math uses High+Low for extrema, Close for trailing logic;
    /// inner ATR uses H/L/C (ignores Open+Volume in its own update_bar).
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(period) per bar — rescans extrema windows. Two period-deep `Vec` buffers.
    /// Inner `Atr` cost lands recursively via `inner`.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Atrts)];

    type Config = AtrtsConfig;
    type Runtime = ATRTrailingStop;

    fn create(cfg: AtrtsConfig) -> ATRTrailingStop {
        let period = cfg.period.resolved();
        let multiplier = cfg.multiplier.resolved();
        let atr_order = cfg.atr_smoother.resolved();
        let atr = Atr::from_smoother(atr_order.period(), atr_order.id());
        ATRTrailingStop::from_atr(period, multiplier, atr)
    }
}

impl crate::contract::Config for AtrtsConfig {
    fn defaults() -> Self {
        AtrtsConfig {
            period: Param::Solo(14),
            multiplier: Param::Solo(2.0),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 14 })),
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


impl Render for ATRTrailingStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Atrts, "ATR Trail Stop", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_trailing_stop_creation() {
        let ind = ATRTrailingStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_level(), 0.0);
    }

    #[test]
    fn test_atr_trailing_stop_with_params() {
        let ind = ATRTrailingStop::with_params(20, 3.0);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (20, 3.0));
    }

    #[test]
    fn test_atr_trailing_stop_warmup() {
        let mut ind = ATRTrailingStop::with_params(14, 2.0);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_atr_trailing_stop_values_finite() {
        let mut ind = ATRTrailingStop::with_params(14, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_atr_trailing_stop_reset() {
        let mut ind = ATRTrailingStop::with_params(14, 2.0);
        for i in 0..20 {
            ind.feed(&[105.0 + i as f64, 95.0, 101.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_level(), 0.0);
    }

    /// Factory resolves the fixed H/L/C lanes from `const SOURCE`.
    /// Open and Volume are wild (unused). Verifies the correct fields flow.
    #[test]
    fn factory_feeds_resolved_atrts() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Atrts(<<ATRTrailingStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();

        for i in 1..=25usize {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,   // wild — not used
                high: 101.0 + i as f64,
                low: 99.0,
                close: 100.0 + i as f64,
                volume: -1.0,   // wild — not used
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }
}
