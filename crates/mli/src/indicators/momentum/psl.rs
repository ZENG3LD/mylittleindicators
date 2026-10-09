// High-performance Psychological Line (PSL)
// (c) 2024

use crate::engine::ohlcv_field::OhlcvField;

/// Psychological Line — percent of up-bars over the window. PURE core — owns no source;
/// the factory feeds it the resolved scalar via [`Psl::feed`] (the chosen `OhlcvField`
/// lives on the `ContractFactory` variant, lifted from the config via `source`).
#[derive(Clone, Debug)]
pub struct Psl {
    period: usize,
    buffer: Vec<u8>,
    /// Running count of up-flags currently in `buffer` — kept O(1) per bar.
    sum_up: usize,
    index: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
}

impl Psl {
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            buffer: vec![0u8; p],
            sum_up: 0,
            index: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Counts the bar as "up"
    /// when the value rises vs the previous one, over a period-deep ring (O(1)).
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.index == 0 && !self.filled && self.prev_close == 0.0 {
            self.prev_close = value;
            self.index = 1;
            return self.value;
        }
        let up: u8 = if value > self.prev_close { 1 } else { 0 };
        // Ring write + O(1) running-count maintenance (evict the overwritten flag).
        let slot = self.index % self.period;
        let old = self.buffer[slot];
        self.buffer[slot] = up;
        self.sum_up = self.sum_up + up as usize - old as usize;
        self.prev_close = value;
        self.index += 1;
        if self.index >= self.period {
            self.filled = true;
        }
        let len = if self.filled { self.period } else { self.index };
        if len < self.period {
            self.value = 0.0;
            return self.value;
        }
        self.value = 100.0 * (self.sum_up as f64) / (self.period as f64);
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Returns the current PSL value as `f64` (the scalar-slot interface).
    pub fn value_f64(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.buffer.fill(0);
        self.sum_up = 0;
        self.index = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.value = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

impl crate::contract::Oscillator for Psl {
    type Params = crate::indicators::average::moving_average::PeriodConfig;
    fn from_params(p: Self::Params) -> Self {
        Psl::new(p.period)
    }
    fn params_period(p: &Self::Params) -> usize {
        p.period
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Psl`]. Dual-mode: every field is a `Param` — `Solo` = one value,
/// `Many` = a swept set. `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PslConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for Psl {
    const ID: IndicatorId = IndicatorId::Psl;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    // SOURCE defaults to the configurable `Field { default: Close }` — it reads one
    // price field (default close) and counts up-bars; the order can sweep it.
    /// O(1): a running up-count over a period-deep `Vec<u8>` ring (evict-and-add on each
    /// bar, no re-sum) — its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec)],
    );

    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Psl)];
    type Config = PslConfig;
    type Runtime = Psl;

    fn create(cfg: PslConfig) -> Psl {
        Psl::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &PslConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for PslConfig {
    /// Standard psychological-line window is 12 (the catalog default); resolves the
    /// factory's generic-14 vs catalog-12 drift onto the documented intent.
    fn defaults() -> Self {
        PslConfig {
            period: Param::Solo(12),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        Self::machine_defaults_auto()
    }
}


impl Render for Psl {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Psl, "PSL", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psl_creation() {
        let psl = Psl::new(12);
        assert!(!psl.is_ready());
        assert_eq!(psl.value(), 0.0);
        assert_eq!(psl.period(), 12);
    }

    #[test]
    fn test_psl_uptrend() {
        let mut psl = Psl::new(10);
        for i in 1..=20 {
            psl.feed(100.0 + i as f64 * 2.0); // always up
        }
        assert!(psl.is_ready());
        // All bars up = PSL should be 100
        assert!(psl.value() > 80.0, "PSL should be high in uptrend, got {}", psl.value());
    }

    #[test]
    fn test_psl_downtrend() {
        let mut psl = Psl::new(10);
        for i in 1..=20 {
            psl.feed(200.0 - i as f64 * 2.0); // always down
        }
        assert!(psl.is_ready());
        // All bars down = PSL should be 0
        assert!(psl.value() < 20.0, "PSL should be low in downtrend, got {}", psl.value());
    }

    #[test]
    fn test_psl_range() {
        let mut psl = Psl::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = psl.feed(price);
            if psl.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "PSL should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_psl_all_up_is_exactly_100() {
        // Locks the O(1) running up-count: a strictly rising series fills the window
        // with up-flags, so PSL must be EXACTLY 100 (not just "high").
        let mut psl = Psl::new(5);
        for i in 0..20 {
            psl.feed(100.0 + i as f64);
        }
        assert!((psl.value() - 100.0).abs() < 1e-9, "all-up PSL must be exactly 100, got {}", psl.value());
    }

    #[test]
    fn test_psl_reset() {
        let mut psl = Psl::new(10);
        for i in 1..=20 {
            psl.feed(100.0 + i as f64);
        }
        assert!(psl.is_ready());
        psl.reset();
        assert!(!psl.is_ready());
        assert_eq!(psl.value(), 0.0);
    }
}
