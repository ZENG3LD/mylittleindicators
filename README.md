# mylittleindicators

Multi-stream financial indicators library for Rust. **559 bar indicators + 21 event primitives** across 35 categories. Consumes the full surface of [`digdigdig3`](https://github.com/ZENG3LD/digdigdig3) exchange connectors — OHLCV bars, ticks, orderbook (snapshot + delta), funding, mark price, open interest, liquidations, ticker, agg trades, plus 12+ extended stream types (option greeks, basis, index price, settlement, block trades, L3, etc.).

## What's inside

- **`bar_indicators/`** — 559 single-stream indicators (averages, momentum, volatility, channels, divergence, kalman, signal processing, statistics, candles, levels, accumulation, adaptive, regression, chaos, entropy, trend, trend-stops, zigzag, ratio, position, statistical scoring, sentiment, ticker, mark price, funding, OI, liquidations, book, book advanced, clusters, hybrid tick+book, microstructure, tick advanced, greeks, index basis, volatility advanced, stress, risk funding, composites).
- **`events/`** — 21 event primitives (swing detection, divergence, candle patterns, line cross, price-line cross, pivot, threshold, volume events, volatility regime, regime gate, relative position, statistical wick, confluence, BOS, FVG, oscillator+divergence, oscillator+volume weight, direction detector, and 3 cross-asset events: beta, cointegration proxy, relative strength).
- **`catalog/`** — `IndicatorSignature` + `EventSignature` with `input_stream` / `aux_streams` routing metadata. Used by codegen, UI rendering, live validators.
- **`data_loader/`** — `StreamKind`, `TimedEvent`, `EnrichedHistory`, `RestFetcher` abstraction over dig3.

## Two parallel factories

```rust
use mylittleindicators::bar_indicators::{
    bar_indicator_id::BarIndicatorId,
    instance_factory::{IndicatorConfig, IndicatorInstance},
};

let cfg = IndicatorConfig::new(BarIndicatorId::Rsi, "rsi".into(), vec![14]);
let mut rsi = IndicatorInstance::create(&cfg)?;
let v = rsi.update_bar(open, high, low, close, volume, Some(ts_ms));

use mylittleindicators::events::{EventId, EventConfig, EventInstance};

let cfg = EventConfig::new(EventId::SwingDetection, "swing".into())
    .with_string_param("mode", "percent")
    .with_param("threshold_pct", 0.5);
let mut swing = EventInstance::create(&cfg)?;
let v = swing.update_bar(o, h, l, c, vol);
```

Both factories follow the same shape: `Id` enum + `Config` builder + `Instance::create()` + `update_*` dispatch + `value()` / `is_ready()` / `reset()`.

## Stream coverage

Every `IndicatorSignature` / `EventSignature` declares which streams it consumes:

```rust
pub struct IndicatorSignature {
    pub input_stream: StreamKind,            // primary stream
    pub aux_streams: &'static [StreamKind],  // secondary streams (multi-stream composites)
    // ... + role_kind, output_kind, params, ...
}
```

`StreamKind` mirrors dig3 exactly (Bar, Tick, OrderBook, OrderbookDelta, Funding, MarkPrice, OpenInterest, Liquidation, Ticker, AggTrade, LongShortRatio, OptionGreeks, VolatilityIndex, HistoricalVolatility, Basis, IndexPrice, CompositeIndex, InsuranceFund, Settlement, BlockTrade, OrderbookL3, RiskLimit, PredictedFunding, FundingSettlement, MarketWarning, AuctionEvent, MarkPriceKline, IndexPriceKline, PremiumIndexKline).

Consumers route events by checking `signature.accepts(kind) == primary == kind || aux_streams.contains(&kind)`.

## OrderBook DOM pattern

For live consumers (chart apps, backtesters): mirror `mylittlechart::orderbook-service` — maintain a `dig3-station::OrderBookTracker` per (exchange, symbol). On every event:

```rust
Event::OrderbookSnapshot { point, .. } => {
    tracker.apply_snapshot(&book);
    // feed snapshot to indicators
}
Event::OrderbookDelta { point, .. } => {
    tracker.apply_delta(&delta);
    // reconstruct full book from tracker.top_bids(50) / top_asks(50)
    // feed reconstructed book to snapshot-aware indicators
}
```

This is what real-world UIs do — 1 snapshot from the exchange + 1000+ delta updates per minute = continuous DOM state for all book-aware indicators.

## Hard rules

- `BarIndicatorId` is **single-stream**. Multi-symbol (cross-asset beta) and multi-TF indicators live in `events::`.
- **No hardcoded magic numbers** in `IndicatorInstance::create()`. All thresholds/multipliers/periods come from `config.additional_params` with sensible `unwrap_or` defaults — so runtime tuning by validators and UIs works.
- `is_ready()` must return true after **any** valid update path (bar, tick, or L2), not just the original one.
- Every `BarIndicatorId` / `EventId` MUST have a catalog signature. Discovery via `MasterCatalog::iter_signatures()` / `MasterEventCatalog::iter_signatures()`.

## Status

Tested live on Binance + Bybit FuturesCross + 6 secondary venues against `digdigdig3 v0.3.9`. ~84% combined indicator+event pass rate on live BTC data in a 60-second slice. Remaining failures are environment-bound: streams the exchanges don't emit on the tested instrument (e.g. greeks on non-Deribit, settlement events outside settlement window), or 24h+ time-window indicators (funding decay, ARCH effects in stable periods).

## License

MIT
