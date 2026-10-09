// Volatility of Volatility: rolling std of ATR (or abs returns) over window

use crate::engine::contract_engine::SmootherId;
use crate::indicators::volatility::atr::Atr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum VoVSource {
    Atr(usize, SmootherId),
    AbsReturn,
}

#[derive(Debug, Clone)]
pub struct VolOfVol {
    source: VoVSource,
    // internal state
    atr: Option<Atr>,
    prev_close: f64,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl VolOfVol {
    pub fn new(source: VoVSource, window: usize) -> Self {
        let atr = match source {
            VoVSource::Atr(p, t) => Some(Atr::from_smoother(p, t)),
            _ => None,
        };
        Self {
            source,
            atr,
            prev_close: 0.0,
            window: window.max(2),
            buf: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        if let Some(a) = self.atr.as_mut() {
            a.reset();
        }
        self.prev_close = 0.0;
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed resolved `[high, low, close]` lanes. The `AbsReturn` source rolls the
    /// std of |log-return| (close only); the `Atr` source feeds the embedded ATR
    /// (high/low/close) and rolls the std of the ATR series.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let x = match self.source {
            VoVSource::AbsReturn => {
                if self.prev_close <= 0.0 {
                    self.prev_close = close.max(1e-12);
                    return self.value;
                }
                let r = (close / self.prev_close).ln().abs();
                self.prev_close = close.max(1e-12);
                r
            }
            VoVSource::Atr(..) => match self.atr.as_mut() {
                Some(a) => a.feed(&[high, low, close]),
                None => return self.value,
            },
        };
        self.roll(x)
    }

    fn roll(&mut self, x: f64) -> f64 {
        let old = self.buf[self.idx];
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        self.sum += x - old;
        self.sumsq += x * x - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            self.value = var.max(0.0).sqrt();
        }
        self.value
    }


    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for VolOfVol {
    fn default() -> Self {
        Self::new(VoVSource::AbsReturn, 20)
    }
}

// -- contract -----------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`VolOfVol`] — volatility source + rolling std window.
///
/// `source` is swept as a `Param<VoVSource>` (AbsReturn or Atr variant);
/// `window` is swept as `Param<usize>`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VovConfig {
    pub source: Param<VoVSource>,
    pub window: Param<usize>,
}

impl Indicator for VolOfVol {
    const ID: IndicatorId = IndicatorId::Vov;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bar-fed: high/low/close (AbsReturn reads close; Atr needs the full range).
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Vov)];
    type Config = VovConfig;
    type Runtime = VolOfVol;

    fn create(cfg: VovConfig) -> VolOfVol {
        VolOfVol::new(cfg.source.resolved(), cfg.window.resolved())
    }
}

impl crate::contract::Config for VovConfig {
    fn defaults() -> Self {
        VovConfig {
            source: Param::Solo(VoVSource::AbsReturn),
            window: Param::Solo(20),
        }
    }
    fn machine_defaults() -> Self {
        // window: Class A usize — auto range(2,4048,1)
        // source: Class Q plain enum — all variants (AbsReturn, Atr with canonical period/smoother)
        let mut s = Self::machine_defaults_auto();
        s.source = Param::many(vec![
            VoVSource::AbsReturn,
            VoVSource::Atr(14, SmootherId::Rma),
        ]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolOfVol {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vov, "Vol of Vol", Color::hex(0x9C27B0))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_of_vol_creation_atr() {
        let vov = VolOfVol::new(VoVSource::Atr(14, SmootherId::Rma), 20);
        assert!(!vov.is_ready());
        assert_eq!(vov.value(), 0.0);
    }

    #[test]
    fn test_vol_of_vol_creation_abs_return() {
        let vov = VolOfVol::new(VoVSource::AbsReturn, 20);
        assert!(!vov.is_ready());
        assert_eq!(vov.value(), 0.0);
    }

    #[test]
    fn test_vol_of_vol_warmup() {
        let mut vov = VolOfVol::new(VoVSource::Atr(14, SmootherId::Rma), 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vov.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(vov.is_ready());
    }

    #[test]
    fn test_vol_of_vol_non_negative() {
        let mut vov = VolOfVol::new(VoVSource::AbsReturn, 20);
        for i in 0..35 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vov.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= 0.0, "VoV should be non-negative");
        }
    }

    #[test]
    fn test_vol_of_vol_reset() {
        let mut vov = VolOfVol::new(VoVSource::Atr(14, SmootherId::Rma), 20);
        for i in 0..30 {
            let p = 100.0 + i as f64;
            vov.feed(&[p + 1.0, p - 1.0, p]);
        }
        vov.reset();
        assert!(!vov.is_ready());
        assert_eq!(vov.value(), 0.0);
    }
}
