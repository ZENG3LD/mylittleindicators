//! f32 image of [`super::MarketSample`].
//!
//! Strings, ids, and timestamps stay off the kernel. Time is the third feed
//! axis, not a numeric column. Optional `f64` fields become `0.0` when absent.
//! Book and delta snapshots keep their levels. A launch packs one homogeneous
//! series of these samples into the columns [`GpuColumns`] the kernel reads.
//!
//! Scalar slots, per sample:
//! - `s0` primary number (rate, mark, oi, tick price, last, …)
//! - `s1` `s2` `s3` the next numeric fields of that variant, in the order below
//! - `side` `1` buy / `-1` sell / `0` when the variant has no side

use crate::core::types::{
    AggTrade, AuctionEvent, Basis, BlockTrade, CompositeIndex, FundingRate, FundingSettlement,
    HistoricalVolatility, IndexPrice, InsuranceFund, L3Action, Liquidation, LongShortRatio,
    MarkPrice, OptionGreeks, OpenInterest, OrderBook, OrderBookSide, OrderbookDelta,
    OrderbookL3Event, PredictedFunding, RiskLimit, SettlementEvent, Ticker, TradeSide,
    VolatilityIndex,
};

use super::ResearchBar;

/// One book level. Same pair as [`crate::core::types::OrderBookLevel`], in f32.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuLevel {
    pub price: f32,
    pub size: f32,
}

impl GpuLevel {
    pub fn new(price: f64, size: f64) -> Self {
        Self { price: price as f32, size: size as f32 }
    }
}

/// Owned f32 sample. One variant per [`super::MarketSample`] arm.
#[derive(Debug, Clone, PartialEq)]
pub enum GpuSample {
    Bar { open: f32, high: f32, low: f32, close: f32, volume: f32 },
    Tick { price: f32, size: f32, is_buy: u32, bid: f32, ask: f32 },
    /// Tick plus the book it hit. Levels are the book.
    TickBook { price: f32, size: f32, is_buy: u32, bids: Vec<GpuLevel>, asks: Vec<GpuLevel> },
    Book { bids: Vec<GpuLevel>, asks: Vec<GpuLevel> },
    Delta { bids: Vec<GpuLevel>, asks: Vec<GpuLevel> },
    Funding { rate: f32, mark_price: f32, index_price: f32, realized_rate: f32 },
    Mark { mark_price: f32, index_price: f32, funding_rate: f32, spot_price: f32 },
    OpenInterest { open_interest: f32, open_interest_value: f32 },
    /// `side`: `1` buy, `-1` sell.
    Liquidation { side: f32, price: f32, quantity: f32, value: f32 },
    Ticker { last_price: f32, bid_price: f32, ask_price: f32, volume_24h: f32 },
    AggTrade { price: f32, quantity: f32, is_buy: u32, quote_qty: f32 },
    LongShort { long_ratio: f32, short_ratio: f32, ratio: f32 },
    Basis { basis: f32, futures_price: f32, index_price: f32, basis_rate: f32 },
    Index { price: f32, high_24h: f32, low_24h: f32, open_24h: f32 },
    HistoricalVol { volatility: f32 },
    VolIndex { value: f32, open: f32, high: f32, low: f32 },
    /// Basket price. Constituent weights, symbols dropped.
    Composite { price: f32, weights: Vec<f32> },
    Insurance { balance: f32 },
    Settlement { settlement_price: f32, settled_price: f32, tax_rate: f32 },
    Block { price: f32, quantity: f32, is_buy: u32, is_iv: u32 },
    /// `side`: `1` bid, `-1` ask. `action`: `1` add, `2` modify, `3` delete.
    L3 { side: f32, price: f32, quantity: f32, action: f32 },
    Risk { tier: f32, max_leverage: f32, max_position_value: f32, mmr: f32 },
    PredictedFunding { predicted_rate: f32 },
    FundingSettlement { settled_rate: f32 },
    Auction { indicative_price: f32, indicative_qty: f32 },
    /// No numeric payload. Warning text stays off the kernel.
    MarketWarning,
    Greeks { delta: f32, gamma: f32, vega: f32, theta: f32, rho: f32, mark_iv: f32 },
}

fn opt(v: Option<f64>) -> f32 {
    v.unwrap_or(0.0) as f32
}

fn levels_of(src: &[crate::core::types::OrderBookLevel]) -> Vec<GpuLevel> {
    src.iter().map(|l| GpuLevel::new(l.price, l.size)).collect()
}

fn buy_flag(is_buy: bool) -> u32 {
    u32::from(is_buy)
}

impl From<&ResearchBar> for GpuSample {
    fn from(b: &ResearchBar) -> Self {
        Self::Bar {
            open: b.open as f32,
            high: b.high as f32,
            low: b.low as f32,
            close: b.close as f32,
            volume: b.volume as f32,
        }
    }
}

impl From<&OrderBook> for GpuSample {
    fn from(book: &OrderBook) -> Self {
        Self::Book { bids: levels_of(&book.bids), asks: levels_of(&book.asks) }
    }
}

impl From<&OrderbookDelta> for GpuSample {
    fn from(d: &OrderbookDelta) -> Self {
        Self::Delta { bids: levels_of(&d.bids), asks: levels_of(&d.asks) }
    }
}

impl From<&crate::core::types::Tick> for GpuSample {
    fn from(t: &crate::core::types::Tick) -> Self {
        Self::Tick {
            price: t.price as f32,
            size: t.size as f32,
            is_buy: buy_flag(t.is_buy),
            bid: opt(t.bid),
            ask: opt(t.ask),
        }
    }
}

impl From<&FundingRate> for GpuSample {
    fn from(fr: &FundingRate) -> Self {
        Self::Funding {
            rate: fr.rate as f32,
            mark_price: opt(fr.mark_price),
            index_price: opt(fr.index_price),
            realized_rate: opt(fr.realized_rate),
        }
    }
}

impl From<&MarkPrice> for GpuSample {
    fn from(mp: &MarkPrice) -> Self {
        Self::Mark {
            mark_price: mp.mark_price as f32,
            index_price: opt(mp.index_price),
            funding_rate: opt(mp.funding_rate),
            spot_price: opt(mp.spot_price),
        }
    }
}

impl From<&OpenInterest> for GpuSample {
    fn from(oi: &OpenInterest) -> Self {
        Self::OpenInterest {
            open_interest: oi.open_interest as f32,
            open_interest_value: opt(oi.open_interest_value),
        }
    }
}

impl From<&Liquidation> for GpuSample {
    fn from(l: &Liquidation) -> Self {
        let side = match l.side {
            TradeSide::Buy => 1.0,
            TradeSide::Sell => -1.0,
        };
        Self::Liquidation {
            side,
            price: l.price as f32,
            quantity: l.quantity as f32,
            value: opt(l.value),
        }
    }
}

impl From<&Ticker> for GpuSample {
    fn from(t: &Ticker) -> Self {
        Self::Ticker {
            last_price: t.last_price as f32,
            bid_price: opt(t.bid_price),
            ask_price: opt(t.ask_price),
            volume_24h: opt(t.volume_24h),
        }
    }
}

impl From<&AggTrade> for GpuSample {
    fn from(t: &AggTrade) -> Self {
        Self::AggTrade {
            price: t.price as f32,
            quantity: t.quantity as f32,
            is_buy: buy_flag(t.is_buy),
            quote_qty: opt(t.quote_qty),
        }
    }
}

impl From<&LongShortRatio> for GpuSample {
    fn from(r: &LongShortRatio) -> Self {
        Self::LongShort {
            long_ratio: r.long_ratio as f32,
            short_ratio: r.short_ratio as f32,
            ratio: opt(r.ratio),
        }
    }
}

impl From<&Basis> for GpuSample {
    fn from(b: &Basis) -> Self {
        Self::Basis {
            basis: b.basis as f32,
            futures_price: opt(b.futures_price),
            index_price: opt(b.index_price),
            basis_rate: opt(b.basis_rate),
        }
    }
}

impl From<&IndexPrice> for GpuSample {
    fn from(p: &IndexPrice) -> Self {
        Self::Index {
            price: p.price as f32,
            high_24h: opt(p.high_24h),
            low_24h: opt(p.low_24h),
            open_24h: opt(p.open_24h),
        }
    }
}

impl From<&HistoricalVolatility> for GpuSample {
    fn from(v: &HistoricalVolatility) -> Self {
        Self::HistoricalVol { volatility: v.volatility as f32 }
    }
}

impl From<&VolatilityIndex> for GpuSample {
    fn from(v: &VolatilityIndex) -> Self {
        Self::VolIndex {
            value: v.value as f32,
            open: opt(v.open),
            high: opt(v.high),
            low: opt(v.low),
        }
    }
}

impl From<&CompositeIndex> for GpuSample {
    fn from(c: &CompositeIndex) -> Self {
        Self::Composite {
            price: c.price as f32,
            weights: c.components.iter().map(|(_, w)| *w as f32).collect(),
        }
    }
}

impl From<&InsuranceFund> for GpuSample {
    fn from(f: &InsuranceFund) -> Self {
        Self::Insurance { balance: f.balance as f32 }
    }
}

impl From<&SettlementEvent> for GpuSample {
    fn from(s: &SettlementEvent) -> Self {
        Self::Settlement {
            settlement_price: s.settlement_price as f32,
            settled_price: opt(s.settled_price),
            tax_rate: opt(s.tax_rate),
        }
    }
}

impl From<&BlockTrade> for GpuSample {
    fn from(t: &BlockTrade) -> Self {
        Self::Block {
            price: t.price as f32,
            quantity: t.quantity as f32,
            is_buy: buy_flag(t.is_buy),
            is_iv: u32::from(t.is_iv),
        }
    }
}

impl From<&OrderbookL3Event> for GpuSample {
    fn from(e: &OrderbookL3Event) -> Self {
        let side = match e.side {
            OrderBookSide::Bid => 1.0,
            OrderBookSide::Ask => -1.0,
        };
        let action = match e.action {
            L3Action::Add => 1.0,
            L3Action::Modify => 2.0,
            L3Action::Delete => 3.0,
        };
        Self::L3 {
            side,
            price: e.price as f32,
            quantity: e.quantity as f32,
            action,
        }
    }
}

impl From<&RiskLimit> for GpuSample {
    fn from(r: &RiskLimit) -> Self {
        Self::Risk {
            tier: r.tier as f32,
            max_leverage: r.max_leverage as f32,
            max_position_value: r.max_position_value as f32,
            mmr: r.mmr as f32,
        }
    }
}

impl From<&PredictedFunding> for GpuSample {
    fn from(p: &PredictedFunding) -> Self {
        Self::PredictedFunding { predicted_rate: p.predicted_rate as f32 }
    }
}

impl From<&FundingSettlement> for GpuSample {
    fn from(s: &FundingSettlement) -> Self {
        Self::FundingSettlement { settled_rate: s.settled_rate as f32 }
    }
}

impl From<&AuctionEvent> for GpuSample {
    fn from(a: &AuctionEvent) -> Self {
        Self::Auction {
            indicative_price: a.indicative_price as f32,
            indicative_qty: a.indicative_qty as f32,
        }
    }
}

impl From<&crate::core::types::MarketWarning> for GpuSample {
    fn from(_: &crate::core::types::MarketWarning) -> Self {
        Self::MarketWarning
    }
}

impl From<&OptionGreeks> for GpuSample {
    fn from(g: &OptionGreeks) -> Self {
        Self::Greeks {
            delta: g.delta as f32,
            gamma: g.gamma as f32,
            vega: g.vega as f32,
            theta: g.theta as f32,
            rho: g.rho as f32,
            mark_iv: g.mark_iv as f32,
        }
    }
}

/// Columns one launch uploads. Book planes are row-major, `depth` levels per sample.
#[derive(Debug, Clone)]
pub struct GpuColumns {
    pub open: Vec<f32>,
    pub high: Vec<f32>,
    pub low: Vec<f32>,
    pub close: Vec<f32>,
    pub volume: Vec<f32>,
    pub s0: Vec<f32>,
    pub s1: Vec<f32>,
    pub s2: Vec<f32>,
    pub s3: Vec<f32>,
    pub side: Vec<f32>,
    pub bid_px: Vec<f32>,
    pub bid_sz: Vec<f32>,
    pub ask_px: Vec<f32>,
    pub ask_sz: Vec<f32>,
    pub bid_n: Vec<f32>,
    pub ask_n: Vec<f32>,
    pub depth: u32,
}

struct Row {
    open: f32,
    high: f32,
    low: f32,
    close: f32,
    volume: f32,
    s0: f32,
    s1: f32,
    s2: f32,
    s3: f32,
    side: f32,
    bids: Vec<GpuLevel>,
    asks: Vec<GpuLevel>,
}

fn row0() -> Row {
    Row {
        open: 0.0,
        high: 0.0,
        low: 0.0,
        close: 0.0,
        volume: 0.0,
        s0: 0.0,
        s1: 0.0,
        s2: 0.0,
        s3: 0.0,
        side: 0.0,
        bids: Vec::new(),
        asks: Vec::new(),
    }
}

impl GpuSample {
    fn row(&self) -> Row {
        let mut r = row0();
        match self {
            Self::Bar { open, high, low, close, volume } => {
                r.open = *open;
                r.high = *high;
                r.low = *low;
                r.close = *close;
                r.volume = *volume;
            }
            Self::Tick { price, size, is_buy, bid, ask } => {
                r.s0 = *price;
                r.s1 = *size;
                r.s2 = *bid;
                r.s3 = *ask;
                r.side = if *is_buy == 1 { 1.0 } else { -1.0 };
            }
            Self::TickBook { price, size, is_buy, bids, asks } => {
                r.s0 = *price;
                r.s1 = *size;
                r.side = if *is_buy == 1 { 1.0 } else { -1.0 };
                r.bids = bids.clone();
                r.asks = asks.clone();
            }
            Self::Book { bids, asks } | Self::Delta { bids, asks } => {
                r.bids = bids.clone();
                r.asks = asks.clone();
            }
            Self::Funding { rate, mark_price, index_price, realized_rate } => {
                r.s0 = *rate;
                r.s1 = *mark_price;
                r.s2 = *index_price;
                r.s3 = *realized_rate;
            }
            Self::Mark { mark_price, index_price, funding_rate, spot_price } => {
                r.s0 = *mark_price;
                r.s1 = *index_price;
                r.s2 = *funding_rate;
                r.s3 = *spot_price;
            }
            Self::OpenInterest { open_interest, open_interest_value } => {
                r.s0 = *open_interest;
                r.s1 = *open_interest_value;
            }
            Self::Liquidation { side, price, quantity, value } => {
                r.side = *side;
                r.s0 = *price;
                r.s1 = *quantity;
                r.s2 = *value;
            }
            Self::Ticker { last_price, bid_price, ask_price, volume_24h } => {
                r.s0 = *last_price;
                r.s1 = *bid_price;
                r.s2 = *ask_price;
                r.s3 = *volume_24h;
            }
            Self::AggTrade { price, quantity, is_buy, quote_qty } => {
                r.s0 = *price;
                r.s1 = *quantity;
                r.s3 = *quote_qty;
                r.side = if *is_buy == 1 { 1.0 } else { -1.0 };
            }
            Self::LongShort { long_ratio, short_ratio, ratio } => {
                r.s0 = *long_ratio;
                r.s1 = *short_ratio;
                r.s2 = *ratio;
            }
            Self::Basis { basis, futures_price, index_price, basis_rate } => {
                r.s0 = *basis;
                r.s1 = *futures_price;
                r.s2 = *index_price;
                r.s3 = *basis_rate;
            }
            Self::Index { price, high_24h, low_24h, open_24h } => {
                r.s0 = *price;
                r.s1 = *high_24h;
                r.s2 = *low_24h;
                r.s3 = *open_24h;
            }
            Self::HistoricalVol { volatility } => r.s0 = *volatility,
            Self::VolIndex { value, open, high, low } => {
                r.s0 = *value;
                r.s1 = *open;
                r.s2 = *high;
                r.s3 = *low;
            }
            Self::Composite { price, .. } => r.s0 = *price,
            Self::Insurance { balance } => r.s0 = *balance,
            Self::Settlement { settlement_price, settled_price, tax_rate } => {
                r.s0 = *settlement_price;
                r.s1 = *settled_price;
                r.s2 = *tax_rate;
            }
            Self::Block { price, quantity, is_buy, is_iv } => {
                r.s0 = *price;
                r.s1 = *quantity;
                r.s2 = *is_iv as f32;
                r.side = if *is_buy == 1 { 1.0 } else { -1.0 };
            }
            Self::L3 { side, price, quantity, action } => {
                r.side = *side;
                r.s0 = *price;
                r.s1 = *quantity;
                r.s2 = *action;
            }
            Self::Risk { tier, max_leverage, max_position_value, mmr } => {
                r.s0 = *tier;
                r.s1 = *max_leverage;
                r.s2 = *max_position_value;
                r.s3 = *mmr;
            }
            Self::PredictedFunding { predicted_rate } => r.s0 = *predicted_rate,
            Self::FundingSettlement { settled_rate } => r.s0 = *settled_rate,
            Self::Auction { indicative_price, indicative_qty } => {
                r.s0 = *indicative_price;
                r.s1 = *indicative_qty;
            }
            Self::MarketWarning => {}
            Self::Greeks { delta, gamma, vega, theta, .. } => {
                r.s0 = *delta;
                r.s1 = *gamma;
                r.s2 = *vega;
                r.s3 = *theta;
            }
        }
        r
    }

    /// Pack `samples` into kernel columns. Empty input yields empty columns and `depth` 1.
    pub fn columns(samples: &[Self]) -> GpuColumns {
        let n = samples.len();
        let rows: Vec<Row> = samples.iter().map(Self::row).collect();
        let mut depth = 1usize;
        for r in &rows {
            depth = depth.max(r.bids.len()).max(r.asks.len());
        }
        let mut cols = GpuColumns {
            open: Vec::with_capacity(n),
            high: Vec::with_capacity(n),
            low: Vec::with_capacity(n),
            close: Vec::with_capacity(n),
            volume: Vec::with_capacity(n),
            s0: Vec::with_capacity(n),
            s1: Vec::with_capacity(n),
            s2: Vec::with_capacity(n),
            s3: Vec::with_capacity(n),
            side: Vec::with_capacity(n),
            bid_px: vec![0.0; n * depth],
            bid_sz: vec![0.0; n * depth],
            ask_px: vec![0.0; n * depth],
            ask_sz: vec![0.0; n * depth],
            bid_n: Vec::with_capacity(n),
            ask_n: Vec::with_capacity(n),
            depth: depth as u32,
        };
        for (i, r) in rows.into_iter().enumerate() {
            cols.open.push(r.open);
            cols.high.push(r.high);
            cols.low.push(r.low);
            cols.close.push(r.close);
            cols.volume.push(r.volume);
            cols.s0.push(r.s0);
            cols.s1.push(r.s1);
            cols.s2.push(r.s2);
            cols.s3.push(r.s3);
            cols.side.push(r.side);
            cols.bid_n.push(r.bids.len() as f32);
            cols.ask_n.push(r.asks.len() as f32);
            for (k, lvl) in r.bids.iter().enumerate() {
                cols.bid_px[i * depth + k] = lvl.price;
                cols.bid_sz[i * depth + k] = lvl.size;
            }
            for (k, lvl) in r.asks.iter().enumerate() {
                cols.ask_px[i * depth + k] = lvl.price;
                cols.ask_sz[i * depth + k] = lvl.size;
            }
        }
        cols
    }
}

/// Per-bar calendar columns for the time adapter. Bar timestamps are unix
/// milliseconds and do not fit f32, so the host reduces each one to the small
/// exact integers the calendar formulas need. Rows are in the same order as the
/// samples of the launch. The values are the ones `CalendarService` gives the
/// CPU feeds, so a calendar formula and its feed agree on every bucket.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuTimes {
    /// `0` Monday .. `6` Sunday.
    pub weekday: Vec<f32>,
    /// UTC hour, `0..=23`.
    pub hour: Vec<f32>,
    /// `1..=12`.
    pub month: Vec<f32>,
    /// Day of month, `1..=31`.
    pub dom: Vec<f32>,
    /// Days in the bar's month, `28..=31`.
    pub dim: Vec<f32>,
    /// Occurrence of the weekday within the month (`1..=5`), as `DayOfWeekInMonthEffect` gives it.
    pub occ: Vec<f32>,
    /// Days to the nearest quarter boundary when that is `<= 15`, else `99`
    /// (read from `QuarterTurnEffect` with a 15 day window, so it agrees with the CPU feed).
    pub qnear: Vec<f32>,
}

impl GpuTimes {
    /// Build the columns from bar times in unix milliseconds.
    pub fn from_ms(times_ms: &[i64]) -> Self {
        use crate::core::types::CalendarService;
        let mut out = GpuTimes::default();
        for &ms in times_ms {
            let secs = ms.div_euclid(1000);
            let (_y, m, d) = CalendarService::ymd_from_timestamp(secs);
            out.weekday.push((CalendarService::weekday_from_timestamp(secs) as usize).min(6) as f32);
            out.hour.push((secs.rem_euclid(86_400) / 3600) as f32);
            out.month.push(m.clamp(1, 12) as f32);
            out.dom.push(d.clamp(1, 31) as f32);
            out.dim.push(CalendarService::days_in_month(_y, m) as f32);
            out.occ.push(
                crate::indicators::calendar::day_of_week_in_month::DayOfWeekInMonthEffect::new()
                    .feed(ms) as f32,
            );
            let qv = crate::indicators::calendar::quarter_turn_effect::QuarterTurnEffect::new(15)
                .feed(ms);
            out.qnear.push(if qv > 0.0 { ((1.0 - qv) * 15.0).round() as f32 } else { 99.0 });
        }
        out
    }

    /// Build the columns from research bars.
    pub fn from_bars(bars: &[ResearchBar]) -> Self {
        let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
        Self::from_ms(&times)
    }

    /// Rows in the adapter.
    pub fn len(&self) -> usize {
        self.weekday.len()
    }

    /// `true` when there are no rows.
    pub fn is_empty(&self) -> bool {
        self.weekday.is_empty()
    }
}
