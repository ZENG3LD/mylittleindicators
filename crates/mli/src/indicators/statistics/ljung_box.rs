// Rolling Ljung-Box Q statistic over k lags on returns; outputs Q (proxy for p-value usage upstream)

#[derive(Debug, Clone)]
pub struct LjungBox {
    window: usize,
    lags: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    q_stat: f64,
}

impl LjungBox {
    pub fn new(window: usize, lags: usize) -> Self {
        let w = window.max(20);
        let k = lags.max(1).min(w / 2);
        Self {
            window: w,
            lags: k,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            q_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.q_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.q_stat = self.compute_q();
            }
        }
        self.last_close = Some(close);
        self.q_stat
    }

    fn compute_q(&self) -> f64 {
        let n = self.window as f64;
        // mean-adjusted
        let mut mean = 0.0;
        for v in &self.vals {
            mean += *v;
        }
        mean /= self.window as f64;
        let x: Vec<f64> = self.vals.iter().map(|&v| v - mean).collect();
        let mut var = 0.0;
        for v in &x {
            var += *v * *v;
        }
        var /= n;
        if var <= 1e-12 {
            return 0.0;
        }
        let mut q = 0.0;
        for h in 1..=self.lags {
            // autocorr at lag h
            let mut num = 0.0;
            for t in h..self.window {
                num += x[t] * x[t - h];
            }
            let rho_h = num / (n * var);
            q += rho_h * rho_h / (n - h as f64);
        }
        q * n * (n + 2.0)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.q_stat
    }

}

impl Default for LjungBox {
    /// Factory defaults: window=200 (clamped to max(200,20)=200), lags=10.
    fn default() -> Self {
        Self::new(200, 10)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, StoreKind, Store, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`LjungBox`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LjungBoxConfig {
    pub period: Param<usize>,
    pub lags: Param<usize>,
}

impl Indicator for LjungBox {
    const ID: IndicatorId = IndicatorId::LjungBox;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::LjungBox)];
    /// O(lags * n): autocorrelation sums over window per lag.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = LjungBoxConfig;
    type Runtime = LjungBox;

    fn create(cfg: LjungBoxConfig) -> LjungBox {
        LjungBox::new(cfg.period.resolved(), cfg.lags.resolved())
    }

    fn source_fields(_cfg: &LjungBoxConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for LjungBoxConfig {
    fn defaults() -> Self {
        LjungBoxConfig { period: Param::Solo(200), lags: Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // lags: Class A.model (autocorrelation lag order) → range(0,5,1), corrected off auto 2..4048.
        let mut s = Self::machine_defaults_auto();
        s.lags = Param::range(0, 5, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LjungBox {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LjungBox, "Ljung-Box", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::indicator_id::IndicatorId;
    use crate::contract::Indicator;

    #[test]
    fn test_ljung_box_creation() {
        let lb = LjungBox::new(50, 10);
        assert!(!lb.is_ready());
        assert_eq!(lb.value(), 0.0);
    }

    #[test]
    fn test_ljung_box_warmup() {
        let mut lb = LjungBox::new(50, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            lb.feed(price);
        }
        assert!(lb.is_ready());
    }

    #[test]
    fn test_ljung_box_non_negative() {
        let mut lb = LjungBox::new(50, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = lb.feed(price);
            assert!(value >= 0.0, "Q-stat should be non-negative");
        }
    }

    #[test]
    fn test_ljung_box_reset() {
        let mut lb = LjungBox::new(50, 10);
        for i in 0..60 {
            lb.feed(100.0 + i as f64);
        }
        lb.reset();
        assert!(!lb.is_ready());
        assert_eq!(lb.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_ljung_box() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::LjungBox(<<LjungBox as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..250 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::LjungBox) >= 0.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<LjungBox as Indicator>::ID, IndicatorId::LjungBox);
    }
}
