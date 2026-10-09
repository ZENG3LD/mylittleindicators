//! Volume-Weighted Moving Average (VWMA) indicator.

use std::collections::VecDeque;

/// Volume-Weighted Moving Average (VWMA) - weights prices by their volume.
///
/// VWMA = Σ(Price × Volume) / Σ(Volume)
///
/// Gives more weight to prices traded with higher volume.
/// Useful for identifying true support/resistance levels.
///
/// # Implementation
///
/// Uses two ring buffers for O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Vwma {
    period: usize,
    pv_buf: VecDeque<f64>,
    v_buf: VecDeque<f64>,
    sum_pv: f64,
    sum_v: f64,
    value: f64,
}

impl Vwma {
    /// Creates a new VWMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include in the calculation
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            pv_buf: VecDeque::with_capacity(p),
            v_buf: VecDeque::with_capacity(p),
            sum_pv: 0.0,
            sum_v: 0.0,
            value: 0.0,
        }
    }

    /// Feed the resolved `[close, volume]` lanes (in `SOURCE` order). The factory
    /// extracts both fields from the bar; the core knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let c = lanes[0];
        let v = lanes[1];
        let pv = c * v;
        self.pv_buf.push_back(pv);
        self.v_buf.push_back(v);
        self.sum_pv += pv;
        self.sum_v += v;
        if self.pv_buf.len() > self.period {
            if let Some(x) = self.pv_buf.pop_front() {
                self.sum_pv -= x;
            }
            if let Some(x) = self.v_buf.pop_front() {
                self.sum_v -= x;
            }
        }
        if self.sum_v > 0.0 {
            self.value = self.sum_pv / self.sum_v;
        }
        self.value
    }

    /// Returns the current VWMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the VWMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.pv_buf.len() == self.period && self.sum_v > 0.0
    }

    /// Resets the VWMA to its initial state.
    pub fn reset(&mut self) {
        self.pv_buf.clear();
        self.v_buf.clear();
        self.sum_pv = 0.0;
        self.sum_v = 0.0;
        self.value = 0.0;
    }

    /// Returns the period of this VWMA.
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Vwma`]: period only — no configurable source (VWMA always
/// consumes the fixed `[close, volume]` lane pair).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VwmaConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for VwmaConfig {
    fn defaults() -> Self {
        VwmaConfig {
            period: Param::Solo(14),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1). No configurable source (intrinsic close/volume).
        Self::machine_defaults_auto()
    }
}

impl Indicator for Vwma {
    const ID: IndicatorId = IndicatorId::Vwma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed `[close, volume]` lanes — the runtime computes `close × volume`. NOT a
    /// configurable single source: the factory extracts both fields and feeds the slice.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::Close, OhlcvField::Volume]));
    /// Volume-weighted — excluded from price-smoother slots (degenerates on a
    /// volume-less scalar series).
    const NEEDS_VOLUME: bool = true;
    /// Two period-deep `VecDeque` rings (price×volume, volume).
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque), Store::window(StoreKind::Deque)],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Vwma)];
    type Config = VwmaConfig;
    type Runtime = Vwma;

    fn create(cfg: VwmaConfig) -> Vwma {
        Vwma::new(cfg.period.resolved())
    }
}


impl Render for Vwma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Vwma, "VWMA", Color::hex(0xFFC107))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwma_basic_calculation() {
        let mut vwma = Vwma::new(3);

        // Price 100, Volume 1000
        vwma.feed(&[100.0, 1000.0]);
        // Price 110, Volume 2000 (more volume = more weight)
        vwma.feed(&[110.0, 2000.0]);
        // Price 105, Volume 1000
        let v3 = vwma.feed(&[105.0, 1000.0]);

        assert!(vwma.is_ready());
        // VWMA = (100*1000 + 110*2000 + 105*1000) / (1000 + 2000 + 1000)
        //      = (100000 + 220000 + 105000) / 4000 = 425000 / 4000 = 106.25
        assert!((v3 - 106.25).abs() < 0.01);
    }

    #[test]
    fn test_vwma_zero_volume() {
        let mut vwma = Vwma::new(2);

        vwma.feed(&[100.0, 0.0]);
        vwma.feed(&[110.0, 0.0]);

        // With zero volume, is_ready should be false
        assert!(!vwma.is_ready());
    }

    #[test]
    fn test_vwma_reset() {
        let mut vwma = Vwma::new(3);
        vwma.feed(&[100.0, 1000.0]);
        vwma.feed(&[110.0, 1000.0]);
        vwma.feed(&[120.0, 1000.0]);
        assert!(vwma.is_ready());

        vwma.reset();
        assert!(!vwma.is_ready());
    }
}
