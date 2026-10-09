// Cointegration Proxy: residual stationarity via AR(1) t-stat on (close - MA(window)) residuals

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

#[derive(Debug, Clone)]
pub struct CointegrationProxy {
    window: usize,
    ma: SmootherSlot,
    residuals: Vec<f64>,
    idx: usize,
    filled: bool,
    pub phi: f64,
    pub t_stat: f64,
}

impl CointegrationProxy {
    pub fn new(window: usize) -> Self {
        Self::from_smoothers(SmootherId::Sma, window)
    }

    pub fn from_smoothers(id: SmootherId, window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            ma: SmootherSlot::new(id, w),
            residuals: vec![0.0; w],
            idx: 0,
            filled: false,
            phi: 0.0,
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.residuals.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.phi = 0.0;
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Returns t-statistic as main value
    pub fn value(&self) -> f64 {
        self.t_stat
    }

    pub fn feed(&mut self, close: f64) -> (f64, f64) {
        let m = self.ma.feed(close);
        let r = close - m;
        self.residuals[self.idx] = r;
        self.idx = (self.idx + 1) % self.window;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.update_stats();
        }
        (self.phi, self.t_stat)
    }

    fn update_stats(&mut self) {
        let n = self.window;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0;
        let mut count = 0.0;
        for i in 1..n {
            let y = self.residuals[(self.idx + i) % n];
            let x = self.residuals[(self.idx + i - 1) % n];
            sx += x;
            sy += y;
            sxx += x * x;
            sxy += x * y;
            count += 1.0;
        }
        let denom = count * sxx - sx * sx;
        self.phi = if denom.abs() > 1e-12 {
            (count * sxy - sx * sy) / denom
        } else {
            0.0
        };
        // residual variance and std error of phi
        let mut se_sum = 0.0;
        for i in 1..n {
            let y = self.residuals[(self.idx + i) % n];
            let x = self.residuals[(self.idx + i - 1) % n];
            let e = y - self.phi * x;
            se_sum += e * e;
        }
        let var = se_sum.max(1e-12) / (count - 1.0).max(1.0);
        let sxx_adj = (sxx - sx * sx / count).max(1e-12);
        let se_phi = (var / sxx_adj).sqrt();
        self.t_stat = if se_phi > 0.0 {
            (self.phi - 1.0) / se_phi
        } else {
            0.0
        };
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Dual-mode config for [`CointegrationProxy`].
/// `period` controls the AR(1) residuals window; `ma` slot controls the detrending smoother.
/// With the default `follow(Sma)`, the MA period follows `period`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct CointegrationProxyConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for CointegrationProxy {
    const ID: IndicatorId = IndicatorId::Coint;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = CointegrationProxyConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Coint)];
    type Config = CointegrationProxyConfig;
    type Runtime = CointegrationProxy;

    fn create(cfg: CointegrationProxyConfig) -> CointegrationProxy {
        let p = cfg.period.resolved();
        let choice = cfg.ma.resolved();
        CointegrationProxy::from_smoothers(choice.kind, p)
    }

    fn source_fields(cfg: &CointegrationProxyConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &CointegrationProxyConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for CointegrationProxyConfig {
    fn defaults() -> Self {
        CointegrationProxyConfig {
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


impl Render for CointegrationProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Coint, "Coint T-Statistic", Color::hex(0x2196F3))
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
    fn test_cointegration_proxy_creation() {
        let cp = CointegrationProxy::new(50);
        assert!(!cp.is_ready());
        assert_eq!(cp.phi, 0.0);
        assert_eq!(cp.t_stat, 0.0);
    }

    #[test]
    fn test_cointegration_proxy_warmup() {
        let mut cp = CointegrationProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cp.feed(price);
        }
        assert!(cp.is_ready());
    }

    #[test]
    fn test_cointegration_proxy_values() {
        let mut cp = CointegrationProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = cp.feed(price);
            assert!(phi.is_finite(), "Phi should be finite");
            assert!(t_stat.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_cointegration_proxy_reset() {
        let mut cp = CointegrationProxy::new(50);
        for i in 0..60 {
            cp.feed(100.0 + i as f64);
        }
        cp.reset();
        assert!(!cp.is_ready());
        assert_eq!(cp.phi, 0.0);
        assert_eq!(cp.t_stat, 0.0);
    }

    #[test]
    fn test_cointegration_proxy_with_ema() {
        let mut cp = CointegrationProxy::from_smoothers(SmootherId::Ema, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = cp.feed(price);
            assert!(phi.is_finite());
            assert!(t_stat.is_finite());
        }
        assert!(cp.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_coint() {
        let mut f = IndicatorOrder::Coint(<<CointegrationProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.15).sin() * 8.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Coint).is_finite());
    }
}
