use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::engine::ohlcv_field::OhlcvField;
use crate::indicators::volatility::atr::Atr;

/// STARC Bands: center = MA(source, n); upper = center + k*ATR(m); lower = center - k*ATR(m)
///
/// Receives [High, Low, Close] lanes; `source` picks which field drives the center MA.
#[derive(Debug, Clone)]
pub struct StarcBands {
    ma_period: usize,
    ma_smoother: SmootherId,
    source: OhlcvField,
    ma: SmootherSlot,
    atr: Atr,
    k: f64,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for StarcBands {
    fn default() -> Self {
        Self::from_smoothers(14, 14, 2.0, SmootherId::Sma, SmootherId::Rma, OhlcvField::Close)
    }
}

impl StarcBands {
    /// Create STARC Bands with default MA type (SMA) and Wilder ATR smoothing (RMA).
    pub fn new(ma_period: usize, atr_period: usize, k: f64) -> Self {
        Self::from_smoothers(ma_period, atr_period, k, SmootherId::Sma, SmootherId::Rma, OhlcvField::Close)
    }

    /// Build from narrow `SmootherId`s — the new contract path and legacy bridge.
    pub fn from_smoothers(
        ma_period: usize,
        atr_period: usize,
        k: f64,
        ma_smoother: SmootherId,
        atr_smoother: SmootherId,
        source: OhlcvField,
    ) -> Self {
        let ma_p = ma_period.max(1);
        Self {
            ma_period: ma_p,
            ma_smoother,
            source,
            ma: SmootherSlot::new(ma_smoother, ma_p),
            atr: Atr::from_smoother(atr_period.max(1), atr_smoother),
            k,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Feed HIGH, LOW, CLOSE lanes (const SOURCE order: [High, Low, Close]).
    /// The `source` field on the config selects which field drives the center MA.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];
        let price = self.source.extract(0.0, high, low, close, 0.0);
        self.middle = self.ma.feed(price);
        let _ = self.atr.feed(&[high, low, close]);
        let atrv = self.atr.value();
        self.upper = self.middle + self.k * atrv;
        self.lower = self.middle - self.k * atrv;
        (self.upper, self.middle, self.lower)
    }



    pub fn value_tuple(&self) -> (f64, f64, f64) { (self.upper, self.middle, self.lower) }
    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }
    pub fn is_ready(&self) -> bool { self.ma.is_ready() && self.atr.is_ready() }

    pub fn reset(&mut self) {
        self.ma = SmootherSlot::new(self.ma_smoother, self.ma_period);
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Param, sweep_f64};
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed dual-mode config for [`StarcBands`].
///
/// Two smoother slots: center MA (default Sma at ma_period 14) and ATR smoothing (default Rma
/// at atr_period 14). STARC allows independent MA and ATR periods. Each slot carries its own
/// period axis; defaults match the respective host period field.
/// `source` picks which OHLCV field drives the center MA (default Close).
/// SOURCE is fixed [High, Low, Close] — ATR always needs all three.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct StarcConfig {
    pub ma_period: Param<usize>,
    pub atr_period: Param<usize>,
    pub k: Param<f64>,
    pub source: Param<OhlcvField>,
    /// Center MA smoother slot. Default Sma at the `ma_period` default (14).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    /// ATR smoother slot. Default Rma (Wilder) at the `atr_period` default (14).
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

impl Indicator for StarcBands {
    const ID: IndicatorId = IndicatorId::Starc;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed [High, Low, Close] — ATR requires all three; `source` in config
    /// picks the field for the center MA (extracted inside `feed`).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[],
    };
    const SLOTS: &'static [Slot] = StarcConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::StarcUpper),
        Output::price(IndicatorOutputId::StarcMiddle),
        Output::price(IndicatorOutputId::StarcLower),
    ];
    type Config = StarcConfig;
    type Runtime = Self;

    fn create(cfg: StarcConfig) -> Self {
        let ma_order = cfg.ma.resolved();
        let atr_order = cfg.atr_ma.resolved();
        StarcBands::from_smoothers(
            ma_order.period(),
            atr_order.period(),
            cfg.k.resolved(),
            ma_order.id(),
            atr_order.id(),
            cfg.source.resolved(),
        )
    }

    fn slot_members(cfg: &StarcConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for StarcConfig {
    /// ma_period 14, atr_period 14, k 2.0, source Close, SMA center at period 14, RMA ATR at period 14.
    fn defaults() -> Self {
        StarcConfig {
            ma_period: Param::Solo(14),
            atr_period: Param::Solo(14),
            k: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 14 })),
            atr_ma: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 14 })),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // ma_period/atr_period→range(2,4048,1), source→all 8
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for StarcBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::StarcUpper, "Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::StarcMiddle, "Middle", Color::hex(0x607D8B))
            .line_output(IndicatorOutputId::StarcLower, "Lower", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starc_bands_creation() {
        let sb = StarcBands::new(20, 14, 2.0);
        assert!(!sb.is_ready());
        assert_eq!(sb.upper, 0.0);
        assert_eq!(sb.lower, 0.0);
    }

    #[test]
    fn test_starc_bands_warmup() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sb.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(sb.is_ready());
    }

    #[test]
    fn test_starc_bands_values() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            sb.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(sb.upper >= sb.middle);
        assert!(sb.middle >= sb.lower);
    }

    #[test]
    fn test_starc_bands_non_default_atr() {
        let mut sb = StarcBands::from_smoothers(20, 14, 2.0, SmootherId::Ema, SmootherId::Ema, OhlcvField::Close);
        assert!(!sb.is_ready());
        for i in 0..30 {
            let p = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (u, m, l) = sb.feed(&[p + 2.0, p - 2.0, p]);
            assert!(u.is_finite());
            assert!(m.is_finite());
            assert!(l.is_finite());
        }
        assert!(sb.is_ready());
    }

    #[test]
    fn test_starc_bands_reset() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            sb.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        sb.reset();
        assert!(!sb.is_ready());
        assert_eq!(sb.upper, 0.0);
        assert_eq!(sb.lower, 0.0);
    }

    #[test]
    fn test_starc_bands_feed_lanes() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            sb.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(sb.is_ready());
        assert!(sb.upper >= sb.middle);
        assert!(sb.middle >= sb.lower);
    }

    #[test]
    fn factory_feeds_resolved_starc() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;
        let cfg = <<StarcBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Starc(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        let upper = f.read(IndicatorOutputId::StarcUpper);
        let lower = f.read(IndicatorOutputId::StarcLower);
        assert!(upper >= lower);
    }
}
