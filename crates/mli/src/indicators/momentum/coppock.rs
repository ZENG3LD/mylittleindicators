//! Coppock Curve indicator.

use crate::engine::contract_engine::SmootherSlot;
use std::collections::VecDeque;

/// Coppock Curve - long-term momentum indicator for market bottoms.
///
/// Coppock = WMA(ROC(roc1_len) + ROC(roc2_len), wma_period)
///
/// Developed by Edwin Coppock. Originally designed to identify buying opportunities
/// in the S&P 500. Best used for identifying long-term bottoms rather than tops.
///
/// PURE core — owns no source; the factory feeds it the resolved scalar via [`CoppockCurve::feed`].
#[derive(Debug, Clone)]
pub struct CoppockCurve {
    roc1_len: usize,
    roc2_len: usize,
    wma: SmootherSlot,
    closes: VecDeque<f64>,
    value: f64,
}

impl CoppockCurve {
    /// Default ctor — ROC-sum smoothed with WMA (the classic Coppock kernel).
    pub fn new(roc1_len: usize, roc2_len: usize, wma_period: usize) -> Self {
        Self::from_smoother(roc1_len, roc2_len, wma_period, SmootherId::Wma)
    }

    /// Build from a narrow `SmootherId` for the smoother + the ROC/WMA periods.
    /// Legacy bridge; the contract path goes through `CoppockConfig`.
    pub fn from_smoother(roc1_len: usize, roc2_len: usize, wma_period: usize, smoother: SmootherId) -> Self {
        let r1 = roc1_len.max(1);
        let r2 = roc2_len.max(1);
        let wp = wma_period.max(1);
        let cap = r1.max(r2) + 1;
        Self {
            roc1_len: r1,
            roc2_len: r2,
            wma: SmootherSlot::new(smoother, wp),
            closes: VecDeque::with_capacity(cap),
            value: 0.0,
        }
    }

    /// Calculates ROC for given period.
    fn roc(&self, len: usize, c: f64) -> f64 {
        if self.closes.len() <= len {
            return 0.0;
        }
        let base = self.closes[self.closes.len() - 1 - len];
        if base.abs() < 1e-12 {
            0.0
        } else {
            100.0 * (c - base) / base
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, price: f64) -> f64 {
        self.closes.push_back(price);
        let max_need = self.roc1_len.max(self.roc2_len) + 1;
        if self.closes.len() > max_need {
            self.closes.pop_front();
        }

        let roc1 = self.roc(self.roc1_len, price);
        let roc2 = self.roc(self.roc2_len, price);
        let s = roc1 + roc2;
        self.value = self.wma.feed(s);
        self.value
    }

    /// Returns the current Coppock Curve value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the Coppock Curve has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.wma.is_ready()
    }

    /// Resets the Coppock Curve to its initial state.
    pub fn reset(&mut self) {
        self.wma.reset();
        self.closes.clear();
        self.value = 0.0;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

impl CoppockCurve {
    /// Build a CoppockCurve from a smoother CHOICE (kind + follow/own period) at `wma_period`.
    pub fn from_choice(choice: SmootherChoice, roc1_len: usize, roc2_len: usize, wma_period: usize) -> Self {
        let r1 = roc1_len.max(1);
        let r2 = roc2_len.max(1);
        let cap = r1.max(r2) + 1;
        Self {
            roc1_len: r1,
            roc2_len: r2,
            wma: choice.build(wma_period),
            closes: VecDeque::with_capacity(cap),
            value: 0.0,
        }
    }
}

/// Typed contract config for [`CoppockCurve`] — `MA(ROC(roc1) + ROC(roc2))`. Dual-mode:
/// every field is a `Param`. The `#[slot]` smoother `ma` is a `Param<SmootherChoice>`
/// that follows `wma_period` by default (WMA, the classic Coppock kernel).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct CoppockConfig {
    pub roc1_len: Param<usize>,
    pub roc2_len: Param<usize>,
    pub wma_period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for CoppockCurve {
    const ID: IndicatorId = IndicatorId::Coppock;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): two indexed ROC reads over a small deque fed to the smoother; the smoother
    /// buffer cost lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    const SLOTS: &'static [Slot] = CoppockConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Coppock)];
    type Config = CoppockConfig;
    type Runtime = CoppockCurve;

    fn create(cfg: CoppockConfig) -> CoppockCurve {
        let r1 = cfg.roc1_len.resolved().max(1);
        let r2 = cfg.roc2_len.resolved().max(1);
        let wma_period = cfg.wma_period.resolved();
        CoppockCurve {
            roc1_len: r1,
            roc2_len: r2,
            wma: cfg.ma.resolved().build(wma_period),
            closes: VecDeque::with_capacity(r1.max(r2) + 1),
            value: 0.0,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &CoppockConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &CoppockConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for CoppockConfig {
    fn defaults() -> Self {
        CoppockConfig {
            roc1_len: Param::Solo(11),
            roc2_len: Param::Solo(14),
            wma_period: Param::Solo(10),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Wma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // roc1_len/roc2_len/wma_period: Class A → auto range(2,4048,1).
        // source: Class O → auto all-8; ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for CoppockCurve {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Coppock, "Coppock", Color::hex(0x009688))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coppock_basic_calculation() {
        let mut coppock = CoppockCurve::new(11, 14, 10);

        // Feed uptrend data
        for i in 1..=50 {
            coppock.feed(100.0 + i as f64 * 2.0);
        }

        assert!(coppock.is_ready());
        // In strong uptrend, Coppock should be positive
        assert!(coppock.value() > 0.0, "Coppock in uptrend should be positive");
    }

    #[test]
    fn test_coppock_downtrend() {
        let mut coppock = CoppockCurve::new(11, 14, 10);

        // Feed downtrend data
        for i in 1..=50 {
            coppock.feed(300.0 - i as f64 * 2.0);
        }

        assert!(coppock.is_ready());
        // In strong downtrend, Coppock should be negative
        assert!(coppock.value() < 0.0, "Coppock in downtrend should be negative");
    }

    #[test]
    fn test_coppock_reset() {
        let mut coppock = CoppockCurve::new(11, 14, 10);

        for i in 1..=50 {
            coppock.feed(100.0 + i as f64);
        }
        assert!(coppock.is_ready());

        coppock.reset();
        assert!(!coppock.is_ready());
        assert!(coppock.value().abs() < 1e-10);
    }

    #[test]
    fn test_coppock_constant_price() {
        let mut coppock = CoppockCurve::new(5, 7, 5);

        // Constant price = 0 ROC
        for _ in 1..=30 {
            coppock.feed(100.0);
        }

        assert!(coppock.is_ready());
        assert!(coppock.value().abs() < 0.1, "Coppock with constant price should be near 0");
    }

    #[test]
    fn test_coppock_with_ema() {
        let mut coppock = CoppockCurve::from_smoother(5, 7, 5, SmootherId::Ema);

        for i in 1..=30 {
            coppock.feed(100.0 + i as f64);
        }

        assert!(coppock.is_ready());
        assert!(coppock.value() > 0.0);
    }

    #[test]
    fn test_coppock_contract_create() {
        let cfg = <<CoppockCurve as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut coppock = <CoppockCurve as Indicator>::create(cfg);
        for i in 1..=50 {
            coppock.feed(100.0 + i as f64 * 2.0);
        }
        assert!(coppock.is_ready());
    }
}
