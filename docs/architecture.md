# mli — architecture

## Three-layer stack

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Layer 3 — consumers                                                     │
│  mylittlequant (backtest), mylittlechart (rendering), OSS users          │
└──────────────────────────────────────────────────────────────────────────┘
                                  ▲
                                  │  bar indicators (500+) +
                                  │  stream indicators (80+) +
                                  │  detectors (16) + events (26)
                                  │
┌─────────────────────────────────────────────────────────────────────────┐
│  Layer 2 — mylittleindicators (this crate)                               │
│  - bar_indicators/, stream_indicators/, events/, detectors/              │
│  - data_loader/ (TimedEvent, EnrichedHistory, RestFetcher trait)         │
│  - re-exports dig3 types (Bar, Tick, FundingRate, ...) as canonical      │
└──────────────────────────────────────────────────────────────────────────┘
                                  ▲
                                  │  Station::subscribe → SubscriptionHandle
                                  │  ExchangeHub::rest()/ws() for direct REST
                                  │
┌─────────────────────────────────────────────────────────────────────────┐
│  Layer 1 — digdigdig3 + digdigdig3-station (data source)                │
│  - dig3: ExchangeHub, REST/WS connectors (22 TRUSTED CEX/DEX)            │
│  - station: persistence, multiplex, warm-start, auto-heal                │
└──────────────────────────────────────────────────────────────────────────┘
```

## Crate layout

```
mylittleindicators/
├── src/
│   ├── core/             — primitive types (Bar, Tick) + value/selector types
│   ├── data_loader/      — TimedEvent enum, EnrichedHistory, RestFetcher
│   │   ├── stream_kind.rs       — enum StreamKind (27 variants)
│   │   ├── timed_event.rs       — enum TimedEvent (27 variants, mirrors dig3 StreamEvent)
│   │   ├── enriched_history.rs  — { bars, events } timeline
│   │   ├── enriched_loader.rs   — EnrichedDataLoader::load (async REST)
│   │   ├── rest_fetcher.rs      — async trait RestFetcher
│   │   ├── exchange_hub_fetcher.rs — ExchangeHubFetcher: hub.rest() + 9 typed methods
│   │   ├── storage.rs           — StorageRoot (legacy binary log; Station owns going forward)
│   │   ├── timeline_merger.rs   — merge_sorted, bar_boundaries, align_to_bars
│   │   └── data_source.rs       — enum DataSource (Binary/Json/Rest/Mixed)
│   ├── bar_indicators/   — 500+ bar-based indicators (price, volume, OI, funding, ...)
│   ├── stream_indicators/ — 80+ stream-based indicators (L2, tick, funding, OI, liq, ...)
│   ├── events/           — 26 event primitives (Divergence, Zigzag, Cross, CandlePattern, ...)
│   ├── detectors/        — 16 detector primitives (consume bars → emit events)
│   ├── catalog/          — registry: data + visual descriptors per indicator
│   └── lib.rs
└── crates/
    └── mli-collector/    — daemon + smoke harness
        ├── src/
        │   ├── main.rs        — daemon: Station consumer + signal handler
        │   ├── config.rs      — TOML → SubscriptionSet
        │   ├── lib.rs         — re-exports config
        │   └── bin/smoke.rs   — diagnostic harness: REST + WS audit across 22 CEX/DEX
        └── collector.toml.example
```

## Layer 1 — dig3 + Station

### dig3 (`digdigdig3` crate, pinned `=0.3.4`)

Pure connector library. Owns:
- `ExchangeHub` — single entry point; `connect_full`, `rest()`, `ws()`, `capabilities()`, `shutdown()`.
- 22 TRUSTED CEX + 4 DEX connectors covering ~95% of crypto market.
- `StreamEvent` enum (~30 variants) over WS.
- `SymbolInput<'_>` per-call: `Raw(&str)` or `Canonical(&Symbol)` — connector resolves.
- `ValidationStamp` empirical capability truth — `connect_full_validated` rejects non-validated.

### digdigdig3-station (`digdigdig3-station`, same workspace pin)

High-level builder over `ExchangeHub`. Owns 9 unified `DataPoint` classes:
`TradePoint`, `AggTradePoint`, `BarPoint`, `TickerPoint`, `ObSnapshotPoint`,
`MarkPricePoint`, `FundingRatePoint`, `OpenInterestPoint`, `LiquidationPoint`.

API:
```rust
let station = Station::builder()
    .storage_root("./collector_data")
    .persistence(PersistenceConfig::on())
    .warm_start(100)
    .build().await?;

let mut handle = station.subscribe(
    SubscriptionSet::new()
        .add(ExchangeId::Binance, "BTCUSDT", AccountType::FuturesCross,
             [Stream::FundingRate, Stream::Liquidation, Stream::Kline(KlineInterval::new("1m"))])
).await?;

while let Some(ev) = handle.recv().await { /* ... */ }
```

What Station handles automatically:
- WS multiplex per `SeriesKey = (exchange, account, symbol, kind)` — N consumers share one WS subscription.
- Warm-start: emit last-N points from disk (or REST backfill) BEFORE live stream.
- Persistence: `<storage_root>/<kind>/<exchange>/<account>/<symbol>/<YYYY-MM-DD>.dat` + sparse `.idx`. Day rotation.
- Auto-heal on disconnect (kline only): REST `get_klines` → `upsert_by_ts` → unsubscribe + subscribe → re-attach.
- Consumer ref-counting: dropping `SubscriptionHandle` decrements; last drop shuts down multiplexer.

## Layer 2 — mli

### data_loader

Foundation for indicator dispatch and historical data loading:

- **`TimedEvent`** — mli's abstraction over `dig3::StreamEvent`. 27 variants
  covering the 9 Station-supported classes + 18 exotic streams (CompositeIndex,
  OptionGreeks, Basis, Settlement, MarketWarning, RiskLimit, OrderbookL3, etc.).
- **`StreamKind`** — discriminator enum used as routing key in EnrichedHistory.
- **`EnrichedHistory { bars: Vec<Bar>, events: Vec<TimedEvent> }`** — timestamp-merged timeline; indicators consume via consumer traits.
- **`RestFetcher` trait** (async) — abstract REST source. Implementation: `ExchangeHubFetcher` over `hub.rest()`.
- **`EnrichedDataLoader`** — orchestrates REST fetching across all enabled `StreamKind`s and merges into `EnrichedHistory`.

### Indicators

- **`bar_indicators/`** — 500+ canonical indicators (MA, RSI, MACD, Bollinger,
  + funding/OI/mark price/ticker advanced + composites).
- **`stream_indicators/`** — 80+ stream-based indicators (tick imbalance,
  liquidation density, OI flow, L2 microstructure, ...).
- **`events/`** — 26 event primitives (Divergence, Zigzag, Cross, Breakout, CandlePattern, ...).
- **`detectors/`** — 16 stateful detectors consuming bar updates → emitting events.

### Consumer traits

Each indicator implements one or more consumer traits:
- `TickConsumer`, `BarConsumer`, `OrderBookConsumer`, `OrderbookDeltaConsumer`
- `FundingRateConsumer`, `LiquidationConsumer`, `OpenInterestConsumer`
- `TickerConsumer`, `MarkPriceConsumer`, ...

Dispatch is by `StreamKind`: incoming `TimedEvent::X` routes to all indicators
implementing `XConsumer`.

## Layer 3 — consumers

- **mylittlequant (mlq)** — private backtester + cartesian optimizer.
  Uses mli as `mlq-indicators` alias. Pinned tightly; mli MUST NOT break
  `IndicatorInstance::{create, update_bar, is_ready, extract}` or
  `BarIndicatorId/IndicatorValue/OutputSelector` contracts.
- **mylittlechart (mlc)** — chart engine. Internal `zengeld-terminal-indicators`
  crate; migration to mli deferred (mlc stays on its own indicator crate).
- **OSS users** — consume via `bar_indicators::*` + `stream_indicators::*` re-exports.

## mli-collector daemon

**Role**: long-running process that subscribes to live streams via Station and
persists to local binary storage. Used as data source for mlq backtests
(replay from disk) and mli development testing.

**Architecture** (post-Station migration):

```
mli-collector
    │
    ├── reads collector.toml
    │
    ├── builds Station::builder()
    │       .storage_root(...)
    │       .persistence(on)
    │       .warm_start(N)
    │
    ├── builds SubscriptionSet from config
    │
    └── station.subscribe() → handle.recv() loop
```

**Supported stream types** (9 — Station kinds):
`trade, agg_trade, ticker, orderbook, mark_price, funding_rate,
open_interest, liquidation, kline (1m/5m/1h/...)`

**NOT supported by daemon** — exotic streams from dig3 StreamEvent that
Station doesn't have `DataPoint` impl for:
`composite_index, option_greeks, basis, settlement_event, market_warning,
risk_limit, predicted_funding, funding_settlement, orderbook_l3,
mark_price_kline, index_price_kline, premium_index_kline, auction_event,
volatility_index, insurance_fund, block_trade, historical_volatility,
index_price`

For audit of those — use `mli-collector-smoke` binary which talks to
`ExchangeHub` directly.

## mli-collector-smoke (diagnostic harness)

E2E audit binary. Not part of normal collection flow.

**What it does**:
1. **Phase 1 (REST)** — calls 13 REST endpoints across all 22 CEX/DEX in
   parallel via JoinSet, 10s per-call timeout.
2. **Phase 2 (WS)** — subscribes to all 27 stream types across all relevant
   account types per exchange. Counts events for ~180s.
3. **Classifies** WS results: `working` (events received), `dropped` (conn ended),
   `unsupported_by_exchange` (returns NotSupported eagerly), `failed`
   (subscribe error), `silent` (subscribed OK but 0 events).
4. **Writes** `smoke_data/smoke_report.json` with full matrix per (exchange ×
   account × stream).

**Used for**: generating bug specs for dig3 team. See
`digdigdig3/docs/plans/smoke_v8_findings_spec.md` for v8 output.

## Data flow at runtime

### Daemon (live collection)
```
Exchange WS  →  dig3 connector  →  StreamEvent
                                     │
                                     ▼
                    Station multiplexer (per-SeriesKey broadcast)
                                     │
                                     ├─→ DataPoint::from_stream_event
                                     ├─→ DiskStore::append (binary .dat)
                                     └─→ broadcast Event to consumer handles
                                                              │
                                                              ▼
                                                  mli-collector recv loop
```

### Indicator pipeline (offline backtest)
```
collector .dat files (or live Station handle)
        │
        ▼
EnrichedDataLoader → EnrichedHistory { bars, events }
        │
        ▼
indicator dispatch (StreamKind → matching XConsumer impls)
        │
        ▼
IndicatorValue → strategy / backtest engine
```

## Key invariants

1. **All exchange access goes through `ExchangeHub`** — no direct connector
   struct imports. mli `data_loader/exchange_hub_fetcher.rs` enforces this.
2. **dig3 is single source of truth for exchange types** — `Bar`, `Tick`,
   `FundingRate`, `Ticker`, `MarkPrice`, `OpenInterest`, `Liquidation`,
   `LongShortRatio`, `OrderBook`, ... mli re-exports them rather than defining
   parallel types.
3. **TimedEvent mirrors dig3 StreamEvent** — adding a new stream type means
   adding to BOTH `dig3::StreamEvent` AND `mli::TimedEvent` AND `mli::StreamKind`.
4. **mlq contract is immovable** — `mlq-core::indicators::*`,
   `mlq-warmup::cache::*`, `BarIndicatorId`, `IndicatorValue`, `OutputSelector`
   may NOT change shape without coordinated mlq update.
5. **Persistence belongs to Station, not mli** — legacy `data_loader/storage.rs`
   (`StorageRoot`) exists for backward compatibility with pre-Station collector
   data; new data goes through Station's `.dat+.idx` format.

## Version pins (current)

| Crate                  | Pin       |
|------------------------|-----------|
| digdigdig3             | `=0.3.4`  |
| digdigdig3-station     | `path = ../../digdigdig3/crates/digdigdig3-station` |
| mylittleindicators     | `0.1.0`   |
| mli-collector          | `0.1.0`   |
