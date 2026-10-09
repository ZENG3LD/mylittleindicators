// Close-to-Close Volatility Percentile over rolling window

#[derive(Debug, Clone)]
pub struct CloseVolPercentile {
    ret_prev_close: f64,
    // ring for recent vol values
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
    percentile: f64,
}

impl CloseVolPercentile {
    pub fn new(_vol_window: usize, percentile_window: usize) -> Self {
        Self {
            ret_prev_close: 0.0,
            window: percentile_window.max(1),
            buf: vec![0.0; percentile_window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
            percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ret_prev_close = 0.0;
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }


    /// Feed the resolved scalar (close, the factory extracts the configured source field).
    /// Knows no transport.
    pub fn feed(&mut self, close: f64) -> (f64, f64) {
        if self.ret_prev_close <= 0.0 {
            self.ret_prev_close = close.max(1e-12);
            return (self.value, self.percentile);
        }
        let r = (close / self.ret_prev_close).ln();
        self.ret_prev_close = close.max(1e-12);

        // Absolute return as vol proxy; percentile rank over rolling window.
        self.value = r.abs();

        let _old = self.buf[self.idx];
        self.buf[self.idx] = self.value;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.buf[i] <= self.value {
                    count_le += 1;
                }
            }
            self.percentile = count_le as f64 / len as f64;
        } else {
            self.percentile = 0.0;
        }
        (self.value, self.percentile)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.percentile
    }

}

impl Default for CloseVolPercentile {
    fn default() -> Self {
        Self::new(50, 100)
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

/// Typed dual-mode config for [`CloseVolPercentile`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct C2cvpConfig {
    pub period: Param<usize>,
}

impl Indicator for CloseVolPercentile {
    const ID: IndicatorId = IndicatorId::C2cvp;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads only close (log-return percentile rank).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::C2cvp)];
    type Config = C2cvpConfig;
    type Runtime = CloseVolPercentile;

    fn create(cfg: C2cvpConfig) -> CloseVolPercentile {
        let p = cfg.period.resolved();
        CloseVolPercentile::new(p, p)
    }
}

impl crate::contract::Config for C2cvpConfig {
    fn defaults() -> Self {
        C2cvpConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for CloseVolPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::C2cvp, "C2C Vol %ile", Color::hex(0x009688))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_vol_percentile_creation() {
        let cvp = CloseVolPercentile::new(20, 50);
        assert!(!cvp.is_ready());
        assert_eq!(cvp.value(), 0.0);
    }

    #[test]
    fn test_close_vol_percentile_warmup() {
        let mut cvp = CloseVolPercentile::new(20, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cvp.feed(price);
        }
        assert!(cvp.is_ready());
    }

    #[test]
    fn test_close_vol_percentile_range() {
        let mut cvp = CloseVolPercentile::new(20, 50);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let (_, pct) = cvp.feed(price);
            assert!(pct >= 0.0 && pct <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_close_vol_percentile_reset() {
        let mut cvp = CloseVolPercentile::new(20, 50);
        for i in 0..60 {
            cvp.feed(100.0 + i as f64);
        }
        cvp.reset();
        assert!(!cvp.is_ready());
        assert_eq!(cvp.value(), 0.0);
    }
}
