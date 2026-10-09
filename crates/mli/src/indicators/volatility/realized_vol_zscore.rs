// Realized Volatility Z-Score: zscore of rolling close-to-close volatility

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

#[derive(Debug, Clone)]
pub struct RealizedVolZscore {
    // for vol estimation over returns
    vol_period: usize,
    r_buf: Vec<f64>,
    r_idx: usize,
    r_filled: bool,
    sum_r: f64,
    sum_r2: f64,
    prev_close: f64,
    curr_vol: f64,

    // for zscore of vol values
    zs_window: usize,
    v_buf: Vec<f64>,
    v_idx: usize,
    v_filled: bool,
    sum_v: f64,
    sum_v2: f64,
    zscore: f64,
}

impl RealizedVolZscore {
    pub fn new(vol_period: usize, zscore_window: usize) -> Self {
        let vp = vol_period.max(2);
        let zw = zscore_window.max(2);
        Self {
            vol_period: vp,
            r_buf: vec![0.0; vp],
            r_idx: 0,
            r_filled: false,
            sum_r: 0.0,
            sum_r2: 0.0,
            prev_close: 0.0,
            curr_vol: 0.0,
            zs_window: zw,
            v_buf: vec![0.0; zw],
            v_idx: 0,
            v_filled: false,
            sum_v: 0.0,
            sum_v2: 0.0,
            zscore: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.r_buf.fill(0.0);
        self.r_idx = 0;
        self.r_filled = false;
        self.sum_r = 0.0;
        self.sum_r2 = 0.0;
        self.prev_close = 0.0;
        self.curr_vol = 0.0;
        self.v_buf.fill(0.0);
        self.v_idx = 0;
        self.v_filled = false;
        self.sum_v = 0.0;
        self.sum_v2 = 0.0;
        self.zscore = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.v_filled
    }

    /// Feed ONE resolved scalar (close).
    pub fn feed(&mut self, close: f64) -> (f64, f64) {
        // update return
        if self.prev_close <= 0.0 {
            self.prev_close = close.max(1e-12);
            return (self.curr_vol, self.zscore);
        }
        let r = (close / self.prev_close).ln();
        self.prev_close = close.max(1e-12);

        // rolling stats for returns
        let old_r = self.r_buf[self.r_idx];
        self.r_buf[self.r_idx] = r;
        self.r_idx = (self.r_idx + 1) % self.vol_period;
        if self.r_idx == 0 {
            self.r_filled = true;
        }
        self.sum_r += r - old_r;
        self.sum_r2 += r * r - old_r * old_r;

        // compute current vol
        let n = if self.r_filled {
            self.vol_period as f64
        } else {
            self.r_idx as f64
        };
        if n >= 2.0 {
            let mean_r = self.sum_r / n;
            let var_r = (self.sum_r2 / n) - mean_r * mean_r;
            self.curr_vol = if var_r > 0.0 { var_r.sqrt() } else { 0.0 };
        } else {
            self.curr_vol = 0.0;
        }

        // feed vol into zscore ring
        let old_v = self.v_buf[self.v_idx];
        self.v_buf[self.v_idx] = self.curr_vol;
        self.v_idx = (self.v_idx + 1) % self.zs_window;
        if self.v_idx == 0 {
            self.v_filled = true;
        }
        self.sum_v += self.curr_vol - old_v;
        self.sum_v2 += self.curr_vol * self.curr_vol - old_v * old_v;
        let m = if self.v_filled {
            self.zs_window as f64
        } else {
            self.v_idx as f64
        };
        if m >= 2.0 {
            let mean_v = self.sum_v / m;
            let var_v = (self.sum_v2 / m) - mean_v * mean_v;
            let std_v = if var_v > 0.0 { var_v.sqrt() } else { 0.0 };
            self.zscore = if std_v > 1e-12 {
                (self.curr_vol - mean_v) / std_v
            } else {
                0.0
            };
        } else {
            self.zscore = 0.0;
        }

        (self.curr_vol, self.zscore)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.zscore
    }
}

// -- contract -----------------------------------------------------------------

use crate::contract::{Param, Render};
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`RealizedVolZscore`] — two window params.
/// `vol_period` = rolling window for close-to-close variance;
/// `zscore_window` = rolling window for z-scoring the resulting vol series.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RvzConfig {
    pub vol_period: Param<usize>,
    pub zscore_window: Param<usize>,
}

/// Two period-deep rings: one for returns (vol_period), one for vol values (zscore_window).
static RVZ_STORES: &[Store] = &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)];

impl Indicator for RealizedVolZscore {
    const ID: IndicatorId = IndicatorId::Rvz;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close-to-close returns -- reads close intrinsically, not a swept field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// Reads only close -> default `Field(Close)` SOURCE applies.
    /// O(1): both inner rings evict with running sums -- no per-bar rescan.
    /// Two Vec(Window) stores: r_buf (vol_period) + v_buf (zscore_window).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, RVZ_STORES);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Rvz)];
    type Config = RvzConfig;
    type Runtime = RealizedVolZscore;

    fn create(cfg: RvzConfig) -> RealizedVolZscore {
        RealizedVolZscore::new(cfg.vol_period.resolved(), cfg.zscore_window.resolved())
    }
}

impl crate::contract::Config for RvzConfig {
    fn defaults() -> Self {
        RvzConfig { vol_period: Param::Solo(21), zscore_window: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // vol_period, zscore_window: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RealizedVolZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rvz, "RV Z-Score", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realized_vol_zscore_creation() {
        let rvz = RealizedVolZscore::new(20, 50);
        assert!(!rvz.is_ready());
        assert_eq!(rvz.value(), 0.0);
    }

    #[test]
    fn test_realized_vol_zscore_warmup() {
        let mut rvz = RealizedVolZscore::new(20, 50);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rvz.feed(price);
        }
        assert!(rvz.is_ready());
    }

    #[test]
    fn test_realized_vol_zscore_values() {
        let mut rvz = RealizedVolZscore::new(20, 50);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (vol, zscore) = rvz.feed(price);
            assert!(vol >= 0.0, "Volatility should be non-negative");
            assert!(zscore.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_realized_vol_zscore_reset() {
        let mut rvz = RealizedVolZscore::new(20, 50);
        for i in 0..70 {
            rvz.feed(100.0 + i as f64);
        }
        rvz.reset();
        assert!(!rvz.is_ready());
        assert_eq!(rvz.value(), 0.0);
    }
}
