//! Exponential Moving Average (EMA) indicator.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Family, Indicator, Output, Param, Smoother};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Exponential Moving Average (EMA) - weighted average giving more weight to recent prices.
///
/// EMA = α × Price + (1 - α) × EMA_prev, where α = 2 / (period + 1)
///
/// # Implementation
///
/// O(1) update complexity, no buffer needed.
#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    alpha: f64,
    value: f64,
    count: usize,
}

impl Ema {
    /// Returns the period of this EMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new EMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Smoothing period (α = 2/(period+1))
    pub fn new(period: usize) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            alpha,
            value: 0.0,
            count: 0,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The caller
    /// (standalone driver or host composite) supplies the value; the core knows
    /// nothing about OHLCV or fields.
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.count == 0 {
            self.value = value;
        } else {
            self.value = self.alpha * value + (1.0 - self.alpha) * self.value;
        }
        self.count += 1;
        self.value
    }

    /// Returns the current EMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current EMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the EMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the EMA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
    }
}

impl Smoother for Ema {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Ema::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Ema`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for EmaConfig {
    fn defaults() -> Self {
        EmaConfig {
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

impl Indicator for Ema {
    const ID: IndicatorId = IndicatorId::Ema;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Ema)];
    type Config = EmaConfig;
    type Runtime = Ema;

    fn create(cfg: EmaConfig) -> Ema {
        Ema::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the
    /// resolved scalar — the core ingests via `feed(f64)`, knowing no OHLCV fields.
    fn source_fields(cfg: &EmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Ema {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Ema, "EMA", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_basic_calculation() {
        let mut ema = Ema::new(3);
        // α = 2/(3+1) = 0.5

        // First bar: EMA = close
        let v1 = ema.feed(10.0);
        assert!((v1 - 10.0).abs() < 1e-10);

        // Second bar: EMA = 0.5*20 + 0.5*10 = 15
        let v2 = ema.feed(20.0);
        assert!((v2 - 15.0).abs() < 1e-10);

        // Third bar: EMA = 0.5*30 + 0.5*15 = 22.5
        let v3 = ema.feed(30.0);
        assert!(ema.is_ready());
        assert!((v3 - 22.5).abs() < 1e-10);
    }

    #[test]
    fn test_ema_alpha_calculation() {
        // Period 9: α = 2/10 = 0.2
        let ema = Ema::new(9);
        assert!((ema.alpha - 0.2).abs() < 1e-10);

        // Period 19: α = 2/20 = 0.1
        let ema = Ema::new(19);
        assert!((ema.alpha - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_ema_reset() {
        let mut ema = Ema::new(3);

        ema.feed(10.0);
        ema.feed(20.0);
        ema.feed(30.0);
        assert!(ema.is_ready());

        ema.reset();
        assert!(!ema.is_ready());
        assert!((ema.value_f64()).abs() < 1e-10);
    }

    #[test]
    fn test_ema_value_types() {
        let mut ema = Ema::new(2);
        ema.feed(10.0);
        ema.feed(20.0);

        let indicator_val = ema.value();
        let f64_val = ema.value_f64();

        let v = indicator_val;
        assert!((v - f64_val).abs() < 1e-10);
    }
}






















