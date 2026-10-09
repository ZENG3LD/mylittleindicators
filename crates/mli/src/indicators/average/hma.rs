//! Hull Moving Average (HMA) indicator.

use crate::indicators::average::wma::Wma;
use crate::engine::ohlcv_field::OhlcvField;

/// Hull Moving Average (HMA) - designed to reduce lag while maintaining smoothness.
///
/// HMA = WMA(2×WMA(period/2) - WMA(period), sqrt(period))
///
/// Created by Alan Hull. Combines two WMAs to reduce lag, then smooths
/// with a shorter WMA.
///
/// # Implementation
///
/// Uses three O(1) WMA instances, so the entire HMA is O(1).
#[derive(Debug, Clone)]
pub struct Hma {
    period: usize,
    wma1: Wma,       // period/2
    wma2: Wma,       // period
    wma_final: Wma,  // sqrt(period)
    count: usize,
    value: f64,
}

impl Hma {
    /// Returns the period of this HMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns the number of bars processed.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Creates a new HMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Main period (internally uses period/2 and sqrt(period))
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        let period2 = (p / 2).max(1);
        let sqrt_period = (p as f64).sqrt().floor() as usize;
        let sqrt_period = sqrt_period.max(1);

        Self {
            period: p,
            wma1: Wma::new(period2),
            wma2: Wma::new(p),
            wma_final: Wma::new(sqrt_period),
            count: 0,
            value: 0.0,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Cascades through
    /// the three inner WMA cores via their own `feed` (no OHLCV-hack).
    pub fn feed(&mut self, value: f64) -> f64 {
        let w1 = self.wma1.feed(value);
        let w2 = self.wma2.feed(value);
        let diff = 2.0 * w1 - w2;
        let hma = self.wma_final.feed(diff);
        self.count += 1;
        self.value = hma;
        self.value
    }

    /// Updates the HMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close), then
    /// delegates to [`Self::feed`]. O(1) operation.

    /// Returns the current HMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current HMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the HMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the HMA to its initial state.
    pub fn reset(&mut self) {
        self.wma1.reset();
        self.wma2.reset();
        self.wma_final.reset();
        self.count = 0;
        self.value = 0.0;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Smoother, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

impl Smoother for Hma {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Hma::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Hma`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for HmaConfig {
    fn defaults() -> Self {
        HmaConfig {
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

impl Indicator for Hma {
    const ID: IndicatorId = IndicatorId::Hma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Three internal WMAs (period/2, period, √period). Own state is O(1); each WMA
    /// is an edge, charged recursively by the barometer (not hand-counted here).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Wma, &[IndicatorOutputId::Wma]),
            Port::new(IndicatorId::Wma, &[IndicatorOutputId::Wma]),
            Port::new(IndicatorId::Wma, &[IndicatorOutputId::Wma]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Hma)];
    type Config = HmaConfig;
    type Runtime = Hma;

    fn create(cfg: HmaConfig) -> Hma {
        Hma::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &HmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Hma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Hma, "HMA", Color::hex(0x00BCD4))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hma_small_period() {
        let mut hma = Hma::new(5);

        // Feed some values
        for i in 1..=10 {
            let value = (i * 10) as f64;
            hma.feed(value);
        }

        assert!(hma.is_ready());
        assert!(hma.value() > 0.0);
    }

    #[test]
    fn test_hma_period_1() {
        let mut hma = Hma::new(1);

        assert_eq!(hma.feed(10.0), 10.0);
        assert_eq!(hma.feed(20.0), 20.0);
    }

    #[test]
    fn test_hma_reset() {
        let mut hma = Hma::new(5);
        for i in 1..=10 {
            hma.feed(i as f64 * 10.0);
        }
        assert!(hma.is_ready());

        hma.reset();
        assert!(!hma.is_ready());
        assert_eq!(hma.count(), 0);
    }
}
