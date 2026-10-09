// High-performance bar-based momentum indicators
pub mod rsi;
pub mod macd;
pub mod roc;
pub mod cci;
pub mod stochastics;
pub mod stochastikd;
pub mod pressure;
pub mod psl;
pub mod cmo;

pub mod bias;
pub mod aroon;

// Новые индикаторы для Multi-Signal Generator
pub mod williams_r;
pub mod ultimate_oscillator;
pub mod tsi;
pub mod dpo;
pub mod kst;
pub mod trix;
pub mod stochastic_rsi;
pub mod fisher_transform;
pub mod connors_rsi;

// Новые варианты RSI с переиспользованием компонентов
pub mod atr_rsi;
pub mod volume_weighted_rsi;
pub mod ehlers_rocket_rsi;
pub mod adaptive_stochastic;

pub mod apo;
pub mod bop;
pub mod center_of_gravity;
pub mod cfo;
pub mod coppock;
pub mod demarker;
pub mod detrended_synthetic_price;
pub mod dpo_percent;
pub mod dss_bressert;
pub mod ehlers_cyber_cycle;
pub mod elder_impulse;
pub mod elder_ray;
pub mod ema_slope;
pub mod ewmac;
pub mod ewmac_robust;
pub mod gator_oscillator;
pub mod ift_rsi;
pub mod intraday_momentum_index;
pub mod kdj;
pub mod macd_hist_zscore;
pub mod momentum_zscore;
pub mod pfe;
pub mod pmo;
pub mod pzo;
pub mod ppo;
pub mod qqe;
pub mod qstick;
pub mod rmi;
pub mod roc_percentile;
pub mod rsi_percentile_bands;
pub mod rsi_percentile_rank;
pub mod rsi_zscore;
pub mod rsioma;
pub mod rsx;
pub mod rvgi;
pub mod smi;
pub mod stc;
pub mod sweep_reversion;
pub mod tdi;
pub mod ultimate_oscillator_smooth;
// pub mod box_momentum:

// Universal Indicator System catalog

pub use rsi::*;
pub use macd::*;
pub use roc::*;
pub use cci::*;
pub use stochastics::*;
pub use pressure::*;
pub use psl::*;
pub use cmo::*;
pub use bias::*;
pub use aroon::*;
pub use stochastikd::*;
pub use williams_r::*;
pub use ultimate_oscillator::*;
pub use tsi::*;
pub use dpo::*;
pub use kst::*;
pub use trix::*;
pub use stochastic_rsi::*;
pub use fisher_transform::*;
pub use connors_rsi::*;
pub use atr_rsi::*;
pub use volume_weighted_rsi::*;
pub use ehlers_rocket_rsi::*;
pub use adaptive_stochastic::*;
// pub use box_momentum::{BoxedMomentum, BoxMomentumFactory};






















