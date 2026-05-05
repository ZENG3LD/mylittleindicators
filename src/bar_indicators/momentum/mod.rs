// High-performance bar-based momentum indicators
pub mod rsi;
pub mod macd;
pub mod roc;
pub mod cci;
pub mod stochastics;
pub mod stochastikd;
pub mod vhf;
pub mod vhf_ma;
pub mod obv;
pub mod swings;
pub mod swings_soft;
pub mod pressure;
pub mod psl;
pub mod kvo;
pub mod cmo;

pub mod bias;
pub mod amat;
pub mod aroon;
pub mod bb_period;
pub mod dm;
pub mod auto_fibo;
pub mod adx;
pub mod di_plus_minus;

// Новые индикаторы для Multi-Signal Generator
pub mod highest;
pub mod lowest;
pub mod candle_patterns;
pub mod ma_cross;
pub mod williams_r;
pub mod parabolic_sar;
pub mod ultimate_oscillator;
pub mod tsi;
pub mod dpo;
pub mod kst;
pub mod vortex_indicator;
pub mod trix;
pub mod stochastic_rsi;
pub mod fisher_transform;
pub mod market_cipher;
pub mod connors_rsi;

// Новые варианты RSI с переиспользованием компонентов
pub mod atr_rsi;
pub mod volume_weighted_rsi;
pub mod ehlers_rocket_rsi;
pub mod adaptive_stochastic;
pub mod multi_timeframe_momentum_divergence;

pub mod neural_momentum_network;
pub mod apo;
pub mod aroon_down;
pub mod aroon_oscillator;
pub mod aroon_up;
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
pub mod gapo;
pub mod gator_oscillator;
pub mod ift_rsi;
pub mod intraday_momentum_index;
pub mod kdj;
pub mod laguerre_rsi;
pub mod macd_hist_zscore;
pub mod macd_histogram;
pub mod macd_signal;
pub mod momentum_zscore;
pub mod pfe;
pub mod pmo;
pub mod ppo;
pub mod ppo_signal;
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
pub mod rwi;
pub mod smi;
pub mod stc;
pub mod sweep_reversion;
pub mod swing_age;
pub mod tdi;
pub mod ultimate_oscillator_smooth;
pub mod zigzag;
// pub mod box_momentum;

// Universal Indicator System catalog
pub mod momentum_catalog;

pub use rsi::*;
pub use macd::*;
pub use roc::*;
pub use cci::*;
pub use stochastics::*;
pub use obv::*;
pub use vhf::*;
pub use swings::*;
pub use pressure::*;
pub use psl::*;
pub use kvo::*;
pub use cmo::*;
pub use bias::*;
pub use amat::*;
pub use aroon::*;
pub use bb_period::*;
pub use dm::*;
pub use vhf_ma::*;
pub use stochastikd::*;
pub use swings_soft::*;
pub use auto_fibo::*;
pub use highest::*;
pub use lowest::*;
pub use candle_patterns::*;
pub use ma_cross::*;
pub use williams_r::*;
pub use parabolic_sar::*;
pub use ultimate_oscillator::*;
pub use tsi::*;
pub use dpo::*;
pub use kst::*;
pub use vortex_indicator::*;
pub use trix::*;
pub use adx::*;
pub use stochastic_rsi::*;
pub use fisher_transform::*;
pub use market_cipher::*;
pub use connors_rsi::*;
pub use atr_rsi::*;
pub use volume_weighted_rsi::*;
pub use ehlers_rocket_rsi::*;
pub use adaptive_stochastic::*;
pub use multi_timeframe_momentum_divergence::*;
pub use neural_momentum_network::*;
// pub use box_momentum::{BoxedMomentum, BoxMomentumFactory};






















