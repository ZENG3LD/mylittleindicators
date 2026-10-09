// R-Squared (coefficient of determination) via rolling linear regression proxy


#[derive(Debug, Clone)]
pub struct RSquared {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl RSquared {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(5, 1024),
            buf: Vec::with_capacity(window.clamp(5, 1024)),
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed one scalar (the resolved price field). Uses only close by default.
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let n = self.window as f64;
            let sx: f64 = (0..self.window).map(|i| i as f64).sum();
            let sy: f64 = self.buf.iter().sum();
            let sxx: f64 = (0..self.window).map(|i| (i as f64).powi(2)).sum();
            let sxy: f64 = (0..self.window).map(|i| (i as f64) * self.buf[i]).sum();
            let syy: f64 = self.buf.iter().map(|&y| y * y).sum();
            let num = (n * sxy - sx * sy).powi(2);
            let den = (n * sxx - sx * sx) * (n * syy - sy * sy);
            self.value = if den.abs() > 1e-12 { num / den } else { 0.0 };
        }
        self.value
    }
}

impl Default for RSquared {
    /// Factory default: window=20.
    fn default() -> Self {
        Self::new(20)
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`RSquared`] — period-only, no smoother slot.
///
/// Every field is a [`Param`] — `Solo` = one value, `Many` = a swept set.
/// `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RSquaredConfig {
    pub period: Param<usize>,
}

impl Indicator for RSquared {
    const ID: IndicatorId = IndicatorId::RSquared;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable price field — default close.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: crate::engine::ohlcv_field::OhlcvField::Close });
    /// O(period) per bar — rescans the window to compute R².
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::RSquared)];
    type Config = RSquaredConfig;
    type Runtime = RSquared;

    fn create(cfg: RSquaredConfig) -> RSquared {
        RSquared::new(cfg.period.resolved())
    }

    // source_fields not overridden: const SOURCE = Field{Close} → default returns Close.
}

impl crate::contract::Config for RSquaredConfig {
    fn defaults() -> Self {
        RSquaredConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RSquared {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RSquared, "R-Squared", Color::hex(0x009688))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r_squared_creation() {
        let r2 = RSquared::new(20);
        assert!(!r2.is_ready());
        assert_eq!(r2.value(), 0.0);
    }

    #[test]
    fn test_r_squared_warmup() {
        let mut r2 = RSquared::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            r2.feed(price);
        }
        assert!(r2.is_ready());
    }

    #[test]
    fn test_r_squared_range() {
        let mut r2 = RSquared::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = r2.feed(price);
            assert!(value >= 0.0 && value <= 1.0, "R^2 should be in [0, 1]");
        }
    }

    #[test]
    fn test_r_squared_reset() {
        let mut r2 = RSquared::new(20);
        for i in 0..25 {
            r2.feed(100.0 + i as f64);
        }
        r2.reset();
        assert!(!r2.is_ready());
        assert_eq!(r2.value(), 0.0);
    }

    /// Factory resolves close (not the wild 9999.0 high) and feeds it.
    /// A linear close ramp has R²=1.0 (perfect linear fit).
    #[test]
    fn factory_feeds_resolved_rsquared() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<RSquared as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::RSquared(cfg).build_solo().unwrap();
        for i in 1..=40 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0,
                close: 100.0 + i as f64,
                volume: 1000.0,
            });
        }
        // Perfect linear series → R² should be very close to 1.0
        let v = f.primary();
        assert!(v > 0.99, "R² on a linear close ramp should be ~1.0, got {v}");
    }
}
