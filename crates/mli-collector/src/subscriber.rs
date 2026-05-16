//! WS subscriber using digdigdig3 `ExchangeHub`.
//!
//! For each exchange in the config:
//! 1. `hub.connect_full(id, account_types, false)` — wires REST + WS.
//! 2. For each subscription, `hub.ws(id, account_type)` → `ws.subscribe(req)`.
//! 3. All WS event streams are merged via `futures_util::stream::select_all`.
//! 4. Events are converted to `TimedEvent` and written via `EventWriter`.
//!
//! # Stream mapping
//!
//! | `StreamType` (dig3)      | `TimedEvent` (mli)        |
//! |--------------------------|---------------------------|
//! | Trade                    | Tick                      |
//! | OrderbookSnapshot        | OrderBook                 |
//! | OrderbookDelta           | OrderbookDelta            |
//! | FundingRate              | Funding                   |
//! | MarkPrice                | MarkPrice                 |
//! | OpenInterestUpdate       | OpenInterest              |
//! | Liquidation              | Liquidation               |
//! | Ticker                   | Ticker                    |
//! | AggTrade                 | AggTrade                  |
//! | LongShortRatio           | LongShortRatio            |
//! | CompositeIndex           | CompositeIndex            |
//! | IndexPrice               | IndexPrice                |
//! | HistoricalVolatility     | HistoricalVolatility      |
//! | InsuranceFund            | InsuranceFund             |
//! | Basis                    | Basis                     |
//! | VolatilityIndex          | VolatilityIndex           |
//! | BlockTrade               | BlockTrade                |
//! | AuctionEvent             | Auction                   |
//! | MarketWarning            | MarketWarning             |
//! | OrderbookL3              | OrderbookL3               |
//! | SettlementEvent          | Settlement                |
//! | RiskLimit                | RiskLimit                 |
//! | PredictedFunding         | PredictedFunding          |
//! | FundingSettlement        | FundingSettlement         |
//! | OptionGreeks             | OptionGreeks              |
//! | Kline                    | Bar                       |
//! | MarkPriceKline etc.      | (dropped)                 |
//! | private events           | (dropped)                 |

use std::sync::Arc;

use anyhow::Result;
use digdigdig3::{
    AccountType, StreamEvent, Symbol, SubscriptionRequest,
    connector_manager::ExchangeHub,
    core::types::TradeSide,
};
use futures_util::{stream::SelectAll, StreamExt};
use mylittleindicators::{
    core::types::Tick,
    data_loader::TimedEvent,
};

use crate::config::CollectorConfig;
use crate::writer::EventWriter;

/// Subscriber wires live WS stream data into binary storage via `ExchangeHub`.
pub struct Subscriber {
    hub: Arc<ExchangeHub>,
    writer: Arc<EventWriter>,
}

impl Subscriber {
    pub fn new(hub: Arc<ExchangeHub>, writer: Arc<EventWriter>) -> Self {
        Self { hub, writer }
    }

    /// Connect all exchanges and start the event loop.
    ///
    /// Merges event streams from all subscribed WS connectors using
    /// `futures_util::stream::select_all` and drives the loop until aborted.
    pub async fn start(&self, config: &CollectorConfig) -> Result<()> {
        // ── Phase 1: connect + subscribe ─────────────────────────────────────
        for ex_cfg in &config.exchanges {
            let Some(id) = ex_cfg.exchange_id() else {
                tracing::warn!("Unknown exchange id {:?}, skipping", ex_cfg.id.0);
                continue;
            };

            let account_types: Vec<AccountType> = ex_cfg.parsed_account_types();
            if account_types.is_empty() {
                tracing::warn!("Exchange {:?}: no valid account_types, skipping", id);
                continue;
            }

            // Connect REST + WS (WS failures are best-effort in connect_full).
            if let Err(e) = self.hub.connect_full(id, &account_types, false).await {
                tracing::error!("connect_full {:?} failed: {e}", id);
                continue;
            }

            // Subscribe each (symbol, account_type, stream_type) triple.
            for sub in &ex_cfg.subscriptions {
                let Some(account_type) = sub.parsed_account_type() else {
                    tracing::warn!("Unknown account_type {:?} in subscription, skipping", sub.account_type.0);
                    continue;
                };
                let Some(stream_type) = sub.parsed_stream_type() else {
                    tracing::warn!("Unknown stream_type {:?} in subscription, skipping", sub.stream_type.0);
                    continue;
                };

                let ws = match self.hub.ws(id, account_type) {
                    Some(ws) => ws,
                    None => {
                        tracing::warn!(
                            "{:?}/{:?}: no WS connector available (exchange may not support this account_type)",
                            id, account_type,
                        );
                        continue;
                    }
                };

                // Connect the WS if not yet connected.
                if let Err(e) = ws.connect(account_type).await {
                    tracing::warn!("{:?}/{:?} ws.connect failed: {e}", id, account_type);
                    continue;
                }

                let symbol = Symbol::with_raw("", "", sub.symbol.clone());
                let req = SubscriptionRequest {
                    symbol,
                    stream_type,
                    account_type,
                    depth: None,
                    update_speed_ms: None,
                };

                if let Err(e) = ws.subscribe(req).await {
                    tracing::warn!("{:?}/{:?}/{}: subscribe failed: {e}", id, account_type, sub.symbol);
                }
            }
        }

        // ── Phase 2: collect event streams ───────────────────────────────────
        let mut streams: SelectAll<
            std::pin::Pin<Box<dyn futures_util::Stream<Item = digdigdig3::core::types::WebSocketResult<StreamEvent>> + Send>>,
        > = SelectAll::new();

        for ex_cfg in &config.exchanges {
            let Some(id) = ex_cfg.exchange_id() else { continue; };
            for at in ex_cfg.parsed_account_types() {
                if let Some(ws) = self.hub.ws(id, at) {
                    streams.push(ws.event_stream());
                }
            }
        }

        if streams.is_empty() {
            tracing::warn!("No WS event streams available; subscriber idle");
            return Ok(());
        }

        tracing::info!("mli-collector: event loop started ({} WS streams)", streams.len());

        // ── Phase 3: event loop ───────────────────────────────────────────────
        while let Some(result) = streams.next().await {
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

        tracing::warn!("All WS event streams ended");
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// StreamEvent → TimedEvent conversion
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a dig3 `PublicTrade` to a mli `Tick`.
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
            // OrderBook has no symbol field; symbol context comes from subscription.
            // Use empty string — writer will filter via StorageRoot which uses symbol
            // directory paths (empty string → root, harmless).
            Some(("".to_string(), TimedEvent::OrderBook(b)))
        }

        StreamEvent::OrderbookDelta(d) => {
            Some(("".to_string(), TimedEvent::OrderbookDelta(d)))
        }

        // ── ticker ────────────────────────────────────────────────────────────
        StreamEvent::Ticker(t) => {
            let symbol = t.symbol.clone();
            Some((symbol, TimedEvent::Ticker(t)))
        }

        // ── kline → Bar ───────────────────────────────────────────────────────
        StreamEvent::Kline(k) => {
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
        StreamEvent::RiskLimit {
            symbol,
            tier,
            max_leverage,
            max_position_value,
            maintenance_margin_rate,
            initial_margin_rate,
            timestamp,
        } => {
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
        StreamEvent::OptionGreeks {
            symbol,
            delta,
            gamma,
            vega,
            theta,
            rho,
            mark_iv,
            bid_iv,
            ask_iv,
            timestamp,
        } => {
            use digdigdig3::core::types::OptionGreeks;
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
            use digdigdig3::core::types::{L3Action, OrderBookSide, OrderSide, OrderbookL3Event};
            let l3_side = if side == OrderSide::Buy {
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

        // ── mark/index/premium price klines — no mli analogue ─────────────────
        StreamEvent::MarkPriceKline { .. }
        | StreamEvent::IndexPriceKline { .. }
        | StreamEvent::PremiumIndexKline { .. } => None,

        // ── private events ─────────────────────────────────────────────────────
        StreamEvent::OrderUpdate(_)
        | StreamEvent::BalanceUpdate(_)
        | StreamEvent::PositionUpdate(_) => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use digdigdig3::core::types::PublicTrade;
    use digdigdig3::StreamType;

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
        use digdigdig3::core::types::{OrderSide, OrderStatus, OrderType, OrderUpdateEvent};
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
    fn account_type_str_parses_correctly() {
        use crate::config::AccountTypeStr;
        assert_eq!(AccountTypeStr("spot".into()).parse(), Some(AccountType::Spot));
        assert_eq!(AccountTypeStr("FuturesCross".into()).parse(), Some(AccountType::FuturesCross));
        assert_eq!(AccountTypeStr("futures_cross".into()).parse(), Some(AccountType::FuturesCross));
        assert_eq!(AccountTypeStr("unknown".into()).parse(), None);
    }

    #[test]
    fn stream_type_str_parses_correctly() {
        use crate::config::StreamTypeStr;
        assert_eq!(StreamTypeStr("fundingrate".into()).parse(), Some(StreamType::FundingRate));
        assert_eq!(StreamTypeStr("funding_rate".into()).parse(), Some(StreamType::FundingRate));
        assert_eq!(StreamTypeStr("liquidation".into()).parse(), Some(StreamType::Liquidation));
        assert_eq!(StreamTypeStr("ticker".into()).parse(), Some(StreamType::Ticker));
    }
}
