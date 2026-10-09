//! Linear Regression Moving Average (LR) indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// Linear Regression Moving Average (LR) - least-squares regression line endpoint.
///
/// Fits a least-squares regression line to the last N prices and returns
/// the endpoint value. Also provides slope, intercept, and R² statistics.
///
/// LR provides excellent smoothing with minimal lag since it projects
/// the trend forward. When `zero_lag` is enabled the fit is projected one bar
/// ahead (`value + slope`), which is equivalent to the Zero-Lag LSMA (ZLSMA).
///
/// # Implementation
///
/// Uses closed-form least squares calculation. O(period) per update.
/// Maximum period is 512 bars.
#[derive(Debug, Clone)]
pub struct LinearRegressionMA {
    period: usize,
    zero_lag: bool,
    slope: f64,
    intercept: f64,
    r2: f64,
    value: f64,
    buf: Vec<f64>,
    initialized: bool,
}

/// Type alias for Linear Regression MA.
pub type Lr = LinearRegressionMA;

impl LinearRegressionMA {
    /// Returns the period of this LR.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new Linear Regression MA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars for regression (1..=512)
    pub fn new(period: usize) -> Self {
        Self::with_zero_lag(period, false)
    }

    /// Creates a new Linear Regression MA with full configuration.
    ///
    /// When `zero_lag` is `true` the regression endpoint is projected one bar
    /// ahead (`value + slope`), producing the Zero-Lag LSMA output.
    ///
    /// # Arguments
    /// * `period`    - Number of bars for regression (1..=512)
    /// * `zero_lag`  - Whether to project one bar forward to reduce lag
    pub fn with_zero_lag(period: usize, zero_lag: bool) -> Self {
        Self {
            period,
            zero_lag,
            slope: 0.0,
            intercept: 0.0,
            r2: 0.0,
            value: 0.0,
            buf: Vec::with_capacity(period),
            initialized: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The factory
    /// (or host composite) resolves the source field and supplies the value.
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.buf.len() == self.period {
            self.buf.remove(0);
        }
        self.buf.push(value);
        if self.buf.len() < self.period {
            self.value = 0.0;
            self.initialized = false;
            return self.value;
        }
        self.initialized = true;
        let x_arr: Vec<f64> = (1..=self.period).map(|x| x as f64).collect();
        let y_arr: Vec<f64> = self.buf.iter().cloned().collect();
        let x_sum: f64 = 0.5 * self.period as f64 * (self.period as f64 + 1.0);
        let x_mul_sum: f64 = x_sum * 2.0f64.mul_add(self.period as f64, 1.0) / 3.0;
        let divisor: f64 = (self.period as f64).mul_add(x_mul_sum, -(x_sum * x_sum));
        let y_sum: f64 = y_arr.iter().sum::<f64>();
        let sum_x_y: f64 = x_arr
            .iter()
            .zip(y_arr.iter())
            .map(|(x, y)| x * y)
            .sum::<f64>();
        self.slope = (self.period as f64).mul_add(sum_x_y, -(x_sum * y_sum)) / divisor;
        self.intercept = y_sum.mul_add(x_mul_sum, -(x_sum * sum_x_y)) / divisor;
        let residuals: Vec<f64> = x_arr
            .iter()
            .zip(y_arr.iter())
            .map(|(x, y)| self.slope.mul_add(*x, self.intercept) - y)
            .collect();
        self.value = residuals.last().unwrap_or(&0.0) + y_arr.last().unwrap_or(&0.0);
        if self.zero_lag {
            self.value += self.slope;
        }
        let mean: f64 = y_arr.iter().sum::<f64>() / y_arr.len() as f64;
        self.r2 = 1.0
            - residuals.iter().map(|r| r * r).sum::<f64>()
                / y_arr.iter().map(|y| (y - mean) * (y - mean)).sum::<f64>();
        self.value
    }


    /// Named output: brace `line` (the regression line value).
    pub fn line(&self) -> f64 {
        self.value
    }

    /// Named output: brace `gradient` (the regression line's slope — synonym of `slope()`).
    pub fn gradient(&self) -> f64 {
        self.slope
    }

    /// Returns the slope of the regression line.
    pub fn slope(&self) -> f64 {
        self.slope
    }

    /// Returns the intercept of the regression line.
    pub fn intercept(&self) -> f64 {
        self.intercept
    }

    /// Returns the R² (coefficient of determination) of the regression.
    pub fn r2(&self) -> f64 {
        self.r2
    }

    /// Returns `true` if the LR has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the LR to its initial state.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.value = 0.0;
        self.slope = 0.0;
        self.intercept = 0.0;
        self.r2 = 0.0;
        self.initialized = false;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for the Linear Regression MA.
///
/// Has a `zero_lag` knob that simple MA configs do not carry.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LrConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    /// Project the regression endpoint one bar ahead (`value + slope`).
    /// Equivalent to the Zero-Lag LSMA when `true`.
    pub zero_lag: Param<bool>,
}

impl crate::contract::Config for LrConfig {
    fn defaults() -> Self {
        LrConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            zero_lag: Param::Solo(false),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(1,10000,1); source: Class O → auto all-8;
        // zero_lag: Class R (bool) → auto both [false, true].
        // A1 2026-07-04: floor from valid_params (runtime — the closed-form least-squares
        // `divisor = period*x_mul_sum - x_sum^2` is exactly 0 at period=1, a single point
        // has zero x-variance; no `valid_params` gate exists to reject it).
        let mut s = Self::machine_defaults_auto();
        s.period = Param::range(2, 10000, 1);
        s
    }
}

impl Indicator for LinearRegressionMA {
    const ID: IndicatorId = IndicatorId::Lr;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Heavy: holds a period-deep `Vec` window AND rescans it (least-squares) every
    /// bar — O(period). The store + Linear update give it the weight that prunes it
    /// from cost-bounded MA sweeps.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::LrLine),
        Output::centered(IndicatorOutputId::LrGradient),
        Output::price(IndicatorOutputId::LrIntercept),
        Output::percent(IndicatorOutputId::LrR2),
    ];
    type Config = LrConfig;
    type Runtime = LinearRegressionMA;

    fn create(cfg: LrConfig) -> LinearRegressionMA {
        LinearRegressionMA::with_zero_lag(cfg.period.resolved(), cfg.zero_lag.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the
    /// resolved scalar — the core ingests via `feed(f64)`, knowing no OHLCV fields.
    fn source_fields(cfg: &LrConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for LinearRegressionMA {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::LrLine, "Linear Regression", Color::hex(0x607D8B))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lr_basic_calculation() {
        let mut lr = LinearRegressionMA::new(5);

        for i in 1..=5 {
            lr.feed(i as f64 * 10.0);
        }

        assert!(lr.is_ready());
        assert!(lr.line() > 0.0);
        // Perfect linear data should have high R²
        assert!(lr.r2() > 0.99);
    }

    #[test]
    fn test_lr_slope_positive() {
        let mut lr = LinearRegressionMA::new(5);

        // Upward trend
        for i in 1..=5 {
            lr.feed(i as f64 * 10.0);
        }

        assert!(lr.slope() > 0.0);
    }

    #[test]
    fn test_lr_reset() {
        let mut lr = LinearRegressionMA::new(3);
        for i in 1..=5 {
            lr.feed(i as f64 * 10.0);
        }
        assert!(lr.is_ready());

        lr.reset();
        assert!(!lr.is_ready());
        assert_eq!(lr.slope(), 0.0);
    }

}
