// KPSS-trend proxy: test against deterministic trend (demean + detrend)

#[derive(Debug, Clone)]
pub struct KpssTrendProxy {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl KpssTrendProxy {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(50, 2048);
        Self {
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.value = self.compute_lm_proxy_trend();
        }
        self.value
    }

    fn compute_lm_proxy_trend(&self) -> f64 {
        let n = self.window;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0; // OLS y~t
        for i in 0..n {
            let t = (i + 1) as f64;
            let y = self.buf[(self.idx + i) % n];
            sx += t;
            sy += y;
            sxx += t * t;
            sxy += t * y;
        }
        let den = (n as f64) * sxx - sx * sx;
        if den.abs() < 1e-12 {
            return 0.0;
        }
        let a = (sxx * sy - sx * sxy) / den; // intercept
        let b = ((n as f64) * sxy - sx * sy) / den; // slope
                                                      // residuals from trend
        let mut eps = vec![0.0; n];
        for (i, slot) in eps.iter_mut().enumerate() {
            let t = (i + 1) as f64;
            let y = self.buf[(self.idx + i) % n];
            *slot = y - (a + b * t);
        }
        // partial sums
        let mut s = 0.0;
        let mut s2_sum = 0.0;
        for &e in &eps {
            s += e;
            s2_sum += s * s;
        }
        // long-run variance proxy (as in level version)
        let mut var = 0.0;
        let mut cov1 = 0.0;
        let e_mean: f64 = eps.iter().sum::<f64>() / n as f64;
        for &e in &eps {
            let d = e - e_mean;
            var += d * d;
        }
        for w in eps.windows(2) {
            cov1 += (w[1] - e_mean) * (w[0] - e_mean);
        }
        var /= n as f64;
        cov1 /= (n - 1) as f64;
        let lrvar = (var + 2.0 * cov1.max(0.0)).max(1e-12);
        (s2_sum / (n as f64 * lrvar)).max(0.0)
    }

    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for KpssTrendProxy {
    /// Factory defaults: window=50 (clamped to [50,2048]).
    fn default() -> Self {
        Self::new(50)
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

/// Own config for [`KpssTrendProxy`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KpssTrendConfig {
    pub period: Param<usize>,
}

impl Indicator for KpssTrendProxy {
    const ID: IndicatorId = IndicatorId::KpssTrend;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::KpssTrend)];
    /// O(n): OLS trend fit + partial sums over window.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = KpssTrendConfig;
    type Runtime = KpssTrendProxy;

    fn create(cfg: KpssTrendConfig) -> KpssTrendProxy {
        KpssTrendProxy::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &KpssTrendConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for KpssTrendConfig {
    fn defaults() -> Self {
        KpssTrendConfig { period: Param::Solo(100) }
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


impl Render for KpssTrendProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::KpssTrend, "KPSS Trend", Color::hex(0x9C27B0))
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
    fn test_kpss_trend_proxy_creation() {
        let kpss = KpssTrendProxy::new(50);
        assert!(!kpss.is_ready());
        assert_eq!(kpss.value, 0.0);
    }

    #[test]
    fn test_kpss_trend_proxy_warmup() {
        let mut kpss = KpssTrendProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kpss.feed(price);
        }
        assert!(kpss.is_ready());
    }

    #[test]
    fn test_kpss_trend_proxy_non_negative() {
        let mut kpss = KpssTrendProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kpss.feed(price);
            assert!(value >= 0.0, "KPSS trend should be non-negative");
        }
    }

    #[test]
    fn test_kpss_trend_proxy_reset() {
        let mut kpss = KpssTrendProxy::new(50);
        for i in 0..60 {
            kpss.feed(100.0 + i as f64);
        }
        kpss.reset();
        assert!(!kpss.is_ready());
        assert_eq!(kpss.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_kpss_trend() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::KpssTrend(<<KpssTrendProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..120 {
            let p = 100.0 + i as f64 * 0.5;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::KpssTrend).is_finite());
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<KpssTrendProxy as Indicator>::ID, IndicatorId::KpssTrend);
    }
}
