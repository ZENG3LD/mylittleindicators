// Rolling Partial Autocorrelation Function (PACF) at given lag k via Levinson-Durbin on window

#[derive(Debug, Clone)]
pub struct Pacf {
    window: usize,
    lag: usize,
    values: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pacf_k: f64,
}

impl Pacf {
    pub fn new(window: usize, lag: usize) -> Self {
        let w = window.max(10);
        let k = lag.max(1).min(w - 2);
        Self {
            window: w,
            lag: k,
            values: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            pacf_k: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.values.fill(0.0);
        self.pacf_k = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, close: f64) -> f64 {
        // work on returns
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.values[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.pacf_k = self.compute_pacf_k();
            }
        }
        self.last_close = Some(close);
        self.pacf_k
    }

    fn compute_pacf_k(&self) -> f64 {
        // Yule-Walker via Levinson-Durbin
        let n = self.window;
        let mean: f64 = self.values.iter().sum::<f64>() / n as f64;
        let x: Vec<f64> = self.values.iter().map(|&v| v - mean).collect();
        let k = self.lag;
        let mut autoc = vec![0.0; k + 1];
        for lag in 0..=k {
            let mut s = 0.0;
            for t in lag..n {
                s += x[t] * x[t - lag];
            }
            autoc[lag] = s / (n as f64);
        }
        if autoc[0].abs() < 1e-12 {
            return 0.0;
        }
        let mut phi = vec![0.0; k + 1];
        let mut v = autoc[0];
        for m in 1..=k {
            let mut sum = 0.0;
            for j in 1..m {
                sum += phi[j] * autoc[m - j];
            }
            let km = (autoc[m] - sum) / v.max(1e-12);
            let mut new_phi = phi.clone();
            new_phi[m] = km;
            for j in 1..m {
                new_phi[j] = phi[j] - km * phi[m - j];
            }
            phi = new_phi;
            v *= 1.0 - km * km;
        }
        phi[k].clamp(-1.0, 1.0)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.pacf_k
    }

}


impl Default for Pacf {
    /// Factory defaults: window=200 (clamped to max(200,10)=200), lag=5.
    fn default() -> Self {
        Self::new(200, 5)
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

/// Typed dual-mode config for [`Pacf`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PacfConfig {
    pub period: Param<usize>,
    pub lag: Param<usize>,
}

impl Indicator for Pacf {
    const ID: IndicatorId = IndicatorId::Pacf;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Pacf)];
    /// O(lag * n): Levinson-Durbin builds up to lag k over window autocorrelations.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = PacfConfig;
    type Runtime = Pacf;

    fn create(cfg: PacfConfig) -> Pacf {
        Pacf::new(cfg.period.resolved(), cfg.lag.resolved())
    }

    fn source_fields(_cfg: &PacfConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for PacfConfig {
    fn defaults() -> Self {
        PacfConfig { period: Param::Solo(200), lag: Param::Solo(5) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // lag: Class A.lag (autocorrelation lag index) → range(1,50,1), corrected off auto 2..4048.
        let mut s = Self::machine_defaults_auto();
        s.lag = Param::range(1, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Pacf {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Pacf, "PACF", Color::hex(0x00BCD4))
            .bounds(-1.0, 1.0)
            .zero_baseline()
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
    fn test_pacf_creation() {
        let pacf = Pacf::new(50, 5);
        assert!(!pacf.is_ready());
        assert_eq!(pacf.value(), 0.0);
    }

    #[test]
    fn test_pacf_warmup() {
        let mut pacf = Pacf::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pacf.feed(price);
        }
        assert!(pacf.is_ready());
    }

    #[test]
    fn test_pacf_range() {
        let mut pacf = Pacf::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pacf.feed(price);
            assert!(value >= -1.0 && value <= 1.0, "PACF should be in [-1, 1]");
        }
    }

    #[test]
    fn test_pacf_reset() {
        let mut pacf = Pacf::new(50, 5);
        for i in 0..60 {
            pacf.feed(100.0 + i as f64);
        }
        pacf.reset();
        assert!(!pacf.is_ready());
        assert_eq!(pacf.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_pacf() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Pacf(<<Pacf as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..250 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        let v = f.read(IndicatorOutputId::Pacf);
        assert!(v >= -1.0 && v <= 1.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<Pacf as Indicator>::ID, IndicatorId::Pacf);
    }
}
