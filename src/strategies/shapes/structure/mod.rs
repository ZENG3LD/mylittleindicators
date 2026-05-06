//! Structure — SMC (Smart Money Concepts) события.
//!
//! Список взят из MLI `SignalKind::Structure`:
//! - BOS (Break of Structure) — пробой структурного high/low
//! - CHoCH (Change of Character) — слом тренда
//! - LiqSweep — съём ликвидности (sweep of equal highs/lows)
//! - FVG (Fair Value Gap) — гэп между баром N-1 и N+1
//! - OrderBlock — последний bullish/bearish бар перед движением
//! - Imbalance — дисбаланс между body/wick
//!
//! Реализация topology специфики каждого события — здесь, через те же
//! `events/` + `conditions/` + `composition/` примитивы.
//!
//! TODO: каждый файл (`bos.rs`, `choch.rs`, etc).
