//! Trend Stop Indicators - индикаторы динамических уровней для трейлинг стопов
//! 
//! Этот модуль содержит индикаторы, которые вычисляют динамические уровни поддержки/сопротивления.
//! ⚠️ Индикаторы НЕ содержат логику стопов - только вычисляют уровни!
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней:
//! 
//! - PSAR Stop - вычисляет значения Parabolic SAR для динамических уровней
//! - SuperTrend Stop - уровни SuperTrend на основе ATR и медианной цены  
//! - Chandelier Stop - уровни на основе ATR от максимумов/минимумов
//! - ATR Trailing Stop - простые динамические уровни на ATR
//! - Volatility Stop - адаптивные уровни на основе волатильности
//! - Swing Stop - уровни на основе свинг хаев/лоуз  
//! - Keltner Stop - уровни на основе каналов Кельтнера
//! - Donchian Stop - уровни на основе каналов Дончиана

pub mod psar_stop;
pub mod supertrend_stop;
pub mod chandelier_stop;
pub mod atr_trailing_stop;
pub mod volatility_stop;
pub mod swing_stop;
pub mod keltner_stop;
pub mod donchian_stop;
pub mod chande_kroll_stop;
pub mod donchian_breakout;
pub mod trend_stop_catalog;

// Re-exports
pub use psar_stop::PSARStop;
pub use supertrend_stop::SuperTrendStop;
pub use chandelier_stop::ChandelierStop;
pub use atr_trailing_stop::ATRTrailingStop;
pub use volatility_stop::VolatilityStop;
pub use swing_stop::SwingStop;
pub use keltner_stop::KeltnerStop;
pub use donchian_stop::DonchianStop; 