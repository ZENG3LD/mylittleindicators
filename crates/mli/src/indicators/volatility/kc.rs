// High-performance Keltner Channel (KC)
// (c) 2024

use super::atr::Atr;
use crate::engine::contract_engine::SmootherId;
#[derive(Debug, Clone)]
pub struct Kc {
    period: usize,
    k_multiplier: f64,
    sma_buf: Vec<f64>,
    sma_sum: f64,
    sma_filled: bool,
    atr: Atr,
    pub upper: f64,
    middle: f64,
    pub lower: f64,
}

impl Kc {
    /// Default ctor — ATR smoothed with RMA (Wilder).
    pub fn new(period: usize, k_multiplier: f64) -> Self {
        Self::from_smoothers(period, k_multiplier, SmootherId::Rma)
    }

    /// Build from a narrow `SmootherId` for the ATR smoother.
    pub fn from_smoothers(period: usize, k_multiplier: f64, atr_smoother: SmootherId) -> Self {
        Self {
            period,
            k_multiplier,
            sma_buf: Vec::with_capacity(period),
            sma_sum: 0.0,
            sma_filled: false,
            atr: Atr::from_smoother(period, atr_smoother),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];

        let typical = (high + low + close) / 3.0;
        if self.sma_buf.len() == self.period {
            let old = self.sma_buf.remove(0);
            self.sma_sum -= old;
        }
        self.sma_buf.push(typical);
        self.sma_sum += typical;
        if self.sma_buf.len() == self.period {
            self.sma_filled = true;
        }
        if self.sma_buf.len() < self.period {
            self.upper = 0.0;
            self.middle = 0.0;
            self.lower = 0.0;
            return (self.upper, self.middle, self.lower);
        }
        self.middle = self.sma_sum / self.period as f64;
        // Atr::update_bar ignores open and volume; pass 0.0 for those.
        let atr = self.atr.feed(&[high, low, close]);
        self.upper = self.middle + self.k_multiplier * atr;
        self.lower = self.middle - self.k_multiplier * atr;
        (self.upper, self.middle, self.lower)
    }

    // Transitional bridge: kp.rs still calls update_bar; delegates to feed.
    // Remove when kp.rs is contracted.



    /// Named getter for the `upper` brace output (KC upper band).
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Named getter for the `middle` brace output (KC middle/SMA line).
    pub fn middle(&self) -> f64 {
        self.middle
    }

    /// Named getter for the `lower` brace output (KC lower band).
    pub fn lower(&self) -> f64 {
        self.lower
    }

    pub fn is_ready(&self) -> bool {
        self.sma_filled && self.atr.is_ready()
    }
    pub fn reset(&mut self) {
        self.sma_buf.clear();
        self.sma_sum = 0.0;
        self.sma_filled = false;
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
}

impl Default for Kc {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Config for [`Kc`]: `period` drives both the SMA middle window and the ATR lookback;
/// `atr_smoother` selects the ATR's internal TR smoother (defaults `follow(Rma)`); and
/// `k_multiplier` is the band-width multiplier.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KcConfig {
    pub period: Param<usize>,
    pub k_multiplier: Param<f64>,
    /// ATR smoother — default `follow(Rma)` at `period`.
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for Kc {
    const ID: IndicatorId = IndicatorId::VoKc;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = KcConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::VoKcUpper),
        Output::price(IndicatorOutputId::VoKcMiddle),
        Output::price(IndicatorOutputId::VoKcLower),
    ];
    type Config = KcConfig;
    type Runtime = Kc;

    fn create(cfg: KcConfig) -> Kc {
        let p  = cfg.period.resolved();
        let ch = cfg.atr_smoother.resolved();
        Kc {
            period: p,
            k_multiplier: cfg.k_multiplier.resolved(),
            sma_buf: Vec::with_capacity(p),
            sma_sum: 0.0,
            sma_filled: false,
            atr: Atr::from_smoother(ch.period.resolve(p), ch.kind),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    fn slot_members(cfg: &KcConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for KcConfig {
    fn defaults() -> Self {
        KcConfig {
            period:       Param::Solo(14),
            k_multiplier: Param::Solo(2.0),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        // #[slot] atr_smoother: leave Solo (deferred sweep wave)
        let mut s = Self::machine_defaults_auto();
        s.k_multiplier = Param::many(crate::contract::sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for Kc {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::VoKcUpper,  "KC Upper",  Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::VoKcMiddle, "KC Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::VoKcLower,  "KC Lower",  Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kc_creation() {
        let kc = Kc::new(20, 2.0);
        assert!(!kc.is_ready());
        assert_eq!(kc.upper(), 0.0);
        assert_eq!(kc.middle(), 0.0);
        assert_eq!(kc.lower(), 0.0);
    }

    #[test]
    fn test_kc_warmup() {
        let mut kc = Kc::new(20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kc.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(kc.is_ready());
    }

    #[test]
    fn test_kc_band_ordering() {
        let mut kc = Kc::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = kc.feed(&[price + 1.0, price - 1.0, price]);
            if kc.is_ready() {
                assert!(upper >= middle, "Upper should be >= middle");
                assert!(middle >= lower, "Middle should be >= lower");
            }
        }
    }

    #[test]
    fn test_kc_reset() {
        let mut kc = Kc::new(20, 2.0);
        for i in 0..25 {
            kc.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        kc.reset();
        assert!(!kc.is_ready());
        assert_eq!(kc.upper(), 0.0);
        assert_eq!(kc.middle(), 0.0);
        assert_eq!(kc.lower(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_kc() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::VoKc(<<Kc as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price,
                high: price + 1.5,
                low: price - 1.5,
                close: price,
                volume: 9999.0, // not used
            });
        }
        assert!(f.is_ready());
        let upper = f.read(IndicatorOutputId::VoKcUpper);
        let lower = f.read(IndicatorOutputId::VoKcLower);
        assert!(upper >= lower);
    }
}
