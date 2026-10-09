// High-performance Archer Moving Averages Trends (AMAT)
use std::collections::VecDeque;
use crate::engine::contract_engine::SmootherSlot;
use crate::engine::contract_engine::SmootherId;

#[derive(Debug, Clone)]
pub struct Amat {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
    pub long_run: bool,
    pub short_run: bool,
    pub initialized: bool,
    fast_ma: SmootherSlot,
    slow_ma: SmootherSlot,
    fast_ma_price: VecDeque<f64>,
    slow_ma_price: VecDeque<f64>,
    has_inputs: bool,
}

impl Amat {
    /// Create AMAT with explicit SmootherId types for fast and slow MAs.
    pub fn from_smoothers(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        fast_id: SmootherId,
        slow_id: SmootherId,
    ) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            long_run: false,
            short_run: false,
            initialized: false,
            fast_ma: SmootherSlot::new(fast_id, fast_period),
            slow_ma: SmootherSlot::new(slow_id, slow_period),
            fast_ma_price: VecDeque::with_capacity(signal_period + 1),
            slow_ma_price: VecDeque::with_capacity(signal_period + 1),
            has_inputs: false,
        }
    }

    /// Default AMAT: SMA fast + SMA slow.
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::from_smoothers(fast_period, slow_period, signal_period, SmootherId::Sma, SmootherId::Sma)
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> i8 {
        self.fast_ma.feed(value);
        self.slow_ma.feed(value);
        if self.slow_ma.is_ready() {
            self.fast_ma_price.push_back(self.fast_ma.value());
            self.slow_ma_price.push_back(self.slow_ma.value());
            if self.fast_ma_price.len() > self.signal_period + 1 {
                self.fast_ma_price.pop_front();
            }
            if self.slow_ma_price.len() > self.signal_period + 1 {
                self.slow_ma_price.pop_front();
            }
            let fast_back = self.fast_ma.value();
            let slow_back = self.slow_ma.value();
            let fast_front = *self.fast_ma_price.front().unwrap();
            let slow_front = *self.slow_ma_price.front().unwrap();
            self.long_run = fast_back - fast_front > 0.0 && slow_back - slow_front < 0.0;
            self.long_run = fast_back - fast_front > 0.0 && slow_back - slow_front > 0.0 || self.long_run;
            self.short_run = fast_back - fast_front < 0.0 && slow_back - slow_front > 0.0;
            self.short_run = fast_back - fast_front < 0.0 && slow_back - slow_front < 0.0 || self.short_run;
        }
        if !self.initialized {
            self.has_inputs = true;
            if self.slow_ma_price.len() > self.signal_period && self.slow_ma.is_ready() {
                self.initialized = true;
            }
        }
        self.value_signal()
    }

    /// Get AMAT signal.
    pub fn value(&self) -> f64 {
        (self.value_signal()) as f64
    }

    /// Get AMAT signal as i8.
    pub fn value_signal(&self) -> i8 {
        if self.long_run {
            1
        } else if self.short_run {
            -1
        } else {
            0
        }
    }

    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.fast_ma_price.clear();
        self.slow_ma_price.clear();
        self.long_run = false;
        self.short_run = false;
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl Default for Amat {
    fn default() -> Self {
        Self::from_smoothers(10, 21, 5, SmootherId::Sma, SmootherId::Sma)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Amat`] — source + two independent smoother slots.
///
/// `fast` and `slow` follow their respective period fields.
/// `signal_period` is a plain usize window for the run-detection look-back.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AmatConfig {
    pub source: Param<OhlcvField>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherChoice>,
    #[slot]
    pub slow: Param<SmootherChoice>,
    pub signal_period: Param<usize>,
}

impl Indicator for Amat {
    const ID: IndicatorId = IndicatorId::Amat;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): both MAs are smoothers; the deque holds signal_period values.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = AmatConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Amat)];
    type Config = AmatConfig;
    type Runtime = Amat;

    fn create(cfg: AmatConfig) -> Amat {
        let fast_period = cfg.fast_period.resolved().max(1);
        let slow_period = cfg.slow_period.resolved().max(1);
        let signal_period = cfg.signal_period.resolved();
        let fast_choice = cfg.fast.resolved();
        let slow_choice = cfg.slow.resolved();
        Amat {
            fast_period,
            slow_period,
            signal_period,
            long_run: false,
            short_run: false,
            initialized: false,
            fast_ma: SmootherSlot::new(fast_choice.id(), fast_period),
            slow_ma: SmootherSlot::new(slow_choice.id(), slow_period),
            fast_ma_price: VecDeque::with_capacity(signal_period + 1),
            slow_ma_price: VecDeque::with_capacity(signal_period + 1),
            has_inputs: false,
        }
    }

    fn source_fields(cfg: &AmatConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &AmatConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AmatConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        AmatConfig {
            source: Param::Solo(OhlcvField::Close),
            fast_period: Param::Solo(10),
            slow_period: Param::Solo(21),
            fast: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            slow: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            signal_period: Param::Solo(5),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // source: Class O — auto all-8; fast_period/slow_period/signal_period: Class A — auto
        // range(2,4048,1) EACH, but fast_period and slow_period then both resolve to the same
        // min (2), failing this config's OWN `valid_params` (fast < slow) at the min corner
        // (2026-07-03 fix). Split into disjoint ranges so `resolved()` stays ordered;
        // signal_period (an independent lookback) keeps the full auto range.
        // fast/slow: #[slot] SmootherChoice — left Solo (deferred wave). [FLAG: slots]
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
}


impl Render for Amat {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Amat, "AMAT", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amat_creation() {
        let amat = Amat::new(8, 21, 9);
        assert!(!amat.is_ready());
        assert_eq!(amat.value_signal(), 0);
    }

    #[test]
    fn test_amat_uptrend() {
        let mut amat = Amat::from_smoothers(5, 10, 5, SmootherId::Ema, SmootherId::Ema);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            amat.feed(price);
        }
        assert!(amat.is_ready());
        assert_eq!(amat.value_signal(), 1, "AMAT should signal long in uptrend");
    }

    #[test]
    fn test_amat_downtrend() {
        let mut amat = Amat::from_smoothers(5, 10, 5, SmootherId::Ema, SmootherId::Ema);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            amat.feed(price);
        }
        assert!(amat.is_ready());
        assert_eq!(amat.value_signal(), -1, "AMAT should signal short in downtrend");
    }

    #[test]
    fn test_amat_reset() {
        let mut amat = Amat::from_smoothers(5, 10, 5, SmootherId::Ema, SmootherId::Ema);
        for i in 1..=50 {
            amat.feed(100.0 + i as f64);
        }
        assert!(amat.is_ready());
        amat.reset();
        assert!(!amat.is_ready());
        assert_eq!(amat.value_signal(), 0);
    }

    #[test]
    fn test_amat_value_types() {
        let amat = Amat::new(5, 10, 5);
        assert_eq!(amat.value(), 0.0);
    }

    #[test]
    fn test_amat_signal_range() {
        let mut amat = Amat::new(5, 10, 5);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let signal = amat.feed(price);
            assert!(signal >= -1 && signal <= 1, "AMAT signal should be -1, 0, or 1, got {}", signal);
        }
    }

    #[test]
    fn test_factory_feeds_resolved_amat() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::{MarketSample};
        let mut f = IndicatorOrder::Amat(<<Amat as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=50 {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: close + 9999.0, // not used by Field source
                low: close - 9999.0,  // not used by Field source
                close,
                volume: 9999.0,
            });
        }
        let s = f.primary();
        assert!(s >= -1.0 && s <= 1.0, "AMAT signal should be in [-1,1], got {s}");
    }
}
