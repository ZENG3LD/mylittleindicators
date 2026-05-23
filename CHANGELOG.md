# Changelog

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
