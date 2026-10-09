// Composite of ADF proxy (from EngleGrangerAdfProxy style) and KPSS proxies into simple stationarity score

use crate::indicators::statistics::engle_granger_adf_proxy::EngleGrangerAdfProxy;
use crate::indicators::statistics::kpss_proxy::KpssProxy;

#[derive(Debug, Clone)]
pub struct AdfKpssComposite {
    adf: EngleGrangerAdfProxy,
    kpss_level: KpssProxy,
    kpss_trend:
        Option<crate::indicators::statistics::kpss_trend_proxy::KpssTrendProxy>,
    pub value: f64,
}

impl AdfKpssComposite {
    pub fn new(window: usize, ma_period: usize, use_trend: bool) -> Self {
        Self {
            adf: EngleGrangerAdfProxy::new(window, ma_period),
            kpss_level: KpssProxy::new(window),
            kpss_trend: if use_trend {
                Some(crate::indicators::statistics::kpss_trend_proxy::KpssTrendProxy::new(window))
            } else {
                None
            },
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.adf.reset();
        self.kpss_level.reset();
        if let Some(t) = &mut self.kpss_trend {
            t.reset();
        }
        self.value = 0.0;
    }

    pub fn feed(&mut self, close: f64) -> f64 {
        let (_phi, adf_t) = self.adf.feed(close);
        let kpss_l = self.kpss_level.feed(close);
        let kpss_t = self
            .kpss_trend
            .as_mut()
            .map(|t| t.feed(close))
            .unwrap_or(0.0); // map to [0,1]
        let adf_score = 1.0 / (1.0 + (-adf_t).abs().exp()); // larger |t| -> more stationary
        let kpss_score = 1.0 / (1.0 + kpss_l.max(kpss_t)); // larger stat -> less stationary
        self.value = 0.5 * (adf_score + kpss_score);
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.adf.is_ready() && self.kpss_level.is_ready()
    }

    /// Returns composite stationarity score
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for AdfKpssComposite {
    /// Factory defaults: window=50, ma_period=20, use_trend=false.
    fn default() -> Self {
        Self::new(50, 20, false)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Dual-mode config for [`AdfKpssComposite`] — ABSORB pattern.
/// Knobs from inner `EngleGrangerAdfProxy` are lifted here directly.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AdfKpssConfig {
    pub source: Param<OhlcvField>,
    /// The rolling window for ADF residuals and KPSS partial sums.
    pub window: Param<usize>,
    /// Smoother slot for the ADF proxy's detrending MA.
    #[slot]
    pub ma: Param<SmootherChoice>,
    /// Whether to also run the trend-variant KPSS test.
    pub use_trend: Param<bool>,
}

impl Indicator for AdfKpssComposite {
    const ID: IndicatorId = IndicatorId::AdfKpss;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::EgAdf, &[IndicatorOutputId::EgAdfTStat]),
            Port::new(IndicatorId::Kpss,  &[IndicatorOutputId::Kpss]),
            Port::new(IndicatorId::KpssTrend, &[IndicatorOutputId::KpssTrend]),
        ],
    };
    const SLOTS: &'static [Slot] = AdfKpssConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::AdfKpss)];
    type Config = AdfKpssConfig;
    type Runtime = AdfKpssComposite;

    fn create(cfg: AdfKpssConfig) -> AdfKpssComposite {
        let window = cfg.window.resolved().max(20);
        let choice = cfg.ma.resolved();
        let ma_p = choice.period.resolve(window);
        let use_trend = cfg.use_trend.resolved();
        AdfKpssComposite {
            adf: EngleGrangerAdfProxy::from_smoothers(choice.kind, window, ma_p),
            kpss_level: KpssProxy::new(window),
            kpss_trend: if use_trend {
                Some(crate::indicators::statistics::kpss_trend_proxy::KpssTrendProxy::new(window))
            } else {
                None
            },
            value: 0.0,
        }
    }

    fn source_fields(cfg: &AdfKpssConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &AdfKpssConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AdfKpssConfig {
    fn defaults() -> Self {
        AdfKpssConfig {
            source: Param::Solo(OhlcvField::Close),
            window: Param::Solo(50),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            use_trend: Param::Solo(false),
        }
    }
    fn machine_defaults() -> Self {
        // source: Class O → auto (all 8 fields).
        // window: Class A → auto range(2,4048,1).
        // use_trend: Class R (bool) → auto [false,true].
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


impl Render for AdfKpssComposite {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AdfKpss, "Stationarity Score", Color::hex(0x2196F3))
            .bounds(0.0, 1.0)
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
    fn test_adf_kpss_composite_creation() {
        let akc = AdfKpssComposite::new(50, 20, false);
        assert!(!akc.is_ready());
        assert_eq!(akc.value, 0.0);
    }

    #[test]
    fn test_adf_kpss_composite_with_trend() {
        let akc = AdfKpssComposite::new(50, 20, true);
        assert!(!akc.is_ready());
    }

    #[test]
    fn test_adf_kpss_composite_warmup() {
        let mut akc = AdfKpssComposite::new(50, 20, false);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            akc.feed(price);
        }
        assert!(akc.is_ready());
    }

    #[test]
    fn test_adf_kpss_composite_range() {
        let mut akc = AdfKpssComposite::new(50, 20, false);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = akc.feed(price);
            assert!(value >= 0.0 && value <= 1.0, "Composite score should be in [0, 1]");
        }
    }

    #[test]
    fn test_adf_kpss_composite_reset() {
        let mut akc = AdfKpssComposite::new(50, 20, false);
        for i in 0..60 {
            akc.feed(100.0 + i as f64);
        }
        akc.reset();
        assert!(!akc.is_ready());
        assert_eq!(akc.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_adf_kpss() {
        let mut f = IndicatorOrder::AdfKpss(<<AdfKpssComposite as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::AdfKpss);
        assert!(v >= 0.0 && v <= 1.0, "Composite score should be in [0, 1], got {v}");
    }
}
