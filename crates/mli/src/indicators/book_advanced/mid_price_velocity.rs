//! MidPriceVelocity — rate of change of mid price over a rolling window.
//!
//! Uses `OrderBook::timestamp` (milliseconds) when available for time-based velocity
//! (units: price per second). Falls back to count-based velocity (price per update)
//! when two consecutive timestamps are identical.
//!
//! velocity = (latest_mid - oldest_mid) / time_span
//!
//! where time_span is in seconds (ms / 1000.0) if timestamps differ, or in updates otherwise.

use std::collections::VecDeque;

use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Rolling mid-price velocity.
#[derive(Clone, Debug)]
pub struct MidPriceVelocity {
    window: usize,
    /// Circular buffer of (mid_price, timestamp_ms) pairs.
    history: VecDeque<(f64, i64)>,
    last_velocity: f64,
}

impl MidPriceVelocity {
    /// Create with rolling window size (number of orderbook updates).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_velocity: 0.0,
        }
    }

    fn compute_velocity(oldest: (f64, i64), latest: (f64, i64)) -> f64 {
        let price_delta = latest.0 - oldest.0;
        let time_delta_ms = latest.1 - oldest.1;
        if time_delta_ms > 0 {
            // time-based: price per second
            price_delta / (time_delta_ms as f64 / 1000.0)
        } else {
            // count-based: price per update (time_delta_ms == 0 means same/no timestamp)
            price_delta
        }
    }
}

impl Default for MidPriceVelocity {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for MidPriceVelocity {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let mid = match book.mid_price() {
            Some(m) => m,
            None => return,
        };

        self.history.push_back((mid, book.timestamp));
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        if self.history.len() < 2 {
            return;
        }

        let oldest = *self.history.front().expect("len >= 2 checked above");
        let latest = *self.history.back().expect("len >= 2 checked above");
        self.last_velocity = Self::compute_velocity(oldest, latest);
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_velocity = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

/// Typed configuration for [`MidPriceVelocity`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MidPriceVelocityConfig {
    /// Rolling window size (number of order-book snapshots).
    pub window: crate::contract::Param<usize>,
}

impl Indicator for MidPriceVelocity {
    const ID: IndicatorId = IndicatorId::MidPriceVelocity;
    /// L2 order-book indicator — rate of change of the mid price.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::MidPriceVelocity)];
    /// O(1) per update (only oldest/newest are accessed, deque push/pop). One window-depth deque.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = MidPriceVelocityConfig;
    type Runtime = MidPriceVelocity;

    fn create(cfg: MidPriceVelocityConfig) -> MidPriceVelocity {
        MidPriceVelocity::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for MidPriceVelocityConfig {
    fn defaults() -> Self {
        MidPriceVelocityConfig { window: crate::contract::Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // window: Class A (rolling window period) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MidPriceVelocity {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::MidPriceVelocity, "Mid Price Velocity", Color::hex(0x9C27B0), 2.0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_book(bid: f64, ask: f64, ts_ms: i64) -> OrderBook {
        OrderBook::from_tuples(&[(bid, 1.0)], &[(ask, 1.0)], ts_ms)
    }

    #[test]
    fn count_based_velocity_zero_timestamps() {
        // All timestamps = 0 → count-based: velocity = price_delta / 1 (latest - oldest)
        let mut ind = MidPriceVelocity::new(3);
        // mid prices: 100, 101, 102
        ind.update_orderbook(&make_book(99.5, 100.5, 0));
        ind.update_orderbook(&make_book(100.5, 101.5, 0));
        ind.update_orderbook(&make_book(101.5, 102.5, 0));
        // oldest_mid=100.0, latest_mid=102.0, delta=2.0, count-based → velocity=2.0
        assert!((ind.value() - 2.0).abs() < 1e-10);
        assert!(ind.is_ready());
    }

    #[test]
    fn time_based_velocity_with_timestamps() {
        let mut ind = MidPriceVelocity::new(2);
        // mid prices: 100.0 at t=0ms, 110.0 at t=1000ms → velocity=10.0 price/sec
        ind.update_orderbook(&make_book(99.5, 100.5, 0));
        ind.update_orderbook(&make_book(109.5, 110.5, 1000));
        assert!((ind.value() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = MidPriceVelocity::new(5);
        for i in 0..4 {
            ind.update_orderbook(&make_book(100.0 + i as f64, 101.0 + i as f64, i as i64 * 100));
        }
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MidPriceVelocity::new(3);
        for i in 0..3 {
            ind.update_orderbook(&make_book(100.0 + i as f64, 101.0 + i as f64, i as i64 * 500));
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_mid_price_velocity() {
        let mut f = IndicatorOrder::MidPriceVelocity(
            <<MidPriceVelocity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Feed stationary mid — velocity converges to 0
        for _ in 0..10 {
            let book = make_book(99.5, 100.5, 0);
            f.feed(0, MarketSample::OrderBook(&book));
        }
        assert_eq!(f.primary(), 0.0);
    }
}

impl MidPriceVelocity {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_velocity
    }
}
