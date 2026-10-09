// StochastikD: Stochastics indicator with Nautilus-style %D logic
// OPTIMIZED: O(1) running sum for %D calculation

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct StochastikD {
    period_k: usize,
    period_d: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,

    // VecDeque for %D buffers (O(1) pop_front)
    c_sub_1: VecDeque<f64>,
    h_sub_l: VecDeque<f64>,

    // Running sums for O(1) %D calculation
    sum_c_sub_1: f64,
    sum_h_sub_l: f64,

    initialized: bool,
    value_k: f64,
    value_d: f64,
}

impl StochastikD {
    pub fn new(period_k: usize, period_d: usize) -> Self {
        assert!(period_k > 0, "period_k must be > 0");
        assert!(period_d > 0, "period_d must be > 0");
        Self {
            period_k,
            period_d,
            highs: Vec::with_capacity(period_k),
            lows: Vec::with_capacity(period_k),
            c_sub_1: VecDeque::with_capacity(period_d),
            h_sub_l: VecDeque::with_capacity(period_d),
            sum_c_sub_1: 0.0,
            sum_h_sub_l: 0.0,
            initialized: false,
            value_k: 0.0,
            value_d: 0.0,
        }
    }

    /// Feed resolved `[high, low, close]` lanes — contract input (SOURCE = KlineSlice[H, L, C]).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        // Fill buffers to period_k
        if self.highs.len() < self.period_k {
            self.highs.push(high);
            self.lows.push(low);
            self.value_k = 0.0;
            self.value_d = 0.0;
            if self.highs.len() == self.period_k {
                self.initialized = true;
            }
            return (self.value_k, self.value_d);
        } else {
            // Shift ring buffer
            for i in 1..self.period_k {
                self.highs[i - 1] = self.highs[i];
                self.lows[i - 1] = self.lows[i];
            }
            self.highs[self.period_k - 1] = high;
            self.lows[self.period_k - 1] = low;
        }

        if !self.initialized {
            self.value_k = 0.0;
            self.value_d = 0.0;
            return (self.value_k, self.value_d);
        }

        let (k_min_low, k_max_high) = self.highs.iter()
            .zip(self.lows.iter())
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), (&h, &l)| (min.min(l), max.max(h)));

        if (k_max_high - k_min_low).abs() < 1e-12 {
            return (self.value_k, self.value_d);
        }

        self.value_k = 100.0 * ((close - k_min_low) / (k_max_high - k_min_low));

        let c1 = close - k_min_low;
        let h1 = k_max_high - k_min_low;

        if self.c_sub_1.len() >= self.period_d {
            let old_c = self.c_sub_1.pop_front().unwrap_or(0.0);
            let old_h = self.h_sub_l.pop_front().unwrap_or(0.0);
            self.sum_c_sub_1 -= old_c;
            self.sum_h_sub_l -= old_h;
        }

        self.c_sub_1.push_back(c1);
        self.h_sub_l.push_back(h1);
        self.sum_c_sub_1 += c1;
        self.sum_h_sub_l += h1;

        self.value_d = if self.sum_h_sub_l.abs() < 1e-12 {
            0.0
        } else {
            100.0 * (self.sum_c_sub_1 / self.sum_h_sub_l)
        };

        (self.value_k, self.value_d)
    }


    pub fn value_k(&self) -> f64 {
        self.value_k
    }

    pub fn value_d(&self) -> f64 {
        self.value_d
    }

    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.c_sub_1.clear();
        self.h_sub_l.clear();
        self.sum_c_sub_1 = 0.0;
        self.sum_h_sub_l = 0.0;
        self.initialized = false;
        self.value_k = 0.0;
        self.value_d = 0.0;
    }

    pub fn period_k(&self) -> usize {
        self.period_k
    }

    pub fn period_d(&self) -> usize {
        self.period_d
    }

    /// Brace-named getter: `k` output (%K).
    #[inline]
    pub fn k(&self) -> f64 {
        self.value_k
    }

    /// Brace-named getter: `d` output (%D).
    #[inline]
    pub fn d(&self) -> f64 {
        self.value_d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stochastikd_creation() {
        let stoch = StochastikD::new(14, 3);
        assert!(!stoch.is_ready());
        assert_eq!(stoch.value_k(), 0.0);
        assert_eq!(stoch.value_d(), 0.0);
        assert_eq!(stoch.period_k(), 14);
        assert_eq!(stoch.period_d(), 3);
    }

    #[test]
    fn test_stochastikd_uptrend() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            stoch.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(stoch.is_ready());
        assert!(stoch.value_k() > 50.0, "StochastikD K should be > 50 in uptrend, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastikd_downtrend() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            stoch.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(stoch.is_ready());
        assert!(stoch.value_k() < 50.0, "StochastikD K should be < 50 in downtrend, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastikd_range() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (k, d) = stoch.feed(&[price + 2.0, price - 2.0, price]);
            if stoch.is_ready() {
                assert!(k >= 0.0 && k <= 100.0, "K should be in [0, 100], got {}", k);
                assert!(d >= 0.0 && d <= 100.0, "D should be in [0, 100], got {}", d);
            }
        }
    }

    #[test]
    fn test_stochastikd_reset() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            stoch.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(stoch.is_ready());
        stoch.reset();
        assert!(!stoch.is_ready());
        assert_eq!(stoch.value_k(), 0.0);
        assert_eq!(stoch.value_d(), 0.0);
    }
}

impl Default for StochastikD {
    fn default() -> Self {
        Self::new(14, 3)
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
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Dual-mode config for [`StochastikD`]: K period + D period. No smoother slot —
/// the Nautilus-style %D is a direct ratio average, not a configurable MA.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StochastikDConfig {
    /// Lookback for highest-high / lowest-low (%K window).
    pub period_k: Param<usize>,
    /// Smoothing period for %D (running-ratio average).
    pub period_d: Param<usize>,
}

impl Indicator for StochastikD {
    const ID: IndicatorId = IndicatorId::Stochkd;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads High, Low, Close — fixed fields.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::StochkdK),
        Output::percent(IndicatorOutputId::StochkdD),
    ];
    /// O(period_k): min/max scan over the H/L window each bar.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),   // highs
            Store::window(StoreKind::Vec),   // lows
            Store::window(StoreKind::Deque), // c_sub_1
            Store::window(StoreKind::Deque), // h_sub_l
        ],
    );

    type Config = StochastikDConfig;
    type Runtime = StochastikD;

    fn create(cfg: StochastikDConfig) -> StochastikD {
        StochastikD::new(cfg.period_k.resolved(), cfg.period_d.resolved())
    }
}

impl crate::contract::Config for StochastikDConfig {
    fn defaults() -> Self {
        StochastikDConfig {
            period_k: Param::Solo(14),
            period_d: Param::Solo(3),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period_k/period_d: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for StochastikD {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::StochkdK,
                "%K",
                Color::hex(0x2196F3),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::StochkdD,
                "%D",
                Color::hex(0xFF9800),
                1.0,
            ))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Stochkd(<<StochastikD as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 100.0, "StochKD must be in [0,100], got {v}");
    }
}
