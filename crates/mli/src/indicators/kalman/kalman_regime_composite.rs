// Composite regime score: combine KalmanRegimeScore with ATR/VoV percentiles

use crate::engine::contract_engine::SmootherId;
use crate::indicators::kalman::kalman_regime_score::KalmanRegimeScore;
use crate::indicators::volatility::atr_percentile::AtrPercentile;
use crate::indicators::volatility::close_to_close_vol_percentile::CloseVolPercentile;

#[derive(Debug, Clone)]
pub struct KalmanRegimeComposite {
    regime: KalmanRegimeScore,
    atrp: AtrPercentile,
    cvp: CloseVolPercentile,
    w_regime: f64,
    w_atr: f64,
    w_vov: f64,
    pub value: f64,
}

impl KalmanRegimeComposite {
    pub fn new(
        dt: f64,
        q: f64,
        r: f64,
        k_window: usize,
        decay: f64,
        atr_period: usize,
        atr_ma: SmootherId,
        atr_pct_window: usize,
        vov_vol_window: usize,
        vov_pct_window: usize,
        w_regime: f64,
        w_atr: f64,
        w_vov: f64,
    ) -> Self {
        Self {
            regime: KalmanRegimeScore::new(dt, q, r, k_window, decay),
            atrp: AtrPercentile::new(atr_period, atr_ma, atr_pct_window),
            cvp: CloseVolPercentile::new(vov_vol_window, vov_pct_window),
            w_regime,
            w_atr,
            w_vov,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.regime.reset();
        self.atrp.reset();
        self.cvp.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atrp.is_ready() && self.cvp.is_ready() && self.regime.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). The three inners
    /// consume close (Kscr regime + vol-percentile) and H/L/C (ATR percentile); open/volume
    /// are unused. Drives each inner through its own `feed`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let r = self.regime.feed(close);
        let a = self.atrp.feed(&[high, low, close]);
        let (_vv, _pct) = self.cvp.feed(close);
        let vv = _pct;
        self.value = (self.w_regime * r + self.w_atr * (1.0 - a) + self.w_vov * (1.0 - vv))
            / (self.w_regime + self.w_atr + self.w_vov).max(1e-9);
        self.value
    }
}

impl Default for KalmanRegimeComposite {
    /// Factory default: dt=1.0, q=0.01, r=0.1, k_window=20, decay=0.95, atr_period=14, atr_ma=SMA, atr_pct_window=100, vov_vol_window=20, vov_pct_window=100, w_regime=0.5, w_atr=0.3, w_vov=0.2.
    fn default() -> Self {
        Self::new(1.0, 0.01, 0.1, 20, 0.95, 14, SmootherId::Sma, 100, 20, 100, 0.5, 0.3, 0.2)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::contract::sweep_f64;
use crate::engine::stream_kind::StreamKind;
use crate::engine::ohlcv_field::OhlcvField;

/// Typed config for [`KalmanRegimeComposite`].
///
/// Weights are fixed at the defaults; for a fully configurable version, expose
/// all constructor params. The current contract captures the most-tuned axes
/// (Kalman params, ATR period, percentile windows, weights).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KalmanRegimeCompositeConfig {
    pub dt: Param<f64>,
    pub process_noise: Param<f64>,
    pub measurement_noise: Param<f64>,
    pub k_window: Param<usize>,
    pub decay: Param<f64>,
    pub atr_period: Param<usize>,
    pub atr_pct_window: Param<usize>,
    pub vov_vol_window: Param<usize>,
    pub vov_pct_window: Param<usize>,
    pub w_regime: Param<f64>,
    pub w_atr: Param<f64>,
    pub w_vov: Param<f64>,
}

impl Indicator for KalmanRegimeComposite {
    const ID: IndicatorId = IndicatorId::Kcomp;
    /// Composite regime classifier — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// The three inners collectively need H/L/C (ATR percentile) and close (Kscr +
    /// vol-percentile). `Fields` flavor: the factory feeds these resolved H/L/C lanes in
    /// order; the composite routes them to each inner's `feed`.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// Outer is O(1) (weighted average of inner outputs). Inner costs are charged via
    /// the declared Ports: Kscr (Kalman regime score), Atrp (ATR percentile), C2cvp
    /// (close-to-close vol percentile).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Kscr, &[IndicatorOutputId::Kscr]),
            Port::new(IndicatorId::Atrp, &[IndicatorOutputId::Atrp]),
            Port::new(IndicatorId::C2cvp, &[IndicatorOutputId::C2cvp]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Kcomp)];

    type Config = KalmanRegimeCompositeConfig;
    type Runtime = KalmanRegimeComposite;

    fn create(cfg: KalmanRegimeCompositeConfig) -> KalmanRegimeComposite {
        KalmanRegimeComposite::new(
            cfg.dt.resolved(),
            cfg.process_noise.resolved(),
            cfg.measurement_noise.resolved(),
            cfg.k_window.resolved(),
            cfg.decay.resolved(),
            cfg.atr_period.resolved(),
            SmootherId::Sma,
            cfg.atr_pct_window.resolved(),
            cfg.vov_vol_window.resolved(),
            cfg.vov_pct_window.resolved(),
            cfg.w_regime.resolved(),
            cfg.w_atr.resolved(),
            cfg.w_vov.resolved(),
        )
    }
}

impl crate::contract::Config for KalmanRegimeCompositeConfig {
    fn defaults() -> Self {
        KalmanRegimeCompositeConfig {
            dt: Param::Solo(1.0),
            process_noise: Param::Solo(0.01),
            measurement_noise: Param::Solo(0.1),
            k_window: Param::Solo(20),
            decay: Param::Solo(0.95),
            atr_period: Param::Solo(14),
            atr_pct_window: Param::Solo(100),
            vov_vol_window: Param::Solo(20),
            vov_pct_window: Param::Solo(100),
            w_regime: Param::Solo(0.5),
            w_atr: Param::Solo(0.3),
            w_vov: Param::Solo(0.2),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // dt: PIN — always 1.0 (one bar). auto leaves f64 Solo; confirmed not touched.
        // process_noise: Class H — 5 decade-spaced values.
        s.process_noise = Param::many(vec![0.0001, 0.001, 0.01, 0.1, 1.0]);
        // measurement_noise: Class H — 5 decade-spaced values.
        s.measurement_noise = Param::many(vec![0.0001, 0.001, 0.01, 0.1, 1.0]);
        // k_window: Class A usize — auto sets range(2,4048,1), correct.
        // decay: Class E kalman sub-case — sweep_f64(0.80, 0.99, 0.01).
        s.decay = Param::many(sweep_f64(0.80, 0.99, 0.01));
        // atr_period: Class A usize — auto sets range(2,4048,1), correct.
        // atr_pct_window: Class A usize — auto sets range(2,4048,1), correct.
        // vov_vol_window: Class A usize — auto sets range(2,4048,1), correct.
        // vov_pct_window: Class A usize — auto sets range(2,4048,1), correct.
        // w_regime: Class D ratio — sweep_f64(0.0, 1.0, 0.05).
        s.w_regime = Param::many(sweep_f64(0.0, 1.0, 0.05));
        // w_atr: Class D ratio — sweep_f64(0.0, 1.0, 0.05).
        s.w_atr = Param::many(sweep_f64(0.0, 1.0, 0.05));
        // w_vov: Class D ratio — sweep_f64(0.0, 1.0, 0.05).
        // Note: generator-side filter must enforce w_regime+w_atr+w_vov > 0; ranges set independently per spec.
        s.w_vov = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for KalmanRegimeComposite {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Kcomp, "Kalman Composite", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_regime_composite_creation() {
        let krc = KalmanRegimeComposite::new(
            1.0, 0.1, 1.0, 20, 0.94,
            14, SmootherId::Ema, 50,
            20, 50,
            0.4, 0.3, 0.3
        );
        assert!(!krc.is_ready());
        assert_eq!(krc.value, 0.0);
    }

    #[test]
    fn test_kalman_regime_composite_values_finite() {
        let mut krc = KalmanRegimeComposite::new(
            1.0, 0.1, 1.0, 20, 0.94,
            14, SmootherId::Ema, 50,
            20, 50,
            0.4, 0.3, 0.3
        );
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = krc.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kalman_regime_composite_reset() {
        let mut krc = KalmanRegimeComposite::new(
            1.0, 0.1, 1.0, 20, 0.94,
            14, SmootherId::Ema, 50,
            20, 50,
            0.4, 0.3, 0.3
        );
        for _i in 0..100 {
            krc.feed(&[105.0, 95.0, 101.0]);
        }
        krc.reset();
        assert!(!krc.is_ready());
        assert_eq!(krc.value, 0.0);
    }

    /// Factory feeds all five OHLCV lanes (KlineSlice source); the composite drives all
    /// three inners (Kscr close, Atrp H/L/C, C2cvp close). Wild open/volume do not
    /// affect the result — only H/L/C/close matter to the inners.
    #[test]
    fn factory_feeds_resolved_kcomp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kcomp(<<KalmanRegimeComposite as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..110 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.5,
                low: price - 1.5,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Kcomp);
        assert!(v.is_finite(), "Kcomp value must be finite, got {v}");
        assert!(v >= 0.0 && v <= 1.0, "Kcomp value must be in [0,1], got {v}");
    }
}
