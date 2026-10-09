// ATR Percentile over rolling window
// Returns percentile rank of current ATR within the last N ATR values [0..1]

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::SmootherId;

#[derive(Debug, Clone)]
pub struct AtrPercentile {
    atr: Atr,
    window: usize,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl AtrPercentile {
    /// Primary constructor: period + window, RMA smoother.
    /// Transitional bridge: un-contracted callers (atr_percentile_trend.rs, kalman_regime_composite.rs,
    /// regime_composite.rs) call `new(atr_period, ma_type, window)` with 3 args; this 3-arg overload
    /// maps `SmootherId` to a `SmootherId` via `to_smoother()`.
    /// Remove 3-arg path when those callers are contracted.
    pub fn new(atr_period: usize, ma_type: crate::engine::contract_engine::SmootherId, window: usize) -> Self {
        Self::from_smoothers(atr_period, ma_type, window)
    }

    /// 2-arg constructor for the contracted default path (RMA smoother).
    pub fn with_period(atr_period: usize, window: usize) -> Self {
        Self::from_smoothers(atr_period, SmootherId::Rma, window)
    }

    pub fn from_smoothers(atr_period: usize, smoother: SmootherId, window: usize) -> Self {
        Self {
            atr: Atr::from_smoother(atr_period, smoother),
            window,
            buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];
        let atr_val = self.atr.feed(&[high, low, close]).abs();
        self.buffer[self.idx] = atr_val;
        self.idx = (self.idx + 1) % self.window.max(1);
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        // Percentile rank
        let len = if self.filled { self.window } else { self.idx };
        if len == 0 {
            self.value = 0.0;
            return self.value;
        }
        let mut count = 0usize;
        for i in 0..len {
            if self.buffer[i] <= atr_val {
                count += 1;
            }
        }
        self.value = (count as f64) / (len as f64);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    // Transitional bridge: atr_percentile_trend.rs, kalman_regime_composite.rs,
    // regime_composite.rs still call update_bar(o,h,l,c,v); delegates to feed(&[h,l,c]).
    // Remove when those callers are contracted.

}

impl Default for AtrPercentile {
    fn default() -> Self {
        Self::with_period(14, 100)
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Config for [`AtrPercentile`]: inner ATR period + smoother shape, and the percentile
/// rolling window.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrPercentileConfig {
    /// ATR lookback period.
    pub atr_period: Param<usize>,
    /// Percentile rolling window depth.
    pub window: Param<usize>,
    /// ATR smoother — default `follow(Rma)` at `atr_period`.
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for AtrPercentile {
    const ID: IndicatorId = IndicatorId::Atrp;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    /// Linear: percentile rank rescans the buffer each update.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = AtrPercentileConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Atrp)];
    type Config = AtrPercentileConfig;
    type Runtime = AtrPercentile;

    fn create(cfg: AtrPercentileConfig) -> AtrPercentile {
        let p  = cfg.atr_period.resolved();
        let ch = cfg.atr_smoother.resolved();
        let w  = cfg.window.resolved();
        AtrPercentile {
            atr: Atr::from_smoother(ch.period.resolve(p), ch.kind),
            window: w,
            buffer: vec![0.0; w.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    fn slot_members(cfg: &AtrPercentileConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrPercentileConfig {
    fn defaults() -> Self {
        AtrPercentileConfig {
            atr_period:   Param::Solo(14),
            window:       Param::Solo(100),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // atr_period, window: Class A usize — auto range(2,4048,1)
        // #[slot] atr_smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for AtrPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Atrp, "ATR %", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_percentile_creation() {
        let ap = AtrPercentile::with_period(14, 50);
        assert!(!ap.is_ready());
        assert_eq!(ap.value(), 0.0);
    }

    #[test]
    fn test_atr_percentile_warmup() {
        let mut ap = AtrPercentile::with_period(14, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ap.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ap.is_ready());
    }

    #[test]
    fn test_atr_percentile_range() {
        let mut ap = AtrPercentile::with_period(14, 50);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let value = ap.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value >= 0.0 && value <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_atr_percentile_reset() {
        let mut ap = AtrPercentile::with_period(14, 50);
        for i in 0..60 {
            ap.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        ap.reset();
        assert!(!ap.is_ready());
        assert_eq!(ap.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_atrp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Atrp(<<AtrPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,   // not used by H/L/C source
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0, // not used
            });
        }
        assert!(f.read(IndicatorOutputId::Atrp) >= 0.0 && f.read(IndicatorOutputId::Atrp) <= 1.0);
    }
}
