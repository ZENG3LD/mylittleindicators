# mli-collector

Live market data collector daemon. Subscribes to WebSocket streams via `digdigdig3::ExchangeHub` and writes events to binary storage for later replay by `mylittleindicators` backtest pipelines.

## Binaries

### `mli-collector` (production daemon)

Reads a TOML config, connects to configured exchanges, subscribes to specified streams, and writes events continuously until Ctrl-C.

```
cargo run -p mli-collector -- collector.toml
```

Config format: see `collector.toml.example` (exchanges, account_types, subscriptions per symbol).

### `mli-collector-smoke` (E2E integration test)

Connects to **all 20 crypto exchanges** known to dig3 v0.2.0, subscribes to all 29 public StreamType variants for each, collects events for a configurable duration, then prints a summary and saves `smoke_data/smoke_report.json`.

```
# 5-minute run (default)
cargo run -p mli-collector --bin mli-collector-smoke -- 300

# 60-second quick check
cargo run -p mli-collector --bin mli-collector-smoke -- 60
```

**Output written to:** `./smoke_data/` (relative to cwd)

## What the smoke test covers

| Category | Detail |
|----------|--------|
| Exchanges | 20 (Binance, Bybit, OKX, KuCoin, Bitget, GateIO, HTX, Deribit, HyperLiquid, BingX, Kraken, MEXC, Coinbase, CryptoCom, Bitfinex, Bitstamp, Gemini, Upbit, dYdX, Lighter) |
| Public StreamType variants | 29 (Ticker, Trade, Orderbook, OrderbookDelta, Kline:1m, MarkPrice, FundingRate, Liquidation, OpenInterest, LongShortRatio, AggTrade, CompositeIndex, MarkPriceKline:1m, IndexPriceKline:1m, PremiumIndexKline:1m, IndexPrice, HistoricalVolatility, InsuranceFund, Basis, OptionGreeks, VolatilityIndex, BlockTrade, AuctionEvent, MarketWarning, OrderbookL3, SettlementEvent, RiskLimit, PredictedFunding, FundingSettlement) |
| Capability check | `hub.capabilities(id)` for WS trades/orderbook/ticker/kline/mark_price/funding_rate; futures-specific streams skip Spot account types automatically |
| Storage | All parsed events written to `./smoke_data/<symbol>/<stream_kind>.bin` via `StorageRoot` |
| Metrics | events per (exchange, account, stream), parse failures, storage writes, connect failures |

## What the smoke test does NOT cover

- Account-private streams: `OrderUpdate`, `BalanceUpdate`, `PositionUpdate` (require API key auth)
- Options chains (Deribit multi-strike subscriptions)
- Data providers (Polygon, Finnhub, etc.) — REST-only, no WS
- Testnet mode

## Smoke report format

`smoke_data/smoke_report.json`:

```json
{
  "duration_secs": 300,
  "connected_exchanges": ["binance", "bybit", ...],
  "connect_failures": { "mexc": "ConnectionTimeout" },
  "subscriptions_active": 142,
  "subscriptions_skipped": 31,
  "subscriptions_failed": 18,
  "total_events": 28734,
  "events_by_exchange": { "binance": 8721, ... },
  "events_by_stream": { "trade": 14502, ... },
  "total_storage_writes": 27890,
  "storage_write_failures": 0,
  "total_parse_failures": 23
}
```

## Honesty notes

- Geo-blocked exchanges (e.g. MEXC from some regions) will fail `connect_full` with a connection error — logged and counted, never silently swallowed.
- Subscribe failures (unsupported stream for a given exchange) are logged at DEBUG and counted in `subscriptions_failed`.
- The 30-second progress timer logs current event/write counts to stdout.
- Exchanges that connect but produce zero events in the window appear with 0 counts in the per-exchange report — not hidden.
