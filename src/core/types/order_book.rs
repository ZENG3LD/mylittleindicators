//! L2 orderbook snapshot types.
//!
//! Minimal representation of a full L2 orderbook consumed by book indicators.
//! Matches the structure from digdigdig3 (price + size, optional order_count).

/// One price level in the order book.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
    /// Number of orders at this level (some exchanges provide this).
    pub order_count: Option<u32>,
}

impl OrderBookLevel {
    pub fn new(price: f64, size: f64) -> Self {
        Self { price, size, order_count: None }
    }
}

impl From<(f64, f64)> for OrderBookLevel {
    fn from((price, size): (f64, f64)) -> Self {
        Self::new(price, size)
    }
}

/// L2 orderbook snapshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderBook {
    /// Bid levels, ordered best-to-worst (highest price first).
    pub bids: Vec<OrderBookLevel>,
    /// Ask levels, ordered best-to-worst (lowest price first).
    pub asks: Vec<OrderBookLevel>,
    /// Snapshot timestamp (milliseconds).
    pub timestamp: i64,
}

impl OrderBook {
    pub fn new(bids: Vec<OrderBookLevel>, asks: Vec<OrderBookLevel>, timestamp: i64) -> Self {
        Self { bids, asks, timestamp }
    }

    /// Construct from tuple slices — convenience for tests.
    pub fn from_tuples(bids: &[(f64, f64)], asks: &[(f64, f64)], timestamp: i64) -> Self {
        Self {
            bids: bids.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            asks: asks.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            timestamp,
        }
    }

    /// Best bid level (highest price).
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.first()
    }

    /// Best ask level (lowest price).
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.first()
    }

    /// Mid price: (best_bid + best_ask) / 2.
    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => Some((b.price + a.price) / 2.0),
            _ => None,
        }
    }

    /// Spread: best_ask - best_bid.
    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => Some(a.price - b.price),
            _ => None,
        }
    }

    /// Sum of bid sizes up to `levels` levels.
    pub fn bid_depth(&self, levels: usize) -> f64 {
        self.bids.iter().take(levels).map(|l| l.size).sum()
    }

    /// Sum of ask sizes up to `levels` levels.
    pub fn ask_depth(&self, levels: usize) -> f64 {
        self.asks.iter().take(levels).map(|l| l.size).sum()
    }
}
