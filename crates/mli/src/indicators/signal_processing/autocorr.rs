// Return Autocorrelation at lag L over rolling window N
#[derive(Debug, Clone)]
pub struct Autocorr {
    lag: usize,
    window: usize,
    // store last window+lag closes to compute returns
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl Autocorr {
    pub fn new(lag: usize, window: usize) -> Self {
        let lag = lag.max(1);
        let window = window.max(2);
        Self {
            lag,
            window,
            closes: vec![0.0; window + lag + 1],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn feed(&mut self, close: f64) -> f64 {
        // push close
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % self.closes.len();
        if self.idx == 0 {
            self.filled = true;
        }

        // compute autocorr over window on log returns with lag
        if !self.filled {
            return self.value;
        }
        let len = self.closes.len();
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        let mut sum_xy = 0.0;
        let mut n = 0.0;
        // walk last `window` pairs
        for k in 0..self.window {
            // indices from newest backwards
            let i_curr = (self.idx + len + len - 1 - k) % len;
            let i_prev = (i_curr + len - 1) % len;
            let i_lag_curr = (i_curr + len - self.lag) % len;
            let i_lag_prev = (i_lag_curr + len - 1) % len;
            let c1 = self.closes[i_prev].max(1e-12);
            let c2 = self.closes[i_curr].max(1e-12);
            let c3 = self.closes[i_lag_prev].max(1e-12);
            let c4 = self.closes[i_lag_curr].max(1e-12);
            let r_t = (c2 / c1).ln();
            let r_tlag = (c4 / c3).ln();
            sum_x += r_t;
            sum_y += r_tlag;
            sum_x2 += r_t * r_t;
            sum_y2 += r_tlag * r_tlag;
            sum_xy += r_t * r_tlag;
            n += 1.0;
        }
        if n >= 2.0 {
            let mx = sum_x / n;
            let my = sum_y / n;
            let cov = (sum_xy / n) - mx * my;
            let vx = (sum_x2 / n) - mx * mx;
            let vy = (sum_y2 / n) - my * my;
            let denom = (vx.max(0.0) * vy.max(0.0)).sqrt();
            self.value = if denom > 1e-12 { cov / denom } else { 0.0 };
        } else {
            self.value = 0.0;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn lag(&self) -> usize {
        self.lag
    }

    pub fn window(&self) -> usize {
        self.window
    }

}


impl Default for Autocorr {
    /// Factory default: lag=1, window=50.
    fn default() -> Self {
        Self::new(1, 50)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Autocorr`]: lag (bars delayed) + rolling window length.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AutocorrConfig {
    pub lag: Param<usize>,
    pub window: Param<usize>,
}

impl Indicator for Autocorr {
    const ID: IndicatorId = IndicatorId::Autocorr;
    /// Not a pluggable family member — a standalone statistical detector.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window) per bar: scans the rolling log-return pairs each update.
    /// One period-deep Vec for the price ring buffer.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Autocorr)];
    type Config = AutocorrConfig;
    type Runtime = Autocorr;

    fn create(cfg: AutocorrConfig) -> Autocorr {
        Autocorr::new(cfg.lag.resolved(), cfg.window.resolved())
    }

    fn source_fields(cfg: &AutocorrConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for AutocorrConfig {
    fn defaults() -> Self {
        AutocorrConfig { lag: Param::Solo(1), window: Param::Solo(50) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        s.lag = Param::range(1, 50, 1); // Class A.lag — bounded 1..=50
        s
    }
}


impl Render for Autocorr {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Autocorr, "Autocorrelation", Color::hex(0x3F51B5))
            .bounds(-1.0, 1.0)
            .reference_line(crate::contract::ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autocorr_creation() {
        let ac = Autocorr::new(5, 20);
        assert!(!ac.is_ready());
        assert_eq!(ac.value(), 0.0);
        assert_eq!(ac.lag(), 5);
        assert_eq!(ac.window(), 20);
    }

    #[test]
    fn test_autocorr_range() {
        let mut ac = Autocorr::new(5, 20);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = ac.feed(price);
            if ac.is_ready() {
                assert!(value >= -1.0 && value <= 1.0, "Autocorr should be in [-1, 1], got {}", value);
            }
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_autocorr_reset() {
        let mut ac = Autocorr::new(5, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            ac.feed(price);
        }
        assert!(ac.is_ready());
        ac.reset();
        assert!(!ac.is_ready());
        assert_eq!(ac.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_autocorr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<Autocorr as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Autocorr(cfg).build_solo().unwrap();
        for i in 1..=80 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        // after warmup the factory resolves close (not the wild high 9999)
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v >= -1.0 && v <= 1.0, "autocorr out of [-1,1]: {v}");
    }
}
