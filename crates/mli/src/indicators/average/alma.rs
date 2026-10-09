//! Arnaud Legoux Moving Average (ALMA) indicator.

use std::collections::VecDeque;
use crate::engine::ohlcv_field::OhlcvField;

/// Arnaud Legoux Moving Average (ALMA) - Gaussian-weighted moving average.
///
/// ALMA = Σ(Price × Weight) / Σ(Weight)
///
/// where weights follow a Gaussian distribution centered at `offset × (period-1)`.
///
/// Designed to reduce lag while maintaining smoothness. The offset parameter
/// controls where the center of the Gaussian is placed (0=oldest, 1=newest).
/// Sigma controls the width of the Gaussian curve.
///
/// # Parameters
/// - `period`: Number of bars
/// - `offset`: Center position, 0-1 (default: 0.85)
/// - `sigma`: Gaussian width (default: 6.0)
///
/// # Implementation
///
/// Precomputes Gaussian weights at construction. O(period) per update.
#[derive(Debug, Clone)]
pub struct Alma {
    period: usize,
    #[allow(dead_code)]
    offset: f64,
    #[allow(dead_code)]
    sigma: f64,
    buffer: VecDeque<f64>,
    weights: Vec<f64>,
    weight_sum: f64,
    value: f64,
}

impl Alma {
    /// Creates a new ALMA with default parameters (offset=0.85, sigma=6.0).
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include
    pub fn new(period: usize) -> Self {
        Self::with_params(period, 0.85, 6.0)
    }

    /// Creates a new ALMA with custom offset and sigma.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include
    /// * `offset` - Gaussian center position (0.0-1.0, higher = more recent)
    /// * `sigma` - Gaussian width (higher = smoother)
    pub fn with_params(period: usize, offset: f64, sigma: f64) -> Self {
        let period = period.max(1);
        let (weights, weight_sum) = Self::build_weights(period, offset, sigma);
        Self {
            period,
            offset,
            sigma,
            buffer: VecDeque::with_capacity(period),
            weights,
            weight_sum,
            value: 0.0,
        }
    }

    #[inline]
    fn build_weights(period: usize, offset: f64, sigma: f64) -> (Vec<f64>, f64) {
        let m = offset * (period as f64 - 1.0);
        let s = (period as f64 / sigma).max(1e-9);
        let mut w = Vec::with_capacity(period);
        let mut sum = 0.0;
        for i in 0..period {
            let x = (i as f64 - m) / s;
            let wi = (-0.5 * x * x).exp();
            w.push(wi);
            sum += wi;
        }
        (w, sum)
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The caller
    /// (standalone driver or host composite) supplies the value; the core knows
    /// nothing about OHLCV or fields.
    pub fn feed(&mut self, value: f64) -> f64 {
        self.buffer.push_back(value);
        if self.buffer.len() > self.period {
            self.buffer.pop_front();
        }
        if self.buffer.len() == self.period {
            let mut acc = 0.0;
            for (i, &p) in self.buffer.iter().enumerate() {
                acc += p * self.weights[i];
            }
            self.value = acc / self.weight_sum;
        }
        self.value
    }

    /// Updates the ALMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close), then
    /// delegates to [`Self::feed`].

    /// Returns the current ALMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns the current ALMA value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the ALMA has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() == self.period
    }

    /// Resets the ALMA to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.value = 0.0;
    }

    /// Returns the period of this ALMA.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Smoother, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Slot-specific params for ALMA as a smoother: period + the two Gaussian shape knobs.
/// Excludes `source` because a smoother slot is fed a pre-extracted scalar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlmaSmootherParams {
    pub period: usize,
    pub offset: f64,
    pub sigma: f64,
}

impl ::core::hash::Hash for AlmaSmootherParams {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.period.hash(state);
        self.offset.to_bits().hash(state);
        self.sigma.to_bits().hash(state);
    }
}

impl Smoother for Alma {
    type Params = AlmaSmootherParams;
    fn from_params(p: AlmaSmootherParams) -> Self {
        Alma::with_params(p.period, p.offset, p.sigma)
    }
    fn params_period(p: &AlmaSmootherParams) -> usize {
        p.period
    }
}

/// The slot sweep for ALMA: its FULL source-less params — period × Gaussian `offset` × `sigma`.
/// This is the member with shape knobs; in a slot it contributes 3 axes (where the 9 period-only
/// smoothers contribute 1), the disjoint-union branch the machine generator was missing. Ranges
/// are curated/coarse (slot sub-component); the standalone `AlmaConfig::machine_defaults` is wider.
impl crate::contract::SlotParamsSweep for AlmaSmootherParams {
    fn machine_params() -> Vec<Self> {
        let mut out = Vec::new();
        for &period in &[5usize, 9, 14, 20, 30, 50, 100, 200] {
            for &offset in &[0.5f64, 0.85, 1.0] {
                for &sigma in &[3.0f64, 6.0, 9.0] {
                    out.push(AlmaSmootherParams { period, offset, sigma });
                }
            }
        }
        out
    }
}

/// Typed config for [`Alma`]: period + source + the Gaussian `offset` (center,
/// 0..1) and `sigma` (width).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AlmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    pub offset: Param<f64>,
    pub sigma: Param<f64>,
}

impl crate::contract::Config for AlmaConfig {
    fn defaults() -> Self {
        AlmaConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            offset: Param::Solo(0.85),
            sigma: Param::Solo(6.0),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        let mut s = Self::machine_defaults_auto();
        // offset: Class K (ALMA Gaussian center position, 0..1) — sweep_f64(0.0,1.0,0.1).
        s.offset = Param::many(sweep_f64(0.0, 1.0, 0.1));
        // sigma: Class K (ALMA Gaussian width) — sweep_f64(1.0,20.0,1.0).
        s.sigma = Param::many(sweep_f64(1.0, 20.0, 1.0));
        s
    }
}

impl Indicator for Alma {
    const ID: IndicatorId = IndicatorId::Alma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(period) weighted-window rescan each bar; holds a period-deep ring
    /// (`VecDeque`) plus a period-deep precomputed `weights` `Vec`.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Alma)];
    type Config = AlmaConfig;
    type Runtime = Alma;

    fn create(cfg: AlmaConfig) -> Alma {
        Alma::with_params(cfg.period.resolved(), cfg.offset.resolved(), cfg.sigma.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &AlmaConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Alma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Alma, "ALMA", Color::hex(0xE91E63))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alma_basic_calculation() {
        let mut alma = Alma::new(5);

        for i in 1..=5 {
            alma.feed(i as f64 * 10.0);
        }

        assert!(alma.is_ready());
        assert!(alma.value() > 0.0);
    }

    #[test]
    fn test_alma_custom_params() {
        let mut alma = Alma::with_params(5, 0.5, 3.0);

        for i in 1..=5 {
            alma.feed(i as f64 * 10.0);
        }

        assert!(alma.is_ready());
    }

    #[test]
    fn test_alma_reset() {
        let mut alma = Alma::new(3);
        alma.feed(10.0);
        alma.feed(20.0);
        alma.feed(30.0);
        assert!(alma.is_ready());

        alma.reset();
        assert!(!alma.is_ready());
    }
}
