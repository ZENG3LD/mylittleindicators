//! MarkPriceVolatility — rolling standard deviation of mark price.
//!
//! Measures short-term volatility of the mark price feed.
//!
//! Output: `Single(std)`.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;

/// Rolling standard deviation of mark price.
///
/// Returns population std of the last `window` mark price observations.
#[derive(Debug, Clone)]
pub struct MarkPriceVolatility {
    window: usize,
    history: VecDeque<f64>,
    last_std: f64,
}

impl MarkPriceVolatility {
    /// Create with given lookback window (min 2).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_std: 0.0,
        }
    }

    fn compute_std(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        variance.sqrt()
    }
}

impl Default for MarkPriceVolatility {
    fn default() -> Self {
        Self::new(20)
    }
}

/// Typed configuration for [`MarkPriceVolatility`]. Consumes `MarkPrice` stream.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MarkPriceVolatilityConfig {
    pub window: crate::contract::Param<usize>,
}

impl Indicator for MarkPriceVolatility {
    const ID: IndicatorId = IndicatorId::MarkPriceVolatility;
    /// Mark-price volatility — `Family::Volatility` is for bar-based vol; this is a
    /// stream-only vol measure and is not pluggable as a bar family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarkPrice];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::MarkPriceVolatility)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = MarkPriceVolatilityConfig;
    type Runtime = MarkPriceVolatility;

    fn create(cfg: MarkPriceVolatilityConfig) -> MarkPriceVolatility {
        MarkPriceVolatility::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for MarkPriceVolatilityConfig {
    fn defaults() -> Self {
        MarkPriceVolatilityConfig { window: crate::contract::Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window: Class A (rolling std lookback) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MarkPriceVolatility {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MarkPriceVolatility, "Mark Price Vol", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl MarkPriceConsumer for MarkPriceVolatility {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.history.push_back(mp.mark_price);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        self.last_std = self.compute_std();
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_std = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_mark_price() {
        let mut f = IndicatorOrder::MarkPriceVolatility(
            <<MarkPriceVolatility as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let mp = MarkPrice {
            mark_price: 50_000.0,
            index_price: None,
            funding_rate: None,
            timestamp: 1,
            ..Default::default()
        };
        for _ in 0..20 {
            f.feed(0, MarketSample::MarkPrice(&mp));
        }
        let v = f.primary();
        // constant price → std = 0
        assert_eq!(v, 0.0);
    }

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice {
            mark_price,
            index_price: None,
            funding_rate: None,
            timestamp: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn constant_price_gives_zero_std() {
        let mut ind = MarkPriceVolatility::new(5);
        for _ in 0..5 {
            ind.update_mark(&make_mp(50000.0));
        }
        let v = ind.value();
        assert!(v.abs() < 1e-10, "std of constant series should be 0, got {v}");
    }

    #[test]
    fn non_constant_price_gives_positive_std() {
        let mut ind = MarkPriceVolatility::new(4);
        for p in [50000.0, 50100.0, 50200.0, 50050.0] {
            ind.update_mark(&make_mp(p));
        }
        let v = ind.value();
        assert!(v > 0.0, "std should be positive for varying prices, got {v}");
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = MarkPriceVolatility::new(5);
        for i in 0..4 {
            ind.update_mark(&make_mp(50000.0 + i as f64 * 10.0));
        }
        assert!(!ind.is_ready());
        ind.update_mark(&make_mp(50040.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceVolatility::new(5);
        for p in [50000.0, 50100.0, 50200.0, 50300.0, 50400.0] {
            ind.update_mark(&make_mp(p));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl MarkPriceVolatility {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_std
    }
}
