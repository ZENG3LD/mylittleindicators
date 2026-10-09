//! KPSS level-stationarity LM statistic over a rolling window.
//!
//! The LM construction (demean → partial sums S_t → Σ S_t² / (n²·LRV)) was
//! already correct; the only weakness was a fixed lag-1 Newey-West long-run
//! variance. Now uses the shared data-adaptive Bartlett-kernel estimator
//! (`newey_west_lrv`, automatic bandwidth). Emits the raw LM statistic; a
//! regime filter compares it to `kpss_critical_values(false)` (level): LM >
//! 0.463 ⇒ reject stationarity at 5%.

use crate::indicators::utils::math::timeseries::newey_west_lrv;

#[derive(Debug, Clone)]
pub struct KpssProxy {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl KpssProxy {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(50, 1024);
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

    /// Returns LM statistic as main value
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.value = self.compute_lm();
        }
        self.value
    }

    fn compute_lm(&self) -> f64 {
        let n = self.window;
        let mean: f64 = self.buf.iter().sum::<f64>() / n as f64;
        // Demeaned residuals (level-stationarity → demean only).
        let eps: Vec<f64> = (0..n).map(|i| self.buf[(self.idx + i) % n] - mean).collect();
        // Partial sums S_t = Σ_{i=1..t} eps_i, accumulate Σ S_t².
        let mut s = 0.0;
        let mut s2_sum = 0.0;
        for &e in &eps {
            s += e;
            s2_sum += s * s;
        }
        // Long-run variance via the shared data-adaptive Bartlett estimator
        // (auto bandwidth), replacing the fixed lag-1 approximation.
        let lrvar = newey_west_lrv(&eps, None).max(1e-12);
        // KPSS LM = (1/n²) Σ S_t² / LRV.
        (s2_sum / (n as f64 * n as f64 * lrvar)).max(0.0)
    }

}

impl Default for KpssProxy {
    /// Factory defaults: window=200 (clamped to [50,1024]).
    fn default() -> Self {
        Self::new(200)
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

/// Own config for [`KpssProxy`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KpssConfig {
    pub period: Param<usize>,
}

impl Indicator for KpssProxy {
    const ID: IndicatorId = IndicatorId::Kpss;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Kpss)];
    /// O(n): rescans the ring buffer to compute partial sums and LRV.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = KpssConfig;
    type Runtime = KpssProxy;

    fn create(cfg: KpssConfig) -> KpssProxy {
        KpssProxy::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &KpssConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for KpssConfig {
    fn defaults() -> Self {
        KpssConfig { period: Param::Solo(200) }
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


impl Render for KpssProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Kpss, "KPSS Statistic", Color::hex(0x9C27B0))
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
    fn test_kpss_proxy_creation() {
        let kpss = KpssProxy::new(50);
        assert!(!kpss.is_ready());
    }

    #[test]
    fn test_kpss_proxy_warmup() {
        let mut kpss = KpssProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kpss.feed(price);
        }
        assert!(kpss.is_ready());
    }

    #[test]
    fn test_kpss_proxy_non_negative() {
        let mut kpss = KpssProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kpss.feed(price);
            assert!(value >= 0.0, "KPSS should be non-negative");
        }
    }

    #[test]
    fn test_kpss_proxy_reset() {
        let mut kpss = KpssProxy::new(50);
        for i in 0..60 {
            kpss.feed(100.0 + i as f64);
        }
        kpss.reset();
        assert!(!kpss.is_ready());
    }

    fn lcg_noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            out.push(((s >> 33) as f64) / (1u64 << 31) as f64 - 1.0);
        }
        out
    }

    #[test]
    fn stationary_low_random_walk_high() {
        let mut k_stat = KpssProxy::new(80);
        let mut lvl = 0.0;
        for &e in &lcg_noise(100, 42) {
            lvl = 0.2 * lvl + e;
            k_stat.feed(100.0 + lvl);
        }
        let mut k_rw = KpssProxy::new(80);
        let mut p = 100.0;
        for &e in &lcg_noise(100, 7) {
            p += e;
            k_rw.feed(p);
        }
        assert!(
            k_stat.value() < k_rw.value(),
            "stationary LM {} should be < random-walk LM {}",
            k_stat.value(),
            k_rw.value()
        );
    }

    #[test]
    fn factory_feeds_resolved_kpss() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kpss(<<KpssProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..250 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::Kpss) >= 0.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<KpssProxy as Indicator>::ID, IndicatorId::Kpss);
    }
}
