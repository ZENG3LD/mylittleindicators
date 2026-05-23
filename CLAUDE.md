# mylittleindicators (mli)

Central financial backend between dig3 (data source) and consumers (mlc renderer, mlq backtester, OSS users).

- **Receives** all data types from dig3 — REST history + WS live (27 stream kinds).
- **Distributes** through consumer traits to ~559 bar indicators + 21 event primitives.
- **Catalogs** indicator/event metadata for codegen (mlq strategy codegen, mlc rendering).
- **NOT** trading, NOT strategy execution, NOT hot-loop optimization (that's mlq).

## Crate layout

```
mylittleindicators/
├── src/
│   ├── core/             — primitive types (Bar, Tick), value/selector enums, AST roles
│   ├── data_loader/      — StreamKind, TimedEvent, EnrichedHistory, RestFetcher
│   ├── bar_indicators/   — 559 bar indicators (BarIndicatorId + IndicatorInstance factory)
│   ├── events/           — 21 event primitives (EventId + EventInstance factory)
│   └── catalog/          — IndicatorSignature + EventSignature + MasterCatalog
└── crates/
    └── mli-collector/    — daemon (Station consumer) + smoke + indicator-validator
```

## Two parallel factories: indicators and events

| | Indicators | Events |
|---|---|---|
| ID enum | `BarIndicatorId` (559 variants) | `EventId` (21 variants) |
| Config | `IndicatorConfig` (params, periods, flags, ma_types, source, inner_indicators) | `EventConfig` (params, periods, flags, string_params, inner_indicators) |
| Factory | `IndicatorInstance::create(&IndicatorConfig)` | `EventInstance::create(&EventConfig)` |
| Update | `update_bar/tick/orderbook/funding/...` (16 methods) | `update_bar` + `update_secondary_bar` (multi-symbol) |
| Signature | `IndicatorSignature` | `EventSignature` |
| Catalog | `MasterCatalog::iter_signatures()` | `MasterEventCatalog::iter_signatures()` |

Both follow identical patterns. Consumers use the same `(input_stream, aux_streams)` routing metadata.

## BarIndicatorId is single-stream only

**Rule**: any indicator that needs **multi-symbol** (CrossAssetBeta, PairsCointegrationProxy, RelativeStrengthCross) or **multi-TF** belongs in `events::` not `BarIndicatorId`. These were moved out in commit `4896be1`.

Single-stream rule: every BarIndicatorId reads ONE primary stream (Bar / Tick / OrderBook / OrderbookDelta / Funding / MarkPrice / OI / Liquidation / Ticker / AggTrade / IndexPrice / etc.). Composites that need multiple streams declare `aux_streams` in their catalog signature.

## Routing model (catalog signature → validator dispatch)

Each `IndicatorSignature` / `EventSignature` carries:
- `input_stream: StreamKind` — primary
- `aux_streams: &'static [StreamKind]` — secondary streams the indicator consumes

The collector/validator dispatches events by checking `signature.accepts(kind)` = `primary == kind || aux_streams.contains(kind)`. Multi-stream composites (FundingOiPressure consumes Funding + OpenInterest, BlockTradeVolumeRatio consumes BlockTrade + AggTrade, etc.) auto-receive all their streams.

## OrderBook DOM pattern (mlc-style)

Live consumers (`mli-collector-indicator-validator`, future mlc-style apps) maintain per-(exchange, symbol) `OrderBookTracker` (from dig3-station). On every event:
- `Event::OrderbookSnapshot` → `tracker.apply_snapshot(book)` then feed snapshot to indicators
- `Event::OrderbookDelta` → `tracker.apply_delta(delta)` then **reconstruct full book** via `tracker.top_bids(50)/top_asks(50)` and feed it to all snapshot-aware indicators
- `Event::Trade` for hybrid indicators → look up current book from tracker, call `update_tick_with_book(&tick, &book)`

This mirrors `mylittlechart::orderbook-service` (apply_rest_snapshot / apply_ws_snapshot / apply_ws_delta). 1 snapshot event from exchange = 1000+ usable book updates per minute via delta-derived reconstruction.

## Stream subscriptions (per-account_type)

The validator subscribes to ~40 (exchange, account, symbol, stream) combos targeting the right venue per stream:
- Binance + Bybit FuturesCross: 9 core (Trade, AggTrade, Kline, Ticker, Orderbook, OrderbookDelta, MarkPrice, FundingRate, Liquidation) + OI (Bybit only)
- OKX: OptionGreeks, BlockTrade, IndexPrice, IndexPriceKline, MarkPriceKline, SettlementEvent
- Deribit Options (via `SubscriptionSet::add_raw` for instrument-name passthrough): OptionGreeks, VolatilityIndex, BlockTrade, IndexPrice
- HTX/Hyperliquid/Bitget/KuCoin/MEXC/GateIO: per-exchange where dig3 protocol.rs declares them

Each subscription is tried individually via `SubscribeReport.failed` per-stream — fail-closed on initial subscribe error (dig3 0.3.7+).

## dig3 dependency pin

Always pinned exact: `digdigdig3 = "=0.3.9"` (mli + collector). dig3 ships major Station refactors (0.3.5/6/7/8/9) that we adopt one at a time; `[patch.crates-io]` points at the local path during dev.

Major versions consumed:
- 0.3.4: typed StreamEvent (struct variants, symbol on wrapper not payload), KlineInterval newtype
- 0.3.5: 18 extended Stream variants (greeks/HV/VI/Basis/L3/BlockTrade/Settlement/etc.)
- 0.3.6: blob persistence for 4 string-bearing types
- 0.3.7: fail-closed subscribe (`SubscribeReport.failed`) + kline-only auto-heal (fixes OOM on NotSupported)
- 0.3.8: `SubscriptionSet::add_raw` (Deribit options instrument names)
- 0.3.9: `Stream::OrderbookDelta` + `ObDeltaPoint` + persistence toggle

## mli-collector binaries

- `mli-collector` — production daemon. Subscribes via Station + writes events to disk via Station persistence.
- `mli-collector-smoke` — diagnostic harness: parallel REST + WS audit across all 22 TRUSTED exchanges, ~3min. Outputs per-(exchange × stream) matrix to `smoke_data/smoke_report.json`.
- `mli-collector-indicator-validator` — **live indicator + event harness**. Builds every BarIndicatorId + EventId with sensible defaults, subscribes to ~40 combos, dispatches each Station Event into all matching indicators/events, classifies after N seconds as: pass / never_ready / always_zero / never_received_event / always_nan_inf / create_failed / panic. Writes per-indicator JSON to `validator_report.json` for analysis.

## Common cargo commands

```bash
cd mylittleindicators
cargo check --lib                                              # fast lib check
cargo test --lib                                               # 3053+ unit tests
cd crates/mli-collector
cargo build --release --bin mli-collector-indicator-validator  # release validator
./target/release/mli-collector-indicator-validator.exe --duration-secs 60  # 60s smoke
./target/release/mli-collector-indicator-validator.exe --duration-secs 1800  # 30min run
```

## Test gates

- `cargo check --lib` — must finish clean, 0 errors / 0 warnings
- `cargo test --lib` — 3053 passed, 2 pre-existing failures (catalog count assertions, unrelated)
- `cargo check --all-targets` in crates/mli-collector — must finish clean
- 60s validator → ~470/577 pass on live BTC FuturesCross (current ceiling on this market)

## What's blocked by live-data realities (not code)

- ~50 indicators starve because their stream is genuinely wire-not-present on tested venues (OptionGreeks on non-Deribit, HV/VI mostly Deribit-only, BlockTrade rare, CompositeIndex Binance-only and sparse, etc.)
- ~5 funding/settlement indicators need 8h+ windows to see funding rate change events
- 3 events (Threshold/VolatilityRegime/SwingDetection) fed raw `close` price by their detect_from_values entry; need inner-indicator wrapper (ATR/RSI/percent change) for stable calibration across spot drift

These are NOT code bugs — they need either longer windows, different exchanges, or architectural changes to event factory (inner-indicator scalar feeds).

## Where decisions are recorded

- `docs/architecture.md` — 3-layer stack design (dig3 → Station → mli → consumers)
- dig3-side asks recorded in `digdigdig3/docs/plans/mli-station-asks.md` + `mli-asks-decision.md` (their responses)
- mli-collector smoke runs persist to `crates/mli-collector/smoke_data/smoke_report.json`

## Hard rules

- **NEVER** put multi-symbol or multi-TF indicators in `BarIndicatorId`. They live in `events::`.
- **NEVER** hardcode magic numbers (thresholds, multipliers, periods) in factory `create()` arms. Read from `config.additional_params` with `.unwrap_or(sensible_default)`. Hardcodes break runtime tuning by validator / mlq codegen / mlc UI.
- **NEVER** introduce parallel APIs. Extend existing factory + signature shapes. If a new indicator needs a new stream type that doesn't exist in `StreamKind`, add it once and reuse everywhere.
- **ALWAYS** match `is_ready()` to the actual update path. If indicator can be fed via two paths (bar AND L2), `is_ready` must return true after either path has enough data.
- Indicator catalog signature MUST exist for every `BarIndicatorId` / `EventId` — the validator iterates `MasterCatalog::iter_signatures()` to discover routing; missing signature = silent Bar default fallback.
