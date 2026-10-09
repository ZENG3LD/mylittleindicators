// Engle–Granger ADF Proxy: OLS residuals of close ~ MA(ma_period), then AR(1) t-stat on residuals

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

#[derive(Debug, Clone)]
pub struct EngleGrangerAdfProxy {
    window: usize,
    ma: SmootherSlot,
    residuals: Vec<f64>,
    idx: usize,
    filled: bool,
    pub phi: f64,
    pub t_stat: f64,
}

impl EngleGrangerAdfProxy {
    pub fn new(window: usize, ma_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Sma, window, ma_period)
    }

    pub fn from_smoothers(id: SmootherId, window: usize, ma_period: usize) -> Self {
        let w = window.max(32);
        let p = ma_period.max(5);
        Self {
            window: w,
            ma: SmootherSlot::new(id, p),
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

    pub fn feed(&mut self, close: f64) -> (f64, f64) {
        // OLS against MA is effectively residual = close - MA(close)
        let m = self.ma.feed(close);
        let resid = close - m;
        self.residuals[self.idx] = resid;
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

    // Transitional bridge: adf_kpss_composite.rs still calls update_bar; delegates to feed(close).
    // Remove when adf_kpss_composite.rs is contracted.

    /// AR(1) coefficient on OLS residuals (phi estimate).
    pub fn phi(&self) -> f64 {
        self.phi
    }

    /// ADF t-statistic for the phi estimate.
    pub fn t_stat(&self) -> f64 {
        self.t_stat
    }

}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Dual-mode config for [`EngleGrangerAdfProxy`].
/// `window` is the AR(1) residual window; `ma` slot controls the detrending smoother.
/// With the default `follow(Sma)`, the MA period follows `window`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EngleGrangerAdfConfig {
    pub source: Param<OhlcvField>,
    /// The rolling window for the AR(1) regression on residuals.
    pub window: Param<usize>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for EngleGrangerAdfProxy {
    const ID: IndicatorId = IndicatorId::EgAdf;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = EngleGrangerAdfConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::EgAdfPhi),
        Output::centered(IndicatorOutputId::EgAdfTStat),
    ];
    type Config = EngleGrangerAdfConfig;
    type Runtime = EngleGrangerAdfProxy;

    fn create(cfg: EngleGrangerAdfConfig) -> EngleGrangerAdfProxy {
        let w = cfg.window.resolved();
        let choice = cfg.ma.resolved();
        let ma_p = choice.period.resolve(w);
        EngleGrangerAdfProxy::from_smoothers(choice.kind, w, ma_p)
    }

    fn source_fields(cfg: &EngleGrangerAdfConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &EngleGrangerAdfConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for EngleGrangerAdfConfig {
    fn defaults() -> Self {
        EngleGrangerAdfConfig {
            source: Param::Solo(OhlcvField::Close),
            window: Param::Solo(100),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        // source: Class O → auto (all 8 fields).
        // window: Class A → auto range(2,4048,1).
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


impl Render for EngleGrangerAdfProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EgAdfTStat, "EG ADF", Color::hex(0x9C27B0))
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
    fn test_engle_granger_adf_proxy_creation() {
        let egap = EngleGrangerAdfProxy::new(50, 20);
        assert!(!egap.is_ready());
        assert_eq!(egap.phi, 0.0);
        assert_eq!(egap.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_adf_proxy_warmup() {
        let mut egap = EngleGrangerAdfProxy::new(50, 20);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            egap.feed(price);
        }
        assert!(egap.is_ready());
    }

    #[test]
    fn test_engle_granger_adf_proxy_values() {
        let mut egap = EngleGrangerAdfProxy::new(50, 20);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = egap.feed(price);
            assert!(phi.is_finite(), "Phi should be finite");
            assert!(t_stat.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_engle_granger_adf_proxy_reset() {
        let mut egap = EngleGrangerAdfProxy::new(50, 20);
        for i in 0..60 {
            egap.feed(100.0 + i as f64);
        }
        egap.reset();
        assert!(!egap.is_ready());
        assert_eq!(egap.phi, 0.0);
        assert_eq!(egap.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_adf_proxy_with_ema() {
        let mut egap = EngleGrangerAdfProxy::from_smoothers(SmootherId::Ema, 50, 20);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = egap.feed(price);
            assert!(phi.is_finite());
            assert!(t_stat.is_finite());
        }
        assert!(egap.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_eg_adf() {
        let mut f = IndicatorOrder::EgAdf(<<EngleGrangerAdfProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::EgAdfTStat).is_finite());
    }
}
