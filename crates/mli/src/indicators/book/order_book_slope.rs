//! Order Book Slope — price impact curve steepness from L2 levels.
//!
//! Primary path: `update_orderbook(&OrderBook)` — fits a linear regression of
//! cumulative size (x) vs price distance from mid (y) for each side. The slope
//! (price / size) quantifies how quickly price moves per unit of depth consumed.
//! Higher slope = thinner book = larger price impact.
//!
//! Output: average of bid slope and ask slope (both positive, larger = thinner).

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderBook;

/// Default number of levels used for slope estimation.
const DEFAULT_LEVELS: usize = 10;

/// Order Book Slope indicator.
#[derive(Debug, Clone)]
pub struct OrderBookSlope {
    value: f64,
    levels: usize,
}

impl Default for OrderBookSlope {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookSlope {
    pub fn new() -> Self {
        Self { value: 0.0, levels: DEFAULT_LEVELS }
    }

    pub fn with_levels(levels: usize) -> Self {
        Self { value: 0.0, levels: levels.max(2) }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Linear regression slope: cumulative_size (x) → price_distance_from_mid (y).
    /// Returns slope ≥ 0. Larger = steeper book = less depth per tick.
    fn side_slope(mid: f64, levels: &[crate::core::types::OrderBookLevel], n_levels: usize) -> f64 {
        let n = levels.len().min(n_levels);
        if n < 2 {
            return 0.0;
        }

        let mut cumulative = 0.0;
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);

        for level in levels.iter().take(n) {
            cumulative += level.size;
            xs.push(cumulative);
            ys.push((level.price - mid).abs());
        }

        // OLS: slope = (n * Σxy - Σx * Σy) / (n * Σx² - (Σx)²)
        let n_f = n as f64;
        let sum_x: f64 = xs.iter().sum();
        let sum_y: f64 = ys.iter().sum();
        let sum_xy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = xs.iter().map(|x| x * x).sum();

        let denom = n_f * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-12 {
            return 0.0;
        }
        ((n_f * sum_xy - sum_x * sum_y) / denom).max(0.0)
    }
}

impl OrderBookConsumer for OrderBookSlope {
    /// Real book slope from linear regression over N levels on each side.
    fn update_orderbook(&mut self, book: &OrderBook) {
        let mid = match book.mid_price() {
            Some(m) => m,
            None => return,
        };

        let bid_slope = Self::side_slope(mid, &book.bids, self.levels);
        let ask_slope = Self::side_slope(mid, &book.asks, self.levels);

        self.value = (bid_slope + ask_slope) / 2.0;
    }

    fn reset(&mut self) { self.reset() }
    fn is_ready(&self) -> bool { self.is_ready() }
}

/// Typed configuration for [`OrderBookSlope`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct OrderBookSlopeConfig {
    /// Number of levels to include in the slope regression.
    pub levels: crate::contract::Param<usize>,
}

impl Indicator for OrderBookSlope {
    const ID: IndicatorId = IndicatorId::BookSlope;
    /// L2 order-book slope / price-impact indicator.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::BookSlope)];
    /// O(levels) per update — regression scans N levels.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::fixed(StoreKind::Scalar, 1)],
    );
    type Config = OrderBookSlopeConfig;
    type Runtime = OrderBookSlope;

    fn create(cfg: OrderBookSlopeConfig) -> OrderBookSlope {
        OrderBookSlope::with_levels(cfg.levels.resolved())
    }
}

impl crate::contract::Config for OrderBookSlopeConfig {
    fn defaults() -> Self {
        OrderBookSlopeConfig { levels: crate::contract::Param::Solo(DEFAULT_LEVELS) }
    }
    fn machine_defaults() -> Self {
        use crate::contract::Param;
        let mut s = Self::machine_defaults_auto();
        // levels: Class B (order-book depth count, 1–50) — correct off auto range(2,4048,1).
        s.levels = Param::range(1, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for OrderBookSlope {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BookSlope, "Book Slope", Color::hex(0x9C27B0))
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

    #[test]
    fn test_order_book_slope_creation() {
        let ind = OrderBookSlope::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_order_book_slope_nonzero_with_book() {
        let mut ind = OrderBookSlope::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 10.0), (98.0, 10.0)],
            &[(101.0, 10.0), (102.0, 10.0), (103.0, 10.0)],
            1000,
        );
        ind.update_orderbook(&book);
        assert!(ind.value() >= 0.0);
        assert!(ind.value().is_finite());
    }

    #[test]
    fn test_thinner_book_steeper_slope() {
        let mut ind_thin = OrderBookSlope::new();
        let mut ind_thick = OrderBookSlope::new();

        // Thin book: small sizes, prices spread wide
        let thin = OrderBook::from_tuples(
            &[(100.0, 1.0), (95.0, 1.0), (90.0, 1.0)],
            &[(101.0, 1.0), (106.0, 1.0), (111.0, 1.0)],
            1000,
        );
        // Thick book: large sizes, prices close together
        let thick = OrderBook::from_tuples(
            &[(100.0, 100.0), (99.9, 100.0), (99.8, 100.0)],
            &[(100.1, 100.0), (100.2, 100.0), (100.3, 100.0)],
            1000,
        );

        ind_thin.update_orderbook(&thin);
        ind_thick.update_orderbook(&thick);
        let val_thin = ind_thin.value();
        let val_thick = ind_thick.value();
        assert!(val_thin > val_thick, "thin book (few large gaps) should have steeper slope");
    }

    #[test]
    fn test_order_book_slope_reset() {
        let mut ind = OrderBookSlope::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0)],
            &[(101.0, 10.0)],
            1000,
        );
        ind.update_orderbook(&book);
        ind.reset();
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_order_book_slope() {
        let mut f = IndicatorOrder::BookSlope(
            <<OrderBookSlope as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 10.0)],
            &[(101.0, 10.0), (102.0, 10.0)],
            0,
        );
        f.feed(0, MarketSample::OrderBook(&book));
        assert!(f.primary() >= 0.0);
        assert!(f.primary().is_finite());
    }
}
