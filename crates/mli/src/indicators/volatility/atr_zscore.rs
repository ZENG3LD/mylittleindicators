// ATR Z-Score over rolling window

use crate::indicators::volatility::atr::Atr;

#[derive(Debug, Clone)]
pub struct AtrZscore {
    atr: Atr,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl AtrZscore {
    /// Default ctor — inner ATR smoothed with RMA (Wilder).
    pub fn new(atr_period: usize, window: usize) -> Self {
        Self::from_smoother(atr_period, SmootherId::Rma, window)
    }

    /// Build from a narrow `SmootherId` for the inner ATR smoother + ATR period + z-score window.
    pub fn from_smoother(atr_period: usize, smoother: SmootherId, window: usize) -> Self {
        Self::from_atr(Atr::from_smoother(atr_period, smoother), window)
    }

    fn from_atr(atr: Atr, window: usize) -> Self {
        let w = window.max(2);
        Self {
            atr,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.atr.is_ready()
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let atr_v = self.atr.feed(&[high, low, close]);

        // rolling mean/std via ring buffer
        let old = self.buf[self.idx];
        self.buf[self.idx] = atr_v;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        self.sum += atr_v - old;
        self.sumsq += atr_v * atr_v - old * old;

        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.value = if std > 1e-12 {
                (atr_v - mean) / std
            } else {
                0.0
            };
        } else {
            self.value = 0.0;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{SmootherId, SmootherChoice};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`AtrZscore`] — z-score of ATR over a rolling `window`.
/// `atr_period` = ATR period, `window` = z-score lookback, `atr_smoother` selects the
/// inner ATR smoother (default `follow(Rma)`, Wilder).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrZscoreConfig {
    pub atr_period: Param<usize>,
    pub window: Param<usize>,
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for AtrZscore {
    const ID: IndicatorId = IndicatorId::Atrz;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices -- the inner ATR is h/l/prev-close.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1) rolling-moment update over a `window`-deep heap ring of ATR values; the
    /// inner ATR's smoothing buffer cost lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = AtrZscoreConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Atrz)];
    type Config = AtrZscoreConfig;
    type Runtime = AtrZscore;

    fn create(cfg: AtrZscoreConfig) -> AtrZscore {
        let period = cfg.atr_period.resolved();
        let choice = cfg.atr_smoother.resolved();
        let atr_period = choice.period.resolve(period);
        AtrZscore::from_atr(
            Atr::from_smoother(atr_period, choice.kind),
            cfg.window.resolved(),
        )
    }

    fn slot_members(cfg: &AtrZscoreConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrZscoreConfig {
    fn defaults() -> Self {
        AtrZscoreConfig {
            atr_period: Param::Solo(14),
            window: Param::Solo(100),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // atr_period, window: Class A usize — auto range(2,4048,1)
        // #[slot] atr_smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AtrZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Atrz, "ATR Z", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Param;

    #[test]
    fn test_atr_zscore_creation() {
        let az = AtrZscore::new(14, 50);
        assert!(!az.is_ready());
        assert_eq!(az.value(), 0.0);
    }

    #[test]
    fn test_atr_zscore_warmup() {
        let mut az = AtrZscore::new(14, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            az.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(az.is_ready());
    }

    #[test]
    fn test_atr_zscore_values() {
        let mut az = AtrZscore::new(14, 50);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let value = az.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_atr_zscore_reset() {
        let mut az = AtrZscore::new(14, 50);
        for i in 0..60 {
            az.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        az.reset();
        assert!(!az.is_ready());
        assert_eq!(az.value(), 0.0);
    }

    #[test]
    fn test_atr_zscore_contract_create() {
        let cfg = <<AtrZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.atr_period.resolved(), 14);
        assert_eq!(cfg.window.resolved(), 100);
        let mut az = <AtrZscore as Indicator>::create(cfg);
        for i in 0..120 {
            let price = 100.0 + i as f64;
            az.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(az.is_ready());
    }

    #[test]
    fn test_atr_zscore_own_period() {
        let cfg = AtrZscoreConfig {
            atr_period: Param::Solo(14),
            window: Param::Solo(50),
            atr_smoother: Param::Solo(SmootherChoice::own(SmootherId::Ema, 10)),
        };
        let mut az = <AtrZscore as Indicator>::create(cfg);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            az.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(az.is_ready());
    }
}
