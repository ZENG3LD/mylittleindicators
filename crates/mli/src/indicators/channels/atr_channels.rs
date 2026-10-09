//! atr_channels.rs: High-Performance ATR Channels
//! Улучшенная реализация ATR каналов

use crate::engine::contract_engine::SmootherSlot;
use crate::indicators::volatility::atr::Atr;

/// Режимы расчета ATR Channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum AtrChannelMode {
    /// Close - использует Close цену для средней линии
    #[default]
    Close,
    /// Typical - использует Typical Price (HLC/3) для средней линии
    Typical,
    /// OHLC - использует (Open + High + Low + Close) / 4 для средней линии
    OHLC,
    /// HL - использует (High + Low) / 2 для средней линии
    HL,
}

/// High-Performance ATR Channels
#[derive(Debug, Clone)]
pub struct AtrChannels {
    period: usize,
    multiplier: f64,
    mode: AtrChannelMode,

    // MovingAverage для средней линии
    ma: SmootherSlot,

    // ATR для расчета каналов
    atr: Atr,

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,
}

impl AtrChannels {
    /// Создать классические ATR Channels (Close, SMA, Wilder ATR)
    pub fn new_classic(period: usize, multiplier: f64) -> Self {
        Self::from_smoothers(SmootherId::Sma, SmootherId::Rma, period, multiplier, AtrChannelMode::Close)
    }

    /// Build from narrow `SmootherId`s — legacy bridge.
    pub fn from_smoothers(
        ma_smoother: SmootherId,
        atr_smoother: SmootherId,
        period: usize,
        multiplier: f64,
        mode: AtrChannelMode,
    ) -> Self {
        let p = period.max(1);
        assert!(multiplier > 0.0, "Multiplier must be positive");
        Self {
            period: p,
            multiplier,
            mode,
            ma: SmootherSlot::new(ma_smoother, p),
            atr: Atr::from_smoother(p, atr_smoother),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Feed HIGH, LOW, CLOSE lanes (const SOURCE order).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];
        // Behaviour-preserving: center MA historically smoothed CLOSE regardless of mode.
        self.middle = self.ma.feed(close);
        let atr_value = self.atr.feed(&[high, low, close]);
        if self.is_ready() {
            self.upper = self.middle + self.multiplier * atr_value;
            self.lower = self.middle - self.multiplier * atr_value;
        } else {
            self.upper = 0.0;
            self.lower = 0.0;
        }
        (self.upper, self.middle, self.lower)
    }



    pub fn value_tuple(&self) -> (f64, f64, f64) { (self.upper, self.middle, self.lower) }
    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }

    pub fn channel_width(&self) -> f64 {
        if self.is_ready() { self.upper - self.lower } else { 0.0 }
    }

    pub fn position_in_channel(&self, price: f64) -> f64 {
        if !self.is_ready() || self.upper == self.lower { 0.5 }
        else { ((price - self.lower) / (self.upper - self.lower)).clamp(0.0, 1.0) }
    }

    pub fn is_ready(&self) -> bool { self.ma.is_ready() && self.atr.is_ready() }

    pub fn reset(&mut self) {
        self.ma.reset();
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    pub fn period(&self) -> usize { self.period }
    pub fn multiplier(&self) -> f64 { self.multiplier }
    pub fn mode(&self) -> AtrChannelMode { self.mode }
}

impl Default for AtrChannels {
    fn default() -> Self { Self::new_classic(14, 2.0) }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Param, sweep_f64};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed dual-mode config for [`AtrChannels`].
///
/// Two smoother slots: center MA (default Sma at period 14) and ATR smoothing (default Rma at
/// period 14). Each slot carries its own period — the host period drives the indicator's own
/// calculations while the slot period (defaulting to the same value) is swept independently.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrChannelsConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    pub mode: Param<AtrChannelMode>,
    /// Center MA smoother slot. Default Sma at the host period (14).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    /// ATR smoother slot. Default Rma (Wilder) at the host period (14).
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

impl Indicator for AtrChannels {
    const ID: IndicatorId = IndicatorId::Atrchan;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed High/Low/Close triple.
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
    const SLOTS: &'static [Slot] = AtrChannelsConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AtrchanUpper),
        Output::price(IndicatorOutputId::AtrchanMiddle),
        Output::price(IndicatorOutputId::AtrchanLower),
    ];
    type Config = AtrChannelsConfig;
    type Runtime = Self;

    fn create(cfg: AtrChannelsConfig) -> Self {
        let p = cfg.period.resolved();
        let ma_order = cfg.ma.resolved();
        let atr_order = cfg.atr_ma.resolved();
        AtrChannels {
            period: p,
            multiplier: cfg.multiplier.resolved(),
            mode: cfg.mode.resolved(),
            ma: ma_order.into_slot(),
            atr: Atr::from_smoother(atr_order.period(), atr_order.id()),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    fn slot_members(cfg: &AtrChannelsConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrChannelsConfig {
    /// Period 14, multiplier 2.0, Close mode, SMA center at period 14, RMA ATR at period 14.
    fn defaults() -> Self {
        AtrChannelsConfig {
            period: Param::Solo(14),
            multiplier: Param::Solo(2.0),
            mode: Param::Solo(AtrChannelMode::Close),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 14 })),
            atr_ma: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 14 })),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s.mode = Param::many(vec![
            AtrChannelMode::Close,
            AtrChannelMode::Typical,
            AtrChannelMode::OHLC,
            AtrChannelMode::HL,
        ]); // Class Q enum — all variants
        s
    }
}


impl Render for AtrChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::AtrchanUpper, "Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::AtrchanMiddle, "Middle", Color::hex(0x607D8B))
            .line_output(IndicatorOutputId::AtrchanLower, "Lower", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_channels_creation() {
        let ac = AtrChannels::new_classic(14, 2.0);
        assert!(!ac.is_ready());
        assert_eq!(ac.period(), 14);
        assert_eq!(ac.multiplier(), 2.0);
    }

    #[test]
    fn test_atr_channels_warmup() {
        let mut ac = AtrChannels::new_classic(14, 2.0);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ac.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_atr_channels_values() {
        let mut ac = AtrChannels::new_classic(14, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = ac.feed(&[price + 1.0, price - 1.0, price]);
            if ac.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
            }
        }
    }

    #[test]
    fn test_atr_channels_reset() {
        let mut ac = AtrChannels::new_classic(14, 2.0);
        for i in 0..20 {
            ac.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        ac.reset();
        assert!(!ac.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_atrchan() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;
        let cfg = <<AtrChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Atrchan(cfg).build_solo().unwrap();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        let upper = f.read(IndicatorOutputId::AtrchanUpper);
        let lower = f.read(IndicatorOutputId::AtrchanLower);
        assert!(upper >= lower, "upper={} lower={}", upper, lower);
    }
}
