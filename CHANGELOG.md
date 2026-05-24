# Changelog

## 0.1.2 — 2026-05-24

Consumes dig3 0.3.10 (BitMEX + Bitstamp connectors, polling/derived
stream layers, 7 new `Stream` variants wired into Station). Validator
pass-rate up to ~89% combined on 150s BTC slice.

### dig3 pin bump

`digdigdig3 = "=0.3.9"` → `"=0.3.10"`. Picks up:
- `Stream::LongShortRatio` (Binance/Bybit/OKX REST polling)
- `Stream::HistoricalVolatility` (Deribit REST polling)
- `Stream::Basis` derived (joins MarkPrice + IndexPrice)
- `Stream::FundingSettlement` derived (monitors FundingRate transitions)
- `Stream::PredictedFunding` (BitMEX `instrument` WS + OKX funding-rate)
- `Stream::OrderbookL3` (Bitstamp `live_orders_*` WS, real wire path)
- `BasisPoint` field rename `.basis` → `.value` (mark − index)
- `Stream::AuctionEvent` removed upstream (no public anonymous wire
  source exists)

### Newly passing indicators (live-verified on 150s slice)

| Indicator                  | Stream                | Last value example       |
|----------------------------|-----------------------|--------------------------|
| `LongShortRatioMomentum`   | LongShortRatio        | `Single(-0.0004)`        |
| `HvMomentum`               | HistoricalVolatility  | `Single(0.00477)`        |
| `L3OrderRate`              | OrderbookL3 (Bitstamp)| `Single(9.3)` orders/sec |
| `L3LargeOrderTracker`      | OrderbookL3 (Bitstamp)| `Triple(side, qty, px)`  |
| `L3SpooferScore`           | OrderbookL3 (Bitstamp)| `Single(0.107)`          |

Total flip: +5 indicators move `never_received_event` → `pass`.

Additional indicators that now receive events but are calibration- or
market-state-bound (status not yet `pass` but pipeline is alive):
- `LongShortExtremeDetector` (always_zero — narrow LSR band)
- `RatioVsPriceDivergence`   (never_ready — needs longer LSR history)
- `HvSpike`                  (always_zero — calm HV)
- `L3CancelRatio` / `QuoteStuffingDetector` / `QuoteLifecycleTracker`
  (Bitstamp wire delivers OrderbookL3 events but Bitstamp is mostly
  create-side, cancels rare)
- `PredictedFundingExtreme`  (4 ev seen, threshold not crossed)
- `FundingSettlementImpact`  (242 ev seen but no actual settlement
  inside slice)

### Code changes (validator only — no mli-core API change)

- New `Event::LongShortRatio` dispatch arm + `try_update_long_short_ratio`
  method + `long_short_ratio_point_to_core` mapper.
- Removed `Event::AuctionEvent` dispatch (variant deleted upstream).
- BitMEX + Deribit subscribes now go through `.add_raw()` (venue-native
  instrument names that the canonical normalizer doesn't know).
- Deribit HV symbol passes verbatim to `get_historical_volatility()` as
  a currency code (`"BTC"`, not `"BTC-PERPETUAL"`).

### Validator pass-rate delta

| Bucket     | 0.1.1     | 0.1.2     |
|------------|-----------|-----------|
| Indicators | 472/556 (84.9%) | 494/556 (88.8%) |
| Events     |  19/21  (90.5%) |  20/21  (95.2%) |
| Combined   | 489/577 (84.7%) | 514/577 (89.1%) |

### Still gated on wire / market state (not code)

- `Basis` derived indicators (`BasisMomentum`, `BasisExtreme`,
  `BasisZScore`, `IndexCorrelationBreakdown`): `Stream::Basis`
  subscribed on Binance + OKX (both MarkPrice + IndexPrice present),
  derived joiner spawn returns OK but no `Event::Basis` reaches our
  consumer in the 150s window. Need dig3-side diagnostic to confirm
  whether the joiner buffer is dropping events or the emit path is
  hooked up.
- `Settlement` primary indicators (`SettlementApproachSignal`,
  `SettlementPriceMomentum`): live wire stayed silent in 150s
  (settlement is an 8h-cycle event).
- `OptionGreeks` / `VolatilityIndex` / `BlockTrade`: subscribed via
  Deribit ATM strikes (auto-picked at startup) but option flow quiet
  on this slice.

## 0.1.1 — 2026-05-24

Bug fixes + indicator routing cleanup. Live validator pass-rate up from
84.7% to ~88% combined (BTC FuturesCross 60s slice).

- Cross-asset events (`CrossAssetBeta`, `PairsCointegrationProxy`,
  `RelativeStrengthCross`) now receive both primary AND secondary symbol
  feeds via `EventInstance::update_secondary_bar(o,h,l,c,v,ts)`.
- `PivotChannels` (`BarIndicatorId::Pivotchan`): seed first-period
  pivots from accumulated H/L/C so output is usable before a full
  period elapses; fallback active R/S to `price ± range/2` instead of
  leaking `±inf` sentinels into `Channel3` → NaN downstream.
- `TradeBookAbsorption`: absorption threshold is now ratio-based
  (`tick.size > visible_top * ratio`, default 0.5 via
  `additional_params["ratio"]`) instead of strict absolute. On deep
  BTC books the strict path almost never fired.
- `EventInstance::Threshold` and `VolatilityRegimeDetector` accept an
  optional inner indicator via `EventConfig.inner_indicators[0]`. When
  set, the inner runs on each `update_bar` and its `.value().main()`
  scalar feeds `detect_from_values`. Removes the spot-drift coupling
  where raw close (~75000) couldn't be compared to RSI-scale (30/70)
  or ATR-scale (5/50) thresholds.
- Catalog fixes (routing-only, no logic change):
  - `IndexPriceMomentum`: `input_stream` MarkPrice + aux Bar (matches
    the actual `update_mark` impl path).
  - `PriceVsIndexSpread`: aux Bar (needs spot close for spread).
  - `LongSqueezeDetector` + `OiPriceCorrelation`: aux MarkPrice
    (`is_ready` requires both `oi_seen && price_seen`).
  - `StopHuntDetector`, `SettlementVsMarkSpread`,
    `FundingSettlementImpact`: aux MarkPrice (dual-consumer pattern).
  - `SettledFundingMomentum`: `input_stream` FundingSettlement
    (matches `FundingSettlementConsumer` impl).
- f64 user output now human-readable: per-magnitude decimal precision
  via `fmt_f64()`, no more `Single(-4.5e-5)` Debug formatting.

### Known limitations (not mli — needs dig3 / live data)

These indicators are gated on streams that don't reach the validator:

- **Missing from dig3 0.3.9 `Stream` enum entirely** (no protocol
  registration on any exchange): `LongShortRatio`, `OrderbookL3`,
  `Basis`, `AuctionEvent`, `PredictedFunding`, `FundingSettlement`,
  `HistoricalVolatility`, `MarketWarning` (Hyperliquid declares but
  returns `StreamNotSupported` at subscribe). Indicators consuming
  these: ~14, status `never_received_event`.
- **Subscribed OK, wire silent on test runs**: `OptionGreeks`,
  `VolatilityIndex` (Deribit needs liquid option + active trading),
  `BlockTrade`, `SettlementEvent` (8h cycle), `InsuranceFund`,
  `RiskLimit` (Bybit broadcast-only on actual fund events),
  `CompositeIndex` (Binance index symbol-specific).
- **Always-zero on calm market** (correct behaviour, not bugs):
  `Cusum`/`StCusum`/`BpCusum`/`ArchLm` (statistical change-point
  detectors), `MonthTurn`/`SomEom`/`SoqEoq` (calendar-boundary
  triggers), `FundingTimeDecay`/`FundingDirectionShift` (no funding
  rate change inside slice).

## 0.1.0 — 2026-05-23

Initial public release.

- 559 bar indicators across 35 categories with single-stream catalog signatures.
- 21 event primitives with native `EventInstance::create` factory (mirrors `IndicatorInstance` shape).
- Unified `IndicatorSignature` / `EventSignature` catalog with `input_stream` + `aux_streams` routing metadata.
- Multi-symbol indicators (CrossAssetBeta, PairsCointegrationProxy, RelativeStrengthCross) live in `events::` with `update_secondary_bar()` hook — `BarIndicatorId` is strictly single-stream.
- Cross-stream composites (FundingOiPressure, SqueezeProbability, MarketStress, etc.) declare `aux_streams` so consumers route multiple stream kinds to one indicator.
- OrderBook DOM pattern supported via `dig3-station::OrderBookTracker` reconstruction (consumer-side; mlc-style apply_snapshot + apply_delta).
- Hybrid tick+book indicators (HiddenLiquidityDetector, TradeBookAbsorption, SweepImpactAnalyzer) consume paired feeds via `update_tick_with_book`.
- All factory `create()` arms read params from `additional_params` — zero hardcoded magic numbers.
- Compatible with `digdigdig3 = "=0.3.9"` (typed StreamEvent + 18 extended stream variants + Stream::OrderbookDelta + add_raw for instrument-name passthrough).
- Edition 2024.
