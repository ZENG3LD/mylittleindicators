# Primitives roadmap

План преобразования "странных" индикаторов в семейство composite-detectors (primitives).

## Контекст

В `bar_indicators/` ~40 файлов фактически являются composite-detectors — индикаторами которые комбинируют OHLCV + N inner indicators и выдают **event-сигнал** (Signal±1, DoubleFlag, custom event-struct).

Сейчас они:
- захардкожены (DonchianBreakout, MaCross, 14 divergence файлов с разными inner oscillators)
- не параметризуются по выбору inner indicator
- частично сломан dedup (MaCross/MACD из-за неполного IndicatorKey)

Цель: семейство параметризованных primitives что заменяет ~40 хардкоженных файлов на ~10 настраиваемых.

## Семейства primitives (целевая таксономия)

| Primitive | Параметры | Заменяет |
|---|---|---|
| **Crossover** | subject, reference, direction | MaCross, SslChannel, MACD signal cross |
| **Breakout** | level, confirmation (close/wick/retest) | DonchianBreakout, любой channel break |
| **WickAtLevel** | level, wick_side, body_min_ratio | new — wick rejection |
| **PatternAtLevel** | pattern_kind, level, tolerance | new — candle pattern + level proximity |
| **SweepFailure** | extremum, confirmation_lookback | SfpDetector, SweepReversionIndex |
| **Divergence** | price_source, oscillator, type | 14 файлов divergence/ → один |
| **TrendDirection** | source (MA/HA/Supertrend) | SlopeDirectionLine, HeikinAshiTrend, Supertrend |
| **TrendRun** | fast, slow, run_length | Amat |
| **Confluence** | conditions, combine_mode | MarketCipher, NeuralMomentumNetwork, MultiDivergence |
| **SwingDetection** | mode (percent/atr/time/candle/lookahead) | 5 zigzag/ файлов → один |

## Архитектурный подход

Вариант **A (Inline composition)** — выбран:
- `IndicatorConfig` расширяется полем `inner_indicators: Vec<IndicatorConfig>`
- Primitive struct владеет `Box<IndicatorInstance>` для каждой зависимости
- Сигнатура `update_bar(o,h,l,c,v)` остаётся унифицированной — primitive внутри прокачивает свои inner и применяет detection логику
- Dedup на уровне mlq (вне mli) — через корректный `IndicatorKey` с `param_hash` по всему конфигу включая `inner_indicators`

Что **не делаем**:
- Не делаем topological-aware warmup в mli (это зона потребителя)
- Не делаем role-based slice references (`update_role(values: &[f64])`)
- Не делаем parallel enum `CompositeIndicatorInstance`

## Шаги (порядок выполнения)

### Шаг 1: Починить IndicatorKey

`src/catalog/indicator_key.rs` — расширить структуру:
```rust
struct IndicatorKey {
    indicator_id: BarIndicatorId,
    period: u16,
    ma_type: Option<MovingAverageType>,
    output: OutputSelector,
    param_hash: u64,  // ← новый
}
```

`compute_param_hash(&IndicatorConfig) -> u64` хеширует:
- `periods[1..]` (помимо первого)
- `additional_params` (sorted by key)
- `ma_types` (sorted by key, помимо "ma_type")
- `flags`
- `source` (если не Close)
- `component_configs` (sorted by key, рекурсивно)
- `inner_indicators` (когда появится — рекурсивно)

После этого MaCross(9, 21, EMA/EMA) и MaCross(9, 30, EMA/SMA) получают разные ключи.

**Без этого primitives добавлять нельзя** — каждый новый composite сразу ломает dedup.

### Шаг 2: Расширить IndicatorConfig

`src/bar_indicators/instance_factory.rs` — добавить поле:
```rust
struct IndicatorConfig {
    // существующие поля
    inner_indicators: Vec<IndicatorConfig>,
}
```

Builder methods:
```rust
fn with_inner(mut self, inner: IndicatorConfig) -> Self
```

`compute_param_hash` обновить чтобы учитывал `inner_indicators` рекурсивно.

### Шаг 3: Бенчмарк существующих composites через новый ключ

Перед добавлением новых primitives проверить что **MaCross / MACD / SqueezeMomentum** теперь корректно дедуплятся (по param_hash). Никаких регрессов поведения.

### Шаг 4: Первый primitive — Crossover

`src/bar_indicators/primitives/crossover.rs`:
```rust
struct Crossover {
    subject: Box<IndicatorInstance>,
    reference: Box<IndicatorInstance>,
    direction: CrossDirection,
    state: CrossState,
}
```

Factory wiring через `inner_indicators[0]` = subject, `inner_indicators[1]` = reference.

Output: `IndicatorValue::Signal(i8)`.

Tests: воспроизвести `MaCross(SMA(9), SMA(21))` через Crossover + verify output совпадает с legacy MaCross на одном bar history.

### Шаг 5: Crossover заменяет хардкоды

- MaCross → deprecated/удалить (использовать Crossover)
- SslChannel → deprecated (subject=MA(high), reference=MA(low))
- MACD signal cross — отдельный случай поверх MACD (subject=MACD line, reference=MACD signal через OutputSelector)

### Шаг 6: Второй primitive — Breakout

`src/bar_indicators/primitives/breakout.rs`:
```rust
struct Breakout {
    level_indicator: Box<IndicatorInstance>,  // или 2 (upper/lower)
    confirmation: BreakoutConfirmation,        // CloseThrough | WickThrough | RetestAfter(N)
    state: BreakoutState,
}
```

Заменяет DonchianBreakout (level=Donchian.upper).

### Шаг 7: Divergence — унификация 14 файлов

`src/bar_indicators/primitives/divergence.rs`:
```rust
struct Divergence {
    oscillator: Box<IndicatorInstance>,
    lookback: usize,
    detection_kind: DivergenceKind,  // Regular | Hidden
    state: DivergenceState,
}
```

После реализации — все 14 файлов в `divergence/` могут быть удалены (заменены этим primitive с разными inner oscillators).

### Шаг 8: WickAtLevel + PatternAtLevel

Новые primitives (раньше отсутствовали):
```rust
struct WickAtLevel {
    level: Box<IndicatorInstance>,
    wick_side: WickSide,
    body_min_ratio: f64,
    tolerance: f64,
}

struct PatternAtLevel {
    pattern_detector: Box<IndicatorInstance>,
    level: Box<IndicatorInstance>,
    tolerance: f64,
}
```

### Шаг 9: SwingDetection (унификация zigzag)

5 zigzag файлов → один primitive с mode parameter.

### Шаг 10: TrendDirection / TrendRun / SweepFailure / Confluence

Аналогичная unification остальных категорий.

## Что не входит в этот roadmap

- **Topological-aware warmup** — задача mlq optimizer, не mli
- **Slice cache dedup** — задача mlq, но требует корректный IndicatorKey (шаг 1)
- **Role-based primitives** (`update_role(values)`) — не выбран архитектурно
- **Regime classifiers** (Kalman, MarketRegimeFilter, RegimeComposite v1-v4) — отдельная задача, не primitives
- **Pure detectors** (WickSpike, FvgDetector, BosChochDetector) — autonomous, остаются как есть, могут быть inner для primitives (PatternAtLevel(WickSpike, ...))

## Текущий статус

| Шаг | Статус |
|---|---|
| 1. IndicatorKey + param_hash | TODO |
| 2. IndicatorConfig.inner_indicators | TODO |
| 3. Бенчмарк dedup на legacy composites | TODO |
| 4. Crossover primitive | TODO |
| 5. MaCross/SslChannel deprecation | TODO |
| 6. Breakout primitive | TODO |
| 7. Divergence unification | TODO |
| 8. WickAtLevel + PatternAtLevel | TODO |
| 9. SwingDetection unification | TODO |
| 10. Остальные primitives | TODO |
