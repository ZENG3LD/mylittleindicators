// Residual Stationarity Proxy: run regression close ~ SMA(window) and compute variance ratio of residuals

#[derive(Debug, Clone)]
pub struct ResidualStationarity {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    ratio: f64,
}

impl ResidualStationarity {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            closes: vec![0.0; w],
            idx: 0,
            filled: false,
            ratio: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.closes.fill(0.0);
        self.ratio = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        let n = self.window;
        self.closes[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.ratio = self.compute_ratio();
        }
        self.ratio
    }

    fn compute_ratio(&self) -> f64 {
        let n = self.window;
        let mut mean = 0.0;
        for i in 0..n {
            mean += self.closes[i];
        }
        mean /= n as f64;
        // simple SMA as regressor; residuals = close - SMA
        let mut sma = vec![0.0; n];
        let mut sum = 0.0;
        for (i, slot) in sma.iter_mut().enumerate() {
            sum += self.closes[i];
            *slot = sum / ((i + 1) as f64);
        }
        let res: Vec<f64> = self.closes[..n].iter().zip(sma.iter()).map(|(&c, &s)| c - s).collect();
        // variance ratio: Var(res) / Var(close)
        let mut var_res = 0.0;
        let mut var_close = 0.0;
        for (&r, &c) in res.iter().zip(self.closes[..n].iter()) {
            var_res += r * r;
            var_close += (c - mean) * (c - mean);
        }
        if var_close > 0.0 {
            (var_res / n as f64) / (var_close / n as f64)
        } else {
            0.0
        }
    }

    pub fn value(&self) -> f64 {
        self.ratio
    }
}

impl Default for ResidualStationarity {
    /// Factory defaults: window=100 (clamped to max(100,20)=100).
    fn default() -> Self {
        Self::new(100)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`ResidualStationarity`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ResidStatConfig {
    pub period: Param<usize>,
}

impl Indicator for ResidualStationarity {
    const ID: IndicatorId = IndicatorId::ResidStat;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::ResidStat)];
    /// O(n): single-pass variance computation over the ring buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = ResidStatConfig;
    type Runtime = ResidualStationarity;

    fn create(cfg: ResidStatConfig) -> ResidualStationarity {
        ResidualStationarity::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &ResidStatConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for ResidStatConfig {
    fn defaults() -> Self {
        ResidStatConfig { period: Param::Solo(100) }
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


impl Render for ResidualStationarity {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ResidStat, "Residual Stat", Color::hex(0x9C27B0))
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
    fn test_residual_stationarity_creation() {
        let rs = ResidualStationarity::new(50);
        assert!(!rs.is_ready());
        assert_eq!(rs.value(), 0.0);
    }

    #[test]
    fn test_residual_stationarity_warmup() {
        let mut rs = ResidualStationarity::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rs.feed(price);
        }
        assert!(rs.is_ready());
    }

    #[test]
    fn test_residual_stationarity_non_negative() {
        let mut rs = ResidualStationarity::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rs.feed(price);
            assert!(value >= 0.0, "Variance ratio should be non-negative");
        }
    }

    #[test]
    fn test_residual_stationarity_reset() {
        let mut rs = ResidualStationarity::new(50);
        for i in 0..60 {
            rs.feed(100.0 + i as f64);
        }
        rs.reset();
        assert!(!rs.is_ready());
        assert_eq!(rs.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_resid_stat() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::ResidStat(<<ResidualStationarity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..150 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::ResidStat) >= 0.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<ResidualStationarity as Indicator>::ID, IndicatorId::ResidStat);
    }
}
