//! Engle–Granger residual unit-root statistic.
//!
//! Wraps [`CointegrationProxy`] (an AR(1) Dickey-Fuller t-stat on `close − SMA`
//! residuals) and emits the raw DF t-statistic. The prior version mapped that
//! t-stat through a *normal* CDF (`p = 1 − Φ(t)`) — statistically wrong: the
//! Engle-Granger residual statistic does NOT follow a standard normal (nor the
//! plain Dickey-Fuller distribution — it has its own tables that depend on the
//! number of regressors and sample size). A correct p-value requires
//! MacKinnon's EG response surfaces, which are out of scope here; that lives in
//! the statistical-validation layer (mlsv). We therefore emit the raw test
//! statistic and let a regime filter threshold it directly (more negative ⇒
//! stronger residual mean reversion). `p_value`/`normal_cdf`/local `erf`
//! (a duplicate of `distributions::erf`) are removed.
//!
//! NOTE: despite the name this is a SINGLE-stream residual test against an SMA,
//! not a true two-series cointegration test — that belongs in `events::`
//! (deferred). Name retained for backward-compat with existing catalog ids.

use crate::engine::contract_engine::SmootherId;
use crate::indicators::statistics::cointegration_proxy::CointegrationProxy;

#[derive(Debug, Clone)]
pub struct EngleGrangerProxy {
    inner: CointegrationProxy,
    pub t_stat: f64,
}

impl EngleGrangerProxy {
    pub fn new(window: usize) -> Self {
        Self {
            inner: CointegrationProxy::new(window),
            t_stat: 0.0,
        }
    }

    /// Creates a new EngleGrangerProxy with a configurable MA smoother id.
    pub fn with_smoother(window: usize, id: SmootherId) -> Self {
        Self {
            inner: CointegrationProxy::from_smoothers(id, window),
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    pub fn value(&self) -> f64 {
        self.t_stat
    }

    pub fn feed(&mut self, close: f64) -> f64 {
        let (_phi, t) = self.inner.feed(close);
        self.t_stat = t;
        self.t_stat
    }
}

impl Default for EngleGrangerProxy {
    /// Factory defaults: window=100, SMA smoother.
    fn default() -> Self {
        Self::with_smoother(100, SmootherId::Sma)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Dual-mode config for [`EngleGrangerProxy`].
/// `period` is the AR(1)/MA window; `ma` slot controls the detrending smoother kind.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EngleGrangerProxyConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for EngleGrangerProxy {
    const ID: IndicatorId = IndicatorId::EgCoint;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Coint, &[IndicatorOutputId::Coint])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::EgCoint)];
    const SLOTS: &'static [Slot] = EngleGrangerProxyConfig::SLOTS;
    type Config = EngleGrangerProxyConfig;
    type Runtime = EngleGrangerProxy;

    fn create(cfg: EngleGrangerProxyConfig) -> EngleGrangerProxy {
        let p = cfg.period.resolved();
        let choice = cfg.ma.resolved();
        EngleGrangerProxy::with_smoother(p, choice.kind)
    }

    fn source_fields(cfg: &EngleGrangerProxyConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &EngleGrangerProxyConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for EngleGrangerProxyConfig {
    fn defaults() -> Self {
        EngleGrangerProxyConfig {
            period: Param::Solo(100),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // source: Class O → auto (all 8 fields).
        // ma: #[slot] SmootherChoice → leave Solo (deferred wave).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for EngleGrangerProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EgCoint, "EG Coint T-Stat", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_engle_granger_proxy_creation() {
        let egp = EngleGrangerProxy::new(50);
        assert!(!egp.is_ready());
        assert_eq!(egp.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_proxy_warmup() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            egp.feed(price);
        }
        assert!(egp.is_ready());
    }

    #[test]
    fn test_engle_granger_proxy_emits_finite_stat() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = egp.feed(price);
            assert!(value.is_finite(), "EG residual t-stat should be finite");
        }
    }

    #[test]
    fn test_engle_granger_proxy_reset() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            egp.feed(100.0 + i as f64);
        }
        egp.reset();
        assert!(!egp.is_ready());
        assert_eq!(egp.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_proxy_with_ema() {
        let mut egp = EngleGrangerProxy::with_smoother(50, SmootherId::Ema);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let t = egp.feed(price);
            assert!(t.is_finite());
        }
        assert!(egp.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_eg_coint() {
        let mut f = IndicatorOrder::EgCoint(<<EngleGrangerProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..130 {
            let price = 100.0 + (i as f64 * 0.15).sin() * 8.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::EgCoint).is_finite());
    }
}
