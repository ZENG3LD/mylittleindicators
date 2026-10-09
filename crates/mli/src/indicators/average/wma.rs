//! Weighted Moving Average (WMA) indicator.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Smoother, Store, StoreKind, UpdateComplexity};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Weighted Moving Average (WMA) - linear weights giving more importance to recent prices.
///
/// WMA = (n×Pn + (n-1)×Pn-1 + ... + 1×P1) / (n + (n-1) + ... + 1)
///
/// # Implementation
///
/// Fast O(1) WMA implementation using running sums.
///
/// Instead of recalculating weighted sum on each update (O(n)),
/// we maintain two running sums:
/// - weighted_sum: sum of (value * weight)
/// - unweighted_sum: sum of values (no weights)
///
/// When sliding the window:
/// 1. Remove old value's contribution
/// 2. All remaining weights decrease by 1 (clever math optimization)
/// 3. Add new value with weight = period
///
/// Time complexity: O(1) per update vs O(period) in naive implementation

#[derive(Debug, Clone)]
pub struct Wma {
    period: usize,
    buf: Vec<f64>,           // ring buffer
    idx: usize,              // current write position (oldest element)
    filled: bool,
    weight_sum: f64,         // pre-calculated sum of weights: 1+2+...+period
    weighted_sum: f64,       // running weighted sum: w₁·x₁ + w₂·x₂ + ... + wₙ·xₙ
    unweighted_sum: f64,     // running unweighted sum: x₁ + x₂ + ... + xₙ
    value: f64,
}

impl Wma {
    /// Creates a new WMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Number of bars to weight (must be >= 1)
    pub fn new(period: usize) -> Self {
        let period = period.max(1);

        // Weight sum for WMA: 1 + 2 + 3 + ... + n = n(n+1)/2
        let weight_sum = (period * (period + 1)) as f64 / 2.0;

        Self {
            period,
            buf: Vec::with_capacity(period),
            idx: 0,
            filled: false,
            weight_sum,
            weighted_sum: 0.0,
            unweighted_sum: 0.0,
            value: 0.0,
        }
    }

    /// Returns the period of this WMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns the current WMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current WMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the WMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the WMA to its initial state.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.weighted_sum = 0.0;
        self.unweighted_sum = 0.0;
        self.value = 0.0;
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The caller
    /// (standalone driver or host composite) supplies the value; the core knows
    /// nothing about OHLCV or fields.
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.buf.len() < self.period {
            // Warmup phase: fill buffer and compute initial sums
            self.buf.push(value);
            self.unweighted_sum += value;

            if self.buf.len() == self.period {
                self.filled = true;
                // Calculate initial weighted sum: w₁·x₁ + w₂·x₂ + ... + wₙ·xₙ
                self.weighted_sum = 0.0;
                for (i, &val) in self.buf.iter().enumerate() {
                    let weight = (i + 1) as f64;  // weights: 1, 2, 3, ..., period
                    self.weighted_sum += val * weight;
                }
                self.value = self.weighted_sum / self.weight_sum;
            } else {
                self.value = value;  // Not ready yet
            }
        } else {
            // O(1) sliding window update
            let old_value = self.buf[self.idx];

            // Key optimization: instead of recalculating entire weighted sum,
            // we use the fact that when we remove oldest and add newest:
            // 1. All elements shift their weights down by 1
            // 2. This is equivalent to: new_sum = old_sum - unweighted_sum + period * new_value
            //
            // Proof:
            // Old: w₁·x₁ + w₂·x₂ + ... + wₙ·xₙ
            // New: w₁·x₂ + w₂·x₃ + ... + wₙ·new
            //    = (w₂-1)·x₂ + (w₃-1)·x₃ + ... + (wₙ-1)·xₙ + wₙ·new
            //    = (w₂·x₂ + w₃·x₃ + ... + wₙ·xₙ) - (x₂ + x₃ + ... + xₙ) + wₙ·new
            //    = (old_sum - w₁·x₁) - (unweighted_sum - x₁) + wₙ·new
            //    = old_sum - unweighted_sum + wₙ·new

            self.weighted_sum = self.weighted_sum - self.unweighted_sum + (self.period as f64) * value;
            self.unweighted_sum = self.unweighted_sum - old_value + value;

            // Update ring buffer
            self.buf[self.idx] = value;
            self.idx = (self.idx + 1) % self.period;

            self.value = self.weighted_sum / self.weight_sum;
        }

        self.value
    }
}

impl Smoother for Wma {
    type Params = PeriodConfig;
    fn from_params(p: PeriodConfig) -> Self { Wma::new(p.period) }
    fn params_period(p: &PeriodConfig) -> usize { p.period }
}

/// Own config for [`Wma`] — period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct WmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for WmaConfig {
    fn defaults() -> Self {
        WmaConfig {
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

impl Indicator for Wma {
    const ID: IndicatorId = IndicatorId::Wma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) update (running sums) but holds a period-deep ring `Vec`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Wma)];
    type Config = WmaConfig;
    type Runtime = Wma;

    fn create(cfg: WmaConfig) -> Wma {
        Wma::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &WmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Wma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Wma, "WMA", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wma_correctness() {
        let mut wma = Wma::new(5);

        // Feed values: [10, 20, 30, 40, 50]
        wma.feed(10.0);
        wma.feed(20.0);
        wma.feed(30.0);
        wma.feed(40.0);
        let result1 = wma.feed(50.0);

        // Expected: (1*10 + 2*20 + 3*30 + 4*40 + 5*50) / (1+2+3+4+5)
        //         = (10 + 40 + 90 + 160 + 250) / 15 = 550 / 15 = 36.666...
        assert!((result1 - 36.666666).abs() < 0.001, "Expected ~36.67, got {}", result1);

        // Add new value 60, oldest (10) should be removed
        let result2 = wma.feed(60.0);

        // Expected: (1*20 + 2*30 + 3*40 + 4*50 + 5*60) / 15
        //         = (20 + 60 + 120 + 200 + 300) / 15 = 700 / 15 = 46.666...
        assert!((result2 - 46.666666).abs() < 0.001, "Expected ~46.67, got {}", result2);
    }

    #[test]
    fn test_wma_period_1() {
        let mut wma = Wma::new(1);

        assert_eq!(wma.feed(10.0), 10.0);
        assert_eq!(wma.feed(20.0), 20.0);
        assert_eq!(wma.feed(30.0), 30.0);
    }

    #[test]
    fn test_wma_reset() {
        let mut wma = Wma::new(3);
        wma.feed(10.0);
        wma.feed(20.0);
        wma.feed(30.0);
        assert!(wma.is_ready());

        wma.reset();
        assert!(!wma.is_ready());
    }

}
