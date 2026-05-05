//! Bar and Tick types for market data

/// OHLCV Bar - aggregated candlestick data
///
/// This is the universal bar type used by all indicators.
/// Contains time, open, high, low, close, and volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Bar {
    /// Create a new bar
    #[inline]
    pub fn new(time: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self { time, open, high, low, close, volume }
    }

    /// Create bar without time (for simple calculations)
    #[inline]
    pub fn ohlcv(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self { time: 0, open, high, low, close, volume }
    }

    /// Bar range (high - low)
    #[inline]
    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    /// Bar body size (abs(close - open))
    #[inline]
    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Is bullish (close > open)
    #[inline]
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Is bearish (close < open)
    #[inline]
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Typical price (H+L+C)/3
    #[inline]
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }

    /// HLCC/4 price
    #[inline]
    pub fn hlcc4(&self) -> f64 {
        (self.high + self.low + self.close + self.close) / 4.0
    }

    /// OHLC/4 price
    #[inline]
    pub fn ohlc4(&self) -> f64 {
        (self.open + self.high + self.low + self.close) / 4.0
    }

    /// HL/2 (median price)
    #[inline]
    pub fn hl2(&self) -> f64 {
        (self.high + self.low) / 2.0
    }
}

/// Tick - raw market data point
///
/// Used for order flow analysis, market microstructure,
/// and tick-based indicators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub time: i64,
    pub price: f64,
    pub size: f64,
    /// Trade direction: true = buy (taker bought), false = sell (taker sold)
    pub is_buy: bool,
    /// Best bid at time of tick (if available)
    pub bid: Option<f64>,
    /// Best ask at time of tick (if available)
    pub ask: Option<f64>,
}

impl Tick {
    /// Create a new tick
    #[inline]
    pub fn new(time: i64, price: f64, size: f64, is_buy: bool) -> Self {
        Self { time, price, size, is_buy, bid: None, ask: None }
    }

    /// Create tick with bid/ask
    #[inline]
    pub fn with_quotes(time: i64, price: f64, size: f64, is_buy: bool, bid: f64, ask: f64) -> Self {
        Self { time, price, size, is_buy, bid: Some(bid), ask: Some(ask) }
    }

    /// Spread if bid/ask available
    #[inline]
    pub fn spread(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }

    /// Signed size (positive for buys, negative for sells)
    #[inline]
    pub fn signed_size(&self) -> f64 {
        if self.is_buy { self.size } else { -self.size }
    }
}
