// RegimeComposite: combine Hurst/DFA + SpectralSlope + SpectralEnergyRatio + VolOfVolPercentile + ATR Percentile trend

use crate::indicators::chaos::{dfa::Dfa, hurst_exponent::HurstExponent};
use crate::indicators::signal_processing::{SpectralEnergyRatio, SpectralSlope};
use crate::indicators::volatility::{
    atr_percentile::AtrPercentile, vol_of_vol_percentile::VolOfVolPercentile,
};
use crate::engine::contract_engine::SmootherId;

#[derive(Debug, Clone)]
pub struct RegimeComposite {
    hurst: HurstExponent,
    dfa: Dfa,
    slope: SpectralSlope,
    ser: SpectralEnergyRatio,
    vovp: VolOfVolPercentile,
    atrp: AtrPercentile,
    pub value: f64,
}

impl RegimeComposite {
    pub fn new(
        hurst_window: usize,
        dfa_scales: [usize; 4],
        fft_window: usize,
        ser_low_cut: f64,
        vov_window: usize,
        perc_window: usize,
        atr_period: usize,
    ) -> Self {
        Self {
            hurst: HurstExponent::new(hurst_window),
            dfa: Dfa::new(dfa_scales),
            slope: SpectralSlope::new(fft_window),
            ser: SpectralEnergyRatio::new(fft_window, ser_low_cut),
            vovp: VolOfVolPercentile::with_period(vov_window, perc_window),
            atrp: AtrPercentile::new(atr_period, SmootherId::Rma, perc_window),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.hurst.reset();
        self.dfa.reset();
        self.slope.reset();
        self.ser.reset();
        self.vovp.reset();
        self.atrp.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.hurst.is_ready() && self.dfa.is_ready() && self.slope.is_ready() && self.ser.is_ready()
    }

    /// Feed resolved input lanes — `[open, high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[1];
        let l = lanes[2];
        let c = lanes[3];

        let hval = self.hurst.feed(c);
        let dval = self.dfa.feed(c);
        let sval = self.slope.feed(c);
        let ser = self.ser.feed(c);
        let vovp = self.vovp.feed(&[h, l, c]);
        let atrp = self.atrp.feed(&[h, l, c]);
        // normalize components to [-1,1] where feasible and combine
        let trendiness = (hval - 0.5) * 2.0; // H in (0,1) -> (-1,1)
        let persistence = (1.0 - dval).clamp(0.0, 1.0) * 2.0 - 1.0; // inverse DFA as proxy
        let spectral_trend = (-sval).tanh(); // negative slope => trend
        let low_band_bias = (ser * 2.0 - 1.0).clamp(-1.0, 1.0);
        let vov_state = (vovp * 2.0 - 1.0).clamp(-1.0, 1.0);
        let atr_state = (atrp * 2.0 - 1.0).clamp(-1.0, 1.0);
        self.value = 0.25 * trendiness
            + 0.15 * persistence
            + 0.2 * spectral_trend
            + 0.15 * low_band_bias
            + 0.15 * vov_state
            + 0.1 * atr_state;
        self.value
    }


    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for RegimeComposite {
    /// Factory default: hurst_window=100, dfa_scales=[4,8,16,32], fft_window=128,
    /// ser_low_cut=0.1, vov_window=100, perc_window=100, atr_period=14.
    fn default() -> Self {
        Self::new(100, [4, 8, 16, 32], 128, 0.1, 100, 100, 14)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`RegimeComposite`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RegimeCompositeConfig {
    pub hurst_window: Param<usize>,
    pub dfa_scales: Param<[usize; 4]>,
    pub fft_window: Param<usize>,
    pub ser_low_cut: Param<f64>,
    pub vov_window: Param<usize>,
    pub perc_window: Param<usize>,
    pub atr_period: Param<usize>,
}

impl Indicator for RegimeComposite {
    const ID: IndicatorId = IndicatorId::Rc;
    /// Regime composite — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// All 5 OHLCV fields: HurstExponent/Dfa consume close; SpectralSlope/Ser/Vovp/Atrp
    /// consume the full bar.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// Composite cost: all 6 inners declared as Ports.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Hurst, &[IndicatorOutputId::Hurst]),
            Port::new(IndicatorId::Dfa,   &[IndicatorOutputId::Dfa]),
            Port::new(IndicatorId::Sslope, &[IndicatorOutputId::Sslope]),
            Port::new(IndicatorId::Ser,    &[IndicatorOutputId::Ser]),
            Port::new(IndicatorId::Vovp,   &[IndicatorOutputId::Vovp]),
            Port::new(IndicatorId::Atrp,   &[IndicatorOutputId::Atrp]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Rc)];
    type Config = RegimeCompositeConfig;
    type Runtime = RegimeComposite;

    fn create(cfg: RegimeCompositeConfig) -> RegimeComposite {
        RegimeComposite::new(
            cfg.hurst_window.resolved(),
            cfg.dfa_scales.resolved(),
            cfg.fft_window.resolved(),
            cfg.ser_low_cut.resolved(),
            cfg.vov_window.resolved(),
            cfg.perc_window.resolved(),
            cfg.atr_period.resolved(),
        )
    }
}

impl crate::contract::Config for RegimeCompositeConfig {
    fn defaults() -> Self {
        RegimeCompositeConfig {
            hurst_window: Param::Solo(100),
            dfa_scales: Param::Solo([4, 8, 16, 32]),
            fft_window: Param::Solo(128),
            ser_low_cut: Param::Solo(0.1),
            vov_window: Param::Solo(100),
            perc_window: Param::Solo(100),
            atr_period: Param::Solo(14),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // ser_low_cut: Class G spectral low-cut frequency — sweep_f64(0.01, 0.49, 0.01).
        s.ser_low_cut = Param::many(sweep_f64(0.01, 0.49, 0.01));
        // dfa_scales: Class S fixed [usize;4] — 4 canonical DFA scale presets.
        s.dfa_scales = Param::many(vec![
            [4, 8, 16, 32],
            [8, 16, 32, 64],
            [16, 32, 64, 128],
            [32, 64, 128, 256],
        ]);
        // hurst_window / fft_window / vov_window / perc_window / atr_period:
        // Class A usize — auto range(2,4048,1) covers all.
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RegimeComposite {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rc, "Regime Composite", Color::hex(0x2196F3))
            .bounds(-1.0, 1.0)
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
    fn test_regime_composite_creation() {
        let rc = RegimeComposite::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        assert!(!rc.is_ready());
        assert_eq!(rc.value(), 0.0);
    }

    #[test]
    fn test_regime_composite_warmup() {
        let mut rc = RegimeComposite::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rc.feed(&[price, price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(rc.is_ready());
    }

    #[test]
    fn test_regime_composite_finite() {
        let mut rc = RegimeComposite::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        for i in 0..250 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = rc.feed(&[price, price + 2.0, price - 2.0, price, 1000.0]);
            assert!(value.is_finite(), "Score should be finite, got {}", value);
        }
    }

    #[test]
    fn test_regime_composite_reset() {
        let mut rc = RegimeComposite::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        for i in 0..200 {
            rc.feed(&[100.0 + i as f64, 101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0]);
        }
        rc.reset();
        assert!(!rc.is_ready());
        assert_eq!(rc.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_rc() {
        let mut f = IndicatorOrder::Rc(<<RegimeComposite as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..250 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price - 1.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "RC should be finite, got {v}");
    }
}
