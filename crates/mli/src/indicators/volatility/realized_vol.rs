// Realized Volatility over rolling window using log returns

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
#[derive(Debug, Clone)]
pub struct RealizedVol {
    window: usize,
    // ring buffer of squared log returns
    r2_buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    sum_r2: f64,
    value: f64,
    annualize_factor: f64,
}

impl RealizedVol {
    pub fn new(window: usize, annualize_factor: f64) -> Self {
        Self {
            window: window.max(1),
            r2_buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            prev_close: 0.0,
            sum_r2: 0.0,
            value: 0.0,
            annualize_factor,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.r2_buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.sum_r2 = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed ONE resolved scalar (close) — close-to-close realized vol.
    pub fn feed(&mut self, close: f64) -> f64 {
        if self.prev_close <= 0.0 {
            self.prev_close = close.max(1e-12);
            return self.value;
        }
        let r = (close / self.prev_close).ln();
        self.prev_close = close.max(1e-12);
        let r2 = r * r;

        // update ring buffer
        let old = self.r2_buffer[self.idx];
        self.r2_buffer[self.idx] = r2;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        // update sum
        self.sum_r2 += r2 - old;
        let denom = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if denom > 0.0 {
            let vol = (self.sum_r2 / denom).sqrt();
            self.value = if self.annualize_factor > 0.0 {
                vol * self.annualize_factor
            } else {
                vol
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

// -- contract -----------------------------------------------------------------

use crate::contract::{Param, Render};
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`RealizedVol`] — window + annualization factor.
/// `annualize_factor` is sqrt(252) by default (daily bars annualized); 0.0 = raw.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RvConfig {
    pub window: Param<usize>,
    /// Multiply per-bar RMS vol by this factor. `252.0_f64.sqrt()` for daily bars.
    pub annualize_factor: Param<f64>,
}

/// One period-deep ring of r^2 values (f64 per bar) -- the O(1) running-sum buffer.
static RV_STORES: &[Store] = &[Store::window(StoreKind::Vec)];

impl Indicator for RealizedVol {
    const ID: IndicatorId = IndicatorId::Rv;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close-to-close returns -- reads close intrinsically, not a swept field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// O(1): running sum^2 ring -- no per-bar rescan. Period-deep Vec for the ring.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, RV_STORES);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Rv)];
    type Config = RvConfig;
    type Runtime = RealizedVol;

    fn create(cfg: RvConfig) -> RealizedVol {
        RealizedVol::new(cfg.window.resolved(), cfg.annualize_factor.resolved())
    }
}

impl crate::contract::Config for RvConfig {
    fn defaults() -> Self {
        RvConfig { window: Param::Solo(21), annualize_factor: Param::Solo(252.0_f64.sqrt()) }
    }
    fn machine_defaults() -> Self {
        // window: Class A usize — auto range(2,4048,1)
        // annualize_factor: Class J PIN (discrete {252,365,8760} instrument/timeframe constant)
        //   — auto leaves f64 Solo, confirmed correct, do NOT sweep
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RealizedVol {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rv, "Realized Vol", Color::hex(0x2196F3))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realized_vol_creation() {
        let rv = RealizedVol::new(20, 252.0);
        assert!(!rv.is_ready());
        assert_eq!(rv.value(), 0.0);
    }

    #[test]
    fn test_realized_vol_warmup() {
        let mut rv = RealizedVol::new(20, 252.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rv.feed(price);
        }
        assert!(rv.is_ready());
    }

    #[test]
    fn test_realized_vol_positive() {
        let mut rv = RealizedVol::new(20, 252.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let value = rv.feed(price);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_realized_vol_reset() {
        let mut rv = RealizedVol::new(20, 252.0);
        for i in 0..25 {
            rv.feed(100.0 + i as f64);
        }
        rv.reset();
        assert!(!rv.is_ready());
        assert_eq!(rv.value(), 0.0);
    }
}
