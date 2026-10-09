use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use std::collections::VecDeque;

/// Chaikin Volatility = k-period rate-of-change of EMA(high-low, n).
///
/// CV = 100 * (EMA_n(H-L) - EMA_n(H-L)[k bars ago]) / EMA_n(H-L)[k bars ago]
#[derive(Debug, Clone)]
pub struct ChaikinVolatility {
    ema_range: SmootherSlot,
    /// History of the smoothed range, depth `k + 1`, for the true k-period ROC.
    ema_buf: VecDeque<f64>,
    k: usize,
    value: f64,
    ready: bool,
}

impl ChaikinVolatility {
    /// Default ctor — range smoothed with EMA (the classic Chaikin Volatility kernel).
    pub fn new(n: usize, k: usize) -> Self {
        Self::from_smoother(n, k, SmootherId::Ema)
    }

    /// Build from a narrow `SmootherId` for the range smoother + the n/k periods.
    /// Legacy bridge; the contract path goes through `ChaikinVolatilityConfig`.
    pub fn from_smoother(n: usize, k: usize, smoother: SmootherId) -> Self {
        let kk = k.max(1);
        Self {
            ema_range: SmootherSlot::new(smoother, n.max(1)),
            ema_buf: VecDeque::with_capacity(kk + 1),
            k: kk,
            value: 0.0,
            ready: false,
        }
    }

    /// Feed the resolved `[high, low]` lanes (in `SOURCE` order); smooths `high - low`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let range = (h - l).max(0.0);
        let ema = self.ema_range.feed(range);
        if self.ema_range.is_ready() {
            self.ema_buf.push_back(ema);
            if self.ema_buf.len() > self.k + 1 {
                self.ema_buf.pop_front();
            }
            // True k-period ROC of the smoothed range.
            if self.ema_buf.len() > self.k {
                let past = self.ema_buf[0];
                self.value = if past.abs() < 1e-12 {
                    0.0
                } else {
                    100.0 * (ema - past) / past
                };
                self.ready = true;
            }
        }
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.ema_range.reset();
        self.ema_buf.clear();
        self.value = 0.0;
        self.ready = false;
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`ChaikinVolatility`] — k-period ROC of EMA(high-low).
/// `n_period` = range-smoother lookback; `k_period` = ROC lookback; `smoother` = the
/// range MA shape (default `follow(Ema)` at `n_period`).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ChaikinVolatilityConfig {
    /// Range-smoother lookback period.
    pub n_period: Param<usize>,
    /// ROC lookback period.
    pub k_period: Param<usize>,
    /// Range smoother — default `follow(Ema)` at `n_period`.
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for ChaikinVolatility {
    const ID: IndicatorId = IndicatorId::Cv;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the bar range — smooths `high - low`.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1) k-period ROC over a `k`-deep heap ring of smoothed ranges; the smoother buffer
    /// cost lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    const SLOTS: &'static [Slot] = ChaikinVolatilityConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cv)];
    type Config = ChaikinVolatilityConfig;
    type Runtime = ChaikinVolatility;

    fn create(cfg: ChaikinVolatilityConfig) -> ChaikinVolatility {
        let n  = cfg.n_period.resolved().max(1);
        let kk = cfg.k_period.resolved().max(1);
        let ch = cfg.smoother.resolved();
        ChaikinVolatility {
            ema_range: SmootherSlot::new(ch.kind, ch.period.resolve(n)),
            ema_buf: VecDeque::with_capacity(kk + 1),
            k: kk,
            value: 0.0,
            ready: false,
        }
    }

    fn slot_members(cfg: &ChaikinVolatilityConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ChaikinVolatilityConfig {
    fn defaults() -> Self {
        ChaikinVolatilityConfig {
            n_period: Param::Solo(10),
            k_period: Param::Solo(10),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn machine_defaults() -> Self {
        // n_period, k_period: Class A usize — auto range(2,4048,1)
        // #[slot] smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for ChaikinVolatility {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cv, "Coef of Var", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaikin_volatility_creation() {
        let cv = ChaikinVolatility::new(10, 10);
        assert!(!cv.is_ready());
        assert_eq!(cv.value(), 0.0);
    }

    #[test]
    fn test_chaikin_volatility_warmup() {
        let mut cv = ChaikinVolatility::new(10, 10);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cv.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(cv.is_ready());
    }

    #[test]
    fn test_chaikin_volatility_values() {
        let mut cv = ChaikinVolatility::new(10, 10);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            let value = cv.feed(&[price + 2.0, price - 2.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_chaikin_volatility_rising_range_positive() {
        // Range widening over time -> EMA(range) rising -> positive k-ROC.
        let mut cv = ChaikinVolatility::new(5, 5);
        for i in 0..40 {
            let half = 1.0 + i as f64 * 0.5; // widening bar
            cv.feed(&[100.0 + half, 100.0 - half]);
        }
        assert!(cv.is_ready());
        assert!(cv.value() > 0.0, "widening range should give positive CV, got {}", cv.value());
    }

    #[test]
    fn test_chaikin_volatility_reset() {
        let mut cv = ChaikinVolatility::new(10, 10);
        for _i in 0..40 {
            cv.feed(&[101.0, 99.0]);
        }
        cv.reset();
        assert!(!cv.is_ready());
        assert_eq!(cv.value(), 0.0);
    }

    #[test]
    fn test_chaikin_volatility_contract_create() {
        let cfg = <<ChaikinVolatility as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.smoother.resolved().period.resolve(10), 10);
        let mut cv = <ChaikinVolatility as Indicator>::create(cfg);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            cv.feed(&[price + 2.0, price - 2.0]);
        }
        assert!(cv.is_ready());
    }
}
