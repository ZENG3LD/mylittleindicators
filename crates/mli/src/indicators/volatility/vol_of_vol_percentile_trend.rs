// Vol-of-Vol Percentile Trend: EMA-detrended percentile of VolOfVol

use crate::indicators::volatility::vol_of_vol_percentile::VolOfVolPercentile;

#[derive(Debug, Clone)]
pub struct VolOfVolPercentileTrend {
    inner: VolOfVolPercentile,
    alpha: f64,
    ema: f64,
    init: bool,
    pub value: f64,
}

impl VolOfVolPercentileTrend {
    pub fn new(
        vov_window: usize,
        pct_window: usize,
        alpha: f64,
    ) -> Self {
        Self {
            inner: VolOfVolPercentile::with_period(vov_window, pct_window),
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
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::ohlcv_field::OhlcvField;

/// Typed dual-mode config for [`VolOfVolPercentileTrend`] — three params.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VovptConfig {
    pub vov_period: Param<usize>,
    pub percentile_window: Param<usize>,
    /// EMA detrending factor (alpha).
    pub alpha: Param<f64>,
}

impl VolOfVolPercentileTrend {
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for VolOfVolPercentileTrend {
    fn default() -> Self {
        Self::new(20, 100, 0.05)
    }
}

impl Indicator for VolOfVolPercentileTrend {
    const ID: IndicatorId = IndicatorId::Vovpt;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bar-fed: forwards high/low/close to the embedded VoV percentile.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Vovp, &[IndicatorOutputId::Vovp])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Vovpt)];
    type Config = VovptConfig;
    type Runtime = VolOfVolPercentileTrend;

    fn create(cfg: VovptConfig) -> VolOfVolPercentileTrend {
        VolOfVolPercentileTrend {
            inner: VolOfVolPercentile::with_period(cfg.vov_period.resolved(), cfg.percentile_window.resolved()),
            alpha: cfg.alpha.resolved().clamp(0.01, 1.0),
            ema: 0.0,
            init: false,
            value: 0.0,
        }
    }
}

impl crate::contract::Config for VovptConfig {
    fn defaults() -> Self {
        VovptConfig {
            vov_period: Param::Solo(20),
            percentile_window: Param::Solo(100),
            alpha: Param::Solo(0.05),
        }
    }
    fn machine_defaults() -> Self {
        // vov_period, percentile_window: Class A usize — auto range(2,4048,1)
        let mut s = Self::machine_defaults_auto();
        s.alpha = Param::many(crate::contract::sweep_f64(0.01, 0.99, 0.01)); // Class E alpha/decay
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolOfVolPercentileTrend {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vovpt, "VoV % Trail", Color::hex(0x9C27B0))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_vol_of_vol_percentile_trend_creation() {
        let vovpt = VolOfVolPercentileTrend::new(20, 50, 0.1);
        assert!(!vovpt.is_ready());
        assert_eq!(vovpt.value, 0.0);
    }

    #[test]
    fn test_vol_of_vol_percentile_trend_warmup() {
        let mut vovpt = VolOfVolPercentileTrend::new(20, 50, 0.1);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vovpt.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(vovpt.is_ready());
    }

    #[test]
    fn test_vol_of_vol_percentile_trend_values() {
        let mut vovpt = VolOfVolPercentileTrend::new(20, 50, 0.1);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vovpt.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value.is_finite(), "Trend value should be finite");
        }
    }

    #[test]
    fn test_vol_of_vol_percentile_trend_reset() {
        let mut vovpt = VolOfVolPercentileTrend::new(20, 50, 0.1);
        for i in 0..80 {
            let p = 100.0 + i as f64;
            vovpt.feed(&[p + 1.0, p - 1.0, p]);
        }
        vovpt.reset();
        assert!(!vovpt.is_ready());
        assert_eq!(vovpt.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vovpt() {
        let cfg = <<VolOfVolPercentileTrend as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Vovpt(cfg).build_solo().unwrap();
        for i in 0..130 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Vovpt).is_finite());
    }
}
