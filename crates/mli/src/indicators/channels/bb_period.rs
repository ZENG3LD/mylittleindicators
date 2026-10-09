// Bollinger Bands (stddev по последнему окну period, классика)
// (c) 2024

use crate::engine::contract_engine::{SmootherId, SmootherSlot};

#[derive(Debug, Clone)]
pub struct BbPeriod {
    period: usize,
    stddev_mult: f64,
    buffer: Vec<f64>,
    ma: SmootherSlot,
    middle: f64,
    upper: f64,
    lower: f64,
}

impl Default for BbPeriod {
    fn default() -> Self {
        Self::new(14, 2.0, SmootherId::Sma)
    }
}

impl BbPeriod {
    /// Build from a typed `SmootherId` for the center line.
    pub fn new(period: usize, stddev_mult: f64, ma: SmootherId) -> Self {
        let ma_slot = SmootherSlot::new(ma, period.max(1));
        Self {
            period,
            stddev_mult,
            buffer: Vec::with_capacity(period),
            ma: ma_slot,
            middle: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }

    /// Feed [High, Low, Close] lanes — typical price = (H+L+C)/3.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let typical = (high + low + close) / 3.0;
        if self.buffer.len() == self.period {
            self.buffer.remove(0);
        }
        self.buffer.push(typical);
        self.ma.feed(typical);
        let len = self.buffer.len();
        if len < self.period {
            self.middle = 0.0;
            self.upper = 0.0;
            self.lower = 0.0;
            return (self.middle, self.upper, self.lower);
        }
        let ma_value = self.ma.value();
        let stddev = (self.buffer.iter().map(|&v| (v - ma_value).powi(2)).sum::<f64>() / len as f64).sqrt();
        self.middle = ma_value;
        self.upper = ma_value + self.stddev_mult * stddev;
        self.lower = ma_value - self.stddev_mult * stddev;
        (self.middle, self.upper, self.lower)
    }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn upper(&self) -> f64 { self.upper }
    pub fn lower(&self) -> f64 { self.lower }
    pub fn is_ready(&self) -> bool {
        self.buffer.len() == self.period
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.ma.reset();
        self.middle = 0.0;
        self.upper = 0.0;
        self.lower = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }

}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Render, Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`BbPeriod`] — Bollinger Bands on typical price.
/// One smoother slot for the center line.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct BbPeriodConfig {
    /// Period for both the buffer and the center MA.
    pub period: Param<usize>,
    /// Standard deviation multiplier.
    pub stddev_mult: Param<f64>,
    /// Smoother slot for the center line (default Sma at the host period 14).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
}

impl Indicator for BbPeriod {
    const ID: IndicatorId = IndicatorId::BbPeriod;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C — typical price = (H+L+C)/3.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const SLOTS: &'static [crate::contract::Slot] = BbPeriodConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::BbPeriodMiddle),
        Output::price(IndicatorOutputId::BbPeriodUpper),
        Output::price(IndicatorOutputId::BbPeriodLower),
    ];
    type Config = BbPeriodConfig;
    type Runtime = BbPeriod;

    fn create(cfg: BbPeriodConfig) -> BbPeriod {
        let period = cfg.period.resolved().max(1);
        let stddev_mult = cfg.stddev_mult.resolved();
        let ma_slot = cfg.ma.resolved().into_slot();
        BbPeriod {
            period,
            stddev_mult,
            buffer: Vec::with_capacity(period),
            ma: ma_slot,
            middle: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }

    fn slot_members(cfg: &BbPeriodConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for BbPeriodConfig {
    fn defaults() -> Self {
        BbPeriodConfig {
            period: Param::Solo(14),
            stddev_mult: Param::Solo(2.0),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 14 })),
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
        s.stddev_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for BbPeriod {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::BbPeriodUpper, "BB Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::BbPeriodMiddle, "BB Middle", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::BbPeriodLower, "BB Lower", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_bb_period() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<BbPeriod as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::BbPeriod(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,  // not used — SOURCE = H/L/C
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0,  // not used
            });
        }
        // factory primary = middle; just check it's finite after warm-up
        let v = f.primary();
        assert!(v.is_finite(), "bb_period middle should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bb_period_creation() {
        let bb = BbPeriod::new(20, 2.0, SmootherId::Sma);
        assert!(!bb.is_ready());
        assert_eq!(bb.middle(), 0.0);
        assert_eq!(bb.upper(), 0.0);
        assert_eq!(bb.lower(), 0.0);
        assert_eq!(bb.period(), 20);
    }

    #[test]
    fn test_bb_period_bands() {
        let mut bb = BbPeriod::new(20, 2.0, SmootherId::Sma);
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            bb.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(bb.is_ready());
        assert!(bb.upper() > bb.middle(), "Upper band should be > middle");
        assert!(bb.lower() < bb.middle(), "Lower band should be < middle");
    }

    #[test]
    fn test_bb_period_finite() {
        let mut bb = BbPeriod::new(20, 2.0, SmootherId::Ema);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let (m, u, l) = bb.feed(&[price + 2.0, price - 2.0, price]);
            assert!(m.is_finite() && u.is_finite() && l.is_finite());
        }
    }

    #[test]
    fn test_bb_period_reset() {
        let mut bb = BbPeriod::new(20, 2.0, SmootherId::Sma);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            bb.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(bb.is_ready());
        bb.reset();
        assert!(!bb.is_ready());
        assert_eq!(bb.middle(), 0.0);
        assert_eq!(bb.upper(), 0.0);
        assert_eq!(bb.lower(), 0.0);
    }
}

