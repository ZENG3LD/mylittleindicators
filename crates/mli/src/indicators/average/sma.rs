//! Simple Moving Average (SMA) indicator.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Smoother, Store, StoreKind, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Simple Moving Average (SMA) - arithmetic mean of the last N values from a configurable source.
///
/// SMA = (P1 + P2 + ... + Pn) / n
///
/// # Implementation
///
/// Uses a ring buffer with O(1) update complexity. Maximum period is 512.
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    sum: f64,
    count: usize,
    value: f64,
    buf: Vec<f64>,
    idx: usize,
}

impl Sma {
    /// Returns the period of this SMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new SMA with the specified period. PURE core — owns no source;
    /// the factory feeds it the resolved scalar via `feed` (Field source lives on
    /// the `ContractFactory` variant).
    ///
    /// # Arguments
    /// * `period` - Number of bars to average (1..=512)
    pub fn new(period: usize) -> Self {
        Self {
            period,
            sum: 0.0,
            count: 0,
            value: 0.0,
            buf: Vec::with_capacity(period),
            idx: 0,
        }
    }

    /// 0.1.8 entry. Feeds `close`. The factory path is [`Self::feed`].
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        self.feed(close)
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The caller
    /// (the factory's source-resolving feed, or a host composite) supplies the value;
    /// the core knows nothing about OHLCV or fields.
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.count < self.period {
            self.buf.push(value);
            self.sum += value;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            let old = self.buf[self.idx];
            self.sum += value - old;
            self.buf[self.idx] = value;
            self.idx = (self.idx + 1) % self.period;
        }
        self.value = self.sum / self.count as f64;
        self.value
    }

    /// Returns the current SMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current SMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the SMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the SMA to its initial state.
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.count = 0;
        self.value = 0.0;
        self.buf.fill(0.0);
        self.idx = 0;
    }
}

impl Smoother for Sma {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Sma::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Sma`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for SmaConfig {
    fn defaults() -> Self {
        SmaConfig {
            period: Param::Solo(20),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        Self::machine_defaults_auto()
    }
}

impl Indicator for Sma {
    const ID: IndicatorId = IndicatorId::Sma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Keeps a period-deep ring `Vec` (the window) but updates via a running sum,
    /// so O(1) compute — heavier than EMA (no window store) by the buffer, scaling
    /// with period.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Sma)];
    type Config = SmaConfig;
    type Runtime = Sma;

    fn create(cfg: SmaConfig) -> Sma {
        Sma::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the
    /// resolved scalar — so this `Sma` is pure (no `source` field, no `update_bar`).
    fn source_fields(cfg: &SmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl crate::contract::GpuShader for Sma {
    fn shader_source() -> &'static str {
        include_str!("../../contract/shaders/sma_window.wgsl")
    }

    fn shader_entry() -> &'static str {
        "sma_main"
    }
}

impl Render for Sma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Sma, "SMA", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_sma_basic_calculation() {
        let mut sma = Sma::new(3);

        let v1 = sma.feed(10.0);
        assert!(!sma.is_ready());
        assert!((v1 - 10.0).abs() < 1e-10);

        let v2 = sma.feed(20.0);
        assert!(!sma.is_ready());
        assert!((v2 - 15.0).abs() < 1e-10); // (10+20)/2

        let v3 = sma.feed(30.0);
        assert!(sma.is_ready());
        assert!((v3 - 20.0).abs() < 1e-10); // (10+20+30)/3

        let v4 = sma.feed(40.0);
        assert!((v4 - 30.0).abs() < 1e-10); // (20+30+40)/3
    }

    #[test]
    fn test_sma_period_1() {
        let mut sma = Sma::new(1);

        let v = sma.feed(42.0);
        assert!(sma.is_ready());
        assert!((v - 42.0).abs() < 1e-10);

        let v2 = sma.feed(100.0);
        assert!((v2 - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_sma_reset() {
        let mut sma = Sma::new(3);

        sma.feed(10.0);
        sma.feed(20.0);
        sma.feed(30.0);
        assert!(sma.is_ready());

        sma.reset();
        assert!(!sma.is_ready());
        assert!((sma.value_f64()).abs() < 1e-10);
    }

    #[test]
    fn test_sma_value_types() {
        let mut sma = Sma::new(2);
        sma.feed(10.0);
        sma.feed(20.0);

        // Check both value methods return same result
        let v = sma.value();
        let f64_val = sma.value_f64();
        assert!((v - f64_val).abs() < 1e-10);
    }

    #[test]
    fn test_sma_period_getter() {
        let sma = Sma::new(14);
        assert_eq!(sma.period(), 14);
    }

    /// The `ContractFactory` variant holds the resolved source and the `Source`
    /// feed extracts it before calling the pure core. Build an SMA via the typed
    /// order path (`IndicatorOrder::Sma(..).build()`, source from `Render::defaults()`
    /// = close) and confirm it averages the closes (not the highs) — i.e. the
    /// variant-source feed path is live.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::contract::Param;
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = {
            let mut c = <<Sma as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
            c.period = Param::Solo(3);
            IndicatorOrder::Sma(c).build_solo().unwrap()
        };
        let bar = |close: f64| MarketSample::Bar {
            open: 100.0,
            high: 200.0,
            low: 90.0,
            close,
            volume: 0.0,
        };
        f.feed(0, bar(110.0));
        f.feed(0, bar(120.0));
        f.feed(0, bar(130.0));
        // avg of closes 110/120/130 = 120; highs (200) would give 200.
        let v = f.primary();
        assert!((v - 120.0).abs() < 1e-10, "factory must average CLOSE (120), got {}", v);
    }
}






















