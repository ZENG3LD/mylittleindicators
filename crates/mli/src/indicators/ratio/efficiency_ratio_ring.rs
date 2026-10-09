// Rolling window (ring buffer) version of Efficiency Ratio
// Fast, memory efficient, classic ER


#[derive(Debug, Clone)]
pub struct EfficiencyRatioRingWindow {
    pub period: usize,
    buf: Vec<f64>,
    deltas: Vec<f64>,
    head: usize,
    len: usize,
    value: f64,
    initialized: bool,
}

impl EfficiencyRatioRingWindow {
    pub fn new(period: usize) -> Self {
        assert!(period > 1);
        Self {
            period,
            buf: vec![0.0; period],
            deltas: vec![0.0; period - 1],
            head: 0,
            len: 0,
            value: 0.0,
            initialized: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, v: f64) -> f64 {
        self.update_raw(v)
    }

    pub fn update_raw(&mut self, value: f64) -> f64 {
        let idx = (self.head + self.len) % self.period;
        if self.len < self.period {
            self.buf[idx] = value;
            self.len += 1;
            if self.len == 1 {
                self.value = 0.0;
                self.initialized = false;
                return self.value;
            }
        } else {
            self.buf[self.head] = value;
            self.head = (self.head + 1) % self.period;
        }
        // Compute deltas for the current window
        for i in 0..(self.len.min(self.period) - 1) {
            let idx0 = (self.head + i) % self.period;
            let idx1 = (self.head + i + 1) % self.period;
            self.deltas[i] = (self.buf[idx1] - self.buf[idx0]).abs();
        }
        self.initialized = self.len >= self.period;
        let net_diff = (self.buf[(self.head + self.len - 1) % self.period] - self.buf[self.head]).abs();
        let sum_deltas: f64 = self.deltas[..self.len.min(self.period) - 1].iter().sum();
        self.value = if sum_deltas == 0.0 { 0.0 } else { net_diff / sum_deltas };
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    pub fn get_buf(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.len);
        for i in 0..self.len {
            let idx = (self.head + i) % self.period;
            out.push(self.buf[idx]);
        }
        out
    }
    pub fn get_deltas(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.len.saturating_sub(1));
        for i in 0..self.len.saturating_sub(1) {
            out.push(self.deltas[i]);
        }
        out
    }
    pub fn reset(&mut self) {
        for v in &mut self.buf { *v = 0.0; }
        for d in &mut self.deltas { *d = 0.0; }
        self.head = 0;
        self.len = 0;
        self.value = 0.0;
        self.initialized = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

impl Default for EfficiencyRatioRingWindow {
    fn default() -> Self {
        Self::new(10)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

impl Indicator for EfficiencyRatioRingWindow {
    const ID: IndicatorId = IndicatorId::ErRing;
    /// No family — ring-buffer ER is a specific named output, not a pluggable oscillator.
    /// FLAG: could be Trend if needed; kept &[] to match Er variant for now.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field — default close.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period): each update recomputes all deltas in the ring via a loop over period-1 entries.
    /// Two fixed-period `Vec` buffers (allocated upfront to `period` / `period-1`).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::ErRing)];
    type Config = ErRingConfig;
    type Runtime = EfficiencyRatioRingWindow;

    fn create(cfg: ErRingConfig) -> EfficiencyRatioRingWindow {
        EfficiencyRatioRingWindow::new(cfg.period.resolved())
    }
}

/// Own config for [`EfficiencyRatioRingWindow`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ErRingConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for ErRingConfig {
    fn defaults() -> Self {
        ErRingConfig { period: Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (ring-window ER lookback) → auto now widens to range(1,10000,1),
        // but `EfficiencyRatioRingWindow::new` hard-asserts `period > 1` (its ring-buffer
        // deltas store is sized `period - 1`, undefined at period=1). No `valid_params` gate
        // exists to reject this at the config layer, so the floor must be raised here.
        // A1 2026-07-04: floor from valid_params (runtime assert `period > 1` in `Self::new`).
        let mut s = Self::machine_defaults_auto();
        s.period = Param::range(2, 10000, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for EfficiencyRatioRingWindow {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ErRing, "ER Ring", Color::hex(0x9C27B0))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_ratio_ring_creation() {
        let ind = EfficiencyRatioRingWindow::new(10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_efficiency_ratio_ring_warmup() {
        let mut ind = EfficiencyRatioRingWindow::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_raw(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_efficiency_ratio_ring_values() {
        let mut ind = EfficiencyRatioRingWindow::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.update_raw(price);
        }
        assert!(ind.value().is_finite());
        assert!(ind.value() >= 0.0);
    }

    #[test]
    fn test_efficiency_ratio_ring_reset() {
        let mut ind = EfficiencyRatioRingWindow::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.feed(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    /// The factory resolves the close field and feeds the scalar; pure uptrend => ER → 1.0.
    #[test]
    fn factory_feeds_resolved_scalar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::ErRing(<<EfficiencyRatioRingWindow as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..15 {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "ER ring should be in [0,1], got {v}");
    }
}






















