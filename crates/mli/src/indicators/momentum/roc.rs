//! Rate of Change (ROC) indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// Rate of Change (ROC) - measures percentage change over a specified period.
///
/// ROC = (Value - Value_n) / Value_n  (as decimal, multiply by 100 for percentage)
///
/// or with logarithmic mode:
///
/// ROC = log10(Value / Value_n)
///
/// ROC is an unbounded momentum oscillator:
/// - Positive values: Price is higher than n periods ago
/// - Negative values: Price is lower than n periods ago
/// - Zero crossings: Potential trend changes
///
/// # Parameters
/// - `period`: Lookback period
/// - `use_log`: Use logarithmic calculation
///
/// # Implementation
///
/// PURE core: a period-sized ring buffer, O(1) update. It owns NO source — the
/// factory feeds it the resolved scalar via [`Roc::feed`] (the chosen `OhlcvField`
/// lives on the `ContractFactory` variant, lifted from the config via `source`).
#[derive(Debug, Clone)]
pub struct Roc {
    period: usize,
    buffer: Vec<f64>,
    index: usize,
    filled: bool,
    value: f64,
    use_log: bool,
}

impl Roc {
    /// Creates a new ROC with the specified parameters.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    /// * `use_log` - Use logarithmic calculation instead of percentage
    pub fn new(period: usize, use_log: bool) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            buffer: vec![0.0; p],
            index: 0,
            filled: false,
            value: 0.0,
            use_log,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The caller (the
    /// factory's source-resolving feed, or a host composite) supplies the value; the
    /// core knows nothing about OHLCV or fields.
    pub fn feed(&mut self, value: f64) -> f64 {
        self.buffer[self.index] = value;
        let prev_idx = (self.index + 1) % self.period;
        let ready = self.filled || self.index + 1 >= self.period;
        let prev = if ready { self.buffer[prev_idx] } else { value };

        let roc = if ready {
            if self.use_log {
                (value / prev).log10()
            } else {
                (value - prev) / prev
            }
        } else {
            0.0
        };

        self.value = roc;
        self.index = (self.index + 1) % self.period;
        if self.index == 0 {
            self.filled = true;
        }
        self.value
    }

    /// Returns the current ROC value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the ROC has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the ROC to its initial state.
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
        self.filled = false;
        self.value = 0.0;
    }

    /// Returns the period of this ROC.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Roc`]. Dual-mode: every field is a `Param` — `Solo` = one value,
/// `Many` = a swept set. `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RocConfig {
    pub period: Param<usize>,
    pub use_log: Param<bool>,
    pub source: Param<OhlcvField>,
}

impl Indicator for Roc {
    const ID: IndicatorId = IndicatorId::Roc;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) ring-buffer lookup (current vs period-ago price). The ring is a period-deep
    /// `Vec` — its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec)],
    );

    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Roc)];
    type Config = RocConfig;
    type Runtime = Roc;

    fn create(cfg: RocConfig) -> Roc {
        Roc::new(cfg.period.resolved(), cfg.use_log.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved
    /// scalar — so this `Roc` is pure (no `source` field, no `update_bar`).
    fn source_fields(cfg: &RocConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for RocConfig {
    fn defaults() -> Self {
        RocConfig {
            period: Param::Solo(12),
            use_log: Param::Solo(false),
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
        // period: Class A → auto range(2,4048,1); use_log: Class R → auto both.
        // source: Class O → auto all-8.
        Self::machine_defaults_auto()
    }
}


impl Render for Roc {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Roc, "ROC", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests — pure `feed(scalar)` core
    // =========================================================================

    #[test]
    fn test_roc_basic_calculation() {
        let mut roc = Roc::new(10, false);

        // Feed constant growth data
        for i in 1..=20 {
            roc.feed(100.0 + i as f64);
        }

        assert!(roc.is_ready());
        // With constant growth, ROC should be positive
        assert!(roc.value() > 0.0, "ROC with growth should be positive");
    }

    #[test]
    fn test_roc_decline() {
        let mut roc = Roc::new(10, false);

        // Feed declining data
        for i in 1..=20 {
            roc.feed(200.0 - i as f64);
        }

        assert!(roc.is_ready());
        // With declining prices, ROC should be negative
        assert!(roc.value() < 0.0, "ROC with decline should be negative");
    }

    #[test]
    fn test_roc_percentage_calculation() {
        let mut roc = Roc::new(5, false);

        // Fill buffer with 100, then jump to 110
        // ROC compares current with value 5 bars ago
        roc.feed(100.0);
        roc.feed(100.0);
        roc.feed(100.0);
        roc.feed(100.0);
        roc.feed(100.0);
        let val = roc.feed(110.0);

        // (110 - 100) / 100 = 0.10
        assert!((val - 0.10).abs() < 0.01, "Expected ~0.10, got {}", val);
    }

    #[test]
    fn test_roc_log_mode() {
        let mut roc = Roc::new(5, true);

        for i in 1..=10 {
            roc.feed(100.0 + i as f64 * 2.0);
        }

        assert!(roc.is_ready());
        // Log ROC should be positive for growth
        assert!(roc.value() > 0.0);
    }

    #[test]
    fn test_roc_reset() {
        let mut roc = Roc::new(10, false);

        for i in 1..=20 {
            roc.feed(100.0 + i as f64);
        }
        assert!(roc.is_ready());

        roc.reset();
        assert!(!roc.is_ready());
        assert!((roc.value()).abs() < 1e-10);
    }

    #[test]
    fn test_roc_period_getter() {
        let roc = Roc::new(14, false);
        assert_eq!(roc.period(), 14);
    }

    #[test]
    fn test_roc_constant_price() {
        let mut roc = Roc::new(10, false);

        // Constant price = 0% change
        for _ in 1..=20 {
            roc.feed(100.0);
        }

        assert!(roc.is_ready());
        assert!((roc.value()).abs() < 1e-10, "ROC with constant price should be 0");
    }

    /// The `ContractFactory` variant holds the resolved source (close) and the `Source`
    /// feed extracts it before calling the pure core — proving the factory feeds the
    /// resolved scalar, not a raw OHLCV bar. Build via the typed order path.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::contract::Param;
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = {
            let mut c = <<Roc as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
            c.period = Param::Solo(5);
            IndicatorOrder::Roc(c).build_solo().unwrap()
        };
        let bar = |close: f64| MarketSample::Bar {
            open: 0.0,
            high: 999.0,
            low: 0.0,
            close,
            volume: 0.0,
        };
        // Five bars at 100 fill the ring; a sixth at 110 -> ROC of CLOSE = (110-100)/100 = 0.10.
        // Highs (999) would never produce 0.10 -> proves the close source is resolved.
        for _ in 0..5 {
            f.feed(0, bar(100.0));
        }
        f.feed(0, bar(110.0));
        let v = f.primary();
        assert!((v - 0.10).abs() < 0.01, "factory must ROC the CLOSE (0.10), got {}", v);
    }
}
