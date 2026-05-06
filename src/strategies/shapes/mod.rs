//! Shapes — топологии стратегий.
//!
//! Это **определения** (ShapeSpec — какие роли, операторы, состояния), не
//! генерация кода. Codegen в MLQ читает эти определения и пишет hot-loop Rust.
//!
//! Сейчас 23 shape'а в MLQ + новая папка `structure/` для MLI Structure SMC
//! (BOS/CHoCH/FVG/OrderBlock/LiqSweep/Imbalance) — топологий нет, всё новое.
//!
//! Roadmap:
//! - `role_kind.rs`              — RoleKind taxonomy (Smoother/OscillatorBounded/
//!                                 OscillatorUnbounded/TrendLine/Channel/...)
//! - `cross_2roles.rs`           — MA cross / OperatorClass::Cross over 2 roles
//! - `threshold_1role.rs`        — осциллятор пересекает константу
//! - `threshold_zone_exit.rs`    — выход из зоны (FirstTime semantics, RSI 30/70)
//! - `divergence.rs`             — связан с conditions/divergence
//! - `channel_break.rs` / `channel_touch.rs` — пробой/касание канала
//! - `multi_tf_cascade.rs`       — N MA на N TF, выровнены (MLQ-only)
//! - `htf_state_filter.rs`       — HTF фильтр поверх LTF entry (MLQ-only)
//! - `regime_gate.rs`            — режимный фильтр
//! - `trail_flip.rs`             — trail-stop разворот
//! - `candle_pattern_event.rs`   — events на свечных паттернах
//! - `time_gate.rs` / `sequence_within_n.rs` / `zscore_extreme.rs`
//! - `zero_cross_direction.rs` / `level_break.rs` / `squeeze_setup.rs`
//! - `volatility_breakout.rs`
//! - `slope_direction.rs` / `pivot_touch.rs`
//! - `pools.rs` / `template.rs` / `fill.rs` / `compose.rs` / `all.rs`
//! - `structure/`                — SMC events, новые в MLQ:
//!     - `bos.rs`            — Break of Structure
//!     - `choch.rs`          — Change of Character
//!     - `liq_sweep.rs`      — Liquidity Sweep
//!     - `fvg.rs`            — Fair Value Gap
//!     - `order_block.rs`    — Order Block

pub mod structure;
