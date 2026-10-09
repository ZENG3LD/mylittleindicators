//! McGinley Dynamic indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// McGinley Dynamic - self-adjusting moving average.
///
/// MD = MD_prev + (Price - MD_prev) / (k × (Price/MD_prev)^4)
///
/// where k ≈ period.
///
/// Created by John McGinley. Automatically adjusts its speed based on
/// market conditions. When price moves quickly away from the average,
/// it speeds up to catch up. When price is near the average, it slows down.
///
/// # Implementation
///
/// O(1) update complexity using direct formula calculation.
#[derive(Debug, Clone)]
pub struct McGinleyDynamic {
    period: usize,
    value: f64,
    initialized: bool,
}

impl McGinleyDynamic {
    /// Creates a new McGinley Dynamic with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Base smoothing factor (similar to EMA period)
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            value: 0.0,
            initialized: false,
        }
    }

    /// Updates the McGinley Dynamic with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn feed(&mut self, value: f64) -> f64 {
        let value = value;
        if !self.initialized {
            self.value = value;
            self.initialized = true;
            return self.value;
        }
        let md_prev = self.value;
        if md_prev == 0.0 {
            self.value = value;
            return self.value;
        }
        let ratio = (value / md_prev).abs();
        let denom = (self.period as f64) * ratio.powi(4);
        self.value = md_prev + (value - md_prev) / denom.max(1e-9);
        self.value
    }

    /// Returns the current McGinley Dynamic value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the indicator has received at least one bar.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
    }

    /// Returns the period of this McGinley Dynamic.
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Family, Indicator, Output, Param};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`McGinleyDynamic`]: period + configurable source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct McginleyConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for McginleyConfig {
    fn defaults() -> Self {
        McginleyConfig {
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

impl Indicator for McGinleyDynamic {
    const ID: IndicatorId = IndicatorId::Mcginley;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    // COST = default: O(1) running scalar, no window store (the cheapest leaf, like EMA).
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Mcginley)];
    type Config = McginleyConfig;
    type Runtime = McGinleyDynamic;

    fn create(cfg: McginleyConfig) -> McGinleyDynamic {
        McGinleyDynamic::new(cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &Self::Config) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for McGinleyDynamic {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Mcginley, "McGinley", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcginley_basic_calculation() {
        let mut md = McGinleyDynamic::new(10);

        for i in 1..=20 {
            md.feed(100.0 + i as f64);
        }

        assert!(md.is_ready());
        assert!(md.value() > 100.0);
    }

    #[test]
    fn test_mcginley_first_value() {
        let mut md = McGinleyDynamic::new(10);
        let v = md.feed(100.0);

        assert!(md.is_ready());
        assert_eq!(v, 100.0);
    }

    #[test]
    fn test_mcginley_reset() {
        let mut md = McGinleyDynamic::new(10);
        md.feed(100.0);
        md.feed(110.0);
        assert!(md.is_ready());

        md.reset();
        assert!(!md.is_ready());
    }

}
