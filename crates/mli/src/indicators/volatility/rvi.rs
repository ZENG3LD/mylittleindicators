// High-performance Relative Volatility Index (RVI)
// (c) 2024

use crate::engine::contract_engine::SmootherSlot;
use std::collections::VecDeque;

/// Relative Volatility Index — the volatility analogue of RSI. PURE core — owns no source;
/// the factory feeds it the resolved scalar via [`Rvi::feed`].
#[derive(Debug, Clone)]
pub struct Rvi {
    period: usize,
    scalar: f64,
    // Rolling window of the last `period` closes + running sum/sum-of-squares so the
    // std-dev is O(1) per bar instead of an O(period) rescan.
    window: VecDeque<f64>,
    sum_c: f64,
    sum_c2: f64,
    count: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
    pos_rma: SmootherSlot,
    neg_rma: SmootherSlot,
    ma_close: SmootherSlot,
}

impl Rvi {
    /// Create RVI with default smoother (RMA / Wilder).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Build from a narrow `SmootherId` for the three internal smoothers + period.
    /// Legacy bridge; the contract path goes through `RviConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            scalar: 100.0,
            window: VecDeque::with_capacity(p),
            sum_c: 0.0,
            sum_c2: 0.0,
            count: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
            pos_rma: SmootherSlot::new(smoother, p),
            neg_rma: SmootherSlot::new(smoother, p),
            ma_close: SmootherSlot::new(smoother, p),
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        // Rolling window of the last `period` values + running sum/sum-of-squares.
        self.window.push_back(value);
        self.sum_c += value;
        self.sum_c2 += value * value;
        if self.window.len() > self.period {
            if let Some(old) = self.window.pop_front() {
                self.sum_c -= old;
                self.sum_c2 -= old * old;
            }
        }
        // обновить ma_close (Wilder) — это и есть mean для std
        self.ma_close.feed(value);
        let n = self.window.len();
        let mean = self.ma_close.value();
        // Σ(c - mean)^2 = Σc² - 2·mean·Σc + n·mean²  (O(1), no rescan)
        let sum_sq_dev = (self.sum_c2 - 2.0 * mean * self.sum_c + (n as f64) * mean * mean).max(0.0);
        let mut std = (sum_sq_dev / n as f64).sqrt();
        // корректировка на n-1 (несмещённая оценка)
        if n > 1 {
            std *= (n as f64).sqrt() / ((n - 1) as f64).sqrt();
        }
        // сглаживаем через RMA (Wilder)
        if self.count > 0 {
            if value > self.prev_close {
                self.pos_rma.feed(std);
                self.neg_rma.feed(0.0);
            } else if value < self.prev_close {
                self.pos_rma.feed(0.0);
                self.neg_rma.feed(std);
            } else {
                self.pos_rma.feed(0.0);
                self.neg_rma.feed(0.0);
            }
        }
        self.prev_close = value;
        self.count += 1;
        if self.count >= self.period {
            self.filled = true;
        }
        if !self.filled {
            self.value = 0.0;
            return self.value;
        }
        let pos = self.pos_rma.value();
        let neg = self.neg_rma.value();
        let denom = pos + neg;
        self.value = if denom.abs() < 1e-12 { 0.0 } else { self.scalar * pos / denom };
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.window.clear();
        self.sum_c = 0.0;
        self.sum_c2 = 0.0;
        self.count = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.value = 0.0;
        self.pos_rma.reset();
        self.neg_rma.reset();
        self.ma_close.reset();
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Typed contract config for [`Rvi`] — the volatility analogue of RSI (bounded 0-100).
/// `period` = std-dev window; `source` = the price field; `smoother` = the shape of all
/// three internal RMAs (default `follow(Rma)` at `period`).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RviConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    /// Smoother for all three internal RMAs — default `follow(Rma)` at `period`.
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for Rvi {
    const ID: IndicatorId = IndicatorId::Rvi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close) — the optimizer can sweep it.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) rolling std via running sum/sum-of-squares over a period-deep `VecDeque`; the
    /// three smoother buffers land recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    const SLOTS: &'static [Slot] = RviConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Rvi)];
    type Config = RviConfig;
    type Runtime = Rvi;

    fn create(cfg: RviConfig) -> Rvi {
        let p  = cfg.period.resolved().max(1);
        let ch = cfg.smoother.resolved();
        let sp = ch.period.resolve(p);
        Rvi {
            period: p,
            scalar: 100.0,
            window: VecDeque::with_capacity(p),
            sum_c: 0.0,
            sum_c2: 0.0,
            count: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
            pos_rma: SmootherSlot::new(ch.kind, sp),
            neg_rma: SmootherSlot::new(ch.kind, sp),
            ma_close: SmootherSlot::new(ch.kind, sp),
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &RviConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &RviConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RviConfig {
    fn defaults() -> Self {
        RviConfig {
            period:   Param::Solo(14),
            source:   Param::Solo(OhlcvField::Close),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        // source: Class O OhlcvField — auto all 8 variants
        // #[slot] smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for Rvi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rvi, "RVI", Color::hex(0x4CAF50))
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
    fn test_rvi_creation() {
        let rvi = Rvi::new(14);
        assert!(!rvi.is_ready());
        assert_eq!(rvi.value(), 0.0);
    }

    #[test]
    fn test_rvi_warmup() {
        let mut rvi = Rvi::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rvi.feed(price);
        }
        assert!(rvi.is_ready());
    }

    #[test]
    fn test_rvi_range() {
        let mut rvi = Rvi::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rvi.feed(price);
            assert!(value >= 0.0 && value <= 100.0, "RVI should be in [0, 100]");
        }
    }

    #[test]
    fn test_rvi_with_ema() {
        let mut rvi = Rvi::from_smoother(14, SmootherId::Ema);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.15).sin() * 7.0;
            rvi.feed(price);
        }
        assert!(rvi.is_ready());
    }

    #[test]
    fn test_rvi_constant_is_zero() {
        // Validates the O(1) running-std: a constant series has zero volatility, so the
        // up/down RMAs are both fed 0 and RVI must be exactly 0.
        let mut rvi = Rvi::new(14);
        for _ in 0..40 {
            rvi.feed(100.0);
        }
        assert!(rvi.is_ready());
        assert!(rvi.value().abs() < 1e-9, "RVI on constant price should be 0, got {}", rvi.value());
    }

    #[test]
    fn test_rvi_reset() {
        let mut rvi = Rvi::new(14);
        for i in 0..20 {
            rvi.feed(100.0 + i as f64);
        }
        rvi.reset();
        assert!(!rvi.is_ready());
        assert_eq!(rvi.value(), 0.0);
    }

    #[test]
    fn test_rvi_contract_create() {
        use crate::contract::Param;
        let cfg = <<Rvi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period, Param::Solo(14));
        assert_eq!(cfg.smoother.resolved().period.resolve(14), 14);
        let mut rvi = <Rvi as Indicator>::create(cfg);
        for i in 0..20 {
            rvi.feed(100.0 + i as f64);
        }
        assert!(rvi.is_ready());
    }
}
