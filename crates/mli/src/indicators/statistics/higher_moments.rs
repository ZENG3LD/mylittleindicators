// Rolling skewness and kurtosis of log returns

#[derive(Debug, Clone)]
pub struct HigherMoments {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    pub skew: f64,
    pub kurt: f64,
}

impl HigherMoments {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(3),
            closes: vec![0.0; window.max(3) + 1],
            idx: 0,
            filled: false,
            skew: 0.0,
            kurt: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.skew = 0.0;
        self.kurt = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, close: f64) -> (f64, f64) {
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % self.closes.len();
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return (self.skew, self.kurt);
        }

        // compute returns for last window
        let len = self.closes.len();
        let mut rets: Vec<f64> = Vec::with_capacity(self.window);
        for k in 0..self.window {
            let i_curr = (self.idx + len + len - 1 - k) % len;
            let i_prev = (i_curr + len - 1) % len;
            let c1 = self.closes[i_prev].max(1e-12);
            let c2 = self.closes[i_curr].max(1e-12);
            rets.push((c2 / c1).ln());
        }
        let n = rets.len() as f64;
        if n < 3.0 {
            return (self.skew, self.kurt);
        }
        let mean = rets.iter().sum::<f64>() / n;
        let mut m2 = 0.0;
        let mut m3 = 0.0;
        let mut m4 = 0.0;
        for &r in &rets {
            let d = r - mean;
            let d2 = d * d;
            let d3 = d2 * d;
            let d4 = d3 * d;
            m2 += d2;
            m3 += d3;
            m4 += d4;
        }
        m2 /= n;
        m3 /= n;
        m4 /= n;
        let s2 = m2.max(1e-12);
        self.skew = m3 / s2.powf(1.5);
        self.kurt = m4 / (s2 * s2);
        (self.skew, self.kurt)
    }


    pub fn window(&self) -> usize {
        self.window
    }
}

impl HigherMoments {
    /// Rolling skewness of log-returns.
    pub fn skew(&self) -> f64 {
        self.skew
    }

    /// Rolling kurtosis of log-returns.
    pub fn kurt(&self) -> f64 {
        self.kurt
    }
}

impl Default for HigherMoments {
    /// Factory defaults: window=50 (clamped to max(50,3)=50).
    fn default() -> Self {
        Self::new(50)
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

/// Own config for [`HigherMoments`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HmomConfig {
    pub period: Param<usize>,
}

impl Indicator for HigherMoments {
    const ID: IndicatorId = IndicatorId::Hmom;
    /// Not a pluggable family member — a statistical feature extractor (skewness +
    /// kurtosis of log-returns over a rolling window). Consumed by name, not as a
    /// substitutable oscillator or smoother.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Uses only close (configurable single field).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::HmomSkew),
        Output::magnitude(IndicatorOutputId::HmomKurt),
    ];
    /// O(period): each bar rescans the window to compute moments. One period-deep Vec
    /// for the close ring (window+1 entries).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    type Config = HmomConfig;
    type Runtime = HigherMoments;

    fn create(cfg: HmomConfig) -> HigherMoments {
        HigherMoments::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &HmomConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for HmomConfig {
    fn defaults() -> Self {
        HmomConfig { period: Param::Solo(50) }
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


impl Render for HigherMoments {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::HmomSkew,
                "Skewness",
                Color::hex(0x9C27B0),
            )
            .line_output(
                IndicatorOutputId::HmomKurt,
                "Kurtosis",
                Color::hex(0x7B1FA2),
            )
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_higher_moments_creation() {
        let hm = HigherMoments::new(20);
        assert!(!hm.is_ready());
        assert_eq!(hm.skew, 0.0);
        assert_eq!(hm.kurt, 0.0);
        assert_eq!(hm.window(), 20);
    }

    #[test]
    fn test_higher_moments_finite() {
        let mut hm = HigherMoments::new(20);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (skew, kurt) = hm.feed(price);
            assert!(skew.is_finite(), "Skew should be finite");
            assert!(kurt.is_finite(), "Kurtosis should be finite");
        }
        assert!(hm.is_ready());
    }

    #[test]
    fn test_higher_moments_kurtosis_positive() {
        let mut hm = HigherMoments::new(20);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 0.5;
            hm.feed(price);
        }
        assert!(hm.is_ready());
        {
            let kurt = hm.kurt();
            assert!(kurt >= 0.0, "Kurtosis should be non-negative, got {}", kurt);
        }
    }

    #[test]
    fn test_higher_moments_reset() {
        let mut hm = HigherMoments::new(20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            hm.feed(price);
        }
        assert!(hm.is_ready());
        hm.reset();
        assert!(!hm.is_ready());
        assert_eq!(hm.skew, 0.0);
        assert_eq!(hm.kurt, 0.0);
    }

    /// The factory resolves close from the bar (open/high/low/volume are wildcards to
    /// prove only close is consumed). After warmup, skew (`main()`) is finite.
    #[test]
    fn factory_feeds_resolved_hmom() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Hmom(<<HigherMoments as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=80 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.read(IndicatorOutputId::HmomSkew).is_finite(), "skew should be finite after warmup");
    }
}
