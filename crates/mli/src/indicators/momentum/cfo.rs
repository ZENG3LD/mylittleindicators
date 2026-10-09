//! Chande Forecast Oscillator (CFO).

use crate::indicators::average::lr::LinearRegressionMA;
use crate::engine::ohlcv_field::OhlcvField;

/// Chande Forecast Oscillator (CFO).
///
/// `CFO = 100 * (price - TSF) / price`, where `TSF` is the n-period **time-series
/// forecast** — the endpoint of the least-squares linear-regression line over the last
/// `period` prices. It measures, as a percentage, how far price has run from its own
/// regression trendline: ~0 while price tracks its linear trend, positive when price
/// sits above the forecast, negative below.
///
/// This is the genuine Chande oscillator (regression forecast), NOT `price - MA(price)`
/// — that residual-against-a-lagging-MA form is what [`super::detrended_synthetic_price`]
/// (DSP) computes.
#[derive(Debug, Clone)]
pub struct Cfo {
    period: usize,
    /// The forecast engine: the regression endpoint (`value()`) is the TSF.
    lr: LinearRegressionMA,
    value: f64,
}

impl Cfo {
    /// Default ctor — regression forecast over the fed scalar. The inner regression reads
    /// Close so it can be fed the resolved price as a synthetic close.
    pub fn new(period: usize) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            lr: LinearRegressionMA::new(p),
            value: 0.0,
        }
    }

    /// Feed ONE pre-extracted scalar — the resolved price (const SOURCE = Field{Close}). The
    /// inner regression is a pure Field core; feed it the resolved scalar directly. Knows no
    /// transport.
    pub fn feed(&mut self, value: f64) -> f64 {
        let tsf = self.lr.feed(value);
        self.value = if self.lr.is_ready() && value.abs() > 1e-12 {
            100.0 * (value - tsf) / value
        } else {
            0.0
        };
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.lr.is_ready()
    }
    pub fn reset(&mut self) {
        self.lr.reset();
        self.value = 0.0;
    }
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`Cfo`] — Chande Forecast Oscillator. Owns the regression
/// `period` + price `source`; the forecast is the embedded linear-regression node (no
/// generic smoother slot — the forecast IS a least-squares fit, not a moving average).
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CfoConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for Cfo {
    const ID: IndicatorId = IndicatorId::Cfo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// CFO's own update is O(1) over the forecast; the forecast is an embedded
    /// linear-regression node (its `value` port = TSF). The `Lr` edge charges its full
    /// O(period) least-squares cost recursively — one source of truth for the regression.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Lr, &[IndicatorOutputId::LrLine])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cfo)];
    type Config = CfoConfig;
    type Runtime = Cfo;

    fn create(cfg: CfoConfig) -> Cfo {
        Cfo::new(cfg.period.resolved())
    }

    /// Single configurable price field — the factory resolves it and feeds the scalar.
    fn source_fields(cfg: &CfoConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for CfoConfig {
    fn defaults() -> Self {
        CfoConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        Self::machine_defaults_auto()
    }
}


impl Render for Cfo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cfo, "CFO", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfo_creation() {
        let cfo = Cfo::new(14);
        assert!(!cfo.is_ready());
        assert_eq!(cfo.value(), 0.0);
        assert_eq!(cfo.period(), 14);
    }

    #[test]
    fn test_cfo_min_period() {
        let cfo = Cfo::new(1);
        assert_eq!(cfo.period(), 2); // min period is 2
    }

    #[test]
    fn test_cfo_constant_is_zero() {
        let mut cfo = Cfo::new(5);
        // Constant price: regression endpoint == price -> CFO == 0.
        for _ in 0..20 {
            cfo.feed(100.0);
        }
        assert!(cfo.is_ready());
        assert!(cfo.value().abs() < 1e-9, "CFO should be 0 for constant prices, got {}", cfo.value());
    }

    #[test]
    fn test_cfo_linear_trend_near_zero() {
        let mut cfo = Cfo::new(10);
        // A clean linear ramp lies exactly on its own regression line -> CFO ~= 0.
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            cfo.feed(price);
        }
        assert!(cfo.is_ready());
        assert!(cfo.value().abs() < 1e-6, "CFO on a linear trend should be ~0, got {}", cfo.value());
    }

    #[test]
    fn test_cfo_above_forecast_positive() {
        let mut cfo = Cfo::new(10);
        for _ in 0..12 {
            cfo.feed(100.0);
        }
        // A jump above the established trend leaves price above its regression forecast.
        cfo.feed(110.0);
        assert!(cfo.is_ready());
        assert!(cfo.value() > 0.0, "CFO should be > 0 when price is above its forecast, got {}", cfo.value());
    }

    #[test]
    fn test_cfo_below_forecast_negative() {
        let mut cfo = Cfo::new(10);
        for _ in 0..12 {
            cfo.feed(100.0);
        }
        // A drop below the established trend leaves price below its regression forecast.
        cfo.feed(90.0);
        assert!(cfo.is_ready());
        assert!(cfo.value() < 0.0, "CFO should be < 0 when price is below its forecast, got {}", cfo.value());
    }

    #[test]
    fn test_cfo_reset() {
        let mut cfo = Cfo::new(5);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            cfo.feed(price);
        }
        assert!(cfo.is_ready());
        cfo.reset();
        assert!(!cfo.is_ready());
        assert_eq!(cfo.value(), 0.0);
    }

    #[test]
    fn test_cfo_finite_values() {
        let mut cfo = Cfo::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = cfo.feed(price);
            assert!(value.is_finite(), "CFO should always be finite");
        }
    }

    #[test]
    fn test_cfo_contract_create() {
        let cfg = <<Cfo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 14);
        let mut cfo = <Cfo as Indicator>::create(cfg);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            cfo.feed(price);
        }
        assert!(cfo.is_ready());
    }

    /// The factory resolves the configured close field (not the wild 9999 high) and feeds the
    /// scalar; a linear close ramp lies on its own regression -> CFO ~ 0.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::{MarketSample, Param};
        let mut c = <<Cfo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        c.period = Param::Solo(10);
        let mut f = IndicatorOrder::Cfo(c).build_solo().unwrap();
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().abs() < 1e-3, "factory CFO on a linear close should be ~0, got {}", f.primary());
    }
}
