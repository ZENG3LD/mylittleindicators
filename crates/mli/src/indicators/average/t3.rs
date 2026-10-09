//! Tillson T3 Moving Average indicator.

use super::ema::Ema;
use crate::engine::ohlcv_field::OhlcvField;

/// Tillson T3 Moving Average - ultra-smooth, low-lag moving average.
///
/// T3 = c1×EMA6 + c2×EMA5 + c3×EMA4 + c4×EMA3
///
/// where coefficients depend on the volume factor `a` (default: 0.7).
///
/// Uses 6 cascaded EMAs combined with polynomial coefficients to achieve
/// exceptional smoothness while minimizing lag.
///
/// # Parameters
/// - `period`: EMA period for each of the 6 stages
/// - `a`: Volume factor (default: 0.7, range 0-1)
///
/// # Implementation
///
/// Uses six O(1) EMA instances, so the entire T3 is O(1).
#[derive(Debug, Clone)]
pub struct T3 {
    period: usize,
    a: f64,
    e1: Ema,
    e2: Ema,
    e3: Ema,
    e4: Ema,
    e5: Ema,
    e6: Ema,
    value: f64,
}

impl T3 {
    /// Creates a new T3 with default volume factor (a=0.7).
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - EMA period for each cascade stage
    pub fn new(period: usize) -> Self {
        Self::with_alpha(period, 0.7)
    }

    /// Creates a new T3 with custom volume factor.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - EMA period for each cascade stage
    /// * `a` - Volume factor (0.0-1.0, higher = smoother)
    pub fn with_alpha(period: usize, a: f64) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            a,
            e1: Ema::new(p),
            e2: Ema::new(p),
            e3: Ema::new(p),
            e4: Ema::new(p),
            e5: Ema::new(p),
            e6: Ema::new(p),
            value: 0.0,
        }
    }

    /// Updates the T3 with a new bar and returns the current value.
    ///
    /// The price is taken from the configured `source` field (default close).
    pub fn feed(&mut self, value: f64) -> f64 {
        let v = value;
        let e1 = self.e1.feed(v);
        let e2 = self.e2.feed(e1);
        let e3 = self.e3.feed(e2);
        let e4 = self.e4.feed(e3);
        let e5 = self.e5.feed(e4);
        let e6 = self.e6.feed(e5);
        let a = self.a;
        self.value = e6 * (a * a * a)
            + e5 * (3.0 * a * a * (1.0 - a))
            + e4 * (3.0 * a * (1.0 - a) * (1.0 - a))
            + e3 * ((1.0 - a) * (1.0 - a) * (1.0 - a));
        self.value
    }

    /// Returns the current T3 value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the T3 has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.e6.is_ready()
    }

    /// Resets the T3 to its initial state.
    pub fn reset(&mut self) {
        self.e1.reset();
        self.e2.reset();
        self.e3.reset();
        self.e4.reset();
        self.e5.reset();
        self.e6.reset();
        self.value = 0.0;
    }

    /// Returns the period of this T3.
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`T3`]: period + source + the volume factor `vfactor` (0..1,
/// the Tillson smoothing coefficient `a`) — moved off the raw `additional_params`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct T3Config {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    pub vfactor: Param<f64>,
}

impl crate::contract::Config for T3Config {
    fn defaults() -> Self {
        T3Config {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            vfactor: Param::Solo(0.7),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        let mut s = Self::machine_defaults_auto();
        // vfactor: Class D (volume factor 0..1, T3 taxonomy) — sweep_f64(0.0,1.0,0.1).
        // Note: taxonomy §1.5 lists vfactor as Class E sub-case with step 0.1
        // (same 0.0..1.0 domain as Class D but documented under E — using step 0.1
        // per §2 Class E vfactor sub-case: 0.0..=1.0 step 0.1).
        s.vfactor = Param::many(sweep_f64(0.0, 1.0, 0.1));
        s
    }
}

impl Indicator for T3 {
    const ID: IndicatorId = IndicatorId::T3;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Six cascaded EMAs (Tillson). Own state is O(1) (the `a` coefficient); the six
    /// EMAs are edges, charged recursively by the barometer (not hand-approximated).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::T3)];
    type Config = T3Config;
    type Runtime = T3;

    fn create(cfg: T3Config) -> T3 {
        T3::with_alpha(cfg.period.resolved(), cfg.vfactor.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &Self::Config) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for T3 {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::T3, "T3", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_t3_basic_calculation() {
        let mut t3 = T3::new(5);

        for i in 1..=20 {
            t3.feed(i as f64 * 10.0);
        }

        assert!(t3.is_ready());
        assert!(t3.value() > 0.0);
    }

    #[test]
    fn test_t3_custom_alpha() {
        let mut t3 = T3::with_alpha(5, 0.9);

        for i in 1..=20 {
            t3.feed(i as f64 * 10.0);
        }

        assert!(t3.is_ready());
    }

    #[test]
    fn test_t3_reset() {
        let mut t3 = T3::new(3);
        for i in 1..=20 {
            t3.feed(i as f64 * 10.0);
        }
        assert!(t3.is_ready());

        t3.reset();
        assert!(!t3.is_ready());
    }

}
