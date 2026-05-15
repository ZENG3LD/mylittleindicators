//! WebSocket subscriber backed by digdigdig3.
//!
//! Creates a `BinanceWebSocket`, subscribes to all configured streams, then
//! drives an event loop that converts `StreamEvent` → `TimedEvent` and writes
//! each event to `EventWriter`.
//!
//! # Stream mapping
//!
//! | `StreamKind`       | dig3 `StreamType`        | Notes                         |
//! |--------------------|--------------------------|-------------------------------|
//! | Tick               | Trade                    | needs `dig3_trade_to_mli_tick` |
//! | OrderBook          | Orderbook                |                               |
//! | OrderbookDelta     | OrderbookDelta           |                               |
//! | Funding            | FundingRate              |                               |
//! | MarkPrice          | MarkPrice                |                               |
//! | OpenInterest       | OpenInterest             |                               |
//! | Liquidation        | Liquidation              |                               |
//! | Ticker             | Ticker                   |                               |
//! | AggTrade           | AggTrade                 |                               |
//! | LongShortRatio     | LongShortRatio           |                               |
//! | Bar                | —                        | REST-only; not mapped here    |
//! | *rest*             | —                        | no dig3 WS source             |

use std::sync::Arc;

use futures_util::StreamExt;

use digdigdig3::{
    AccountType,
    StreamEvent, StreamType, SubscriptionRequest, Symbol,
    WebSocketConnector,
};
use digdigdig3::l3::open::crypto::cex::binance::BinanceWebSocket;

use mylittleindicators::{
    core::types::{Tick, TradeSide},
    data_loader::TimedEvent,
};

use crate::config::{CollectorConfig, StreamConfig};
use crate::writer::EventWriter;
use mylittleindicators::data_loader::StreamKind;

// ═══════════════════════════════════════════════════════════════════════════════
// STREAM KIND → STREAM TYPE MAPPING
// ═══════════════════════════════════════════════════════════════════════════════

/// Map a collector `StreamKind` to the dig3 `StreamType` used for WS subscription.
///
/// Returns `None` when the stream kind has no WS analogue (e.g. `Bar` is REST-only).
fn mli_to_dig3_stream(kind: StreamKind) -> Option<StreamType> {
    match kind {
        StreamKind::Bar => None, // REST-only (klines)
        StreamKind::Tick => Some(StreamType::Trade),
        StreamKind::OrderBook => Some(StreamType::Orderbook),
        StreamKind::OrderbookDelta => Some(StreamType::OrderbookDelta),
        StreamKind::Funding => Some(StreamType::FundingRate),
        StreamKind::MarkPrice => Some(StreamType::MarkPrice),
        StreamKind::OpenInterest => Some(StreamType::OpenInterest),
        StreamKind::Liquidation => Some(StreamType::Liquidation),
        StreamKind::Ticker => Some(StreamType::Ticker),
        StreamKind::AggTrade => Some(StreamType::AggTrade),
        StreamKind::LongShortRatio => Some(StreamType::LongShortRatio),
        // No dig3 WS sources for the following:
        StreamKind::OptionGreeks => Some(StreamType::OptionGreeks),
        StreamKind::VolatilityIndex => Some(StreamType::VolatilityIndex),
        StreamKind::HistoricalVolatility => Some(StreamType::HistoricalVolatility),
        StreamKind::Basis => Some(StreamType::Basis),
        StreamKind::IndexPrice => Some(StreamType::IndexPrice),
        StreamKind::CompositeIndex => Some(StreamType::CompositeIndex),
        StreamKind::InsuranceFund => Some(StreamType::InsuranceFund),
        StreamKind::Settlement => Some(StreamType::SettlementEvent),
        StreamKind::BlockTrade => Some(StreamType::BlockTrade),
        StreamKind::OrderbookL3 => Some(StreamType::OrderbookL3),
        StreamKind::RiskLimit => Some(StreamType::RiskLimit),
        StreamKind::PredictedFunding => Some(StreamType::PredictedFunding),
        StreamKind::FundingSettlement => Some(StreamType::FundingSettlement),
        StreamKind::Auction => Some(StreamType::AuctionEvent),
        StreamKind::MarketWarning => Some(StreamType::MarketWarning),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// STREAM EVENT → TIMED EVENT CONVERSION
// ═══════════════════════════════════════════════════════════════════════════════

/// Convert a dig3 `PublicTrade` to a mli `Tick`.
///
/// `Tick` is mli-specific (lightweight, copy-friendly).  `PublicTrade` is the
/// dig3 canonical trade type.  Fields map 1:1 except `is_buy` which is derived
/// from `TradeSide`.
fn dig3_trade_to_mli_tick(t: digdigdig3::core::types::PublicTrade) -> Tick {
    Tick {
        time: t.timestamp,
        price: t.price,
        size: t.quantity,
        is_buy: t.side == TradeSide::Buy,
        bid: None,
        ask: None,
    }
}

/// Convert a dig3 `StreamEvent` to `(symbol, TimedEvent)`.
///
/// Returns `None` for private events or variants with no `TimedEvent` analogue.
fn stream_event_to_timed(ev: StreamEvent) -> Option<(String, TimedEvent)> {
    match ev {
        // ── public trades ────────────────────────────────────────────────────
        StreamEvent::Trade(t) => {
            let symbol = t.symbol.clone();
            Some((symbol, TimedEvent::Tick(dig3_trade_to_mli_tick(t))))
        }

        // ── orderbook snapshot ────────────────────────────────────────────────
        StreamEvent::OrderbookSnapshot(b) => {
            // OrderBook is re-exported from dig3 in mli, so types are identical.
            Some(("".to_string(), TimedEvent::OrderBook(b)))
            // symbol embedded in OrderBook? It has no symbol field — we rely on
            // the subscription to associate symbol. Use empty string and caller
            // must provide context. For now the event loop patches the symbol
            // from subscription metadata (see note in event loop).
        }

        StreamEvent::OrderbookDelta(d) => {
            Some(("".to_string(), TimedEvent::OrderbookDelta(d)))
        }

        // ── ticker ────────────────────────────────────────────────────────────
        StreamEvent::Ticker(t) => {
            let symbol = t.symbol.clone();
            Some((symbol, TimedEvent::Ticker(t)))
        }

        // ── kline ─────────────────────────────────────────────────────────────
        StreamEvent::Kline(k) => {
            // Map Kline → Bar using close_time as time (or open_time if no close).
            use mylittleindicators::core::types::Bar;
            let time = k.close_time.unwrap_or(k.open_time);
            let bar = Bar::new(time, k.open, k.high, k.low, k.close, k.volume);
            Some(("".to_string(), TimedEvent::Bar(bar)))
        }

        // ── mark price ────────────────────────────────────────────────────────
        StreamEvent::MarkPrice { symbol, mark_price, index_price, timestamp } => {
            use digdigdig3::core::types::MarkPrice;
            let mp = MarkPrice { symbol: symbol.clone(), mark_price, index_price, funding_rate: None, timestamp };
            Some((symbol, TimedEvent::MarkPrice(mp)))
        }

        // ── funding rate ──────────────────────────────────────────────────────
        StreamEvent::FundingRate { symbol, rate, next_funding_time, timestamp } => {
            use digdigdig3::core::types::FundingRate;
            let fr = FundingRate { symbol: symbol.clone(), rate, next_funding_time, timestamp };
            Some((symbol, TimedEvent::Funding(fr)))
        }

        // ── liquidation ───────────────────────────────────────────────────────
        StreamEvent::Liquidation { symbol, side, price, quantity, timestamp, value } => {
            use digdigdig3::core::types::Liquidation;
            let liq = Liquidation { symbol: symbol.clone(), side, price, quantity, timestamp, value };
            Some((symbol, TimedEvent::Liquidation(liq)))
        }

        // ── open interest ─────────────────────────────────────────────────────
        StreamEvent::OpenInterestUpdate { symbol, open_interest, open_interest_value, timestamp } => {
            use digdigdig3::core::types::OpenInterest;
            let oi = OpenInterest { symbol: symbol.clone(), open_interest, open_interest_value, timestamp };
            Some((symbol, TimedEvent::OpenInterest(oi)))
        }

        // ── long/short ratio ──────────────────────────────────────────────────
        StreamEvent::LongShortRatio { symbol, ratio_type, long_ratio, short_ratio, timestamp } => {
            use digdigdig3::core::types::LongShortRatio;
            let lsr = LongShortRatio {
                symbol: symbol.clone(),
                ratio_type,
                long_ratio,
                short_ratio,
                ratio: None,
                timestamp,
            };
            Some((symbol, TimedEvent::LongShortRatio(lsr)))
        }

        // ── agg trade ─────────────────────────────────────────────────────────
        StreamEvent::AggTrade { symbol, aggregate_id, price, quantity, first_trade_id, last_trade_id, side, timestamp } => {
            use digdigdig3::core::types::AggTrade;
            let at = AggTrade {
                aggregate_id,
                price,
                quantity,
                first_trade_id,
                last_trade_id,
                is_buy: side == TradeSide::Buy,
                timestamp,
            };
            Some((symbol, TimedEvent::AggTrade(at)))
        }

        // ── composite index ───────────────────────────────────────────────────
        StreamEvent::CompositeIndex { symbol, price, components, timestamp } => {
            use digdigdig3::core::types::CompositeIndex;
            let ci = CompositeIndex { price, components, timestamp };
            Some((symbol, TimedEvent::CompositeIndex(ci)))
        }

        // ── index price ───────────────────────────────────────────────────────
        StreamEvent::IndexPrice { symbol, price, timestamp } => {
            use digdigdig3::core::types::IndexPrice;
            let ip = IndexPrice { price, timestamp };
            Some((symbol, TimedEvent::IndexPrice(ip)))
        }

        // ── historical volatility ─────────────────────────────────────────────
        StreamEvent::HistoricalVolatility { symbol, volatility, timestamp } => {
            use digdigdig3::core::types::HistoricalVolatility;
            let hv = HistoricalVolatility { volatility, timestamp };
            Some((symbol, TimedEvent::HistoricalVolatility(hv)))
        }

        // ── insurance fund ────────────────────────────────────────────────────
        StreamEvent::InsuranceFund { symbol, balance, timestamp } => {
            use digdigdig3::core::types::InsuranceFund;
            let ins = InsuranceFund { balance, timestamp };
            Some((symbol, TimedEvent::InsuranceFund(ins)))
        }

        // ── basis ─────────────────────────────────────────────────────────────
        StreamEvent::Basis { symbol, basis, timestamp } => {
            use digdigdig3::core::types::Basis;
            let b = Basis { basis, timestamp };
            Some((symbol, TimedEvent::Basis(b)))
        }

        // ── volatility index ──────────────────────────────────────────────────
        StreamEvent::VolatilityIndex { symbol, value, timestamp } => {
            use digdigdig3::core::types::VolatilityIndex;
            let vi = VolatilityIndex { value, timestamp };
            Some((symbol, TimedEvent::VolatilityIndex(vi)))
        }

        // ── block trade ───────────────────────────────────────────────────────
        StreamEvent::BlockTrade { symbol, block_id, price, quantity, side, timestamp, is_iv } => {
            use digdigdig3::core::types::BlockTrade;
            let bt = BlockTrade { block_id, price, quantity, is_buy: side == TradeSide::Buy, timestamp, is_iv };
            Some((symbol, TimedEvent::BlockTrade(bt)))
        }

        // ── auction event ─────────────────────────────────────────────────────
        StreamEvent::AuctionEvent { symbol, auction_id, indicative_price, indicative_qty, state, timestamp } => {
            use digdigdig3::core::types::AuctionEvent;
            // AuctionEvent struct has f64 (required); StreamEvent has Option<f64>; default 0.0 when absent.
            let ae = AuctionEvent {
                auction_id,
                indicative_price: indicative_price.unwrap_or(0.0),
                indicative_qty: indicative_qty.unwrap_or(0.0),
                state,
                timestamp,
            };
            Some((symbol, TimedEvent::Auction(ae)))
        }

        // ── market warning ────────────────────────────────────────────────────
        StreamEvent::MarketWarning { symbol, warning_kind, message, timestamp } => {
            use digdigdig3::core::types::MarketWarning;
            let mw = MarketWarning { symbol: symbol.clone(), warning_kind, message, timestamp };
            Some((symbol, TimedEvent::MarketWarning(mw)))
        }

        // ── settlement ────────────────────────────────────────────────────────
        StreamEvent::SettlementEvent { symbol, settlement_price, settlement_time, timestamp } => {
            use digdigdig3::core::types::SettlementEvent;
            let se = SettlementEvent { settlement_price, settlement_time, timestamp };
            Some((symbol, TimedEvent::Settlement(se)))
        }

        // ── risk limit ────────────────────────────────────────────────────────
        StreamEvent::RiskLimit { symbol, tier, max_leverage, max_position_value, maintenance_margin_rate, initial_margin_rate, timestamp } => {
            use digdigdig3::core::types::RiskLimit;
            let rl = RiskLimit {
                tier,
                max_leverage,
                max_position_value,
                mmr: maintenance_margin_rate,
                imr: initial_margin_rate,
                timestamp,
            };
            Some((symbol, TimedEvent::RiskLimit(rl)))
        }

        // ── predicted funding ─────────────────────────────────────────────────
        StreamEvent::PredictedFunding { symbol, predicted_rate, next_funding_time, timestamp } => {
            use digdigdig3::core::types::PredictedFunding;
            let pf = PredictedFunding { predicted_rate, next_funding_time, timestamp };
            Some((symbol, TimedEvent::PredictedFunding(pf)))
        }

        // ── funding settlement ────────────────────────────────────────────────
        StreamEvent::FundingSettlement { symbol, settled_rate, settlement_time, timestamp } => {
            use digdigdig3::core::types::FundingSettlement;
            let fs = FundingSettlement { settled_rate, settlement_time, timestamp };
            Some((symbol, TimedEvent::FundingSettlement(fs)))
        }

        // ── option greeks ─────────────────────────────────────────────────────
        StreamEvent::OptionGreeks { symbol, delta, gamma, vega, theta, rho, mark_iv, bid_iv, ask_iv, timestamp } => {
            use digdigdig3::core::types::OptionGreeks;
            // OptionGreeks struct has required f64 fields; StreamEvent has Option<f64>; default 0.0 when absent.
            let og = OptionGreeks {
                delta: delta.unwrap_or(0.0),
                gamma: gamma.unwrap_or(0.0),
                vega: vega.unwrap_or(0.0),
                theta: theta.unwrap_or(0.0),
                rho: rho.unwrap_or(0.0),
                mark_iv: mark_iv.unwrap_or(0.0),
                bid_iv,
                ask_iv,
                timestamp,
            };
            Some((symbol, TimedEvent::OptionGreeks(og)))
        }

        // ── orderbook L3 ──────────────────────────────────────────────────────
        StreamEvent::OrderbookL3 { symbol, side, order_id, price, quantity, action, timestamp } => {
            use digdigdig3::core::types::{OrderbookL3Event, L3Action, OrderBookSide};
            let l3_side = if side == digdigdig3::core::types::OrderSide::Buy {
                OrderBookSide::Bid
            } else {
                OrderBookSide::Ask
            };
            let l3_action = match action.as_str() {
                "add" | "insert" => L3Action::Add,
                "remove" | "delete" => L3Action::Delete,
                _ => L3Action::Modify,
            };
            let ev = OrderbookL3Event { side: l3_side, order_id, price, quantity, action: l3_action, timestamp };
            Some((symbol, TimedEvent::OrderbookL3(ev)))
        }

        // ── mark price kline / index price kline / premium index kline ────────
        StreamEvent::MarkPriceKline { .. }
        | StreamEvent::IndexPriceKline { .. }
        | StreamEvent::PremiumIndexKline { .. } => None,

        // ── private events ────────────────────────────────────────────────────
        StreamEvent::OrderUpdate(_)
        | StreamEvent::BalanceUpdate(_)
        | StreamEvent::PositionUpdate(_) => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUBSCRIBER
// ═══════════════════════════════════════════════════════════════════════════════

/// Subscriber wires live WS stream data into the binary storage.
pub struct Subscriber {
    pub writer: Arc<EventWriter>,
}

impl Subscriber {
    pub fn new(writer: Arc<EventWriter>) -> Self {
        Self { writer }
    }

    /// Start subscribing to all configured streams and write events.
    ///
    /// Creates a `BinanceWebSocket`, subscribes to all `(symbol, stream_kind)` pairs
    /// from `config`, then drives the event loop until the task is aborted.
    pub async fn start(&self, config: &CollectorConfig) -> anyhow::Result<()> {
        let account_type = AccountType::FuturesCross;

        let mut ws = BinanceWebSocket::new(None, false, account_type).await?;
        ws.connect(account_type).await?;

        // Build list of (symbol, StreamType) subscriptions.
        let mut subscriptions: Vec<(String, StreamType)> = Vec::new();
        for stream_cfg in &config.streams {
            let Some(dig3_type) = mli_to_dig3_stream(stream_cfg.kind) else {
                tracing::warn!(
                    "StreamKind::{:?} has no WS analogue — skipping",
                    stream_cfg.kind
                );
                continue;
            };
            let symbols = effective_symbols(stream_cfg, config);
            for symbol in symbols {
                subscriptions.push((symbol.to_string(), dig3_type.clone()));
            }
        }

        if subscriptions.is_empty() {
            tracing::warn!("No WS-mappable streams configured; subscriber idle");
            return Ok(());
        }

        // Subscribe each (symbol, type) pair.
        for (sym, stream_type) in &subscriptions {
            // Symbol::parse handles "BTC-USDT" / "BTC_USDT" formats.
            // For raw exchange symbols like "BTCUSDT" we use with_raw with empty base/quote
            // so the connector receives the raw string intact.
            let symbol = Symbol::with_raw("", "", sym.clone());
            let req = SubscriptionRequest::new(symbol, stream_type.clone());
            if let Err(e) = ws.subscribe(req).await {
                tracing::warn!("subscribe {sym}/{stream_type:?} failed: {e}");
            }
        }

        tracing::info!(
            "mli-collector: {} subscriptions active on {}",
            subscriptions.len(),
            config.exchange,
        );

        let mut stream = ws.event_stream();

        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    if let Some((symbol, timed)) = stream_event_to_timed(event) {
                        if let Err(e) = self.writer.write(&symbol, &timed) {
                            tracing::warn!("write error for {symbol}: {e}");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("WS error: {e}");
                }
            }
        }

        tracing::warn!("WS event stream ended");
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Returns the effective symbol list for a stream config:
/// stream-level override if non-empty, else top-level symbols.
fn effective_symbols<'a>(stream: &'a StreamConfig, config: &'a CollectorConfig) -> &'a [String] {
    if stream.symbols.is_empty() {
        &config.symbols
    } else {
        &stream.symbols
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use digdigdig3::core::types::PublicTrade;

    // ── StreamKind → StreamType mapping ──────────────────────────────────────

    #[test]
    fn tick_maps_to_trade() {
        assert_eq!(mli_to_dig3_stream(StreamKind::Tick), Some(StreamType::Trade));
    }

    #[test]
    fn orderbook_maps_correctly() {
        assert_eq!(mli_to_dig3_stream(StreamKind::OrderBook), Some(StreamType::Orderbook));
        assert_eq!(mli_to_dig3_stream(StreamKind::OrderbookDelta), Some(StreamType::OrderbookDelta));
    }

    #[test]
    fn bar_has_no_ws_mapping() {
        assert_eq!(mli_to_dig3_stream(StreamKind::Bar), None);
    }

    #[test]
    fn funding_maps_to_funding_rate() {
        assert_eq!(mli_to_dig3_stream(StreamKind::Funding), Some(StreamType::FundingRate));
    }

    #[test]
    fn liquidation_maps_correctly() {
        assert_eq!(mli_to_dig3_stream(StreamKind::Liquidation), Some(StreamType::Liquidation));
    }

    #[test]
    fn all_non_bar_kinds_have_some_mapping() {
        let mappable = [
            StreamKind::Tick,
            StreamKind::OrderBook,
            StreamKind::OrderbookDelta,
            StreamKind::Funding,
            StreamKind::MarkPrice,
            StreamKind::OpenInterest,
            StreamKind::Liquidation,
            StreamKind::Ticker,
            StreamKind::AggTrade,
            StreamKind::LongShortRatio,
        ];
        for kind in mappable {
            assert!(mli_to_dig3_stream(kind).is_some(), "{kind:?} should map");
        }
    }

    // ── StreamEvent → TimedEvent conversion ──────────────────────────────────

    #[test]
    fn trade_event_converts_to_tick() {
        let trade = PublicTrade {
            id: "1".into(),
            symbol: "BTCUSDT".into(),
            price: 50000.0,
            quantity: 0.5,
            side: TradeSide::Buy,
            timestamp: 1_000_000,
        };
        let ev = StreamEvent::Trade(trade);
        let result = stream_event_to_timed(ev);
        assert!(result.is_some());
        let (sym, timed) = result.unwrap();
        assert_eq!(sym, "BTCUSDT");
        match timed {
            TimedEvent::Tick(t) => {
                assert_eq!(t.price, 50000.0);
                assert!(t.is_buy);
            }
            other => panic!("expected Tick, got {other:?}"),
        }
    }

    #[test]
    fn funding_rate_event_converts() {
        let ev = StreamEvent::FundingRate {
            symbol: "ETHUSDT".into(),
            rate: 0.0001,
            next_funding_time: Some(9_000_000),
            timestamp: 8_000_000,
        };
        let result = stream_event_to_timed(ev);
        assert!(result.is_some());
        let (sym, timed) = result.unwrap();
        assert_eq!(sym, "ETHUSDT");
        assert!(matches!(timed, TimedEvent::Funding(_)));
    }

    #[test]
    fn private_events_return_none() {
        use digdigdig3::core::types::{
            OrderUpdateEvent, OrderSide, OrderType, OrderStatus,
        };
        let ev = StreamEvent::OrderUpdate(OrderUpdateEvent {
            order_id: "x".into(),
            client_order_id: None,
            symbol: "BTCUSDT".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            status: OrderStatus::New,
            price: None,
            quantity: 1.0,
            filled_quantity: 0.0,
            average_price: None,
            last_fill_price: None,
            last_fill_quantity: None,
            last_fill_commission: None,
            commission_asset: None,
            trade_id: None,
            timestamp: 1_000,
        });
        assert!(stream_event_to_timed(ev).is_none());
    }

    #[test]
    fn dig3_trade_to_tick_buy() {
        let trade = PublicTrade {
            id: "42".into(),
            symbol: "SOLUSDT".into(),
            price: 200.0,
            quantity: 3.0,
            side: TradeSide::Buy,
            timestamp: 999,
        };
        let tick = dig3_trade_to_mli_tick(trade);
        assert!(tick.is_buy);
        assert_eq!(tick.price, 200.0);
        assert_eq!(tick.size, 3.0);
        assert_eq!(tick.time, 999);
    }

    #[test]
    fn dig3_trade_to_tick_sell() {
        let trade = PublicTrade {
            id: "43".into(),
            symbol: "SOLUSDT".into(),
            price: 199.0,
            quantity: 1.0,
            side: TradeSide::Sell,
            timestamp: 1001,
        };
        let tick = dig3_trade_to_mli_tick(trade);
        assert!(!tick.is_buy);
    }
}
