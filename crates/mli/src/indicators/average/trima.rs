//! Triangular Moving Average (TRIMA) indicator.

use super::sma::Sma;
use crate::engine::ohlcv_field::OhlcvField;

/// Triangular Moving Average (TRIMA) - double-smoothed SMA.
///
/// TRIMA = SMA(SMA(price, period), period)
///
/// Applies two SMA passes to achieve a triangular-weighted average
/// that gives more weight to the middle of the period. The two inner SMAs are a
/// FIXED `Port` (TRIMA *is* SMA-of-SMA by definition), not a configurable slot.
///
/// # Implementation
///
/// Uses two cascaded SMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Trima {
    period: usize,
    sma1: Sma,
    sma2: Sma,
    value: f64,
}

impl Trima {
    /// Creates a new TRIMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal SMAs
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            sma1: Sma::new(p),
            sma2: Sma::new(p),
            value: 0.0,
        }
    }

    /// Resets the TRIMA to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.sma1.reset();
        self.sma2.reset();
        self.value = 0.0;
    }

    /// Returns `true` if the TRIMA has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.sma1.is_ready() && self.sma2.is_ready()
    }

    /// Returns the current TRIMA value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Cascades through
    /// the two inner SMA cores via their own `feed` (no OHLCV-hack).
    pub fn feed(&mut self, value: f64) -> f64 {
        let a = self.sma1.feed(value);
        self.value = self.sma2.feed(a);
        self.value
    }

    /// Returns the current TRIMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns the period of this TRIMA.
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Smoother, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

impl Smoother for Trima {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Trima::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Trima`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TrimaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for TrimaConfig {
    fn defaults() -> Self {
        TrimaConfig {
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

impl Indicator for Trima {
    const ID: IndicatorId = IndicatorId::Trima;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Two cascaded SMAs (triangular = SMA of SMA). Own state is O(1); each SMA is an
    /// edge, its window charged recursively by the barometer (not hand-counted here).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Sma, &[IndicatorOutputId::Sma]),
            Port::new(IndicatorId::Sma, &[IndicatorOutputId::Sma]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Trima)];
    type Config = TrimaConfig;
    type Runtime = Trima;

    fn create(cfg: TrimaConfig) -> Trima {
        Trima::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the
    /// resolved scalar; the core cascades it through the two inner SMAs via `feed`.
    fn source_fields(cfg: &TrimaConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Trima {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Trima, "TRIMA", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trima_basic_calculation() {
        let mut trima = Trima::new(3);

        for i in 1..=6 {
            trima.feed(i as f64 * 10.0);
        }

        assert!(trima.is_ready());
        assert!(trima.value() > 0.0);
    }

    #[test]
    fn test_trima_reset() {
        let mut trima = Trima::new(3);
        for i in 1..=6 {
            trima.feed(i as f64 * 10.0);
        }
        assert!(trima.is_ready());

        trima.reset();
        assert!(!trima.is_ready());
    }

}
