// ATR Channels (ATR Bands)
// Middle = MA(Close), Upper = Middle + k*ATR, Lower = Middle - k*ATR

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::volatility::atr::Atr;
/// ATR Channels: Middle = MA(Close), Upper = Middle + k*ATR, Lower = Middle - k*ATR
#[derive(Debug, Clone)]
pub struct AtrChannels {
    ma: SmootherSlot,
    atr: Atr,
    k: f64,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl AtrChannels {
    /// Default ctor — SMA(20) for middle, RMA(14) for ATR, k=2.0.
    pub fn new(ma_period: usize, atr_period: usize, k: f64) -> Self {
        Self::from_smoothers(ma_period, SmootherId::Sma, atr_period, SmootherId::Rma, k)
    }

    /// Build from explicit smoother shapes.
    ///
    /// - `ma_smoother` / `ma_period` — shape + period of the middle-line smoother (feeds Close)
    /// - `atr_smoother` / `atr_period` — shape + period of the ATR internal smoother (feeds TR)
    /// - `k` — band-width multiplier
    pub fn from_smoothers(
        ma_period: usize,
        ma_smoother: SmootherId,
        atr_period: usize,
        atr_smoother: SmootherId,
        k: f64,
    ) -> Self {
        Self {
            ma: SmootherSlot::new(ma_smoother, ma_period),
            atr: Atr::from_smoother(atr_period, atr_smoother),
            k,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Feed a bar given as `lanes = [high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];
        let ma_val  = self.ma.feed(close);
        // open and volume are not used by ATR — pass 0.0
        let atr_val = self.atr.feed(&[high, low, close]);
        self.middle = ma_val;
        self.upper  = ma_val + self.k * atr_val;
        self.lower  = ma_val - self.k * atr_val;
        (self.upper, self.middle, self.lower)
    }

    pub fn upper(&self)  -> f64 { self.upper  }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self)  -> f64 { self.lower  }


    pub fn is_ready(&self) -> bool { self.ma.is_ready() && self.atr.is_ready() }

    pub fn reset(&mut self) {
        self.ma.reset();
        self.atr.reset();
        self.upper  = 0.0;
        self.middle = 0.0;
        self.lower  = 0.0;
    }
}

impl Default for AtrChannels {
    fn default() -> Self {
        Self::new(20, 14, 2.0)
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Config for [`AtrChannels`]: two independent period fields + two smoother slots (middle
/// MA + ATR smoother), and the band-width multiplier `k`.
///
/// Each slot defaults to `follow(kind)`: the MA smoother follows `ma_period`, the ATR
/// smoother follows `atr_period`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrcConfig {
    /// Period for the middle-line MA (feeds Close).
    pub ma_period: Param<usize>,
    /// Period for the ATR's internal TR smoother.
    pub atr_period: Param<usize>,
    /// Band-width multiplier.
    pub k: Param<f64>,
    /// Shape of the middle-line smoother — default `follow(Sma)` at `ma_period`.
    #[slot]
    pub ma_smoother: Param<SmootherChoice>,
    /// Shape of the ATR smoother — default `follow(Rma)` at `atr_period`.
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for AtrChannels {
    const ID: IndicatorId = IndicatorId::Atrc;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close feeds the middle MA; H/L/C feed the ATR.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    /// O(1): both smoothers run recursively; no window buffer on the outer struct.
    /// One inner port for the embedded Atr (the MA smoother is direct, not a sub-indicator).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = AtrcConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AtrcUpper),
        Output::price(IndicatorOutputId::AtrcMiddle),
        Output::price(IndicatorOutputId::AtrcLower),
    ];
    type Config = AtrcConfig;
    type Runtime = AtrChannels;

    fn create(cfg: AtrcConfig) -> AtrChannels {
        let ma_p   = cfg.ma_period.resolved();
        let atr_p  = cfg.atr_period.resolved();
        let ma_ch  = cfg.ma_smoother.resolved();
        let atr_ch = cfg.atr_smoother.resolved();
        AtrChannels {
            ma:     SmootherSlot::new(ma_ch.kind, ma_ch.period.resolve(ma_p)),
            atr:    Atr::from_smoother(atr_ch.period.resolve(atr_p), atr_ch.kind),
            k:      cfg.k.resolved(),
            upper:  0.0,
            middle: 0.0,
            lower:  0.0,
        }
    }

    fn slot_members(cfg: &AtrcConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrcConfig {
    fn defaults() -> Self {
        AtrcConfig {
            ma_period:   Param::Solo(20),
            atr_period:  Param::Solo(14),
            k:           Param::Solo(2.0),
            ma_smoother:  Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // ma_period, atr_period: Class A usize — auto range(2,4048,1)
        // #[slot] ma_smoother, atr_smoother: leave Solo (deferred sweep wave)
        let mut s = Self::machine_defaults_auto();
        s.k = Param::many(crate::contract::sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for AtrChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::AtrcUpper,  "ATR-C Upper",  Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::AtrcMiddle, "ATR-C Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::AtrcLower,  "ATR-C Lower",  Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_channels_creation() {
        let ac = AtrChannels::new(20, 14, 2.0);
        assert!(!ac.is_ready());
        assert_eq!(ac.upper(), 0.0);
        assert_eq!(ac.middle(), 0.0);
        assert_eq!(ac.lower(), 0.0);
    }

    #[test]
    fn test_atr_channels_warmup() {
        let mut ac = AtrChannels::new(20, 14, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ac.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_atr_channels_band_ordering() {
        let mut ac = AtrChannels::new(20, 14, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ac.feed(&[price + 1.0, price - 1.0, price]);
            if ac.is_ready() {
                assert!(ac.upper() >= ac.middle(), "Upper should be >= middle");
                assert!(ac.middle() >= ac.lower(), "Middle should be >= lower");
            }
        }
    }

    #[test]
    fn test_atr_channels_from_smoothers() {
        let mut ac = AtrChannels::from_smoothers(10, SmootherId::Ema, 7, SmootherId::Ema, 1.5);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ac.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_atr_channels_reset() {
        let mut ac = AtrChannels::new(20, 14, 2.0);
        for i in 0..25 {
            ac.feed(&[101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64]);
        }
        ac.reset();
        assert!(!ac.is_ready());
        assert_eq!(ac.upper(), 0.0);
        assert_eq!(ac.middle(), 0.0);
        assert_eq!(ac.lower(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_atrc() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Atrc(<<AtrChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price,
                high: price + 1.5,
                low:  price - 1.5,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let upper = f.read(IndicatorOutputId::AtrcUpper);
        let lower = f.read(IndicatorOutputId::AtrcLower);
        assert!(upper >= lower);
    }
}
