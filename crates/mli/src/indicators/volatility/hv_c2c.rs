// Historical Volatility (Close-to-Close) - annualized std dev of log returns

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

#[derive(Debug, Clone)]
pub struct HistoricalVolatilityC2C {
    window: usize,
    rets: Vec<f64>,
    idx: usize,
    count: usize,
    prev_close: f64,
    initialized: bool,
    value: f64,
}

impl HistoricalVolatilityC2C {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(5, 1024),
            rets: Vec::with_capacity(window.clamp(5, 1024)),
            idx: 0,
            count: 0,
            prev_close: 0.0,
            initialized: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rets.clear();
        self.idx = 0;
        self.count = 0;
        self.prev_close = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed ONE resolved scalar (close) — close-to-close realized estimator.
    pub fn feed(&mut self, c: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.value;
        }
        let ret = (c / self.prev_close).ln();
        self.prev_close = c;
        if self.count < self.window {
            self.rets.push(ret);
            self.count += 1;
        } else {
            self.rets[self.idx] = ret;
        }
        self.idx = (self.idx + 1) % self.window;

        if self.is_ready() {
            // compute mean
            let n = self.window as f64;
            let mut sum = 0.0;
            for &r in &self.rets[0..self.window] {
                sum += r;
            }
            let mean = sum / n;
            let mut var = 0.0;
            for &r in &self.rets[0..self.window] {
                let d = r - mean;
                var += d * d;
            }
            var /= n.max(1.0);
            let daily_vol = var.sqrt();
            self.value = daily_vol * (252.0_f64).sqrt();
        } else {
            self.value = 0.0;
        }
        self.value
    }
}

// -- contract -----------------------------------------------------------------

use crate::contract::{Param, Render};
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`HistoricalVolatilityC2C`] — period only.
///
/// Distinct from [`super::realized_vol::RvConfig`]: Hvc2c computes population
/// std dev (two-pass rescan for mean+variance) and annualizes by sqrt(252). Rv uses
/// a running RMS ring (no rescan). Same domain concept, different math + cost.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct Hvc2cConfig {
    pub period: Param<usize>,
}

/// Period-deep ring of log-returns (f64 per bar).
static HVC2C_STORES: &[Store] = &[Store::window(StoreKind::Vec)];

impl Indicator for HistoricalVolatilityC2C {
    const ID: IndicatorId = IndicatorId::Hvc2c;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close-to-close returns -- reads close intrinsically, not a swept field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// Reads only close -> default `Field(Close)` SOURCE applies.
    /// O(period): two-pass per-bar rescan (sum for mean, then sum for variance).
    const COST: Cost = Cost::new(UpdateComplexity::Linear, HVC2C_STORES);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Hvc2c)];
    type Config = Hvc2cConfig;
    type Runtime = HistoricalVolatilityC2C;

    fn create(cfg: Hvc2cConfig) -> HistoricalVolatilityC2C {
        HistoricalVolatilityC2C::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for Hvc2cConfig {
    fn defaults() -> Self {
        Hvc2cConfig { period: Param::Solo(30) }
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


impl Render for HistoricalVolatilityC2C {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Hvc2c, "HV C2C", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hv_c2c_creation() {
        let hv = HistoricalVolatilityC2C::new(20);
        assert!(!hv.is_ready());
        assert_eq!(hv.value(), 0.0);
    }

    #[test]
    fn test_hv_c2c_warmup() {
        let mut hv = HistoricalVolatilityC2C::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            hv.feed(price);
        }
        assert!(hv.is_ready());
    }

    #[test]
    fn test_hv_c2c_positive() {
        let mut hv = HistoricalVolatilityC2C::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let value = hv.feed(price);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_hv_c2c_reset() {
        let mut hv = HistoricalVolatilityC2C::new(20);
        for i in 0..25 {
            hv.feed(100.0 + i as f64);
        }
        hv.reset();
        assert!(!hv.is_ready());
        assert_eq!(hv.value(), 0.0);
    }
}
