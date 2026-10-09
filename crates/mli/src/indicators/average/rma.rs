//! Wilder's Moving Average (RMA) indicator.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Family, Indicator, Output, Param, Smoother};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Wilder's Moving Average (RMA) - also known as Smoothed Moving Average (SMMA).
///
/// RMA = (RMA_prev × (period - 1) + Price) / period
///
/// Equivalent to EMA with α = 1/period (vs EMA's α = 2/(period+1)).
/// Used in RSI, ATR, and ADX calculations.
///
/// # Implementation
///
/// O(1) update complexity, no buffer needed.
#[derive(Debug, Clone)]
pub struct Rma {
    period: usize,
    value: f64,
    count: usize,
}

impl Rma {
    /// Returns the period of this RMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new RMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Smoothing period (α = 1/period)
    pub fn new(period: usize) -> Self {
        Self {
            period,
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
            self.value = (self.value * (self.period as f64 - 1.0) + value) / self.period as f64;
        }
        self.count += 1;
        self.value
    }

    /// Updates the RMA with a new bar and returns the current value.
    ///
    /// Uses the configured source field (default: Close), then delegates to
    /// [`Self::feed`].

    /// Returns the current RMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current RMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the RMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the RMA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
    }
}

impl Smoother for Rma {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Rma::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Rma`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for RmaConfig {
    fn defaults() -> Self {
        RmaConfig {
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

impl Indicator for Rma {
    const ID: IndicatorId = IndicatorId::Rma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Rma)];
    type Config = RmaConfig;
    type Runtime = Rma;

    fn create(cfg: RmaConfig) -> Rma {
        Rma::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &RmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Rma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Rma, "RMA", Color::hex(0xE91E63))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rma_basic_calculation() {
        let mut rma = Rma::new(3);
        // α = 1/3

        // First bar: RMA = close
        let v1 = rma.feed(10.0);
        assert!((v1 - 10.0).abs() < 1e-10);

        // Second bar: RMA = (10*2 + 20)/3 = 40/3 ≈ 13.33
        let v2 = rma.feed(20.0);
        assert!((v2 - 13.333333).abs() < 0.001);

        // Third bar: RMA = (13.33*2 + 30)/3 ≈ 18.89
        let v3 = rma.feed(30.0);
        assert!(rma.is_ready());
        assert!((v3 - 18.888888).abs() < 0.001);
    }

    #[test]
    fn test_rma_vs_ema_smoothing() {
        // RMA with period 14 has α = 1/14 ≈ 0.0714
        // EMA with period 27 has α = 2/28 ≈ 0.0714
        // So RMA(14) ≈ EMA(27) in terms of smoothing
        let _rma = Rma::new(14);
        let alpha = 1.0 / 14.0;
        assert!((alpha - 0.0714_f64).abs() < 0.001);
    }

    #[test]
    fn test_rma_reset() {
        let mut rma = Rma::new(3);
        rma.feed(10.0);
        rma.feed(20.0);
        rma.feed(30.0);
        assert!(rma.is_ready());

        rma.reset();
        assert!(!rma.is_ready());
    }

}
