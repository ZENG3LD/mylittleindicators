//! Double Exponential Moving Average (DEMA) indicator.

use super::ema::Ema;
use crate::engine::ohlcv_field::OhlcvField;

/// Double Exponential Moving Average (DEMA) - reduces lag compared to EMA.
///
/// DEMA = 2 × EMA(price) - EMA(EMA(price))
///
/// Created by Patrick Mulloy. Provides faster response to price changes
/// while reducing noise compared to a single EMA.
///
/// # Implementation
///
/// Uses two cascaded EMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Dema {
    ema1: Ema,
    ema2: Ema,
    value: f64,
    count: usize,
    period: usize,
}

impl Dema {
    /// Returns the period of this DEMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new DEMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal EMAs
    pub fn new(period: usize) -> Self {
        Self {
            ema1: Ema::new(period),
            ema2: Ema::new(period),
            value: 0.0,
            count: 0,
            period,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Cascades through
    /// the two inner EMA cores via their own `feed` (no OHLCV-hack).
    pub fn feed(&mut self, value: f64) -> f64 {
        let ema1_val = self.ema1.feed(value);
        let ema2_val = self.ema2.feed(ema1_val);
        self.value = 2.0 * ema1_val - ema2_val;
        self.count += 1;
        self.value
    }

    /// Updates the DEMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close), then
    /// delegates to [`Self::feed`].

    /// Returns the current DEMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current DEMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the DEMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the DEMA to its initial state.
    pub fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.value = 0.0;
        self.count = 0;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Smoother, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

impl Smoother for Dema {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Dema::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Dema`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DemaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for DemaConfig {
    fn defaults() -> Self {
        DemaConfig {
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

impl Indicator for Dema {
    const ID: IndicatorId = IndicatorId::Dema;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// DEMA = 2·EMA(price) − EMA(EMA(price)): two cascaded EMAs. Its OWN state is
    /// O(1) (the combine scalars) — the two EMAs are NOT base; they are edges, so
    /// the barometer charges each EMA's cost recursively instead of hand-counting.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Dema)];
    type Config = DemaConfig;
    type Runtime = Dema;

    fn create(cfg: DemaConfig) -> Dema {
        Dema::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &DemaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Dema {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Dema, "DEMA", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dema_basic_calculation() {
        let mut dema = Dema::new(3);

        dema.feed(10.0);
        dema.feed(20.0);
        let v3 = dema.feed(30.0);

        assert!(dema.is_ready());
        // DEMA should be more responsive than EMA
        assert!(v3 > 20.0); // Should be closer to recent prices
    }

    #[test]
    fn test_dema_reset() {
        let mut dema = Dema::new(3);
        dema.feed(10.0);
        dema.feed(20.0);
        dema.feed(30.0);
        assert!(dema.is_ready());

        dema.reset();
        assert!(!dema.is_ready());
    }
}
