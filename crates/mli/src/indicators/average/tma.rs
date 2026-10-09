//! Triangular Moving Average (TMA) indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// Triangular Moving Average (TMA) - double-smoothed SMA.
///
/// TMA = SMA(SMA(price, period), period)
///
/// Provides extra smoothing with more weight given to middle prices.
/// Slower to react than SMA but smoother.
///
/// # Implementation
///
/// Uses two cascaded SMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Tma {
    period: usize,
    sma1: super::sma::Sma,
    sma2: super::sma::Sma,
    value: f64,
    initialized: bool,
}

impl Tma {
    /// Returns the period of this TMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the TMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.sma1.is_ready() && self.sma2.is_ready()
    }

    /// Resets the TMA to its initial state.
    pub fn reset(&mut self) {
        self.sma1.reset();
        self.sma2.reset();
        self.value = 0.0;
        self.initialized = false;
    }

    /// Creates a new TMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal SMAs
    pub fn new(period: usize) -> Self {
        Self {
            period,
            sma1: super::sma::Sma::new(period),
            sma2: super::sma::Sma::new(period),
            value: 0.0,
            initialized: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Cascades through
    /// the two inner SMA cores via their own `feed` (no OHLCV-hack).
    pub fn feed(&mut self, value: f64) -> f64 {
        let sma1_val = self.sma1.feed(value);
        let sma2_val = self.sma2.feed(sma1_val);
        self.value = sma2_val;
        if !self.initialized && self.sma2.is_ready() {
            self.initialized = true;
        }
        self.value
    }

    /// Updates the TMA with a new bar and returns the current value.
    ///
    /// Both SMA cores receive already-resolved scalars via their own `feed`.

    /// Returns the current TMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current TMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the TMA has been fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Smoother, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

impl Smoother for Tma {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Tma::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Tma`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for TmaConfig {
    fn defaults() -> Self {
        TmaConfig {
            period: Param::Solo(14),
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

impl Indicator for Tma {
    const ID: IndicatorId = IndicatorId::Tma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Two cascaded SMAs. Own state is O(1); each SMA is an edge, so its period-deep
    /// window is charged recursively by the barometer (not hand-counted here).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Sma, &[IndicatorOutputId::Sma]),
            Port::new(IndicatorId::Sma, &[IndicatorOutputId::Sma]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Tma)];
    type Config = TmaConfig;
    type Runtime = Tma;

    fn create(cfg: TmaConfig) -> Tma {
        Tma::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &TmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Tma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Tma, "TMA", Color::hex(0xCDDC39))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tma_basic_calculation() {
        let mut tma = Tma::new(3);

        // Need 2*period - 1 bars for TMA to be ready
        for i in 1..=5 {
            tma.feed(i as f64 * 10.0);
        }

        assert!(tma.is_ready());
        assert!(tma.value() > 0.0);
    }

    #[test]
    fn test_tma_reset() {
        let mut tma = Tma::new(3);
        for i in 1..=6 {
            tma.feed(i as f64 * 10.0);
        }
        assert!(tma.is_ready());

        tma.reset();
        assert!(!tma.is_ready());
        assert!(!tma.is_initialized());
    }
}
