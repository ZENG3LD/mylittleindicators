//! Triple Exponential Moving Average (TEMA) indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// Triple Exponential Moving Average (TEMA) - further reduces lag compared to DEMA.
///
/// TEMA = 3 × EMA1 - 3 × EMA2 + EMA3
///
/// where EMA1 = EMA(price), EMA2 = EMA(EMA1), EMA3 = EMA(EMA2)
///
/// Created by Patrick Mulloy. Provides even faster response than DEMA
/// while maintaining smoothness.
///
/// # Implementation
///
/// Uses three cascaded EMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Tema {
    period: usize,
    ema1: super::ema::Ema,
    ema2: super::ema::Ema,
    ema3: super::ema::Ema,
    value: f64,
    initialized: bool,
}

impl Tema {
    /// Returns the period of this TEMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the TEMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.ema1.is_ready() && self.ema2.is_ready() && self.ema3.is_ready()
    }

    /// Resets the TEMA to its initial state.
    pub fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.ema3.reset();
        self.value = 0.0;
        self.initialized = false;
    }

    /// Creates a new TEMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for all three internal EMAs
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ema1: super::ema::Ema::new(period),
            ema2: super::ema::Ema::new(period),
            ema3: super::ema::Ema::new(period),
            value: 0.0,
            initialized: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Cascades through
    /// the three inner EMA cores via their own `feed` (no OHLCV-hack).
    pub fn feed(&mut self, value: f64) -> f64 {
        let ema1_val = self.ema1.feed(value);
        let ema2_val = self.ema2.feed(ema1_val);
        let ema3_val = self.ema3.feed(ema2_val);
        self.value = 3.0 * ema1_val - 3.0 * ema2_val + ema3_val;
        if !self.initialized && self.ema3.is_ready() {
            self.initialized = true;
        }
        self.value
    }

    /// Updates the TEMA with a new bar and returns the current value.
    ///
    /// All three EMA cores receive already-resolved scalars via their own `feed`.

    /// Returns the current TEMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current TEMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the TEMA has been fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Smoother, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

impl Smoother for Tema {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Tema::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Tema`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TemaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for TemaConfig {
    fn defaults() -> Self {
        TemaConfig {
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

impl Indicator for Tema {
    const ID: IndicatorId = IndicatorId::Tema;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// TEMA = 3·EMA − 3·EMA(EMA) + EMA(EMA(EMA)): three cascaded EMAs. Own state is
    /// O(1); the three EMAs are edges, charged recursively by the barometer.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Tema)];
    type Config = TemaConfig;
    type Runtime = Tema;

    fn create(cfg: TemaConfig) -> Tema {
        Tema::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &TemaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Tema {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Tema, "TEMA", Color::hex(0x3F51B5))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tema_basic_calculation() {
        let mut tema = Tema::new(3);

        tema.feed(10.0);
        tema.feed(20.0);
        let v3 = tema.feed(30.0);

        assert!(tema.is_ready());
        // TEMA should be even more responsive than DEMA
        assert!(v3 > 20.0);
    }

    #[test]
    fn test_tema_reset() {
        let mut tema = Tema::new(3);
        tema.feed(10.0);
        tema.feed(20.0);
        tema.feed(30.0);
        assert!(tema.is_ready());

        tema.reset();
        assert!(!tema.is_ready());
        assert!(!tema.is_initialized());
    }
}
