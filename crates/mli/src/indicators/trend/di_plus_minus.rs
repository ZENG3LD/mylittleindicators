//! Directional Indicator (+DI/-DI)
//!
//! Wrapper around ADX that exposes +DI and -DI as a Double value.
//! +DI measures upward price movement strength
//! -DI measures downward price movement strength

use crate::engine::contract_engine::SmootherId;
use crate::indicators::trend::adx::Adx;

/// Directional Indicator (+DI/-DI)
///
/// Returns Double(plus_di, minus_di) for trend direction analysis.
/// Values range from 0 to 100.
/// - When +DI > -DI: uptrend
/// - When -DI > +DI: downtrend
/// - Crossovers signal potential trend changes
#[derive(Debug, Clone)]
pub struct DiPlusMinus {
    adx: Adx,
}

impl DiPlusMinus {
    /// Create with default period (14, RMA smoother).
    pub fn new() -> Self {
        Self::with_period(14)
    }

    /// Create with custom period (RMA ATR default).
    pub fn with_period(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Create with custom period and ATR smoothing type.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            adx: Adx::from_smoother(period, smoother),
        }
    }

    /// Feed resolved input lanes `[high, low, close]` — the pure core computation.
    pub fn feed(&mut self, lanes: &[f64]) {
        self.adx.feed(lanes);
    }

    /// Get current +DI value.
    pub fn plus_di(&self) -> f64 {
        self.adx.plus_di()
    }

    /// Get current -DI value.
    pub fn minus_di(&self) -> f64 {
        self.adx.minus_di()
    }

    /// Brace-named getter for the `plus` output (delegates to `plus_di()`).
    #[inline]
    pub fn plus(&self) -> f64 {
        self.adx.plus_di()
    }

    /// Brace-named getter for the `minus` output (delegates to `minus_di()`).
    #[inline]
    pub fn minus(&self) -> f64 {
        self.adx.minus_di()
    }

    /// Get ADX value (trend strength).
    pub fn adx_value(&self) -> f64 {
        self.adx.value()
    }


    /// Check if indicator is ready.
    pub fn is_ready(&self) -> bool {
        self.adx.is_ready()
    }

    /// Reset indicator state.
    pub fn reset(&mut self) {
        self.adx.reset();
    }

    /// Get period.
    pub fn period(&self) -> usize {
        self.adx.period()
    }
}

impl Default for DiPlusMinus {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

impl Indicator for DiPlusMinus {
    const ID: IndicatorId = IndicatorId::DiPlusMinus;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Adx, &[IndicatorOutputId::Adx])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::DiPlusMinusPlus),
        Output::percent(IndicatorOutputId::DiPlusMinusMinus),
    ];
    type Config = DiPlusMinusConfig;
    type Runtime = DiPlusMinus;

    fn create(cfg: DiPlusMinusConfig) -> DiPlusMinus {
        DiPlusMinus::from_smoother(cfg.period.resolved(), SmootherId::Rma)
    }
}

/// Typed config for [`DiPlusMinus`] — its own period axis (not shared with Adx).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DiPlusMinusConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for DiPlusMinusConfig {
    fn defaults() -> Self {
        DiPlusMinusConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for DiPlusMinus {
    fn rendering() -> RenderSpec {
        use crate::contract::RenderOutput;
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::DiPlusMinusPlus, "+DI", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::DiPlusMinusMinus, "-DI", Color::hex(0xF44336), 1.0))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_di_plus_minus_creation() {
        let di = DiPlusMinus::new();
        assert_eq!(di.period(), 14);
        assert!(!di.is_ready());
    }

    #[test]
    fn test_di_plus_minus_warmup() {
        let mut di = DiPlusMinus::from_smoother(7, SmootherId::Ema);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 4.0;
            di.feed(&[price + 1.5, price - 1.5, price]);
        }
        assert!(di.is_ready());
        assert!(di.plus_di().is_finite());
        assert!(di.minus_di().is_finite());
    }

    #[test]
    fn test_di_plus_minus_value() {
        let mut di = DiPlusMinus::with_period(5);
        for i in 0..20 {
            let price = 100.0 + (i as f64) * 0.5;
            di.feed(&[price + 1.0, price - 1.0, price + 0.5]);
        }
        assert!(di.plus_di() >= 0.0 && di.plus_di() <= 100.0);
        assert!(di.minus_di() >= 0.0 && di.minus_di() <= 100.0);
    }

    #[test]
    fn test_di_plus_minus_reset() {
        let mut di = DiPlusMinus::new();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            di.feed(&[price + 2.0, price - 2.0, price]);
        }
        di.reset();
        assert!(!di.is_ready());
    }

    #[test]
    fn test_factory_feeds_resolved_di_plus_minus() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::DiPlusMinus(<<DiPlusMinus as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0,
            });
        }
        // factory value() = primary output (plus_di)
        assert!(f.primary().is_finite());
    }
}
