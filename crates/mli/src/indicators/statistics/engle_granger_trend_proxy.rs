// Engle–Granger Trend Proxy: OLS with trend term (time index) before residual ADF proxy

#[derive(Debug, Clone)]
pub struct EngleGrangerTrendProxy {
    window: usize,
    // rolling buffers for close and time
    closes: Vec<f64>,
    times: Vec<f64>,
    idx: usize,
    filled: bool,
    pub t_stat: f64,
}

impl EngleGrangerTrendProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(32);
        Self {
            window: w,
            closes: vec![0.0; w],
            times: (0..w).map(|i| i as f64).collect(),
            idx: 0,
            filled: false,
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> f64 {
        self.t_stat
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        self.closes[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.update_stats();
        }
        self.t_stat
    }

    fn update_stats(&mut self) {
        let n = self.window;
        let mut st = 0.0;
        let mut stt = 0.0;
        let mut sy = 0.0;
        let mut syt = 0.0;
        let count = n as f64;
        // regression y ~ a + c*t (trend-only with intercept)
        for i in 0..n {
            let y = self.closes[(self.idx + i) % n];
            let t = self.times[i];
            st += t;
            stt += t * t;
            sy += y;
            syt += y * t;
        }
        // Solve for c (trend coef) in normal equations [[n, sum t],[sum t, sum t2]] * [a,c] = [sum y, sum y t]
        let denom = count * stt - st * st;
        let c = if denom.abs() > 1e-12 {
            (count * syt - st * sy) / denom
        } else {
            0.0
        };
        // residuals = y - (a + c*t); compute AR(1) t-like stat on residuals
        let a = (sy - c * st) / count;
        let mut resid = vec![0.0; n];
        for (i, (slot, &t)) in resid.iter_mut().zip(self.times[..n].iter()).enumerate() {
            let y = self.closes[(self.idx + i) % n];
            *slot = y - (a + c * t);
        }
        let mut rx = 0.0;
        let mut ry = 0.0;
        let mut rxx = 0.0;
        let mut rxy = 0.0;
        let rc = (n - 1) as f64;
        for i in 1..n {
            let y = resid[i];
            let x = resid[i - 1];
            rx += x;
            ry += y;
            rxx += x * x;
            rxy += x * y;
        }
        let d = rc * rxx - rx * rx;
        let phi = if d.abs() > 1e-12 {
            (rc * rxy - rx * ry) / d
        } else {
            0.0
        };
        let mut se = 0.0;
        for i in 1..n {
            let y = resid[i];
            let x = resid[i - 1];
            let e = y - phi * x;
            se += e * e;
        }
        let var = se.max(1e-12) / (rc - 1.0).max(1.0);
        let se_phi = (var / (rxx - rx * rx / rc).max(1e-12)).sqrt();
        self.t_stat = if se_phi > 0.0 {
            (phi - 1.0) / se_phi
        } else {
            0.0
        };
    }
}

impl Default for EngleGrangerTrendProxy {
    /// Factory defaults: window=100 (clamped to max(100,32)=100).
    fn default() -> Self {
        Self::new(100)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`EngleGrangerTrendProxy`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EgTrendConfig {
    pub period: Param<usize>,
}

impl Indicator for EngleGrangerTrendProxy {
    const ID: IndicatorId = IndicatorId::EgTrend;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::EgTrend)];
    /// O(n): trend OLS + AR(1) residual pass over window.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = EgTrendConfig;
    type Runtime = EngleGrangerTrendProxy;

    fn create(cfg: EgTrendConfig) -> EngleGrangerTrendProxy {
        EngleGrangerTrendProxy::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &EgTrendConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for EgTrendConfig {
    fn defaults() -> Self {
        EgTrendConfig { period: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for EngleGrangerTrendProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EgTrend, "EG Trend", Color::hex(0x2196F3))
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
    fn test_engle_granger_trend_proxy_creation() {
        let egtp = EngleGrangerTrendProxy::new(50);
        assert!(!egtp.is_ready());
        assert_eq!(egtp.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_trend_proxy_warmup() {
        let mut egtp = EngleGrangerTrendProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            egtp.feed(price);
        }
        assert!(egtp.is_ready());
    }

    #[test]
    fn test_engle_granger_trend_proxy_values() {
        let mut egtp = EngleGrangerTrendProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = egtp.feed(price);
            assert!(value.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_engle_granger_trend_proxy_reset() {
        let mut egtp = EngleGrangerTrendProxy::new(50);
        for i in 0..60 {
            egtp.feed(100.0 + i as f64);
        }
        egtp.reset();
        assert!(!egtp.is_ready());
        assert_eq!(egtp.t_stat, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_eg_trend() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::EgTrend(<<EngleGrangerTrendProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..150 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::EgTrend).is_finite());
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<EngleGrangerTrendProxy as Indicator>::ID, IndicatorId::EgTrend);
    }
}
