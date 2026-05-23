# Changelog

## 0.1.1 — 2026-05-24

- Cross-asset events (`CrossAssetBeta`, `PairsCointegrationProxy`,
  `RelativeStrengthCross`) now receive both primary AND secondary symbol
  feeds via `EventInstance::update_secondary_bar(o,h,l,c,v,ts)`. Live
  validator confirms beta/cointegration/strength calculations against
  BTCUSDT (primary) + ETHUSDT (secondary).

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
