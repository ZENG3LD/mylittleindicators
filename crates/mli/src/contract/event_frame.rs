//! Packed event columns for the stream (non-bar) formulas.
//!
//! [`super::GpuSample`] keeps four scalar slots per event and drops timestamps and the
//! optional fields several consumers read. The event formulas (codes 900..=999) use this
//! richer frame instead: one `f32` matrix of [`EVENT_COLS`] columns, a `side` column and a
//! `ts` column. Layout, per stream (absent `Option` fields are `0.0`; where a CPU consumer
//! only uses a pair when BOTH halves are present, the constructor zeroes both otherwise):
//!
//! | stream            | x0         | x1          | x2        | x3        | x4       | x5          | x6         | x7        |
//! |-------------------|------------|-------------|-----------|-----------|----------|-------------|------------|-----------|
//! | Tick              | price      | size        | bid       | ask       |          |             |            |           |
//! | Ticker            | last       | bid         | ask       | high_24h  | low_24h  | volume_24h  | pct_24h    | chg_24h   |
//! | OptionGreeks      | delta      | gamma       | vega      | theta     | rho      | mark_iv     | bid_iv     | ask_iv    |
//! | RiskLimit         | tier       | max_leverage| max_pos   | mmr       | imr      |             |            |           |
//! | Funding           | rate       | mark        | index     | realized  | estimated| premium     | prev_index |           |
//! | PredictedFunding  | rate       | countdown_ms| ts>0      |           |          |             |            |           |
//! | LongShortRatio    | long       | short       | ratio     | buy_ratio | sell_ratio|            |            |           |
//! | Liquidation       | price      | quantity    | quote_value|           |          |             |            |           |
//! | OpenInterest      | oi         | oi_value    |           |           |          |             |            |           |
//! | MarkPrice         | mark       | index       | funding   |           |          |             |            |           |
//! | AggTrade          | price      | quantity    | quote_qty |           |          |             |            |           |
//! | BlockTrade        | price      | quantity    | is_iv     |           |          |             |            |           |
//! | Auction           | indicative | qty         |           |           |          |             |            |           |
//! | InsuranceFund     | balance    |             |           |           |          |             |            |           |
//! | HistoricalVol     | volatility |             |           |           |          |             |            |           |
//! | VolatilityIndex   | value      | open        | high      | low       | close    |             |            |           |
//! | Settlement        | price      | countdown_ms| ts>0      |           |          |             |            |           |
//!
//! `side` is `1` buy / `-1` sell (ticks, agg trades, block trades, liquidations: the
//! `TradeSide`), `0` when the stream has no side. `ts` is MILLISECONDS since the first event of the
//! frame as `f32`: exact for spans up to 16,777,216 ms (4.66 hours); beyond that consecutive
//! integers round to even, so window-edge membership can differ by 1-2 ms from the CPU. Chunk
//! longer series into frames, or accept that jitter. UNTESTED on GPU.

use crate::core::types::{
    AggTrade, AuctionEvent, BlockTrade, FundingRate, HistoricalVolatility, InsuranceFund,
    Liquidation, LongShortRatio, MarkPrice, OpenInterest, OptionGreeks, PredictedFunding,
    MarketWarning, RiskLimit, SettlementEvent, Ticker, TradeSide, VolatilityIndex,
};
use crate::core::types::Tick;

/// Matrix columns in a frame.
pub const EVENT_COLS: usize = 8;

/// Column-major event frame: `x[c * n + i]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuEventFrame {
    pub n: usize,
    pub x: Vec<f32>,
    pub side: Vec<f32>,
    pub ts: Vec<f32>,
}

fn o(v: Option<f64>) -> f64 {
    v.unwrap_or(0.0)
}

impl GpuEventFrame {
    /// Build from rows of up to eight values, a side and a unix-ms timestamp.
    pub fn from_rows(rows: &[([f64; EVENT_COLS], f32, i64)]) -> Self {
        let n = rows.len();
        let t0 = rows.first().map(|r| r.2).unwrap_or(0);
        let mut f = GpuEventFrame {
            n,
            x: vec![0.0; n * EVENT_COLS],
            side: Vec::with_capacity(n),
            ts: Vec::with_capacity(n),
        };
        for (i, (v, s, t)) in rows.iter().enumerate() {
            for c in 0..EVENT_COLS {
                f.x[c * n + i] = v[c] as f32;
            }
            f.side.push(*s);
            f.ts.push((*t - t0) as f32);
        }
        f
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// L3 events: `x0` price, `x1` quantity, `x2` action (1 add, 2 modify, 3 delete), side 1 bid / -1 ask.
    pub fn from_l3(v: &[crate::core::types::OrderbookL3Event]) -> Self {
        use crate::core::types::{L3Action, OrderBookSide};
        let r: Vec<_> = v
            .iter()
            .map(|e| {
                let act = match e.action {
                    L3Action::Add => 1.0,
                    L3Action::Modify => 2.0,
                    L3Action::Delete => 3.0,
                };
                (
                    [e.price, e.quantity, act, 0.0, 0.0, 0.0, 0.0, 0.0],
                    if e.side == OrderBookSide::Bid { 1.0 } else { -1.0 },
                    e.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    /// Book deltas: `x0` total changed levels, `x1` levels with size > 0 (added / updated).
    pub fn from_deltas(v: &[crate::core::types::OrderbookDelta]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|d| {
                let upd = d.updated_bids().count() + d.updated_asks().count();
                ([d.total_changes() as f64, upd as f64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0, d.timestamp)
            })
            .collect();
        Self::from_rows(&r)
    }

    /// Basis: `x0` basis.
    pub fn from_basis(v: &[crate::core::types::Basis]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|b| ([b.basis, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0, b.timestamp))
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_ticks(v: &[Tick]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|t| {
                (
                    [t.price, t.size, o(t.bid), o(t.ask), 0.0, 0.0, 0.0, 0.0],
                    if t.is_buy { 1.0 } else { -1.0 },
                    t.time,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_tickers(v: &[Ticker]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|t| {
                let (h, l) = match (t.high_24h, t.low_24h) {
                    (Some(h), Some(l)) => (h, l),
                    _ => (0.0, 0.0),
                };
                let (b, a) = match (t.bid_price, t.ask_price) {
                    (Some(b), Some(a)) => (b, a),
                    _ => (0.0, 0.0),
                };
                (
                    [
                        t.last_price,
                        b,
                        a,
                        h,
                        l,
                        o(t.volume_24h),
                        o(t.price_change_percent_24h),
                        o(t.price_change_24h),
                    ],
                    0.0,
                    t.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_greeks(v: &[OptionGreeks]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                let (b, a) = match (g.bid_iv, g.ask_iv) {
                    (Some(b), Some(a)) => (b, a),
                    _ => (0.0, 0.0),
                };
                (
                    [g.delta, g.gamma, g.vega, g.theta, g.rho, g.mark_iv, b, a],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_risk_limits(v: &[RiskLimit]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [
                        g.tier as f64,
                        g.max_leverage,
                        g.max_position_value,
                        g.mmr,
                        g.imr,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_funding(v: &[FundingRate]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [
                        g.rate,
                        o(g.mark_price),
                        o(g.index_price),
                        o(g.realized_rate),
                        o(g.estimated_rate),
                        o(g.premium),
                        o(g.prev_index_price),
                        0.0,
                    ],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_predicted_funding(v: &[PredictedFunding]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [
                        g.predicted_rate,
                        (g.next_funding_time - g.timestamp).max(0) as f64,
                        if g.timestamp > 0 { 1.0 } else { 0.0 },
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_long_short(v: &[LongShortRatio]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [
                        g.long_ratio,
                        g.short_ratio,
                        o(g.ratio),
                        o(g.buy_ratio),
                        o(g.sell_ratio),
                        0.0,
                        0.0,
                        0.0,
                    ],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_liquidations(v: &[Liquidation]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [g.price, g.quantity, g.quote_value(), 0.0, 0.0, 0.0, 0.0, 0.0],
                    if matches!(g.side, TradeSide::Buy) { 1.0 } else { -1.0 },
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_open_interest(v: &[OpenInterest]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [g.open_interest, o(g.open_interest_value), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_mark(v: &[MarkPrice]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [g.mark_price, o(g.index_price), o(g.funding_rate), 0.0, 0.0, 0.0, 0.0, 0.0],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_agg_trades(v: &[AggTrade]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [g.price, g.quantity, o(g.quote_qty), 0.0, 0.0, 0.0, 0.0, 0.0],
                    if g.is_buy { 1.0 } else { -1.0 },
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_block_trades(v: &[BlockTrade]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [g.price, g.quantity, if g.is_iv { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0, 0.0],
                    if g.is_buy { 1.0 } else { -1.0 },
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_auctions(v: &[AuctionEvent]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [g.indicative_price, g.indicative_qty, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_insurance(v: &[InsuranceFund]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| ([g.balance, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0, g.timestamp))
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_historical_vol(v: &[HistoricalVolatility]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| ([g.volatility, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0, g.timestamp))
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_vol_index(v: &[VolatilityIndex]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [
                        g.value,
                        o(g.open),
                        o(g.high),
                        o(g.low),
                        o(g.close),
                        0.0,
                        0.0,
                        0.0,
                    ],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_settlements(v: &[SettlementEvent]) -> Self {
        let r: Vec<_> = v
            .iter()
            .map(|g| {
                (
                    [
                        g.settlement_price,
                        (g.settlement_time - g.timestamp).max(0) as f64,
                        if g.timestamp > 0 { 1.0 } else { 0.0 },
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ],
                    0.0,
                    g.timestamp,
                )
            })
            .collect();
        Self::from_rows(&r)
    }

    pub fn from_market_warnings(v: &[MarketWarning]) -> Self {
        let r: Vec<_> = v.iter().map(|g| ([0.0; EVENT_COLS], 0.0, g.timestamp)).collect();
        Self::from_rows(&r)
    }
}

/// One input stream of a merged (as-of joined) frame: `width` values per event.
#[derive(Debug, Clone, Default)]
pub struct MergedStream {
    pub width: usize,
    pub events: Vec<(i64, Vec<f64>)>,
}

impl MergedStream {
    /// Build from any slice: `ts` gives the event time in ms, `vals` its `width` values.
    pub fn from_fn<T>(
        items: &[T],
        width: usize,
        ts: impl Fn(&T) -> i64,
        vals: impl Fn(&T) -> Vec<f64>,
    ) -> Self {
        MergedStream {
            width,
            events: items.iter().map(|t| (ts(t), vals(t))).collect(),
        }
    }

    /// One value per event, e.g. `MergedStream::scalar(&funding, |f| f.timestamp, |f| f.rate)`.
    pub fn scalar<T>(items: &[T], ts: impl Fn(&T) -> i64, v: impl Fn(&T) -> f64) -> Self {
        Self::from_fn(items, 1, ts, |t| vec![v(t)])
    }
}

impl GpuEventFrame {
    /// As-of join of several streams on the host, the layout of the multi-stream rows (codes
    /// 1070..). One frame row per input event, ordered by time (ties: stream order, then input
    /// order). Stream `s` owns `width_s` consecutive columns, assigned in stream order (at most
    /// [`EVENT_COLS`] in total); they hold that stream's LATEST values (`0.0` before its first
    /// event). `side` is the 1-based index of the stream that produced the row, so a kernel
    /// knows which consumer fired. `ts` is ms from the first row. Per-formula slot order is
    /// documented on the formulas (it is the `Multi[..]` order of the manifest row).
    pub fn merged(streams: &[MergedStream]) -> Self {
        let mut offs = Vec::with_capacity(streams.len());
        let mut tot = 0usize;
        for s in streams {
            offs.push(tot);
            tot += s.width;
        }
        assert!(tot <= EVENT_COLS, "merged frame holds at most {EVENT_COLS} columns");
        let mut ev: Vec<(i64, usize, usize)> = Vec::new();
        for (si, s) in streams.iter().enumerate() {
            for (k, e) in s.events.iter().enumerate() {
                ev.push((e.0, si, k));
            }
        }
        ev.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
        let mut cur = [0.0f64; EVENT_COLS];
        let rows: Vec<([f64; EVENT_COLS], f32, i64)> = ev
            .iter()
            .map(|&(t, si, k)| {
                for (c, v) in streams[si].events[k].1.iter().enumerate().take(streams[si].width) {
                    cur[offs[si] + c] = *v;
                }
                (cur, (si + 1) as f32, t)
            })
            .collect();
        Self::from_rows(&rows)
    }
}
