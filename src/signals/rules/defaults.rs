//! Default signal profiles for common indicators
//!
//! This module provides ready-to-use SignalProfiles for popular indicators.
//! Uses BarIndicatorId for type-safe indicator identification.

use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use super::config::{DetectorConfig, ValueSource};
use super::profile::SignalProfile;

// ============================================================================
// Template helpers
// ============================================================================

fn bounded_oscillator(id: BarIndicatorId, name: &str, upper: f64, lower: f64) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: overbought/oversold levels", name))
        .with_detectors([
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, upper, lower),
            DetectorConfig::divergence("divergence", format!("{} Divergence", name), ValueSource::Main, 14),
        ])
}

fn zero_cross_oscillator(id: BarIndicatorId, name: &str, tolerance: f64) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: zero line crossings", name))
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, tolerance),
            DetectorConfig::divergence("divergence", format!("{} Divergence", name), ValueSource::Main, 14),
        ])
}

fn overlay_crossover(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: price crossover signals", name))
        .with_detectors([
            DetectorConfig::price_crossover("crossover", format!("Price Cross {}", name)),
        ])
}

fn channel_indicator(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: channel breakouts", name))
        .with_detectors([
            DetectorConfig::channel("channel", "Channel Position", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

fn histogram_signal(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: pattern signals", name))
        .with_detectors([
            DetectorConfig::histogram("signal", format!("{} Signal", name), ValueSource::Main),
        ])
}

fn volatility_swing(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: volatility changes", name))
        .with_detectors([
            DetectorConfig::swing("swing", format!("{} Swings", name), ValueSource::Main, 5),
        ])
}

fn swing_levels(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: swing highs/lows", name))
        .with_detectors([
            DetectorConfig::swing("swing", format!("{} Swings", name), ValueSource::Main, 5),
        ])
}

fn zero_cross_threshold(id: BarIndicatorId, name: &str, tolerance: f64, upper: f64, lower: f64) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: zero cross and extreme levels", name))
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, tolerance),
            DetectorConfig::threshold("levels", "Extreme Levels", ValueSource::Main, upper, lower),
        ])
}

fn dual_line_crossover(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: line crossovers", name))
        .with_detectors([
            DetectorConfig::crossover("cross", format!("{} Crossover", name), ValueSource::First, ValueSource::Second),
        ])
}

fn dual_crossover_threshold(id: BarIndicatorId, name: &str, upper: f64, lower: f64) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: crossover and overbought/oversold", name))
        .with_detectors([
            DetectorConfig::crossover("cross", format!("{} Crossover", name), ValueSource::First, ValueSource::Second),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::First, upper, lower),
        ])
}

fn dual_crossover_zerocross(id: BarIndicatorId, name: &str, tolerance: f64) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: crossover and zero cross", name))
        .with_detectors([
            DetectorConfig::crossover("cross", format!("{} Crossover", name), ValueSource::First, ValueSource::Second),
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::First, tolerance),
        ])
}

fn threshold_only(id: BarIndicatorId, name: &str, upper: f64, lower: f64) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: threshold levels", name))
        .with_detectors([
            DetectorConfig::threshold("levels", "Signal Levels", ValueSource::Main, upper, lower),
        ])
}

// ============================================================================
// Main dispatch
// ============================================================================

/// Get default signal profile for an indicator by BarIndicatorId
pub fn default_profile(indicator_id: BarIndicatorId) -> Option<SignalProfile> {
    match indicator_id {

        // ============================================================
        // Average / Moving Averages
        // ============================================================
        BarIndicatorId::Sma
        | BarIndicatorId::Ema
        | BarIndicatorId::Wma
        | BarIndicatorId::Dema
        | BarIndicatorId::Tema
        | BarIndicatorId::Kama
        | BarIndicatorId::Hma
        | BarIndicatorId::Frama
        | BarIndicatorId::Vidya
        | BarIndicatorId::Alma
        | BarIndicatorId::Ama
        | BarIndicatorId::AvFrama
        | BarIndicatorId::AvVidya
        | BarIndicatorId::Ehlersfa
        | BarIndicatorId::Ehlersz
        | BarIndicatorId::Framaadv
        | BarIndicatorId::Jma
        | BarIndicatorId::Lr
        | BarIndicatorId::Mcginley
        | BarIndicatorId::Rma
        | BarIndicatorId::T3
        | BarIndicatorId::Tma
        | BarIndicatorId::Trima
        | BarIndicatorId::Vwma => Some(ma_profile(indicator_id)),

        // Vwap is special — stays as swing profile
        BarIndicatorId::Vwap => Some(vwap_profile()),

        // ============================================================
        // Momentum — Bounded Oscillators (0–100 or 0–1)
        // ============================================================
        BarIndicatorId::Rsi => Some(rsi_profile()),
        BarIndicatorId::Stoch | BarIndicatorId::Stochkd => Some(stochastic_profile()),
        BarIndicatorId::StochRsi => Some(stoch_rsi_profile()),
        BarIndicatorId::ConnorsRsi => Some(bounded_oscillator(indicator_id, "ConnorsRSI", 70.0, 30.0)),
        BarIndicatorId::AdaptiveStoch => Some(dual_crossover_threshold(indicator_id, "Adaptive Stochastic", 80.0, 20.0)),
        BarIndicatorId::Uo => Some(bounded_oscillator(indicator_id, "Ultimate Oscillator", 70.0, 30.0)),
        BarIndicatorId::UoSmooth => Some(threshold_only(indicator_id, "UO Smooth", 70.0, 30.0)),
        BarIndicatorId::Rmi => Some(bounded_oscillator(indicator_id, "RMI", 70.0, 30.0)),
        BarIndicatorId::Rsioma => Some(bounded_oscillator(indicator_id, "RSIOMA", 70.0, 30.0)),
        BarIndicatorId::Rsx => Some(bounded_oscillator(indicator_id, "RSX", 70.0, 30.0)),
        BarIndicatorId::AtrRsi => Some(bounded_oscillator(indicator_id, "ATR RSI", 70.0, 30.0)),
        BarIndicatorId::LaguerreRsi => Some(threshold_only(indicator_id, "Laguerre RSI", 0.8, 0.2)),
        BarIndicatorId::IftRsi => Some(zero_cross_threshold(indicator_id, "IFT RSI", 0.05, 0.5, -0.5)),
        BarIndicatorId::Imi => Some(threshold_only(indicator_id, "IMI", 70.0, 30.0)),
        BarIndicatorId::Dss => Some(threshold_only(indicator_id, "DSS", 80.0, 20.0)),
        BarIndicatorId::Kdj => Some(dual_crossover_threshold(indicator_id, "KDJ", 80.0, 20.0)),
        BarIndicatorId::Tdi => Some(dual_crossover_threshold(indicator_id, "TDI", 68.0, 32.0)),
        BarIndicatorId::Psl => Some(threshold_only(indicator_id, "Psychological Line", 75.0, 25.0)),
        BarIndicatorId::Rvgi => Some(dual_crossover_zerocross(indicator_id, "RVGI", 0.001)),
        BarIndicatorId::Vortex => Some(dual_crossover_threshold(indicator_id, "Vortex", 1.1, 0.9)),
        BarIndicatorId::Demarker => Some(threshold_only(indicator_id, "DeMarker", 0.7, 0.3)),
        BarIndicatorId::Vwrsi => Some(bounded_oscillator(indicator_id, "VWRSI", 70.0, 30.0)),
        BarIndicatorId::Stc => Some(threshold_only(indicator_id, "STC", 75.0, 25.0)),
        BarIndicatorId::AroonOsc => Some(zero_cross_threshold(indicator_id, "Aroon Oscillator", 2.0, 50.0, -50.0)),
        BarIndicatorId::AroonUp => Some(threshold_only(indicator_id, "Aroon Up", 70.0, 30.0)),
        BarIndicatorId::AroonDown => Some(threshold_only(indicator_id, "Aroon Down", 70.0, 30.0)),
        BarIndicatorId::Tii => Some(threshold_only(indicator_id, "TII", 80.0, 20.0)),
        BarIndicatorId::RelTrendPos => Some(threshold_only(indicator_id, "Rel Trend Pos", 70.0, 30.0)),
        BarIndicatorId::C2cvp => Some(threshold_only(indicator_id, "C2C Vol Percentile", 80.0, 20.0)),
        BarIndicatorId::Vovp => Some(threshold_only(indicator_id, "Vol of Vol Percentile", 80.0, 20.0)),
        BarIndicatorId::DfaPct => Some(threshold_only(indicator_id, "DFA Percent", 60.0, 40.0)),
        BarIndicatorId::HurstPct => Some(threshold_only(indicator_id, "Hurst Percent", 60.0, 40.0)),
        BarIndicatorId::RsiPctRank => Some(threshold_only(indicator_id, "RSI Pct Rank", 80.0, 20.0)),
        BarIndicatorId::DayWeekMonth => Some(threshold_only(indicator_id, "Day/Week/Month", 90.0, 10.0)),
        BarIndicatorId::Screstp => Some(threshold_only(indicator_id, "Spectral Crest Pct", 80.0, 20.0)),
        BarIndicatorId::Sflatp => Some(threshold_only(indicator_id, "Spectral Flatness Pct", 70.0, 30.0)),
        BarIndicatorId::Sslopep => Some(zero_cross_threshold(indicator_id, "Spectral Slope Pct", 2.0, 20.0, -20.0)),
        BarIndicatorId::DivStrength => Some(threshold_only(indicator_id, "Divergence Strength", 70.0, 30.0)),
        BarIndicatorId::Hlva => Some(swing_levels(indicator_id, "HLVA")),
        BarIndicatorId::Cci => Some(cci_profile()),
        BarIndicatorId::Mfi => Some(mfi_profile()),
        BarIndicatorId::WilliamsR => Some(williams_r_profile()),
        BarIndicatorId::Cmo => Some(cmo_profile()),
        BarIndicatorId::Roc => Some(roc_profile()),

        // ============================================================
        // Momentum — Centered Oscillators (around zero)
        // ============================================================
        BarIndicatorId::Macd => Some(macd_profile()),
        BarIndicatorId::Adx => Some(adx_profile()),
        BarIndicatorId::Aroon => Some(aroon_profile()),
        BarIndicatorId::Supertrend => Some(supertrend_profile()),
        BarIndicatorId::Psar => Some(psar_profile()),
        BarIndicatorId::Tsi => Some(tsi_profile()),
        BarIndicatorId::Ppo => Some(ppo_profile()),
        BarIndicatorId::Ao => Some(ao_profile()),
        BarIndicatorId::Apo => Some(zero_cross_oscillator(indicator_id, "APO", 0.01)),
        BarIndicatorId::Bias => Some(zero_cross_oscillator(indicator_id, "Bias", 0.001)),
        BarIndicatorId::Bop => Some(zero_cross_threshold(indicator_id, "Balance of Power", 0.05, 0.5, -0.5)),
        BarIndicatorId::Cfo => Some(zero_cross_oscillator(indicator_id, "CFO", 0.1)),
        BarIndicatorId::Dpo => Some(zero_cross_oscillator(indicator_id, "DPO", 0.1)),
        BarIndicatorId::DpoPct => Some(zero_cross_oscillator(indicator_id, "DPO Pct", 0.1)),
        BarIndicatorId::Trix => Some(zero_cross_oscillator(indicator_id, "TRIX", 0.00001)),
        BarIndicatorId::Coppock => Some(zero_cross_oscillator(indicator_id, "Coppock", 0.1)),
        BarIndicatorId::Gator => Some(volatility_swing(indicator_id, "Gator")),
        BarIndicatorId::Qqe => Some(dual_crossover_zerocross(indicator_id, "QQE", 1.0)),
        BarIndicatorId::Pmo => Some(dual_crossover_zerocross(indicator_id, "PMO", 0.5)),
        BarIndicatorId::PpoSignal => Some(zero_cross_oscillator(indicator_id, "PPO Signal", 0.05)),
        BarIndicatorId::Kst => Some(dual_crossover_zerocross(indicator_id, "KST", 1.0)),
        BarIndicatorId::ElderRay => Some(elder_ray_profile()),
        BarIndicatorId::EhlersCc => Some(zero_cross_oscillator(indicator_id, "Ehlers CC", 0.01)),
        BarIndicatorId::Dsp => Some(zero_cross_oscillator(indicator_id, "DSP", 0.1)),
        BarIndicatorId::Ewmac => Some(zero_cross_oscillator(indicator_id, "EWMAC", 0.1)),
        BarIndicatorId::EwmacRobust => Some(zero_cross_oscillator(indicator_id, "EWMAC Robust", 0.1)),
        BarIndicatorId::Amat => Some(zero_cross_oscillator(indicator_id, "AMAT", 0.1)),
        BarIndicatorId::MarketCipher => Some(zero_cross_oscillator(indicator_id, "Market Cipher", 0.5)),
        BarIndicatorId::MoFisher => Some(zero_cross_oscillator(indicator_id, "MO Fisher", 0.1)),
        BarIndicatorId::MomZscore => Some(zero_cross_threshold(indicator_id, "Mom Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::NeuralMom => Some(zero_cross_oscillator(indicator_id, "Neural Momentum", 0.1)),
        BarIndicatorId::Pressure => Some(zero_cross_oscillator(indicator_id, "Pressure", 0.1)),
        BarIndicatorId::RsiZscore => Some(zero_cross_threshold(indicator_id, "RSI Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::Hmom => Some(zero_cross_oscillator(indicator_id, "Hilbert Momentum", 0.01)),
        BarIndicatorId::Cusum => Some(zero_cross_oscillator(indicator_id, "CUSUM", 1.0)),
        BarIndicatorId::Roof => Some(zero_cross_oscillator(indicator_id, "Roofing Filter", 0.01)),
        BarIndicatorId::Mrf => Some(zero_cross_oscillator(indicator_id, "Mean Reversion Filter", 0.1)),
        BarIndicatorId::Zmad => Some(zero_cross_threshold(indicator_id, "ZMAD", 0.1, 2.0, -2.0)),
        BarIndicatorId::Wave => Some(zero_cross_oscillator(indicator_id, "Wave", 0.01)),
        BarIndicatorId::Sbp => Some(zero_cross_oscillator(indicator_id, "SBP", 0.01)),
        BarIndicatorId::Sbprhl => Some(zero_cross_oscillator(indicator_id, "SBPRHL", 0.01)),
        BarIndicatorId::Cyber => Some(zero_cross_oscillator(indicator_id, "Cyber Cycle", 0.01)),
        BarIndicatorId::Asi => Some(zero_cross_oscillator(indicator_id, "ASI", 1.0)),
        BarIndicatorId::Cho => Some(zero_cross_oscillator(indicator_id, "Chaikin Osc", 1000.0)),
        BarIndicatorId::Eom => Some(zero_cross_oscillator(indicator_id, "Ease of Movement", 0.01)),
        BarIndicatorId::Di => Some(zero_cross_oscillator(indicator_id, "Demand Index", 0.5)),
        BarIndicatorId::Pfe => Some(zero_cross_threshold(indicator_id, "PFE", 2.0, 50.0, -50.0)),
        BarIndicatorId::Smi => Some(smi_profile()),
        BarIndicatorId::EhlersRocket => Some(zero_cross_threshold(indicator_id, "Ehlers Rocket RSI", 0.02, 0.5, -0.5)),
        BarIndicatorId::Ravi => Some(zero_cross_oscillator(indicator_id, "RAVI", 0.1)),
        BarIndicatorId::AdxSlope => Some(zero_cross_oscillator(indicator_id, "ADX Slope", 0.01)),
        BarIndicatorId::LrSlope => Some(zero_cross_oscillator(indicator_id, "LR Slope", 0.0001)),
        BarIndicatorId::KamaSlope => Some(zero_cross_oscillator(indicator_id, "KAMA Slope", 0.0001)),
        BarIndicatorId::EmaSlope => Some(zero_cross_oscillator(indicator_id, "EMA Slope", 0.0001)),
        BarIndicatorId::Didi => Some(didi_profile()),
        BarIndicatorId::TrEr => Some(threshold_only(indicator_id, "Trend vs ER", 0.7, 0.3)),
        BarIndicatorId::Atrz => Some(zero_cross_threshold(indicator_id, "ATR Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::Rvz => Some(zero_cross_threshold(indicator_id, "RV Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::Kscr => Some(zero_cross_oscillator(indicator_id, "Kalman Score", 0.1)),
        BarIndicatorId::Kslope => Some(zero_cross_oscillator(indicator_id, "Kalman Slope", 0.0001)),
        BarIndicatorId::Kslopez => Some(zero_cross_threshold(indicator_id, "Kalman Slope Z", 0.1, 2.0, -2.0)),
        BarIndicatorId::PriceZscore => Some(zero_cross_threshold(indicator_id, "Price Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::Autocorr => Some(zero_cross_threshold(indicator_id, "Autocorrelation", 0.05, 0.5, -0.5)),
        BarIndicatorId::Pacf => Some(zero_cross_threshold(indicator_id, "PACF", 0.05, 0.3, -0.3)),
        BarIndicatorId::AvwapDist => Some(zero_cross_oscillator(indicator_id, "AVWAP Distance", 0.001)),
        BarIndicatorId::VwapDist => Some(zero_cross_oscillator(indicator_id, "VWAP Distance", 0.001)),
        BarIndicatorId::DistLevels => Some(zero_cross_oscillator(indicator_id, "Distance to Levels", 0.001)),
        BarIndicatorId::Sslope => Some(zero_cross_oscillator(indicator_id, "Spectral Slope", 0.01)),
        BarIndicatorId::Sslopez => Some(zero_cross_threshold(indicator_id, "Spectral Slope Z", 0.1, 2.0, -2.0)),
        BarIndicatorId::StCusum => Some(zero_cross_oscillator(indicator_id, "Structural CUSUM", 1.0)),
        BarIndicatorId::BpCusum => Some(zero_cross_oscillator(indicator_id, "BP CUSUM", 1.0)),
        BarIndicatorId::ResidStat => Some(zero_cross_threshold(indicator_id, "Residual Stat", 0.1, 2.0, -2.0)),
        BarIndicatorId::KpssZ => Some(zero_cross_threshold(indicator_id, "KPSS Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::VrZAgg => Some(zero_cross_threshold(indicator_id, "VR Z-Score Agg", 0.1, 2.0, -2.0)),
        BarIndicatorId::BookSlope => Some(zero_cross_oscillator(indicator_id, "Book Slope", 0.001)),
        BarIndicatorId::OrderBookSlope => Some(zero_cross_oscillator(indicator_id, "Order Book Slope", 0.001)),
        BarIndicatorId::Sent => Some(zero_cross_threshold(indicator_id, "Sentiment", 2.0, 20.0, -20.0)),
        BarIndicatorId::Sentr => Some(zero_cross_oscillator(indicator_id, "Sentiment Rate", 0.1)),
        BarIndicatorId::MacdHist => Some(histogram_signal(indicator_id, "MACD Hist")),
        BarIndicatorId::MacdHistZ => Some(zero_cross_oscillator(indicator_id, "MACD Hist Z", 0.1)),
        BarIndicatorId::MacdSignal => Some(zero_cross_oscillator(indicator_id, "MACD Signal", 0.001)),
        BarIndicatorId::Qstick => Some(zero_cross_oscillator(indicator_id, "Qstick", 0.01)),
        BarIndicatorId::RocPct => Some(zero_cross_oscillator(indicator_id, "ROC Pct", 0.1)),
        BarIndicatorId::Cog => Some(zero_cross_oscillator(indicator_id, "COG", 0.01)),
        BarIndicatorId::Gapo => Some(volatility_swing(indicator_id, "GAPO")),
        BarIndicatorId::DiPlusMinus => Some(dual_line_crossover(indicator_id, "DI+/DI-")),
        BarIndicatorId::Dm => Some(zero_cross_oscillator(indicator_id, "DM", 0.01)),
        BarIndicatorId::MaCross => Some(zero_cross_oscillator(indicator_id, "MA Cross", 0.5)),
        BarIndicatorId::AutoFibo => Some(swing_levels(indicator_id, "Auto Fibo")),
        BarIndicatorId::Highest => Some(swing_levels(indicator_id, "Highest")),
        BarIndicatorId::Lowest => Some(swing_levels(indicator_id, "Lowest")),

        // Momentum — misc
        BarIndicatorId::RsiPctBands => Some(rsi_pct_bands_profile()),
        BarIndicatorId::BbPeriod => None, // informational metadata only
        BarIndicatorId::SwingAge => None,  // informational, not tradeable
        BarIndicatorId::Swings => Some(swings_profile()),
        BarIndicatorId::SwingsSoft => Some(swings_soft_profile()),
        BarIndicatorId::SweepRev => Some(histogram_signal(indicator_id, "Sweep Reversal")),
        BarIndicatorId::MtfMomDiv => Some(histogram_signal(indicator_id, "MTF Mom Div")),
        BarIndicatorId::MoObv => Some(divergence_swing(indicator_id, "MO OBV")),
        BarIndicatorId::Rwi => Some(rwi_profile()),
        BarIndicatorId::Vhf => Some(threshold_only(indicator_id, "VHF", 0.4, 0.2)),
        BarIndicatorId::VhfMa => Some(threshold_only(indicator_id, "VHF MA", 0.4, 0.2)),

        // ============================================================
        // Signal Processing
        // ============================================================
        BarIndicatorId::Butter => Some(overlay_crossover(indicator_id, "Butterworth")),
        BarIndicatorId::Cheby => Some(overlay_crossover(indicator_id, "Chebyshev")),
        BarIndicatorId::Decyc => Some(overlay_crossover(indicator_id, "Decycler")),
        BarIndicatorId::Ess => Some(overlay_crossover(indicator_id, "Super Smoother")),
        BarIndicatorId::Sg => Some(overlay_crossover(indicator_id, "Savitzky-Golay")),
        BarIndicatorId::Sbwf => Some(overlay_crossover(indicator_id, "Whittaker Filter")),
        BarIndicatorId::Hampel => Some(overlay_crossover(indicator_id, "Hampel")),
        BarIndicatorId::Rc => Some(overlay_crossover(indicator_id, "Renko")),
        BarIndicatorId::Rc2 => Some(overlay_crossover(indicator_id, "Renko 2")),
        BarIndicatorId::Rc3 => Some(overlay_crossover(indicator_id, "Renko 3")),
        BarIndicatorId::Rc4 => Some(overlay_crossover(indicator_id, "Renko 4")),
        BarIndicatorId::Scf => Some(volatility_swing(indicator_id, "SCF")),
        BarIndicatorId::Screst => Some(volatility_swing(indicator_id, "Spectral Crest")),
        BarIndicatorId::Sflux => Some(volatility_swing(indicator_id, "Spectral Flux")),
        BarIndicatorId::Shmpr => Some(volatility_swing(indicator_id, "SHMPR")),
        BarIndicatorId::Slmpr => Some(volatility_swing(indicator_id, "SLMPR")),
        BarIndicatorId::Sroll => Some(volatility_swing(indicator_id, "Spectral Rolloff")),
        BarIndicatorId::Sroll95 => Some(volatility_swing(indicator_id, "Sroll 95")),
        BarIndicatorId::Srollp => Some(threshold_only(indicator_id, "Srollp", 80.0, 20.0)),
        BarIndicatorId::Srollrp => Some(volatility_swing(indicator_id, "Srollrp")),
        BarIndicatorId::Stft => None, // frequency domain — no universal threshold
        BarIndicatorId::Fft => None,  // frequency domain — no universal threshold
        BarIndicatorId::Hdc => Some(volatility_swing(indicator_id, "Homodyne DC")),
        BarIndicatorId::Hilb => None, // multi-component spectral — visual only
        BarIndicatorId::Lz => Some(volatility_swing(indicator_id, "Lempel-Ziv")),
        BarIndicatorId::Sentent => Some(volatility_swing(indicator_id, "Sentent Entropy")),
        BarIndicatorId::Logicand => Some(threshold_only(indicator_id, "Logic AND", 0.9, 0.1)),
        BarIndicatorId::Logicor => Some(threshold_only(indicator_id, "Logic OR", 0.9, 0.1)),
        BarIndicatorId::Logicxor => Some(threshold_only(indicator_id, "Logic XOR", 0.9, 0.1)),
        BarIndicatorId::Logicsign => Some(zero_cross_oscillator(indicator_id, "Logic Sign", 0.5)),
        BarIndicatorId::Hyst => Some(swing_levels(indicator_id, "Hyst")),
        BarIndicatorId::Tenc => Some(zero_cross_threshold(indicator_id, "TENC", 0.05, 0.5, -0.5)),
        BarIndicatorId::Thresh => Some(threshold_only(indicator_id, "Thresh", 0.7, 0.3)),
        BarIndicatorId::Esine => Some(zero_cross_oscillator(indicator_id, "Even Sinewave", 0.01)),
        BarIndicatorId::Wcomp => None,  // compression ratio — data quality, not tradeable
        BarIndicatorId::Ser => Some(zero_cross_oscillator(indicator_id, "SER", 0.01)),
        BarIndicatorId::Sflat => Some(threshold_only(indicator_id, "Spectral Flatness", 0.7, 0.3)),

        // ============================================================
        // Channels / Bands
        // ============================================================
        BarIndicatorId::Bb => Some(bollinger_profile()),
        BarIndicatorId::Kc => Some(keltner_profile()),
        BarIndicatorId::Dc => Some(donchian_profile()),
        BarIndicatorId::Atrchan => Some(atr_channel_profile()),
        BarIndicatorId::Adaptivebb => Some(channel_indicator(indicator_id, "Adaptive BB")),
        BarIndicatorId::Adaptivechan => Some(channel_indicator(indicator_id, "Adaptive Chan")),
        BarIndicatorId::Starc => Some(channel_indicator(indicator_id, "STARC Bands")),
        BarIndicatorId::Envelope => Some(channel_indicator(indicator_id, "Envelope")),
        BarIndicatorId::Pricechan => Some(channel_indicator(indicator_id, "Price Channel")),
        BarIndicatorId::Regchan => Some(channel_indicator(indicator_id, "Regression Channel")),
        BarIndicatorId::Stddevchan => Some(channel_indicator(indicator_id, "StdDev Channel")),
        BarIndicatorId::Medchan => Some(channel_indicator(indicator_id, "Median Channel")),
        BarIndicatorId::Vwapchan => Some(channel_indicator(indicator_id, "VWAP Channel")),
        BarIndicatorId::Fibochan => Some(channel_indicator(indicator_id, "Fibo Channel")),
        BarIndicatorId::Projbands => Some(channel_indicator(indicator_id, "Projection Bands")),
        BarIndicatorId::Qrchan => Some(channel_indicator(indicator_id, "QR Channel")),
        BarIndicatorId::Theilsenchan => Some(channel_indicator(indicator_id, "Theil-Sen Channel")),
        BarIndicatorId::Trimabands => Some(channel_indicator(indicator_id, "TRIMA Bands")),
        BarIndicatorId::Volprofchan => Some(channel_indicator(indicator_id, "Vol Profile Channel")),
        BarIndicatorId::Percentilech => Some(channel_indicator(indicator_id, "Percentile Channel")),
        BarIndicatorId::Dpobands => Some(channel_indicator(indicator_id, "DPO Bands")),
        BarIndicatorId::Atrc => Some(channel_indicator(indicator_id, "ATR Channels")),
        BarIndicatorId::Darvas => Some(darvas_profile()),
        BarIndicatorId::Vprb => Some(channel_indicator(indicator_id, "VPRB")),
        BarIndicatorId::Percentb => Some(threshold_only(indicator_id, "%B", 1.0, 0.0)),
        BarIndicatorId::Bbmetrics => Some(bbmetrics_profile()),
        BarIndicatorId::Kcmetrics => Some(volatility_swing(indicator_id, "Keltner Metrics")),
        BarIndicatorId::Dcmetrics => Some(volatility_swing(indicator_id, "DC Metrics")),
        BarIndicatorId::Dcpos => Some(threshold_only(indicator_id, "DC Position", 0.9, 0.1)),
        BarIndicatorId::Dcwidth => Some(volatility_swing(indicator_id, "DC Width")),
        BarIndicatorId::Keltpos => Some(threshold_only(indicator_id, "Keltner Position", 0.9, 0.1)),
        BarIndicatorId::Keltbw => Some(volatility_swing(indicator_id, "Keltner BW")),
        BarIndicatorId::Keltdist => Some(zero_cross_threshold(indicator_id, "Keltner Distance", 0.1, 2.0, -2.0)),
        BarIndicatorId::Medchanpos => Some(threshold_only(indicator_id, "Median Chan Pos", 0.9, 0.1)),
        BarIndicatorId::Pchwidth => Some(volatility_swing(indicator_id, "Price Chan Width")),
        BarIndicatorId::Pchosc => Some(zero_cross_threshold(indicator_id, "Price Chan Osc", 2.0, 50.0, -50.0)),
        BarIndicatorId::Regchanwidth => Some(volatility_swing(indicator_id, "Reg Chan Width")),
        BarIndicatorId::Stddevwidth => Some(volatility_swing(indicator_id, "StdDev Width")),
        BarIndicatorId::Vwapchanwidth => Some(volatility_swing(indicator_id, "VWAP Chan Width")),
        BarIndicatorId::Envbw => Some(volatility_swing(indicator_id, "Envelope BW")),
        BarIndicatorId::Ichimoku => Some(ichimoku_profile()),
        BarIndicatorId::Ichimokupos => Some(zero_cross_threshold(indicator_id, "Ichimoku Pos", 1.0, 5.0, -5.0)),
        BarIndicatorId::Ichimokuthick => Some(volatility_swing(indicator_id, "Ichimoku Thickness")),
        BarIndicatorId::Pivotchan => Some(pivot_chan_profile()),

        // ============================================================
        // Volatility
        // ============================================================
        BarIndicatorId::Atr => Some(atr_profile()),
        BarIndicatorId::Tr => Some(volatility_swing(indicator_id, "True Range")),
        BarIndicatorId::Natr => Some(volatility_swing(indicator_id, "NATR")),
        BarIndicatorId::Atrbw => Some(volatility_swing(indicator_id, "ATR BW")),
        BarIndicatorId::Atrp => Some(threshold_only(indicator_id, "ATR %", 3.0, 0.5)),
        BarIndicatorId::Atrpt => Some(overlay_crossover(indicator_id, "ATR % Trailing")),
        BarIndicatorId::Cv => Some(volatility_swing(indicator_id, "CV")),
        BarIndicatorId::Dvr => Some(volatility_swing(indicator_id, "Dynamic Vol Regime")),
        BarIndicatorId::Fuzzy => Some(volatility_swing(indicator_id, "Fuzzy")),
        BarIndicatorId::Har => Some(volatility_swing(indicator_id, "HAR")),
        BarIndicatorId::Hvc2c => Some(volatility_swing(indicator_id, "HVC2C")),
        BarIndicatorId::Kp => Some(volatility_swing(indicator_id, "Kase Peak")),
        BarIndicatorId::Nr => Some(threshold_only(indicator_id, "Narrow Range", 5.0, 1.0)),
        BarIndicatorId::Pgry => Some(volatility_swing(indicator_id, "PGRY")),
        BarIndicatorId::Rbvj => Some(volatility_swing(indicator_id, "Realized Bipower Jumps")),
        BarIndicatorId::Rcb => Some(volatility_swing(indicator_id, "Range Compression")),
        BarIndicatorId::Rp => Some(volatility_swing(indicator_id, "Realized Parkinson")),
        BarIndicatorId::Rq => Some(volatility_swing(indicator_id, "Realized Quarticity")),
        BarIndicatorId::Rv => Some(volatility_swing(indicator_id, "Realized Vol")),
        BarIndicatorId::Avr => Some(volatility_swing(indicator_id, "Average Vol Range")),
        BarIndicatorId::Bpv => Some(volatility_swing(indicator_id, "Bipower Variance")),
        BarIndicatorId::Ui => Some(threshold_only(indicator_id, "Ulcer Index", 10.0, 1.0)),
        BarIndicatorId::Vbd => Some(threshold_only(indicator_id, "Volatility Breakout", 1.0, 0.0)),
        BarIndicatorId::Vbexp => Some(volatility_swing(indicator_id, "Vol Expansion")),
        BarIndicatorId::VoDc => Some(volatility_swing(indicator_id, "Vol of DC")),
        BarIndicatorId::VoKc => Some(volatility_swing(indicator_id, "Vol of KC")),
        BarIndicatorId::VoMi => Some(volatility_swing(indicator_id, "Vol of MI")),
        BarIndicatorId::VoVr => Some(volatility_swing(indicator_id, "Vol of Ratio")),
        BarIndicatorId::Vov => Some(volatility_swing(indicator_id, "Vol of Vol")),
        BarIndicatorId::Vovpt => Some(volatility_swing(indicator_id, "Vol of Vol % Trail")),
        BarIndicatorId::Abb => Some(volatility_swing(indicator_id, "Adaptive BB BW")),
        BarIndicatorId::Wvf => Some(threshold_only(indicator_id, "Williams VixFix", 15.0, 3.0)),
        BarIndicatorId::Sqmom => Some(sqmom_profile()),
        BarIndicatorId::Chop => Some(threshold_only(indicator_id, "Choppiness Index", 61.8, 38.2)),
        BarIndicatorId::Rvol => Some(threshold_only(indicator_id, "RVOL", 2.0, 0.5)),
        BarIndicatorId::Garch => Some(volatility_swing(indicator_id, "GARCH")),
        BarIndicatorId::Egarch => Some(volatility_swing(indicator_id, "EGARCH")),
        BarIndicatorId::Var => Some(volatility_swing(indicator_id, "VAR")),
        BarIndicatorId::Hurst => Some(threshold_only(indicator_id, "Hurst", 0.6, 0.4)),
        BarIndicatorId::FractalDim => Some(threshold_only(indicator_id, "Fractal Dim", 1.6, 1.4)),
        BarIndicatorId::Dfa => Some(volatility_swing(indicator_id, "DFA")),
        BarIndicatorId::Rvi => Some(threshold_only(indicator_id, "RVI", 60.0, 40.0)),

        // ============================================================
        // Levels / Support-Resistance (overlay)
        // ============================================================
        BarIndicatorId::Avwap => Some(avwap_profile()),
        BarIndicatorId::Avwaprev => Some(avwap_rev_profile()),
        BarIndicatorId::Avwaptouch => Some(histogram_signal(indicator_id, "AVWAP Touch")),
        BarIndicatorId::Bos => Some(swing_levels(indicator_id, "BOS")),
        BarIndicatorId::Camarilla => Some(swing_levels(indicator_id, "Camarilla")),
        BarIndicatorId::Demark => Some(swing_levels(indicator_id, "DeMark")),
        BarIndicatorId::Floorpivot => Some(swing_levels(indicator_id, "Floor Pivot")),
        BarIndicatorId::Fvg => Some(fvg_profile()),
        BarIndicatorId::Fvgalt => Some(swing_levels(indicator_id, "FVG Alt")),
        BarIndicatorId::Fvgdur => Some(threshold_only(indicator_id, "FVG Duration", 10.0, 1.0)),
        BarIndicatorId::Fvgrev => Some(histogram_signal(indicator_id, "FVG Reversal")),
        BarIndicatorId::Liqgap => Some(swing_levels(indicator_id, "Liquidity Gap")),
        BarIndicatorId::Pivavwap => Some(piv_avwap_profile()),
        BarIndicatorId::Pivot => None, // static price levels — visual only
        BarIndicatorId::Rmid => Some(overlay_crossover(indicator_id, "Range Mid")),
        BarIndicatorId::Rquart => Some(range_quart_profile()),
        BarIndicatorId::Swingstr => Some(threshold_only(indicator_id, "Swing Strength", 70.0, 30.0)),
        BarIndicatorId::Woodie => Some(swing_levels(indicator_id, "Woodie")),
        BarIndicatorId::Cpr => Some(cpr_profile()),
        BarIndicatorId::VwapLevels => None, // static VWAP price levels — visual only

        // ============================================================
        // Candle Patterns
        // ============================================================
        BarIndicatorId::Candleanatomy => None, // body/wick sizes — visual only
        BarIndicatorId::Darkcloudcover => Some(histogram_signal(indicator_id, "Dark Cloud Cover")),
        BarIndicatorId::Doji => Some(histogram_signal(indicator_id, "Doji")),
        BarIndicatorId::Engulfing => Some(histogram_signal(indicator_id, "Engulfing")),
        BarIndicatorId::Eveningstar => Some(histogram_signal(indicator_id, "Evening Star")),
        BarIndicatorId::Hammer => Some(histogram_signal(indicator_id, "Hammer")),
        BarIndicatorId::Harami => Some(histogram_signal(indicator_id, "Harami")),
        BarIndicatorId::Heikinashi => None, // price-level overlay — visual only
        BarIndicatorId::Marubozu => Some(histogram_signal(indicator_id, "Marubozu")),
        BarIndicatorId::Morningstar => Some(histogram_signal(indicator_id, "Morning Star")),
        BarIndicatorId::Patternrec => Some(histogram_signal(indicator_id, "Pattern Recognition")),
        BarIndicatorId::Piercingpattern => Some(histogram_signal(indicator_id, "Piercing Pattern")),
        BarIndicatorId::Sfp => Some(histogram_signal(indicator_id, "SFP")),
        BarIndicatorId::Shootingstar => Some(histogram_signal(indicator_id, "Shooting Star")),
        BarIndicatorId::Threeblackcrows => Some(histogram_signal(indicator_id, "Three Black Crows")),
        BarIndicatorId::Threewhitesoldiers => Some(histogram_signal(indicator_id, "Three White Soldiers")),
        BarIndicatorId::Tweezer => Some(histogram_signal(indicator_id, "Tweezer")),
        BarIndicatorId::Wickspike => Some(histogram_signal(indicator_id, "Wick Spike")),
        BarIndicatorId::CandlePatterns => Some(histogram_signal(indicator_id, "Candle Patterns")),
        BarIndicatorId::HaTrend => Some(histogram_signal(indicator_id, "HA Trend")),

        // ============================================================
        // Volume indicators
        // ============================================================
        BarIndicatorId::Obv => Some(obv_profile()),
        BarIndicatorId::Cmf => Some(cmf_profile()),
        BarIndicatorId::Ad => Some(ad_profile()),
        BarIndicatorId::Fi => Some(zero_cross_oscillator(indicator_id, "Force Index", 1000.0)),
        BarIndicatorId::Kvo => Some(kvo_profile()),
        BarIndicatorId::Vroc => Some(zero_cross_threshold(indicator_id, "Volume ROC", 1.0, 50.0, -50.0)),
        BarIndicatorId::NviPvi => Some(nvi_pvi_profile()),
        BarIndicatorId::Tmf => Some(zero_cross_threshold(indicator_id, "Twiggs Money Flow", 0.01, 0.2, -0.2)),
        BarIndicatorId::Pvo => Some(zero_cross_oscillator(indicator_id, "PVO", 1.0)),
        BarIndicatorId::Pzo => Some(zero_cross_threshold(indicator_id, "PZO", 2.0, 15.0, -15.0)),
        BarIndicatorId::Trin => Some(threshold_only(indicator_id, "TRIN", 2.0, 0.5)),
        BarIndicatorId::Vdelta => Some(vdelta_profile()),
        BarIndicatorId::Vfi => Some(zero_cross_threshold(indicator_id, "VFI", 0.05, 0.2, -0.2)),
        BarIndicatorId::Vo => Some(zero_cross_oscillator(indicator_id, "Volume Osc", 1.0)),
        BarIndicatorId::Vpin => Some(threshold_only(indicator_id, "VPIN", 0.6, 0.2)),
        BarIndicatorId::Vprofile => None, // histogram of volume at price — visual only
        BarIndicatorId::Vpt => Some(divergence_swing(indicator_id, "VPT")),
        BarIndicatorId::Vz => Some(zero_cross_threshold(indicator_id, "Volume Z-Score", 0.1, 2.0, -2.0)),
        BarIndicatorId::Vzo => Some(zero_cross_threshold(indicator_id, "VZO", 2.0, 5.0, -5.0)),
        BarIndicatorId::Poc => Some(swing_levels(indicator_id, "POC")),
        BarIndicatorId::Wad => Some(divergence_swing(indicator_id, "Williams A/D")),
        BarIndicatorId::Pvt => Some(divergence_swing(indicator_id, "PVT")),
        BarIndicatorId::Ii => Some(zero_cross_oscillator(indicator_id, "Intraday Intensity", 1000.0)),
        BarIndicatorId::Iip => Some(zero_cross_threshold(indicator_id, "II Percent", 2.0, 20.0, -20.0)),
        BarIndicatorId::Iir => Some(threshold_only(indicator_id, "II Ratio", 1.5, 0.5)),
        BarIndicatorId::BookImb => Some(book_imb_profile()),
        BarIndicatorId::Ofi => Some(ofi_profile()),
        BarIndicatorId::QueueImb => Some(queue_imb_profile()),
        BarIndicatorId::ClQueueImb => Some(cl_queue_imb_profile()),
        BarIndicatorId::OrderFlowImb => Some(zero_cross_oscillator(indicator_id, "Order Flow Imb", 0.1)),
        BarIndicatorId::MarketMicro => None, // composite microstructure — informational
        BarIndicatorId::TickVolume => Some(threshold_only(indicator_id, "Tick Volume", 2.0, 0.5)),

        // ============================================================
        // Trend indicators
        // ============================================================
        BarIndicatorId::GannHilo => Some(overlay_crossover(indicator_id, "Gann HiLo")),
        BarIndicatorId::Gmma => Some(overlay_crossover(indicator_id, "GMMA")),
        BarIndicatorId::Sdl => Some(overlay_crossover(indicator_id, "SDL")),
        BarIndicatorId::Ssl => Some(ssl_profile()),
        BarIndicatorId::Alligator => Some(alligator_profile()),
        BarIndicatorId::Zlsma => Some(overlay_crossover(indicator_id, "ZLSMA")),
        BarIndicatorId::Eit => Some(eit_profile()),
        BarIndicatorId::ElderImpulse => Some(histogram_signal(indicator_id, "Elder Impulse")),
        BarIndicatorId::Kregime => Some(kregime_profile()),

        // ============================================================
        // Accumulation
        // ============================================================
        // (Most handled in Volume above; remaining here)

        // ============================================================
        // Adaptive / Kalman
        // ============================================================
        BarIndicatorId::Adaptivema => Some(overlay_crossover(indicator_id, "Adaptive MA")),
        BarIndicatorId::Mama => Some(overlay_crossover(indicator_id, "MAMA")),
        BarIndicatorId::Kalman => Some(overlay_crossover(indicator_id, "Kalman")),
        BarIndicatorId::Ekf => Some(overlay_crossover(indicator_id, "EKF")),
        BarIndicatorId::Ukf => Some(overlay_crossover(indicator_id, "UKF")),
        BarIndicatorId::Abgfilter => Some(overlay_crossover(indicator_id, "ABG Filter")),
        BarIndicatorId::Kcomp => Some(overlay_crossover(indicator_id, "KCOMP")),
        BarIndicatorId::Particle => Some(overlay_crossover(indicator_id, "Particle Filter")),
        BarIndicatorId::Rts => Some(overlay_crossover(indicator_id, "RTS Smoother")),

        // ============================================================
        // Trend Stop
        // ============================================================
        BarIndicatorId::Atrts => Some(overlay_crossover(indicator_id, "ATR Trail Stop")),
        BarIndicatorId::Chand => Some(chand_profile()),
        BarIndicatorId::Cks => Some(overlay_crossover(indicator_id, "CKS Stop")),
        BarIndicatorId::Donbo => Some(donbo_profile()),
        BarIndicatorId::Dons => Some(overlay_crossover(indicator_id, "Donchian Stop")),
        BarIndicatorId::Kelts => Some(overlay_crossover(indicator_id, "Keltner Stop")),
        BarIndicatorId::Psars => Some(overlay_crossover(indicator_id, "PSAR Stop")),
        BarIndicatorId::Supts => Some(overlay_crossover(indicator_id, "Support Stop")),
        BarIndicatorId::TsSwings => Some(ts_swings_profile()),
        BarIndicatorId::Volts => Some(overlay_crossover(indicator_id, "Volatility Stop")),
        BarIndicatorId::VoltsAtr => Some(overlay_crossover(indicator_id, "Volatility ATR Stop")),

        // ============================================================
        // Chaos / Fractal
        // ============================================================
        BarIndicatorId::Ac => Some(ac_profile()),
        BarIndicatorId::ChaosOsc => Some(zero_cross_oscillator(indicator_id, "Chaos Oscillator", 0.1)),
        BarIndicatorId::Fractals => Some(fractals_profile()),
        BarIndicatorId::WilliamsMfi => Some(williams_mfi_profile()),

        // ============================================================
        // Divergence indicators
        // ============================================================
        BarIndicatorId::CciDiv => Some(histogram_signal(indicator_id, "CCI Divergence")),
        BarIndicatorId::ClassicDiv => Some(histogram_signal(indicator_id, "Classic Divergence")),
        BarIndicatorId::HiddenDiv => Some(histogram_signal(indicator_id, "Hidden Divergence")),
        BarIndicatorId::MacdDiv => Some(histogram_signal(indicator_id, "MACD Divergence")),
        BarIndicatorId::MacdHistDiv => Some(histogram_signal(indicator_id, "MACD Hist Divergence")),
        BarIndicatorId::MultiDiv => Some(histogram_signal(indicator_id, "Multi Divergence")),
        BarIndicatorId::ObvDiv => Some(histogram_signal(indicator_id, "OBV Divergence")),
        BarIndicatorId::RsiDiv => Some(histogram_signal(indicator_id, "RSI Divergence")),
        BarIndicatorId::StochDiv => Some(histogram_signal(indicator_id, "Stoch Divergence")),
        BarIndicatorId::VolDiv => Some(histogram_signal(indicator_id, "Volume Divergence")),
        BarIndicatorId::WilliamsDiv => Some(histogram_signal(indicator_id, "Williams Divergence")),
        BarIndicatorId::ZigzagDiv => Some(histogram_signal(indicator_id, "Zigzag Divergence")),

        // ============================================================
        // Regression
        // ============================================================
        BarIndicatorId::Arima => Some(overlay_crossover(indicator_id, "ARIMA")),
        BarIndicatorId::Arimax => Some(overlay_crossover(indicator_id, "ARIMAX")),
        BarIndicatorId::PolyReg => Some(overlay_crossover(indicator_id, "PolyReg")),

        // ============================================================
        // Ratio / Efficiency
        // ============================================================
        BarIndicatorId::Er => Some(threshold_only(indicator_id, "Efficiency Ratio", 0.7, 0.3)),
        BarIndicatorId::ErRing => Some(threshold_only(indicator_id, "ER Ring", 0.7, 0.3)),
        BarIndicatorId::RangeAtr => Some(threshold_only(indicator_id, "Range/ATR", 1.5, 0.7)),
        BarIndicatorId::SpreadAnalyzer => Some(volatility_swing(indicator_id, "Spread Analyzer")),

        // ============================================================
        // Entropy / Information Theory
        // ============================================================
        BarIndicatorId::Apen => Some(volatility_swing(indicator_id, "Approx Entropy")),
        BarIndicatorId::Conden => Some(volatility_swing(indicator_id, "Conditional Entropy")),
        BarIndicatorId::Fisher => Some(zero_cross_oscillator(indicator_id, "Fisher", 0.01)),
        BarIndicatorId::Infog => Some(volatility_swing(indicator_id, "Information Gain")),
        BarIndicatorId::Jsd => Some(threshold_only(indicator_id, "JS Divergence", 0.5, 0.1)),
        BarIndicatorId::Kld => Some(volatility_swing(indicator_id, "KL Divergence")),
        BarIndicatorId::Mi => Some(volatility_swing(indicator_id, "Mutual Information")),
        BarIndicatorId::Pe => Some(volatility_swing(indicator_id, "Permutation Entropy")),
        BarIndicatorId::Sampen => Some(volatility_swing(indicator_id, "Sample Entropy")),
        BarIndicatorId::Shannon => Some(volatility_swing(indicator_id, "Shannon Entropy")),
        BarIndicatorId::Te => Some(volatility_swing(indicator_id, "Transfer Entropy")),
        BarIndicatorId::Xmil => Some(volatility_swing(indicator_id, "Cross MI")),

        // ============================================================
        // Statistics / Econometric
        // ============================================================
        BarIndicatorId::Adf => Some(threshold_only(indicator_id, "ADF", -1.9, -3.5)),
        BarIndicatorId::AdfKpss => Some(threshold_only(indicator_id, "ADF-KPSS", 0.7, 0.3)),
        BarIndicatorId::ArchLm => Some(threshold_only(indicator_id, "ARCH LM", 3.8, 0.0)),
        BarIndicatorId::ArchLmPval => Some(threshold_only(indicator_id, "ARCH LM P-Value", 0.1, 0.05)),
        BarIndicatorId::Coint => Some(threshold_only(indicator_id, "Cointegration", 0.0, -3.0)),
        BarIndicatorId::EgAdf => Some(threshold_only(indicator_id, "EG ADF", 0.0, -3.0)),
        BarIndicatorId::EgCoint => Some(threshold_only(indicator_id, "EG Cointegration", 0.0, -3.0)),
        BarIndicatorId::EgTrend => Some(threshold_only(indicator_id, "EG Trend", 0.7, 0.3)),
        BarIndicatorId::HalfLifeMr => Some(threshold_only(indicator_id, "Half-Life MR", 100.0, 5.0)),
        BarIndicatorId::Kpss => Some(threshold_only(indicator_id, "KPSS", 0.739, 0.119)),
        BarIndicatorId::KpssTrend => Some(threshold_only(indicator_id, "KPSS Trend", 0.739, 0.119)),
        BarIndicatorId::LjungBox => Some(threshold_only(indicator_id, "Ljung-Box", 20.0, 0.0)),
        BarIndicatorId::Pp => Some(threshold_only(indicator_id, "Phillips-Perron", -1.9, -3.5)),
        BarIndicatorId::PvCoherence => Some(threshold_only(indicator_id, "PV Coherence", 0.7, 0.3)),
        BarIndicatorId::RSquared => Some(threshold_only(indicator_id, "R-Squared", 0.7, 0.3)),
        BarIndicatorId::Vr => Some(threshold_only(indicator_id, "Variance Ratio", 1.2, 0.8)),
        BarIndicatorId::VrAgg => Some(threshold_only(indicator_id, "VR Aggregate", 1.2, 0.8)),
        BarIndicatorId::Za => Some(threshold_only(indicator_id, "Zivot-Andrews", -1.9, -4.0)),

        // ============================================================
        // Clusters / Order Book
        // ============================================================
        BarIndicatorId::DomWoq => None, // microstructure depth data — no clean threshold

        // ============================================================
        // Position / Calendar
        // ============================================================
        BarIndicatorId::HolidayProx => Some(threshold_only(indicator_id, "Holiday Proximity", 0.8, 0.2)),
        BarIndicatorId::HourDay => Some(threshold_only(indicator_id, "Hour of Day", 16.0, 9.0)),
        BarIndicatorId::MonthQtr => Some(threshold_only(indicator_id, "Month of Quarter", 3.0, 1.0)),
        BarIndicatorId::MonthTurn => Some(threshold_only(indicator_id, "Month Turn", 0.9, 0.1)),
        BarIndicatorId::QtrTurn => Some(threshold_only(indicator_id, "Quarter Turn", 0.9, 0.1)),
        BarIndicatorId::Session => None, // categorical session ID — not meaningful
        BarIndicatorId::SomEom => Some(threshold_only(indicator_id, "SOM/EOM", 0.9, 0.1)),
        BarIndicatorId::SoqEoq => Some(threshold_only(indicator_id, "SOQ/EOQ", 0.9, 0.1)),
        BarIndicatorId::SowEow => Some(threshold_only(indicator_id, "SOW/EOW", 0.9, 0.1)),
        BarIndicatorId::WeekMonth => Some(threshold_only(indicator_id, "Week of Month", 4.0, 2.0)),
        BarIndicatorId::Weekday => Some(threshold_only(indicator_id, "Weekday", 5.0, 2.0)),

        // ============================================================
        // Zigzag variants
        // ============================================================
        BarIndicatorId::Zigzag => Some(swing_levels(indicator_id, "Zigzag")),
        BarIndicatorId::ZigzagAtr => Some(swing_levels(indicator_id, "Zigzag ATR")),
        BarIndicatorId::ZigzagCandle => Some(swing_levels(indicator_id, "Zigzag Candle")),
        BarIndicatorId::ZigzagClassic => Some(swing_levels(indicator_id, "Zigzag Classic")),
        BarIndicatorId::ZigzagLookahead => Some(swing_levels(indicator_id, "Zigzag Lookahead")),
        BarIndicatorId::ZigzagTime => Some(swing_levels(indicator_id, "Zigzag Time")),

        // Catch-all for truly unknown/future variants
        _ => None,
    }
}

// ============================================================================
// Shared helper — divergence + swing
// ============================================================================

fn divergence_swing(id: BarIndicatorId, name: &str) -> SignalProfile {
    SignalProfile::new(id, format!("{} Default", name))
        .with_description(format!("{}: divergence and swing signals", name))
        .with_detectors([
            DetectorConfig::divergence("divergence", format!("{} Divergence", name), ValueSource::Main, 14),
            DetectorConfig::swing("swing", format!("{} Swings", name), ValueSource::Main, 5),
        ])
}

// ============================================================================
// Momentum Indicators
// ============================================================================

/// RSI default profile - overbought/oversold with divergence
pub fn rsi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Rsi, "RSI Default")
        .with_description("RSI signals: overbought/oversold levels, centerline cross, divergences")
        .with_detectors([
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, 70.0, 30.0),
            DetectorConfig::threshold("extreme", "Extreme Levels", ValueSource::Main, 80.0, 20.0),
            DetectorConfig::threshold("centerline", "Centerline Cross", ValueSource::Main, 50.0, 50.0),
            DetectorConfig::divergence("divergence", "RSI Divergence", ValueSource::Main, 14),
            DetectorConfig::swing("swing", "RSI Swings", ValueSource::Main, 5),
        ])
}

/// Stochastic default profile - %K/%D crossovers and overbought/oversold
pub fn stochastic_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Stoch, "Stochastic Default")
        .with_description("Stochastic signals: K/D crossovers, overbought/oversold zones")
        .with_detectors([
            DetectorConfig::crossover("kd_cross", "%K/%D Crossover", ValueSource::First, ValueSource::Second),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::First, 80.0, 20.0),
            DetectorConfig::divergence("divergence", "Stochastic Divergence", ValueSource::First, 14),
        ])
}

/// Stochastic RSI profile
pub fn stoch_rsi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::StochRsi, "StochRSI Default")
        .with_description("Stochastic RSI: K/D crossovers and overbought/oversold")
        .with_detectors([
            DetectorConfig::crossover("kd_cross", "K/D Crossover", ValueSource::First, ValueSource::Second),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::First, 0.8, 0.2),
        ])
}

/// CCI default profile - zero cross and extreme levels
pub fn cci_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Cci, "CCI Default")
        .with_description("CCI signals: zero line cross, overbought/oversold at +/-100")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 5.0),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, 100.0, -100.0),
            DetectorConfig::threshold("extreme", "Extreme Levels", ValueSource::Main, 200.0, -200.0),
            DetectorConfig::divergence("divergence", "CCI Divergence", ValueSource::Main, 14),
        ])
}

/// MFI default profile - similar to RSI but volume-weighted
pub fn mfi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Mfi, "MFI Default")
        .with_description("Money Flow Index: overbought/oversold levels")
        .with_detectors([
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, 80.0, 20.0),
            DetectorConfig::threshold("extreme", "Extreme Levels", ValueSource::Main, 90.0, 10.0),
            DetectorConfig::divergence("divergence", "MFI Divergence", ValueSource::Main, 14),
        ])
}

/// Williams %R default profile
pub fn williams_r_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::WilliamsR, "Williams %R Default")
        .with_description("Williams %R: overbought/oversold levels (-20/-80)")
        .with_detectors([
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, -20.0, -80.0),
            DetectorConfig::threshold("extreme", "Extreme Levels", ValueSource::Main, -10.0, -90.0),
        ])
}

/// CMO (Chande Momentum Oscillator) default profile
pub fn cmo_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Cmo, "CMO Default")
        .with_description("Chande Momentum Oscillator: zero cross and overbought/oversold")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 2.0),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, 50.0, -50.0),
        ])
}

/// ROC (Rate of Change) default profile
pub fn roc_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Roc, "ROC Default")
        .with_description("Rate of Change: zero line crossings")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 0.1),
            DetectorConfig::divergence("divergence", "ROC Divergence", ValueSource::Main, 10),
        ])
}

/// SMI (Stochastic Momentum Index) profile
pub fn smi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Smi, "SMI Default")
        .with_description("SMI: line crossover and overbought/oversold")
        .with_detectors([
            DetectorConfig::crossover("cross", "SMI Crossover", ValueSource::First, ValueSource::Second),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::First, 40.0, -40.0),
        ])
}

/// RSI Pct Bands profile
pub fn rsi_pct_bands_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::RsiPctBands, "RSI Pct Bands Default")
        .with_description("RSI Percentile Bands: RSI within channel bands")
        .with_detectors([
            DetectorConfig::threshold("ob_os", "RSI Levels", ValueSource::ChannelMiddle, 70.0, 30.0),
            DetectorConfig::channel("channel", "Band Position", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

/// Swings profile
pub fn swings_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Swings, "Swings Default")
        .with_description("Swings: high/low channel breakouts")
        .with_detectors([
            DetectorConfig::channel("channel", "Swing Channel", ValueSource::First, ValueSource::Second),
        ])
}

/// Swings Soft profile
pub fn swings_soft_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::SwingsSoft, "Swings Soft Default")
        .with_description("Swings Soft: smoothed high/low channel")
        .with_detectors([
            DetectorConfig::channel("channel", "Soft Swing Channel", ValueSource::First, ValueSource::Second),
        ])
}

/// RWI profile
pub fn rwi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Rwi, "RWI Default")
        .with_description("Random Walk Index: crossover and trend threshold")
        .with_detectors([
            DetectorConfig::crossover("cross", "RWI High/Low Cross", ValueSource::First, ValueSource::Second),
            DetectorConfig::threshold("levels", "Trend Level", ValueSource::First, 1.0, 0.5),
        ])
}

/// DIDI Index profile
pub fn didi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Didi, "DIDI Default")
        .with_description("DIDI Index: crossover and zero cross on short ratio")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::First, 0.001),
            DetectorConfig::crossover("cross", "DIDI Crossover", ValueSource::First, ValueSource::Second),
        ])
}

// ============================================================================
// Trend Indicators
// ============================================================================

/// MACD default profile - comprehensive signals
pub fn macd_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Macd, "MACD Default")
        .with_description("MACD signals: signal line cross, zero line cross, histogram changes")
        .with_detectors([
            DetectorConfig::crossover("signal_cross", "Signal Line Cross", ValueSource::MacdLine, ValueSource::MacdSignal),
            DetectorConfig::zero_cross("zero_cross", "Zero Line Cross", ValueSource::MacdLine, 0.001),
            DetectorConfig::histogram("histogram", "Histogram Direction", ValueSource::MacdHistogram),
            DetectorConfig::divergence("divergence", "MACD Divergence", ValueSource::MacdLine, 14),
        ])
}

/// ADX default profile - trend strength and DI crossovers
pub fn adx_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Adx, "ADX Default")
        .with_description("ADX signals: trend strength levels, DI crossovers")
        .with_detectors([
            DetectorConfig::threshold("trend_strength", "Trend Strength", ValueSource::Main, 25.0, 20.0),
            DetectorConfig::threshold("strong_trend", "Strong Trend", ValueSource::Main, 40.0, 15.0),
        ])
}

/// Aroon default profile
pub fn aroon_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Aroon, "Aroon Default")
        .with_description("Aroon signals: up/down crossovers, extreme levels")
        .with_detectors([
            DetectorConfig::crossover("aroon_cross", "Aroon Up/Down Cross", ValueSource::First, ValueSource::Second),
            DetectorConfig::threshold("strong_up", "Strong Uptrend", ValueSource::First, 70.0, 30.0),
            DetectorConfig::threshold("strong_down", "Strong Downtrend", ValueSource::Second, 70.0, 30.0),
        ])
}

/// SuperTrend default profile
pub fn supertrend_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Supertrend, "SuperTrend Default")
        .with_description("SuperTrend signals: trend direction changes")
        .with_detectors([
            DetectorConfig::zero_cross("direction", "Trend Direction Change", ValueSource::Main, 0.5),
        ])
}

/// Parabolic SAR default profile
pub fn psar_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Psar, "Parabolic SAR Default")
        .with_description("Parabolic SAR: trend reversal signals")
        .with_detectors([
            DetectorConfig::zero_cross("reversal", "SAR Reversal", ValueSource::Main, 0.1),
        ])
}

/// SSL Channel profile
pub fn ssl_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ssl, "SSL Default")
        .with_description("SSL Channel: up/down line crossovers")
        .with_detectors([
            DetectorConfig::crossover("cross", "SSL Crossover", ValueSource::First, ValueSource::Second),
        ])
}

/// Alligator profile
pub fn alligator_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Alligator, "Alligator Default")
        .with_description("Alligator: Lips cross Jaw signal")
        .with_detectors([
            DetectorConfig::crossover("lips_jaw", "Lips/Jaw Cross", ValueSource::First, ValueSource::Third),
        ])
}

/// Elder Impulse + Trend profile
pub fn eit_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Eit, "EIT Default")
        .with_description("Elder Impulse Trend: composite histogram trend score")
        .with_detectors([
            DetectorConfig::histogram("signal", "EIT Signal", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Zero Cross", ValueSource::Main, 0.5),
        ])
}

/// Kalman Regime profile
pub fn kregime_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Kregime, "Kalman Regime Default")
        .with_description("Kalman Regime: regime change signals")
        .with_detectors([
            DetectorConfig::histogram("signal", "Regime Signal", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Regime Change", ValueSource::Main, 0.1),
        ])
}

// ============================================================================
// Channel Indicators
// ============================================================================

/// Bollinger Bands default profile
pub fn bollinger_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Bb, "Bollinger Bands Default")
        .with_description("Bollinger Bands: band touches, squeeze, expansion")
        .with_detectors([
            DetectorConfig::channel("band_touch", "Band Touch", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

/// Keltner Channel default profile
pub fn keltner_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Kc, "Keltner Channel Default")
        .with_description("Keltner Channel: channel breakouts and returns")
        .with_detectors([
            DetectorConfig::channel("channel", "Channel Position", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

/// Donchian Channel default profile
pub fn donchian_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Dc, "Donchian Channel Default")
        .with_description("Donchian Channel: breakout signals")
        .with_detectors([
            DetectorConfig::channel("breakout", "Channel Breakout", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

/// ATR Channel default profile
pub fn atr_channel_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Atrchan, "ATR Channel Default")
        .with_description("ATR-based channel: volatility envelope signals")
        .with_detectors([
            DetectorConfig::channel("channel", "ATR Channel", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

/// Darvas Box profile
pub fn darvas_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Darvas, "Darvas Box Default")
        .with_description("Darvas Box: box breakout signals")
        .with_detectors([
            DetectorConfig::channel("box", "Box Breakout", ValueSource::First, ValueSource::Second),
        ])
}

/// BB Metrics profile
pub fn bbmetrics_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Bbmetrics, "BB Metrics Default")
        .with_description("BB Metrics: %B levels and bandwidth changes")
        .with_detectors([
            DetectorConfig::threshold("pct_b", "%B Levels", ValueSource::First, 1.0, 0.0),
            DetectorConfig::swing("bw_swing", "Bandwidth Swings", ValueSource::Second, 5),
        ])
}

/// Pivot Channel profile
pub fn pivot_chan_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Pivotchan, "Pivot Channel Default")
        .with_description("Pivot Channel: R1/S1 channel overlay")
        .with_detectors([
            DetectorConfig::channel("channel", "Pivot Channel", ValueSource::ChannelUpper, ValueSource::ChannelLower),
        ])
}

// ============================================================================
// Ichimoku
// ============================================================================

/// Ichimoku Cloud default profile - comprehensive signals
pub fn ichimoku_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ichimoku, "Ichimoku Default")
        .with_description("Ichimoku: TK cross, price vs cloud, cloud twist")
        .with_detectors([
            DetectorConfig::crossover("tk_cross", "Tenkan/Kijun Cross", ValueSource::IchimokuTenkan, ValueSource::IchimokuKijun),
            DetectorConfig::crossover("cloud_twist", "Cloud Twist (Senkou A/B)", ValueSource::IchimokuSenkouA, ValueSource::IchimokuSenkouB),
        ])
}

// ============================================================================
// Volume Indicators
// ============================================================================

/// OBV default profile
pub fn obv_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Obv, "OBV Default")
        .with_description("On Balance Volume: trend confirmation and divergences")
        .with_detectors([
            DetectorConfig::divergence("divergence", "OBV Divergence", ValueSource::Main, 14),
            DetectorConfig::swing("swing", "OBV Swings", ValueSource::Main, 5),
        ])
}

/// VWAP default profile
pub fn vwap_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Vwap, "VWAP Default")
        .with_description("VWAP: price crossover signals")
        .with_detectors([
            DetectorConfig::swing("swing", "VWAP Swings", ValueSource::Main, 5),
        ])
}

/// CMF default profile
pub fn cmf_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Cmf, "CMF Default")
        .with_description("Chaikin Money Flow: zero line cross and accumulation/distribution")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 0.01),
            DetectorConfig::threshold("strong", "Strong Flow", ValueSource::Main, 0.25, -0.25),
        ])
}

/// Accumulation/Distribution default profile
pub fn ad_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ad, "A/D Default")
        .with_description("Accumulation/Distribution: divergence signals")
        .with_detectors([
            DetectorConfig::divergence("divergence", "A/D Divergence", ValueSource::Main, 14),
        ])
}

/// KVO (Klinger Volume Oscillator) profile
pub fn kvo_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Kvo, "KVO Default")
        .with_description("Klinger Volume Oscillator: line crossover and zero cross")
        .with_detectors([
            DetectorConfig::crossover("cross", "KVO Crossover", ValueSource::First, ValueSource::Second),
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::First, 1000.0),
        ])
}

/// NVI/PVI profile
pub fn nvi_pvi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::NviPvi, "NVI/PVI Default")
        .with_description("NVI/PVI: NVI cross PVI signal")
        .with_detectors([
            DetectorConfig::crossover("cross", "NVI/PVI Crossover", ValueSource::First, ValueSource::Second),
        ])
}

/// Volume Delta profile
pub fn vdelta_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Vdelta, "Volume Delta Default")
        .with_description("Volume Delta: histogram sign changes and zero cross")
        .with_detectors([
            DetectorConfig::histogram("signal", "Volume Delta Signal", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Zero Cross", ValueSource::Main, 100.0),
        ])
}

/// Book Imbalance profile
pub fn book_imb_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::BookImb, "Book Imbalance Default")
        .with_description("Book Imbalance: histogram and threshold signals")
        .with_detectors([
            DetectorConfig::histogram("signal", "Imbalance Signal", ValueSource::Main),
            DetectorConfig::threshold("levels", "Imbalance Levels", ValueSource::Main, 0.3, -0.3),
        ])
}

/// OFI profile
pub fn ofi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ofi, "OFI Default")
        .with_description("Order Flow Imbalance: histogram and zero cross")
        .with_detectors([
            DetectorConfig::histogram("signal", "OFI Signal", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Zero Cross", ValueSource::Main, 100.0),
        ])
}

/// Queue Imbalance profile
pub fn queue_imb_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::QueueImb, "Queue Imbalance Default")
        .with_description("Queue Imbalance: histogram and threshold signals")
        .with_detectors([
            DetectorConfig::histogram("signal", "Queue Signal", ValueSource::Main),
            DetectorConfig::threshold("levels", "Queue Levels", ValueSource::Main, 0.3, -0.3),
        ])
}

/// CL Queue Imbalance profile
pub fn cl_queue_imb_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::ClQueueImb, "CL Queue Imbalance Default")
        .with_description("CL Queue Imbalance: histogram and zero cross")
        .with_detectors([
            DetectorConfig::histogram("signal", "CL Queue Signal", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Zero Cross", ValueSource::Main, 0.1),
        ])
}

// ============================================================================
// Volatility Indicators
// ============================================================================

/// ATR default profile
pub fn atr_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Atr, "ATR Default")
        .with_description("Average True Range: volatility expansion/contraction")
        .with_detectors([
            DetectorConfig::swing("volatility", "Volatility Changes", ValueSource::Main, 5),
        ])
}

/// Squeeze Momentum profile
pub fn sqmom_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Sqmom, "Squeeze Momentum Default")
        .with_description("Squeeze Momentum: histogram and zero cross")
        .with_detectors([
            DetectorConfig::histogram("signal", "Squeeze Signal", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Zero Cross", ValueSource::Main, 0.01),
        ])
}

// ============================================================================
// Levels / Support-Resistance
// ============================================================================

/// AVWAP profile
pub fn avwap_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Avwap, "AVWAP Default")
        .with_description("Anchored VWAP: price crossover and swing levels")
        .with_detectors([
            DetectorConfig::price_crossover("crossover", "Price Cross AVWAP"),
            DetectorConfig::swing("swing", "AVWAP Swings", ValueSource::Main, 5),
        ])
}

/// AVWAP Reversal profile
pub fn avwap_rev_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Avwaprev, "AVWAP Reversal Default")
        .with_description("AVWAP Reversal: price crossover signal")
        .with_detectors([
            DetectorConfig::price_crossover("crossover", "Price Cross AVWAP Rev"),
            DetectorConfig::swing("swing", "AVWAP Rev Swings", ValueSource::Main, 5),
        ])
}

/// Pivot AVWAP profile
pub fn piv_avwap_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Pivavwap, "Pivot AVWAP Default")
        .with_description("Pivot AVWAP: price crossover and swing")
        .with_detectors([
            DetectorConfig::price_crossover("crossover", "Price Cross Pivot AVWAP"),
            DetectorConfig::swing("swing", "Pivot AVWAP Swings", ValueSource::Main, 5),
        ])
}

/// FVG (Fair Value Gap) profile
pub fn fvg_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Fvg, "FVG Default")
        .with_description("Fair Value Gap: gap zone channel")
        .with_detectors([
            DetectorConfig::channel("gap_zone", "FVG Zone", ValueSource::First, ValueSource::Second),
        ])
}

/// CPR (Central Pivot Range) profile
pub fn cpr_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Cpr, "CPR Default")
        .with_description("Central Pivot Range: BC/TC channel")
        .with_detectors([
            DetectorConfig::channel("channel", "CPR Channel", ValueSource::Second, ValueSource::Third),
        ])
}

/// Range Quartiles profile
pub fn range_quart_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Rquart, "Range Quartiles Default")
        .with_description("Range Quartiles: Q1/Q3 channel")
        .with_detectors([
            DetectorConfig::channel("channel", "Q1/Q3 Channel", ValueSource::First, ValueSource::Third),
        ])
}

// ============================================================================
// Moving Averages
// ============================================================================

/// Generic moving average profile
pub fn ma_profile(indicator_id: BarIndicatorId) -> SignalProfile {
    SignalProfile::new(indicator_id, format!("{:?} Default", indicator_id))
        .with_description("Moving average: price crossover signals")
        .with_detectors([
            DetectorConfig::price_crossover("crossover", "Price Crossover MA"),
        ])
}

// ============================================================================
// Other Oscillators
// ============================================================================

/// TSI default profile
pub fn tsi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Tsi, "TSI Default")
        .with_description("True Strength Index: zero cross and signal line")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 2.0),
            DetectorConfig::threshold("ob_os", "Overbought/Oversold", ValueSource::Main, 25.0, -25.0),
        ])
}

/// PPO default profile
pub fn ppo_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ppo, "PPO Default")
        .with_description("Percentage Price Oscillator: zero cross")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 0.1),
        ])
}

/// Awesome Oscillator default profile
pub fn ao_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ao, "AO Default")
        .with_description("Awesome Oscillator: zero cross and saucer signals")
        .with_detectors([
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 0.1),
            DetectorConfig::histogram("histogram", "AO Histogram", ValueSource::Main),
        ])
}

/// Accelerator Oscillator profile
pub fn ac_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Ac, "AC Default")
        .with_description("Accelerator Oscillator: histogram and zero cross")
        .with_detectors([
            DetectorConfig::histogram("signal", "AC Histogram", ValueSource::Main),
            DetectorConfig::zero_cross("zero", "Zero Line Cross", ValueSource::Main, 0.01),
        ])
}

/// Elder Ray profile
pub fn elder_ray_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::ElderRay, "Elder Ray Default")
        .with_description("Elder Ray: Bull/Bear power histogram and zero cross")
        .with_detectors([
            DetectorConfig::histogram("signal", "Elder Ray Signal", ValueSource::First),
            DetectorConfig::zero_cross("zero", "Bull Power Zero Cross", ValueSource::First, 0.1),
        ])
}

// ============================================================================
// Chaos / Fractal
// ============================================================================

/// Fractals profile
pub fn fractals_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Fractals, "Fractals Default")
        .with_description("Fractals: fractal high/low swing pivots")
        .with_detectors([
            DetectorConfig::swing("swing", "Fractal Pivots", ValueSource::First, 5),
        ])
}

/// Williams MFI profile
pub fn williams_mfi_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::WilliamsMfi, "Williams MFI Default")
        .with_description("Williams Market Facilitation Index: histogram signal")
        .with_detectors([
            DetectorConfig::histogram("signal", "Williams MFI Signal", ValueSource::Main),
            DetectorConfig::swing("swing", "MFI Swings", ValueSource::Main, 5),
        ])
}

// ============================================================================
// Trend Stop
// ============================================================================

/// Chandelier Exit profile
pub fn chand_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Chand, "Chandelier Exit Default")
        .with_description("Chandelier Exit: long stop price crossover")
        .with_detectors([
            DetectorConfig::crossover("cross", "Long Exit Cross", ValueSource::Price, ValueSource::First),
        ])
}

/// Donchian Breakout profile
pub fn donbo_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::Donbo, "Donchian Breakout Default")
        .with_description("Donchian Breakout: upper/lower channel")
        .with_detectors([
            DetectorConfig::channel("channel", "Donchian Channel", ValueSource::First, ValueSource::Second),
        ])
}

/// TS Swings profile
pub fn ts_swings_profile() -> SignalProfile {
    SignalProfile::new(BarIndicatorId::TsSwings, "TS Swings Default")
        .with_description("TS Swings: high/low stop channel")
        .with_detectors([
            DetectorConfig::channel("channel", "TS Swings Channel", ValueSource::First, ValueSource::Second),
        ])
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile_exists() {
        assert!(default_profile(BarIndicatorId::Rsi).is_some());
        assert!(default_profile(BarIndicatorId::Macd).is_some());
        assert!(default_profile(BarIndicatorId::Bb).is_some());
        assert!(default_profile(BarIndicatorId::Adx).is_some());
        assert!(default_profile(BarIndicatorId::Stoch).is_some());
        assert!(default_profile(BarIndicatorId::Ichimoku).is_some());
    }

    #[test]
    fn test_unknown_indicator_returns_none() {
        assert!(default_profile(BarIndicatorId::Session).is_none());
        assert!(default_profile(BarIndicatorId::Wcomp).is_none());
        assert!(default_profile(BarIndicatorId::DomWoq).is_none());
    }

    #[test]
    fn test_rsi_profile_has_expected_detectors() {
        let profile = rsi_profile();
        assert!(profile.get_detector("ob_os").is_some());
        assert!(profile.get_detector("extreme").is_some());
        assert!(profile.get_detector("divergence").is_some());
    }

    #[test]
    fn test_macd_profile_has_expected_detectors() {
        let profile = macd_profile();
        assert!(profile.get_detector("signal_cross").is_some());
        assert!(profile.get_detector("zero_cross").is_some());
        assert!(profile.get_detector("histogram").is_some());
        assert!(profile.get_detector("divergence").is_some());
    }

    #[test]
    fn test_profile_customization() {
        let mut profile = rsi_profile();
        profile.update_threshold("ob_os", 80.0, 20.0);
        let detector = profile.get_detector("ob_os").unwrap();
        if let super::super::config::DetectorParams::Threshold { upper, lower, .. } =
            &detector.params
        {
            assert_eq!(*upper, 80.0);
            assert_eq!(*lower, 20.0);
        }
    }

    #[test]
    fn test_ma_variants() {
        for id in [
            BarIndicatorId::Sma,
            BarIndicatorId::Ema,
            BarIndicatorId::Wma,
            BarIndicatorId::Dema,
            BarIndicatorId::Tema,
            BarIndicatorId::Alma,
            BarIndicatorId::Jma,
            BarIndicatorId::T3,
        ] {
            assert!(default_profile(id).is_some(), "Missing profile for {:?}", id);
        }
    }

    #[test]
    fn test_new_oscillator_profiles() {
        assert!(default_profile(BarIndicatorId::StochRsi).is_some());
        assert!(default_profile(BarIndicatorId::ConnorsRsi).is_some());
        assert!(default_profile(BarIndicatorId::Qqe).is_some());
        assert!(default_profile(BarIndicatorId::Pmo).is_some());
        assert!(default_profile(BarIndicatorId::Kst).is_some());
        assert!(default_profile(BarIndicatorId::Stc).is_some());
    }

    #[test]
    fn test_channel_profiles() {
        assert!(default_profile(BarIndicatorId::Adaptivebb).is_some());
        assert!(default_profile(BarIndicatorId::Starc).is_some());
        assert!(default_profile(BarIndicatorId::Envelope).is_some());
        assert!(default_profile(BarIndicatorId::Regchan).is_some());
    }

    #[test]
    fn test_candle_patterns() {
        assert!(default_profile(BarIndicatorId::Doji).is_some());
        assert!(default_profile(BarIndicatorId::Engulfing).is_some());
        assert!(default_profile(BarIndicatorId::Hammer).is_some());
        assert!(default_profile(BarIndicatorId::Marubozu).is_some());
    }

    #[test]
    fn test_divergence_profiles() {
        assert!(default_profile(BarIndicatorId::RsiDiv).is_some());
        assert!(default_profile(BarIndicatorId::MacdDiv).is_some());
        assert!(default_profile(BarIndicatorId::ObvDiv).is_some());
    }

    #[test]
    fn test_volatility_profiles() {
        assert!(default_profile(BarIndicatorId::Chop).is_some());
        assert!(default_profile(BarIndicatorId::Hurst).is_some());
        assert!(default_profile(BarIndicatorId::Sqmom).is_some());
        assert!(default_profile(BarIndicatorId::Atrp).is_some());
    }

    #[test]
    fn test_trend_stop_profiles() {
        assert!(default_profile(BarIndicatorId::Atrts).is_some());
        assert!(default_profile(BarIndicatorId::Chand).is_some());
        assert!(default_profile(BarIndicatorId::Donbo).is_some());
        assert!(default_profile(BarIndicatorId::TsSwings).is_some());
    }

    #[test]
    fn test_stat_profiles() {
        assert!(default_profile(BarIndicatorId::Adf).is_some());
        assert!(default_profile(BarIndicatorId::Kpss).is_some());
        assert!(default_profile(BarIndicatorId::RSquared).is_some());
        assert!(default_profile(BarIndicatorId::Vr).is_some());
    }
}
