//! price_channels.rs: High-Performance Price Channels
//! Каналы цен - похожие на Donchian, но с MA сглаживанием самих экстремумов
//!
//! Особенности:
//! - Circular buffer O(1) operations
//! - MA сглаживание максимумов и минимумов
//! - Поддержка ВСЕХ 19 типов MA
//! - Дополнительные методы аналогично другим каналам

use crate::engine::contract_engine::SmootherSlot;

/// Режимы расчета Price Channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum PriceChannelMode {
    /// Raw - сырые max/min без сглаживания (как Donchian)
    #[default]
    Raw,
    /// Smoothed - с MA сглаживанием max/min
    Smoothed,
}

/// High-Performance Price Channels
#[derive(Debug, Clone)]
pub struct PriceChannels {
    period: usize,
    mode: PriceChannelMode,

    // Circular buffers для high/low - O(1) operations
    high_buffer: Vec<f64>,
    low_buffer: Vec<f64>,
    buffer_index: usize,
    buffer_filled: bool,

    // MovingAverages для сглаживания (только для Smoothed режима)
    ma_high: Option<SmootherSlot>,
    ma_low: Option<SmootherSlot>,

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,
}

impl PriceChannels {
    /// Создать Raw Price Channels (аналогично Donchian)
    pub fn new_raw(period: usize) -> Self {
        Self::from_smoothers(period, PriceChannelMode::Raw, None)
    }

    /// Создать Smoothed Price Channels с указанным smoother
    pub fn new_smoothed(period: usize, smoother: SmootherId) -> Self {
        Self::from_smoothers(period, PriceChannelMode::Smoothed, Some(smoother))
    }

    /// Build from a `SmootherId` — legacy bridge for derivatives still calling this.
    pub fn from_smoothers(period: usize, mode: PriceChannelMode, smoother: Option<SmootherId>) -> Self {
        let p = period.max(1).min(512);
        let (ma_high, ma_low) = match (mode, smoother) {
            (PriceChannelMode::Smoothed, Some(id)) => (
                Some(SmootherSlot::new(id, p)),
                Some(SmootherSlot::new(id, p)),
            ),
            _ => (None, None),
        };
        Self {
            period: p,
            mode,
            high_buffer: Vec::with_capacity(p),
            low_buffer: Vec::with_capacity(p),
            buffer_index: 0,
            buffer_filled: false,
            ma_high,
            ma_low,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Feed HIGH and LOW lanes (const SOURCE order: [High, Low]).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low  = lanes[1];

        if self.buffer_filled {
            self.high_buffer[self.buffer_index] = high;
            self.low_buffer[self.buffer_index] = low;
        } else {
            self.high_buffer.push(high);
            self.low_buffer.push(low);
        }

        self.buffer_index = (self.buffer_index + 1) % self.period;

        if self.high_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }

        match self.mode {
            PriceChannelMode::Raw => self.calculate_raw_channels(),
            PriceChannelMode::Smoothed => self.calculate_smoothed_channels(),
        }

        (self.upper, self.middle, self.lower)
    }

    /// Legacy bar-level update for derivatives that call `self.channels.update_bar`.

    fn calculate_raw_channels(&mut self) {
        if self.high_buffer.is_empty() || self.low_buffer.is_empty() {
            self.upper = 0.0;
            self.lower = 0.0;
        } else {
            let buffer_len = if self.buffer_filled { self.period } else { self.high_buffer.len() };
            let (lower, upper) = self.high_buffer.iter()
                .zip(self.low_buffer.iter())
                .take(buffer_len)
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.upper = upper;
            self.lower = lower;
        }
        self.middle = (self.upper + self.lower) / 2.0;
    }

    fn calculate_smoothed_channels(&mut self) {
        if let (Some(ma_high), Some(ma_low)) = (&mut self.ma_high, &mut self.ma_low) {
            let buffer_len = if self.buffer_filled { self.period } else { self.high_buffer.len() };
            let (current_min, current_max) = self.high_buffer.iter()
                .zip(self.low_buffer.iter())
                .take(buffer_len)
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.upper = ma_high.feed(current_max);
            self.lower = ma_low.feed(current_min);
        } else {
            self.calculate_raw_channels();
        }
        self.middle = (self.upper + self.lower) / 2.0;
    }


    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }

    pub fn channel_width(&self) -> f64 {
        if self.is_ready() { self.upper - self.lower } else { 0.0 }
    }

    pub fn position_in_channel(&self, price: f64) -> f64 {
        if !self.is_ready() || self.upper == self.lower {
            0.5
        } else {
            ((price - self.lower) / (self.upper - self.lower)).clamp(0.0, 1.0)
        }
    }

    pub fn is_ready(&self) -> bool {
        match self.mode {
            PriceChannelMode::Raw => self.buffer_filled,
            PriceChannelMode::Smoothed => {
                if let (Some(ma_high), Some(ma_low)) = (&self.ma_high, &self.ma_low) {
                    self.buffer_filled && ma_high.is_ready() && ma_low.is_ready()
                } else {
                    self.buffer_filled
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.high_buffer.clear();
        self.low_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;
        if let Some(ref mut ma) = self.ma_high { ma.reset(); }
        if let Some(ref mut ma) = self.ma_low  { ma.reset(); }
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    pub fn period(&self) -> usize { self.period }
    pub fn mode(&self) -> PriceChannelMode { self.mode }
}

impl Default for PriceChannels {
    fn default() -> Self { Self::new_raw(20) }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::contract::Param;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed config for [`PriceChannels`].
///
/// `ma` is the smoother slot for `Smoothed` mode (member + own period).
/// In `Raw` mode the slot is present but ignored at runtime.
/// Default: Sma at period 20 — matches the legacy `Sma(period: 20)` default.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct PriceChannelsConfig {
    pub period: Param<usize>,
    pub mode: Param<PriceChannelMode>,
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
}

impl Indicator for PriceChannels {
    const ID: IndicatorId = IndicatorId::Pricechan;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed High/Low pair.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
        inner: &[],
    };
    const SLOTS: &'static [Slot] = PriceChannelsConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::PricechanUpper),
        Output::price(IndicatorOutputId::PricechanMiddle),
        Output::price(IndicatorOutputId::PricechanLower),
    ];
    type Config = PriceChannelsConfig;
    type Runtime = Self;

    fn create(cfg: PriceChannelsConfig) -> Self {
        let period = cfg.period.resolved();
        let mode = cfg.mode.resolved();
        let smoother = if mode == PriceChannelMode::Smoothed {
            let order = cfg.ma.resolved();
            Some(order.id())
        } else {
            None
        };
        PriceChannels::from_smoothers(period, mode, smoother)
    }

    fn slot_members(cfg: &PriceChannelsConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for PriceChannelsConfig {
    fn defaults() -> Self {
        PriceChannelsConfig {
            period: Param::Solo(20),
            mode: Param::Solo(PriceChannelMode::Raw),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 20 })),
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
        s.mode = Param::many(vec![
            PriceChannelMode::Raw,
            PriceChannelMode::Smoothed,
        ]); // Class Q enum — all variants
        s
    }
}


impl Render for PriceChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::PricechanUpper, "Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::PricechanMiddle, "Middle", Color::hex(0x607D8B))
            .line_output(IndicatorOutputId::PricechanLower, "Lower", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_channels_creation() {
        let pc = PriceChannels::new_raw(20);
        assert!(!pc.is_ready());
        assert_eq!(pc.upper(), 0.0);
        assert_eq!(pc.lower(), 0.0);
    }

    #[test]
    fn test_price_channels_warmup() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pc.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(pc.is_ready());
    }

    #[test]
    fn test_price_channels_values() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pc.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(pc.upper() >= pc.middle());
        assert!(pc.middle() >= pc.lower());
    }

    #[test]
    fn test_price_channels_smoothed() {
        let mut pc = PriceChannels::new_smoothed(20, SmootherId::Ema);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            pc.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(pc.is_ready());
        assert!(pc.channel_width() >= 0.0);
    }

    #[test]
    fn test_price_channels_reset() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            pc.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        pc.reset();
        assert!(!pc.is_ready());
        assert_eq!(pc.upper(), 0.0);
        assert_eq!(pc.lower(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_pricechan() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;
        let cfg = <<PriceChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Pricechan(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        let upper = f.read(IndicatorOutputId::PricechanUpper);
        let lower = f.read(IndicatorOutputId::PricechanLower);
        assert!(upper >= lower, "upper={} lower={}", upper, lower);
    }
}
