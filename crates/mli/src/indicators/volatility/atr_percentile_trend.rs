// ATR Percentile Trend: EMA-detrended ATR percentile

use crate::indicators::volatility::atr_percentile::AtrPercentile;

#[derive(Debug, Clone)]
pub struct AtrPercentileTrend {
    inner: AtrPercentile,
    alpha: f64,
    ema: f64,
    init: bool,
    pub value: f64,
}

impl AtrPercentileTrend {
    pub fn new(
        atr_period: usize,
        window: usize,
        alpha: f64,
    ) -> Self {
        Self {
            inner: AtrPercentile::with_period(atr_period, window),
            alpha: alpha.clamp(0.01, 1.0),
            ema: 0.0,
            init: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.ema = 0.0;
        self.init = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let p = self.inner.feed(lanes);
        if !self.init {
            self.ema = p;
            self.init = true;
        }
        self.ema = self.alpha * p + (1.0 - self.alpha) * self.ema;
        self.value = p - self.ema;
        self.value
    }

}

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed config for [`AtrPercentileTrend`].
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrptConfig {
    /// ATR lookback period.
    pub atr_period: Param<usize>,
    /// Percentile rolling window depth.
    pub window: Param<usize>,
    /// EMA detrending factor (alpha). Smaller = slower detrend.
    pub alpha: Param<f64>,
    /// ATR smoother — default `follow(Rma)` at `atr_period`.
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl AtrPercentileTrend {
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Indicator for AtrPercentileTrend {
    const ID: IndicatorId = IndicatorId::Atrpt;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atrp, &[IndicatorOutputId::Atrp])],
    };
    const SLOTS: &'static [Slot] = AtrptConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Atrpt)];
    type Config = AtrptConfig;
    type Runtime = AtrPercentileTrend;

    fn create(cfg: AtrptConfig) -> AtrPercentileTrend {
        let p  = cfg.atr_period.resolved();
        let ch = cfg.atr_smoother.resolved();
        AtrPercentileTrend {
            inner: AtrPercentile::from_smoothers(
                ch.period.resolve(p),
                ch.kind,
                cfg.window.resolved(),
            ),
            alpha: cfg.alpha.resolved().clamp(0.01, 1.0),
            ema: 0.0,
            init: false,
            value: 0.0,
        }
    }

    fn slot_members(cfg: &AtrptConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrptConfig {
    fn defaults() -> Self {
        AtrptConfig {
            atr_period:   Param::Solo(14),
            window:       Param::Solo(50),
            alpha:        Param::Solo(0.1),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // atr_period, window: Class A usize — auto range(2,4048,1)
        // #[slot] atr_smoother: leave Solo (deferred sweep wave)
        let mut s = Self::machine_defaults_auto();
        s.alpha = Param::many(crate::contract::sweep_f64(0.01, 0.99, 0.01)); // Class E alpha/decay
        s
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for AtrPercentileTrend {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Atrpt, "ATR Trail", Color::hex(0xF44336))
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
    fn test_atr_percentile_trend_creation() {
        let apt = AtrPercentileTrend::new(14, 50, 0.1);
        assert!(!apt.is_ready());
        assert_eq!(apt.value, 0.0);
    }

    #[test]
    fn test_atr_percentile_trend_warmup() {
        let mut apt = AtrPercentileTrend::new(14, 50, 0.1);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            apt.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(apt.is_ready());
    }

    #[test]
    fn test_atr_percentile_trend_values() {
        let mut apt = AtrPercentileTrend::new(14, 50, 0.1);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let value = apt.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_atr_percentile_trend_reset() {
        let mut apt = AtrPercentileTrend::new(14, 50, 0.1);
        for i in 0..60 {
            apt.feed(&[100.0 + i as f64, 99.0, 100.0 + i as f64]);
        }
        apt.reset();
        assert!(!apt.is_ready());
        assert_eq!(apt.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_atrpt() {
        let cfg = <<AtrPercentileTrend as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Atrpt(cfg).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 2.0, low: price - 2.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Atrpt).is_finite());
    }
}
