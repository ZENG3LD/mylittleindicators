//! Unified instance factory for bar indicators used by ML/features
//! Provides `IndicatorConfig` and `IndicatorInstance` with constructors in indicators layer
//!
//! # Source Type Routing
//!
//! This factory intelligently routes indicator creation based on the indicator's data requirements:
//!
//! - **PriceOnly indicators**: Accept a user-configurable source (OHLCV field selection)
//!   - Examples: SMA, EMA, RSI, Bollinger Bands
//!   - UI shows source dropdown (Close, Open, High, Low, HL/2, HLC/3, OHLC/4)
//!   - Factory calls `.with_source(period, config.source)`
//!
//! - **VolumeOnly indicators**: Use volume data exclusively (no source selection)
//!   - Examples: OBV, Volume Delta, Volume Profile
//!   - UI hides source dropdown
//!   - Factory calls `.new(period)` - volume is accessed internally
//!
//! - **PriceAndVolume indicators**: Combine price and volume with internal formulas
//!   - Examples: VWAP, VWMA, MFI, Chaikin Money Flow
//!   - UI hides source dropdown
//!   - Factory calls `.new(period)` - indicators use specific OHLCV fields internally
//!
//! Use `IndicatorConfig::requires_source_selection()` to determine if the UI should
//! display source selection controls for a given indicator.

use std::collections::HashMap;

use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

// Averages
use crate::bar_indicators::average::{
    alma::Alma,
    ama::Ama,
    dema::Dema,
    ehlers_fractal_adaptive_ma::EhlersFractalAdaptiveMa,
    ehlers_zero_lag_ema::EhlersZeroLagEma,
    ema::Ema,
    frama::Frama,
    hma::Hma,
    jurik_ma::JurikMa,
    lr::Lr,
    mcginley_dynamic::McGinleyDynamic,
    moving_average::{MovingAverageProvider, MovingAverageType},
    rma::Rma,
    sma::Sma,
    t3::T3,
    tema::Tema,
    tma::Tma,
    trima::Trima,
    vidya::Vidya,
    vwap::Vwap,
    vwma::Vwma,
    wma::Wma,
};

// Momentum
use crate::bar_indicators::accumulation::accumulative_swing_index::AccumulativeSwingIndex;
use crate::bar_indicators::accumulation::chaikin_money_flow::ChaikinMoneyFlow;
use crate::bar_indicators::accumulation::chaikin_oscillator::ChaikinOscillator;
use crate::bar_indicators::accumulation::demand_index::DemandIndex;
use crate::bar_indicators::accumulation::ease_of_movement::EaseOfMovement;
use crate::bar_indicators::accumulation::force_index::ForceIndex;
use crate::bar_indicators::accumulation::intraday_intensity::IntradayIntensity;
use crate::bar_indicators::accumulation::williams_ad::WilliamsAd;
use crate::bar_indicators::momentum::adx::Adx as AdxIndicator;
use crate::bar_indicators::momentum::di_plus_minus::DiPlusMinus;
use crate::bar_indicators::momentum::{
    amat::Amat, bias::Bias, cci::Cci, cmo::Cmo, macd::Macd, obv::Obv, roc::Roc, rsi::Rsi,
    stochastics::Stochastics,
};
use crate::bar_indicators::momentum::{
    apo::Apo, bop::Bop, cfo::Cfo, connors_rsi::ConnorsRsi, coppock::CoppockCurve,
    demarker::Demarker, detrended_synthetic_price::DetrendedSyntheticPrice,
    dpo::DetrendedPriceOscillator, dpo_percent::DpoPercent, dss_bressert::DssBressert,
    elder_impulse::ElderImpulseSystem, elder_ray::ElderRay, fisher_transform::FisherTransform, ift_rsi::IftRsi,
    intraday_momentum_index::IntradayMomentumIndex, kdj::Kdj, kst::KnowSureThing,
    laguerre_rsi::LaguerreRsi, parabolic_sar::ParabolicSAR, pmo::Pmo, qqe::Qqe, qstick::Qstick,
    rmi::Rmi, rsi_percentile_bands::RsiPercentileBands, rsi_percentile_rank::RsiPercentileRank,
    rsx::Rsx, rvgi::Rvgi, rwi::Rwi, smi::Smi, stc::Stc, stochastic_rsi::StochasticRsi, trix::Trix,
    tsi::TrueStrengthIndex, ultimate_oscillator::UltimateOscillator,
    ultimate_oscillator_smooth::UltimateOscillatorSmooth, vortex_indicator::VortexIndicator,
    williams_r::WilliamsR,
};
use crate::bar_indicators::momentum::{
    dm::Dm, psl::Psl, stochastikd::StochastikD, swings::Swings, swings_soft::SwingsSoft,
    vhf::Vhf as VhfSimple, vhf_ma::VhfMa,
};

// Volatility
use crate::bar_indicators::volatility::{
    
    atr::Atr, atr_channels::AtrChannels as AtrBands, bipower_variance::BipowerVariance,
    chaikin_volatility::ChaikinVolatility, choppiness_index::ChoppinessIndex, dc::Dc,
    fuzzy::FuzzyCandlesticks, har_rv::HarRv, hv_c2c::HistoricalVolatilityC2C, kc::Kc, kp::Kp,
    mass_index::MassIndex, natr::Natr, realized_quarticity::RealizedQuarticity, rvi::Rvi,
    squeeze_momentum::SqueezeMomentum, true_range::TrueRange, ulcer_index::UlcerIndex,
    volatility_percentile_rank_bands::VolatilityPercentileRankBands, vr::Vr, wvf::Wvf,
};
// duplicate cleanup done above

// Channels / Adaptive
use crate::bar_indicators::channels::adaptive_bollinger_bands::AdaptiveBollingerBands;
use crate::bar_indicators::channels::atr_channels::AtrChannelMode;
use crate::bar_indicators::channels::adaptive_channels::{
    AdaptationMode, AdaptiveChannels, CenterLineType,
};
use crate::bar_indicators::channels::bollinger_bands::BollingerBands;
use crate::bar_indicators::channels::darvas_box::DarvasBox;
use crate::bar_indicators::channels::donchian_channel::DonchianChannel;
use crate::bar_indicators::channels::donchian_position::DonchianPosition;
use crate::bar_indicators::channels::donchian_width::DonchianWidth;
use crate::bar_indicators::channels::dpo_bands::DpoBands;
use crate::bar_indicators::channels::envelope_bandwidth::EnvelopeBandwidth;
use crate::bar_indicators::channels::envelope_channels::EnvelopeChannels;
use crate::bar_indicators::channels::fibonacci_channels::{
    FibonacciChannelMode, FibonacciChannels,
};
use crate::bar_indicators::channels::ichimoku_cloud::IchimokuCloud;
use crate::bar_indicators::channels::ichimoku_cloud_position::IchimokuCloudPosition;
use crate::bar_indicators::channels::ichimoku_cloud_thickness::IchimokuCloudThickness;
use crate::bar_indicators::channels::keltner_bandwidth::KeltnerBandwidth;
use crate::bar_indicators::channels::keltner_channel::KeltnerChannel;
use crate::bar_indicators::channels::keltner_distance::KeltnerDistance;
use crate::bar_indicators::channels::keltner_position::KeltnerPosition;
use crate::bar_indicators::channels::median_channel_position::MedianChannelPosition;
use crate::bar_indicators::channels::median_channels::MedianChannels;
use crate::bar_indicators::channels::bollinger_metrics::BollingerMetrics;
use crate::bar_indicators::channels::donchian_channel_metrics::DonchianMetrics;
use crate::bar_indicators::channels::keltner_channel_metrics::KeltnerMetrics;
use crate::bar_indicators::channels::percent_b::PercentB;
use crate::bar_indicators::channels::percentile_channels::{
    PercentileBasis, PercentileChannels,
};
use crate::bar_indicators::channels::pivot_channels::PivotChannels;
use crate::bar_indicators::channels::price_channel_oscillator::PriceChannelOscillator;
use crate::bar_indicators::channels::price_channel_width::PriceChannelWidth;
use crate::bar_indicators::channels::price_channels::PriceChannels;
use crate::bar_indicators::channels::projection_bands::ProjectionBands;
use crate::bar_indicators::channels::quantile_regression_channels::QuantileRegressionChannels;
use crate::bar_indicators::channels::regression_channel_width::RegressionChannelWidth;
use crate::bar_indicators::channels::regression_channels::RegressionChannels;
use crate::bar_indicators::channels::standard_deviation_channels::StandardDeviationChannels;
use crate::bar_indicators::channels::starc_bands::StarcBands;
use crate::bar_indicators::channels::stddev_channel_width::StdDevChannelWidth;
use crate::bar_indicators::channels::theil_sen_channels::TheilSenChannels;
use crate::bar_indicators::channels::trima_bands::TrimaBands;
use crate::bar_indicators::channels::volume_profile_channels::{
    VolumeProfileChannels, VolumeProfileMode, VolumeProfilePeriod,
};
use crate::bar_indicators::channels::vwap_channel_width::VwapChannelWidth;
use crate::bar_indicators::channels::vwap_channels::VwapChannels;

// BB Period
use crate::bar_indicators::momentum::bb_period::BbPeriod;

// Divergence
use crate::bar_indicators::divergence::{
    CciDivergence, ClassicDivergence, DivergenceStrength, HiddenDivergence,
    MacdDivergence, MacdHistogramDivergence, MomentumDivergence, MultiDivergence,
    ObvDivergence, RsiDivergence, StochasticDivergence, VolumeDivergence,
    WilliamsDivergence,
};

// Volume / Accumulation
use crate::bar_indicators::accumulation::accumulation_distribution::AccumulationDistribution;
use crate::bar_indicators::chaos::williams_indicators::{
    AccelerationDeceleration, Alligator, AwesomeOscillator, MarketFacilitationIndex,
};
use crate::bar_indicators::momentum::kvo::Kvo;
use crate::bar_indicators::ratio::efficiency_ratio::EfficiencyRatioFullHistory;
use crate::bar_indicators::ratio::efficiency_ratio_ring::EfficiencyRatioRingWindow;
use crate::bar_indicators::ratio::spread_analyzer::SpreadAnalyzer;
use crate::bar_indicators::statistics::adf_proxy::AdfProxy;
use crate::bar_indicators::statistics::arch_lm_proxy::ArchLmProxy;
use crate::bar_indicators::statistics::kpss_proxy::KpssProxy;
use crate::bar_indicators::statistics::ljung_box::LjungBox;
use crate::bar_indicators::statistics::pacf::Pacf;
use crate::bar_indicators::statistics::variance_ratio::VarianceRatio;
use crate::bar_indicators::statistics::variance_ratio_aggregate::VarianceRatioAggregate;
use crate::bar_indicators::trend::adx_slope::AdxSlope;
use crate::bar_indicators::trend::didi_index::DidiIndex;
use crate::bar_indicators::trend::gann_hilo_activator::GannHiLoActivator;
use crate::bar_indicators::trend::gmma_compression::GmmaCompression;
use crate::bar_indicators::trend::heikin_ashi_trend::HeikinAshiTrend;
use crate::bar_indicators::trend::ravi::Ravi as RaviTrend;
use crate::bar_indicators::trend::ssl_channel::SslChannel;
use crate::bar_indicators::trend::supertrend::Supertrend;
use crate::bar_indicators::trend::trend_intensity_index::TrendIntensityIndex;
use crate::bar_indicators::trend_stop::atr_trailing_stop::ATRTrailingStop;
use crate::bar_indicators::trend_stop::chande_kroll_stop::ChandeKrollStop;
use crate::bar_indicators::trend_stop::chandelier_stop::ChandelierStop;
use crate::bar_indicators::trend_stop::donchian_breakout::DonchianBreakout;
use crate::bar_indicators::trend_stop::donchian_stop::DonchianStop;
use crate::bar_indicators::trend_stop::keltner_stop::KeltnerStop;
use crate::bar_indicators::trend_stop::psar_stop::PSARStop;
use crate::bar_indicators::trend_stop::supertrend_stop::SuperTrendStop;
use crate::bar_indicators::trend_stop::swing_stop::SwingStop;
use crate::bar_indicators::trend_stop::volatility_stop::{
    VolatilityStop, VolatilityType,
};
use crate::bar_indicators::volume::mfi::Mfi;
use crate::bar_indicators::volume::nvi_pvi::NegativePositiveVolumeIndex;
use crate::bar_indicators::volume::trin::Trin as TrinIndicator;
use crate::bar_indicators::volume::volume_delta::VolumeDelta;
use crate::bar_indicators::volume::volume_profile::VolumeProfile;
use crate::bar_indicators::volume::vpin::Vpin;
use crate::bar_indicators::volume::vpt::VolumePriceTrend;
use crate::bar_indicators::volume::vroc::VolumeRateOfChange;
// Support/Resistance Levels (Pivots)
use crate::bar_indicators::levels::camarilla_pivots::CamarillaPivots;
use crate::bar_indicators::levels::demark_pivots::DeMarkPivots;
use crate::bar_indicators::levels::floor_trader_pivots::FloorTraderPivots;
use crate::bar_indicators::levels::pivot_points::PivotPoints;
use crate::bar_indicators::levels::woodie_pivots::WoodiePivots;
use crate::bar_indicators::position::central_pivot_range::CentralPivotRange;
use crate::bar_indicators::position::hour_of_day_effect::HourOfDayEffect;
use crate::bar_indicators::position::week_in_month_effect::WeekInMonthEffect;
use crate::bar_indicators::position::start_end_of_quarter_flags::StartEndOfQuarterFlags;
// New Phase 2 timestamp-based indicators
use crate::bar_indicators::signal_processing::time_encoders::TimeEncoders;
use crate::bar_indicators::position::weekday_effect::WeekdayEffect;
use crate::bar_indicators::position::session_effect::SessionEffect;
use crate::bar_indicators::position::month_quarter_effect::MonthQuarterEffect;
use crate::bar_indicators::position::dayofmonth_weekofquarter_effect::DayOfMonthWeekOfQuarterEffect;
use crate::bar_indicators::position::relative_trend_position::RelativeTrendPosition;
use crate::bar_indicators::position::month_turn_effect::MonthTurnEffect;
use crate::bar_indicators::position::quarter_turn_effect::QuarterTurnEffect;
// Phase 3 additions
use crate::bar_indicators::position::start_end_of_month_flags::StartEndOfMonthFlags;
use crate::bar_indicators::position::start_end_of_week_flags::StartEndOfWeekFlags;
use crate::bar_indicators::position::holiday_weekend_proximity::HolidayWeekendProximityEffect;
use crate::bar_indicators::position::day_of_week_in_month::DayOfWeekInMonthEffect;
// Calendar and Time services for timestamp support
use crate::types::CalendarService;
// Chaos / Fractal / Hurst / DFA
use crate::bar_indicators::chaos::chaos_oscillator::ChaosOscillator;
use crate::bar_indicators::chaos::dfa::Dfa;
use crate::bar_indicators::chaos::dfa_percentile::DfaPercentile;
use crate::bar_indicators::chaos::fractal_dimension::FractalDimension;
use crate::bar_indicators::chaos::hurst_exponent::HurstExponent;
use crate::bar_indicators::chaos::hurst_percentile::HurstPercentile;
use crate::bar_indicators::chaos::williams_fractals::WilliamsFractals;
// Z-Score family & EWMAC
use crate::bar_indicators::momentum::ewmac::Ewmac;
use crate::bar_indicators::momentum::macd_hist_zscore::MacdHistZscore;
use crate::bar_indicators::momentum::momentum_zscore::MomentumZscore;
use crate::bar_indicators::momentum::rsi_zscore::RsiZscore;
use crate::bar_indicators::signal_processing::zscore_price_mad::PriceMadZscore;
use crate::bar_indicators::statistics::price_zscore::PriceZScore;
// Ehlers & Filters
use crate::bar_indicators::adaptive::mesa_adaptive_ma::MesaAdaptiveMA;
use crate::bar_indicators::signal_processing::decycler::Decycler;
use crate::bar_indicators::signal_processing::ehlers_super_smoother::EhlersSuperSmoother;
use crate::bar_indicators::signal_processing::roofing_filter::RoofingFilter;
use crate::bar_indicators::trend::ehlers_instantaneous_trendline::EhlersInstantaneousTrendline;
// RSI variants
use crate::bar_indicators::momentum::atr_rsi::AtrRsi;
use crate::bar_indicators::momentum::rsioma::RsiOma;
use crate::bar_indicators::momentum::tdi::Tdi;
use crate::bar_indicators::momentum::volume_weighted_rsi::VolumeWeightedRsi;
// Logic gates and gates helpers
use crate::bar_indicators::signal_processing::logic_gates::{
    AndGate, OrGate, SignCombiner, XorGate,
};
use crate::bar_indicators::signal_processing::threshold_gate::ThresholdGate;
// Realized vol, autocorr, higher moments, ROC percentile, NR range
use crate::bar_indicators::momentum::roc_percentile::RocPercentile;
use crate::bar_indicators::signal_processing::autocorr::Autocorr;
use crate::bar_indicators::signal_processing::higher_moments::HigherMoments;
use crate::bar_indicators::volatility::nr_range::NrRange;
use crate::bar_indicators::volatility::realized_vol::RealizedVol;
use crate::bar_indicators::volatility::realized_vol_zscore::RealizedVolZscore;
// Market regime and adaptive oscillators
use crate::bar_indicators::momentum::adaptive_stochastic::AdaptiveStochastic;
use crate::bar_indicators::signal_processing::market_regime_filter::MarketRegimeFilter;
// Breakout/filters
use crate::bar_indicators::signal_processing::cusum_filter::CusumFilter;
use crate::bar_indicators::volatility::volatility_breakout_detector::VolatilityBreakoutDetector;
// Entropy/information measures
use crate::bar_indicators::entropy::fisher_information::RollingFisherInformation;
use crate::bar_indicators::entropy::information_gain::InformationGain;
use crate::bar_indicators::entropy::js_divergence::JSDivergence;
use crate::bar_indicators::entropy::kl_divergence::KLDivergence;
use crate::bar_indicators::entropy::mutual_information::MutualInformation;
use crate::bar_indicators::entropy::transfer_entropy::TransferEntropy;
use crate::bar_indicators::signal_processing::lempel_ziv::LempelZivComplexity;
// Gates/composites
use crate::bar_indicators::signal_processing::hysteresis_gate::HysteresisGate;
use crate::bar_indicators::signal_processing::weighted_composite::WeightedComposite;
// Candles/patterns
use crate::bar_indicators::candles::sfp_detector::SfpDetector;
// Momentum extras
use crate::bar_indicators::momentum::ehlers_rocket_rsi::EhlersRocketRsi;
use crate::bar_indicators::momentum::ehlers_cyber_cycle::EhlersCyberCycle;
// Trend/zero-lag
use crate::bar_indicators::trend::zl_sma::ZlSma;
use crate::bar_indicators::trend::efficiency_ratio::EfficiencyRatio;
// Distance/levels
use crate::bar_indicators::levels::hl_value_area::HlValueArea;
use crate::bar_indicators::position::distance_to_levels::DistanceToLevels;
use crate::bar_indicators::position::vwap_distance::VwapDistance;
use crate::bar_indicators::position::avwap_distance::AvwapDistance;
// Sweep reversion
use crate::bar_indicators::momentum::sweep_reversion::{
    SweepReversionIndex, SweepReversionParams,
};
// Entropy/statistics/econometrics
use crate::bar_indicators::entropy::conditional_entropy::ConditionalEntropy;
use crate::bar_indicators::statistics::cointegration_proxy::CointegrationProxy;
use crate::bar_indicators::statistics::engle_granger_adf_proxy::EngleGrangerAdfProxy;
use crate::bar_indicators::statistics::engle_granger_proxy::EngleGrangerProxy;
use crate::bar_indicators::statistics::engle_granger_trend_proxy::EngleGrangerTrendProxy;
use crate::bar_indicators::statistics::half_life_mr::HalfLifeMr;
use crate::bar_indicators::statistics::residual_stationarity::ResidualStationarity;
// Momentum/trend extras
use crate::bar_indicators::momentum::multi_timeframe_momentum_divergence::MultiTimeframeMomentumDivergence;
// Filters/channels
use crate::bar_indicators::signal_processing::hampel_filter::HampelFilter;
// (TheilSenChannels, ProjectionBands) already imported earlier if present
// BOOK/CLUSTERS/ENTROPY categories
use crate::bar_indicators::book::imbalance::BookImbalanceRatio;
use crate::bar_indicators::clusters::{
    market_microstructure::MarketMicrostructure,
    order_book_slope::OrderBookSlope,
    order_flow_imbalance::OrderFlowImbalance,
    queue_imbalance::QueueImbalance,
    tick_volume_analyzer::TickVolumeAnalyzer,
    volume_weighted_price_levels::VolumeWeightedPriceLevels,
};
use crate::bar_indicators::entropy::cross_mutual_information_lags::CrossMutualInformationLags;
// Patterns/ML-style
use crate::bar_indicators::candles::candle_anatomy::CandleAnatomy;
use crate::bar_indicators::candles::pattern_recognition::AdvancedPatternRecognition;
use crate::bar_indicators::momentum::neural_momentum_network::NeuralMomentumNetwork;
use crate::bar_indicators::volatility::adaptive_volatility_regime::AdaptiveVolatilityRegime;
// Price action / structure helpers
use crate::bar_indicators::candles::wick_spike::WickSpike;
use crate::bar_indicators::levels::liquidity_gap_density::LiquidityGapDensity;
use crate::bar_indicators::levels::swing_strength_score::SwingStrengthScore;
use crate::bar_indicators::momentum::swing_age::SwingAge;
use crate::bar_indicators::volatility::range_compression_burst::RangeCompressionBurst;
// Candlestick patterns (Batch 1)
use crate::bar_indicators::candles::patterns::{
    doji::Doji,
    engulfing::Engulfing,
    hammer::Hammer,
    harami::Harami,
    marubozu::Marubozu,
    morning_star::MorningStar,
    piercing_pattern::PiercingPattern,
    shooting_star::ShootingStar,
    three_white_soldiers::ThreeWhiteSoldiers,
    tweezer::Tweezer,
    dark_cloud_cover::DarkCloudCover,
    evening_star::EveningStar,
    three_black_crows::ThreeBlackCrows,
};
use crate::bar_indicators::candles::heikin_ashi::HeikinAshi;
// Average indicators (Batch 1) - Already imported above in the main average block
// REMOVED: OhlcvAverage imports - replaced by MovingAverageWithField
// use crate::bar_indicators::average::ohlcv_average::{...};
// OhlcvField is now imported from moving_average module above (line 22)
// Calendar/time effects excluded
use crate::bar_indicators::levels::break_of_structure::BosChochDetector;
use crate::bar_indicators::levels::fvg_detector::FvgDetector;
// ZigZag family
// ZigZag excluded
// Clusters / Orderflow / Book
// Clusters/book excluded
// Regression models
use crate::bar_indicators::regression::arima::{Arima, ArimaX};
use crate::bar_indicators::regression::garch::{EGarch, Garch};
use crate::bar_indicators::regression::polynomial::PolynomialRegression;
use crate::bar_indicators::regression::var::Var;
// Momentum extras: Aroon family, extremes, pressure, MA cross, simple candle patterns
use crate::bar_indicators::momentum::center_of_gravity::CenterOfGravity;
use crate::bar_indicators::momentum::pfe::Pfe;
use crate::bar_indicators::momentum::aroon::Aroon;
use crate::bar_indicators::momentum::aroon_down::AroonDown;
use crate::bar_indicators::momentum::aroon_oscillator::AroonOscillator;
use crate::bar_indicators::momentum::aroon_up::AroonUp;
use crate::bar_indicators::momentum::candle_patterns::CandlePatterns;
use crate::bar_indicators::momentum::highest::Highest;
use crate::bar_indicators::momentum::lowest::Lowest;
use crate::bar_indicators::momentum::ma_cross::MaCross;
use crate::bar_indicators::momentum::pressure::Pressure;
// Accumulation: Twiggs Money Flow
use crate::bar_indicators::accumulation::tmf::Tmf;
// Positioning: AVWAP Distance
// AvwapDistance excluded
// AutoFibo (requires index feed)
// AutoFibo excluded
// Signal Processing / Spectral
use crate::bar_indicators::signal_processing::butterworth::{
    ButterworthFilter, FilterType as ButterType,
};
use crate::bar_indicators::signal_processing::chebyshev::{
    ChebyshevFilter, ChebyshevType, FilterType as ChebyType,
};
use crate::bar_indicators::signal_processing::cyber_cycle::CyberCycle;
use crate::bar_indicators::signal_processing::ehlers_sinewave::EhlersSinewave;
use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::signal_processing::hilbert::HilbertTransform;
use crate::bar_indicators::signal_processing::hilbert_dominant_cycle::HilbertDominantCycle;
use crate::bar_indicators::signal_processing::savitzky_golay::{
    DerivativeOrder as SgDerivativeOrder, SavitzkyGolayFilter,
};
use crate::bar_indicators::signal_processing::spectral_bandpower::{
    SpectralBandpower,
};
use crate::bar_indicators::signal_processing::spectral_bandwidth_feature::SpectralBandwidthFeature;
use crate::bar_indicators::signal_processing::spectral_centroid_feature::SpectralCentroidFeature;
use crate::bar_indicators::signal_processing::spectral_crest::SpectralCrest;
use crate::bar_indicators::signal_processing::spectral_energy_ratio::SpectralEnergyRatio;
use crate::bar_indicators::signal_processing::spectral_entropy::SpectralEntropy;
use crate::bar_indicators::signal_processing::spectral_entropy_rate::SpectralEntropyRate;
use crate::bar_indicators::signal_processing::spectral_flatness::SpectralFlatness;
use crate::bar_indicators::signal_processing::spectral_high_mid_power_ratio::SpectralHighMidPowerRatio;
use crate::bar_indicators::signal_processing::spectral_low_mid_power_ratio::SpectralLowMidPowerRatio;
use crate::bar_indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::bar_indicators::signal_processing::spectral_slope::SpectralSlope;
use crate::bar_indicators::signal_processing::spectral_slope_zscore::SpectralSlopeZscore;
use crate::bar_indicators::signal_processing::stft_features::StftBandEnergyRatio;
use crate::bar_indicators::signal_processing::wavelet::{
    WaveletTransform, WaveletType,
};
// Kalman Filters
use crate::bar_indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::bar_indicators::kalman::extended_kalman_filter::{
    ExtendedKalmanFilter, ObservationType,
};
use crate::bar_indicators::kalman::particle_filter::{
    ParticleFilter, ResamplingStrategy,
};
use crate::bar_indicators::kalman::unscented_kalman_filter::{
    UnscentedKalmanFilter, UnscentedTransformParams,
};
// Adaptive moving averages
use crate::bar_indicators::adaptive::adaptive_moving_average::{
    AdaptationMode as AmaAdaptationMode, AdaptiveMovingAverage, EfficiencyMethod,
};
use crate::bar_indicators::adaptive::frama::{
    FractalAdaptiveMovingAverage, FractalMethod,
};
use crate::bar_indicators::adaptive::kaufman_adaptive_ma::KaufmanAdaptiveMA;
use crate::bar_indicators::adaptive::vidya::{
    CmoMaType as VidyaCmoMaType, VariableIndexDynamicAverage,
};
// Entropy
use crate::bar_indicators::entropy::approximate_entropy::ApproximateEntropy;
use crate::bar_indicators::entropy::permutation_entropy::PermutationEntropy;
use crate::bar_indicators::entropy::sample_entropy::SampleEntropy;
use crate::bar_indicators::entropy::shannon_entropy::ShannonEntropy;
// Regime composites
use crate::bar_indicators::signal_processing::regime_composite::{
    RegimeComposite, RegimeCompositeParams,
};
use crate::bar_indicators::signal_processing::regime_composite_v2::RegimeCompositeV2;
use crate::bar_indicators::signal_processing::regime_composite_v3::RegimeCompositeV3;
use crate::bar_indicators::signal_processing::regime_composite_v4::RegimeCompositeV4;
// Anchored VWAPs & levels
use crate::bar_indicators::levels::anchored_vwap::{AnchoredVwap, AnchoredVwapParams};
use crate::bar_indicators::levels::avwap_multi_anchor_reversion::AvwapMultiAnchorReversion;
use crate::bar_indicators::levels::avwap_touch_probability::AvwapTouchProbability;
use crate::bar_indicators::levels::pivot_anchored_vwap::PivotAnchoredVwap;
// Volatility percentiles / bandwidths
use crate::bar_indicators::volatility::atr_bandwidth::AtrBandwidth;
use crate::bar_indicators::volatility::atr_percentile::AtrPercentile as AtrPercentileInd;
use crate::bar_indicators::volatility::atr_zscore::AtrZscore;
use crate::bar_indicators::volatility::close_to_close_vol_percentile::CloseVolPercentile;
// Volume
use crate::bar_indicators::volume::relative_volume::RelativeVolume;
use crate::bar_indicators::volume::pvo::Pvo;
use crate::bar_indicators::volume::pzo::Pzo;
use crate::bar_indicators::volume::volume_oscillator::VolumeOscillator;
use crate::bar_indicators::volume::volume_zscore::VolumeZscore;
// Trend/levels helpers
use crate::bar_indicators::levels::rolling_midline::RollingMidline;
use crate::bar_indicators::levels::rolling_quartiles::RollingQuartiles;
use crate::bar_indicators::momentum::ema_slope::EmaSlope;
use crate::bar_indicators::ratio::range_to_atr::RangeToAtr;
use crate::bar_indicators::trend::slope_direction_line::SlopeDirectionLine;
use crate::bar_indicators::volatility::range_percentile::RangePercentile;
use crate::bar_indicators::volatility::rbv_jump_test::RbvJumpTest;
use crate::bar_indicators::volatility::vol_of_vol_percentile::VolOfVolPercentile;
use crate::bar_indicators::volatility::vol_of_vol_percentile_trend::VolOfVolPercentileTrend;
use crate::bar_indicators::volatility::vol_of_vol::{VolOfVol, VoVSource};
// ========================================
// PHASE 2 ADDITIONS
// ========================================
// ACCUMULATION (2)
use crate::bar_indicators::accumulation::intraday_intensity_percent::IntradayIntensityPercent;
use crate::bar_indicators::accumulation::intraday_intensity_ratio::IntradayIntensityRatio;
// KALMAN (8)
use crate::bar_indicators::kalman::alpha_beta_gamma_filter::AlphaBetaGammaFilter;
use crate::bar_indicators::kalman::kalman_regime_composite::KalmanRegimeComposite;
use crate::bar_indicators::kalman::kalman_regime_score::KalmanRegimeScore;
use crate::bar_indicators::kalman::kalman_slope_zscore::KalmanSlopeZscore;
use crate::bar_indicators::kalman::kalman_trend_regime::KalmanTrendRegime;
use crate::bar_indicators::kalman::kalman_trend_slope::KalmanTrendSlope;
// SIGNALPROCESSING (6 new - others already imported)
use crate::bar_indicators::signal_processing::spectral_crest_percentile::SpectralCrestPercentile;
use crate::bar_indicators::signal_processing::spectral_rolloff_95::SpectralRolloff95;
use crate::bar_indicators::signal_processing::spectral_rolloff_percentile::SpectralRolloffPercentile;
use crate::bar_indicators::signal_processing::spectral_slope_percentile::SpectralSlopePercentile;
use crate::bar_indicators::signal_processing::spectral_slope_robust_percentile::SpectralSlopeRobustPercentile;
// MOMENTUM (2)
// STATISTICS (10)
use crate::bar_indicators::statistics::adf_kpss_composite::AdfKpssComposite;
use crate::bar_indicators::statistics::arch_lm_pvalue_proxy::ArchLmPvalueProxy;
use crate::bar_indicators::statistics::bai_perron_cusum::BaiPerronCusum;
use crate::bar_indicators::statistics::kpss_trend_proxy::KpssTrendProxy;
use crate::bar_indicators::statistics::kpss_z_proxy::KpssZProxy;
use crate::bar_indicators::statistics::phillips_perron_proxy::PhillipsPerronProxy;
use crate::bar_indicators::statistics::price_volume_coherence_proxy::PriceVolumeCoherenceProxy;
use crate::bar_indicators::statistics::r_squared::RSquared;
use crate::bar_indicators::statistics::variance_ratio_z_aggregate::VarianceRatioZAggregate;
use crate::bar_indicators::statistics::zivot_andrews_proxy::ZivotAndrewsProxy;
// VOLATILITY (1 new - ChaikinVolatility, UlcerIndex, AdaptiveBollingerBands already imported)
use crate::bar_indicators::volatility::dynamic_volatility_regime::DynamicVolatilityRegime;
// ========================================
// BATCH 3 ADDITIONS (5 indicators - only standard update_bar compatible)
// ========================================
// MOMENTUM (5)
use crate::bar_indicators::momentum::ewmac_robust::EwmacRobust;
use crate::bar_indicators::momentum::gapo::Gapo;
use crate::bar_indicators::momentum::gator_oscillator::GatorOscillator;
use crate::bar_indicators::momentum::macd_histogram::MacdHistogram;
use crate::bar_indicators::momentum::macd_signal::MacdSignal;
use crate::bar_indicators::momentum::ppo::Ppo;
use crate::bar_indicators::momentum::ppo_signal::PpoSignal;

// ========================================
// BATCH 4 ADDITIONS (11 indicators - only standard update_bar compatible)
// ========================================
// SIGNAL_PROCESSING (5) - spectral indicators already imported above, WeightedComposite already imported
// TREND (2)
use crate::bar_indicators::trend::kama_slope::KamaSlope;
use crate::bar_indicators::trend::lr_slope::LrSlope;
// KALMAN (1)
use crate::bar_indicators::kalman::rts_smoother::RtsSmoother;
// VOLUME (2)
use crate::bar_indicators::volume::vfi::Vfi;
use crate::bar_indicators::volume::vzo::Vzo;

// ========================================
// PHASE 4 ADDITIONS (9 indicators - Poc + AutoFibo + 7 ZigZag variants)
// ========================================
// POC (1)
use crate::bar_indicators::volume::poc_detector::PocDetector;
// ZIGZAG FAMILY (5 main variants)
use crate::bar_indicators::zigzag::zigzag_classic::ZigZagClassic;
use crate::bar_indicators::zigzag::zigzag_atr::ZigZagAtr;
use crate::bar_indicators::zigzag::zigzag_candle::ZigZagCandle;
use crate::bar_indicators::zigzag::zigzag_lookahead::ZigZagLookahead;
use crate::bar_indicators::zigzag::zigzag_time::ZigZagTime;
// AUTO FIBO (1)
use crate::bar_indicators::momentum::auto_fibo::AutoFibo;
// ========================================
// PHASE 5 ADDITIONS (3 FVG indicators - Final push to 100%)
// ========================================
// FVG FAMILY (3 indicators)
use crate::bar_indicators::levels::fvg_intensity_alt_score::FvgIntensityAltScore;
use crate::bar_indicators::levels::fvg_duration_intensity_score::FvgDurationIntensityScore;
use crate::bar_indicators::levels::fvg_reversion_probability::FvgReversionProbability;

/// Configuration for an individual component within a composite indicator
///
/// Allows per-component customization of source field, MA type, and period
/// for indicators like MACD, Keltner Channels, Bollinger Bands, etc.
///
/// # Example
/// ```
/// use zengeld_chart_indicators::bar_indicators::instance_factory::ComponentConfig;
/// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
/// use zengeld_chart_indicators::bar_indicators::average::MovingAverageType;
///
/// let fast_ma_config = ComponentConfig {
///     source: Some(OhlcvField::High),
///     ma_type: Some(MovingAverageType::EMA),
///     period: Some(12),
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct ComponentConfig {
    /// Source field for this component (overrides main source if set)
    pub source: Option<OhlcvField>,
    /// MA type for this component (overrides main ma_type if set)
    pub ma_type: Option<MovingAverageType>,
    /// Period for this component (overrides main period if set)
    pub period: Option<usize>,
}

/// Configuration for constructing an indicator instance
#[derive(Debug, Clone)]
pub struct IndicatorConfig {
    pub id: BarIndicatorId,
    pub name: String,
    pub periods: Vec<usize>,
    pub additional_params: HashMap<String, f64>,
    pub ma_types: HashMap<String, MovingAverageType>,  // Named MA types (use "ma_type" for single MA indicators)
    pub flags: HashMap<String, bool>,  // Boolean flags (e.g., "auto_mode")
    pub source: OhlcvField,  // OHLCV field to use as input (default: Close)
    pub component_configs: HashMap<String, ComponentConfig>,  // Per-component configuration (e.g., "fast_ma", "slow_ma")
}

impl IndicatorConfig {
    /// Check if this indicator requires source field selection
    ///
    /// Returns true for PriceOnly indicators that allow the user to select
    /// which OHLCV field (Open, High, Low, Close, HL/2, etc.) to use as input.
    ///
    /// Returns false for VolumeOnly and PriceAndVolume indicators that have
    /// their own internal logic and don't expose source selection to the user.
    pub fn requires_source_selection(&self) -> bool {
        IndicatorInstance::uses_configurable_source(self.id)
    }

    pub fn new(id: BarIndicatorId, name: String, periods: Vec<usize>) -> Self {
        Self {
            id,
            name,
            periods,
            additional_params: HashMap::new(),
            ma_types: HashMap::new(),
            flags: HashMap::new(),
            source: OhlcvField::Close,
            component_configs: HashMap::new(),
        }
    }
    pub fn with_param(mut self, key: impl Into<String>, value: f64) -> Self {
        self.additional_params.insert(key.into(), value);
        self
    }
    /// Sets a boolean flag
    pub fn with_flag(mut self, key: impl Into<String>, value: bool) -> Self {
        self.flags.insert(key.into(), value);
        self
    }
    /// Sets MA type for indicators with single MA (use "ma_type" as key)
    pub fn with_ma_type(mut self, ma_type: MovingAverageType) -> Self {
        self.ma_types.insert("ma_type".to_string(), ma_type);
        self
    }
    /// Sets named MA type for indicators with multiple independent MA types
    pub fn with_named_ma_type(mut self, name: impl Into<String>, ma_type: MovingAverageType) -> Self {
        self.ma_types.insert(name.into(), ma_type);
        self
    }
    /// Sets the OHLCV field to use as input source
    pub fn with_source(mut self, source: OhlcvField) -> Self {
        self.source = source;
        self
    }
}


/// Unified wrapper over concrete indicator implementations with scalar output
#[derive(Clone)]
pub enum IndicatorInstance {
    // Core simple
    Sma(Sma),
    Ema(Ema),
    Rsi(Rsi),
    Macd(Macd),
    BbPeriod(BbPeriod),
    Atr(Atr),

    // Boxed to reduce stack usage (O(1) optimized versions)
    Rma(Box<Rma>),
    Hma(Box<Hma>),  // O(1) composite using Wma
    Wma(Box<Wma>),  // O(1) running sums
    Dema(Box<Dema>),
    Ama(Box<Ama>),  // O(1) ring window ER
    Tma(Box<Tma>),
    Tema(Box<Tema>),
    AvFrama(Box<Frama>),
    Lr(Box<Lr>),
    AvVidya(Box<Vidya>),
    MovingAverage(Box<MovingAverageProvider>),
    Vwap(Box<Vwap>),
    Vwma(Box<Vwma>),
    Alma(Box<Alma>),
    T3(Box<T3>),
    McGinley(Box<McGinleyDynamic>),
    Trima(Box<Trima>),
    JurikMa(Box<JurikMa>),
    EhlersFractalAdaptiveMa(Box<EhlersFractalAdaptiveMa>),
    EhlersZeroLagEma(Box<EhlersZeroLagEma>),

    // Channels / Adaptive
    AdaptiveBollingerBands(Box<AdaptiveBollingerBands>),
    BollingerBands(Box<BollingerBands>),
    DonchianChannel(Box<DonchianChannel>),
    KeltnerChannel(Box<KeltnerChannel>),
    Dcwidth(Box<DonchianWidth>),
    DonchianPosition(Box<DonchianPosition>),
    KeltnerBandwidth(Box<KeltnerBandwidth>),
    Keltpos(Box<KeltnerPosition>),
    Keltdist(Box<KeltnerDistance>),
    PriceChannels(Box<PriceChannels>),
    Pchwidth(Box<PriceChannelWidth>),
    Pchosc(Box<PriceChannelOscillator>),
    VwapChannels(Box<VwapChannels>),
    VwapChannelWidth(Box<VwapChannelWidth>),
    RegressionChannels(Box<RegressionChannels>),
    RegressionChannelWidth(Box<RegressionChannelWidth>),
    EnvelopeChannels(Box<EnvelopeChannels>),
    Envbw(Box<EnvelopeBandwidth>),
    StandardDeviationChannels(Box<StandardDeviationChannels>),
    StdDevChannelWidth(Box<StdDevChannelWidth>),
    StarcBands(Box<StarcBands>),
    AdaptiveChannels(Box<AdaptiveChannels>),
    IchimokuCloud(Box<IchimokuCloud>),
    IchimokuCloudThickness(Box<IchimokuCloudThickness>),
    IchimokuCloudPosition(Box<IchimokuCloudPosition>),
    Pivotchan(Box<PivotChannels>),
    Medchan(Box<MedianChannels>),
    Medchanpos(Box<MedianChannelPosition>),
    QuantileRegressionChannels(Box<QuantileRegressionChannels>),
    TrimaBands(Box<TrimaBands>),
    DpoBands(Box<DpoBands>),
    Percentilech(Box<PercentileChannels>),
    TheilSenChannels(Box<TheilSenChannels>),
    ProjectionBands(Box<ProjectionBands>),
    VolumeProfileChannels(Box<VolumeProfileChannels>),
    FibonacciChannels(Box<FibonacciChannels>),
    DarvasBox(Box<DarvasBox>),

    // Momentum extras
    Roc(Box<Roc>),
    Cci(Box<Cci>),
    Stochastics(Box<Stochastics>),
    Obv(Box<Obv>),
    Vhf(Box<VhfSimple>),
    Swings(Box<Swings>),
    Psl(Box<Psl>),
    // BOOK category (4 indicators)
    BookImb(Box<BookImbalanceRatio>),
    BookSlope(Box<OrderBookSlope>),
    Ofi(Box<OrderFlowImbalance>),
    QueueImb(Box<QueueImbalance>),
    // CLUSTERS category (6 indicators)
    ClQueueImb(Box<QueueImbalance>),
    MarketMicro(Box<MarketMicrostructure>),
    OrderBookSlope(Box<OrderBookSlope>),
    OrderFlowImb(Box<OrderFlowImbalance>),
    TickVolume(Box<TickVolumeAnalyzer>),
    VwapLevels(Box<VolumeWeightedPriceLevels>),
    // ENTROPY category (2 indicators)
    Sampen(Box<SampleEntropy>),
    Xmil(Box<CrossMutualInformationLags>),
    Cmo(Box<Cmo>),
    Bias(Box<Bias>),
    Amat(Box<Amat>),
    Dm(Box<Dm>),
    VhfMa(Box<VhfMa>),
    StochastikD(Box<StochastikD>),
    SwingsSoft(Box<SwingsSoft>),

    // Additional momentum
    Adx(Box<AdxIndicator>),
    DiPlusMinus(Box<DiPlusMinus>),
    WilliamsR(Box<WilliamsR>),
    Demarker(Box<Demarker>),
    ParabolicSAR(Box<ParabolicSAR>),
    UltimateOscillator(Box<UltimateOscillator>),
    UltimateOscillatorSmooth(Box<UltimateOscillatorSmooth>),
    Rwi(Box<Rwi>),
    Bop(Box<Bop>),
    Cfo(Box<Cfo>),
    Rmi(Box<Rmi>),
    Qstick(Box<Qstick>),
    CoppockCurve(Box<CoppockCurve>),
    Apo(Box<Apo>),
    Pmo(Box<Pmo>),
    TrueStrengthIndex(Box<TrueStrengthIndex>),
    DetrendedPriceOscillator(Box<DetrendedPriceOscillator>),
    KnowSureThing(Box<KnowSureThing>),
    Rvgi(Box<Rvgi>),
    Smi(Box<Smi>),
    Stc(Box<Stc>),
    ElderImpulse(Box<ElderImpulseSystem>),
    ElderRay(Box<ElderRay>),
    VortexIndicator(Box<VortexIndicator>),
    StochasticRsi(Box<StochasticRsi>),
    FisherTransform(Box<FisherTransform>),
    LaguerreRsi(Box<LaguerreRsi>),
    Rsx(Box<Rsx>),
    Qqe(Box<Qqe>),
    Kdj(Box<Kdj>),
    ConnorsRsi(Box<ConnorsRsi>),
    Trix(Box<Trix>),
    IftRsi(Box<IftRsi>),
    DpoPercent(Box<DpoPercent>),
    DetrendedSyntheticPrice(Box<DetrendedSyntheticPrice>),
    RsiPercentileRank(Box<RsiPercentileRank>),
    RsiPercentileBands(Box<RsiPercentileBands>),
    DssBressert(Box<DssBressert>),
    IntradayMomentumIndex(Box<IntradayMomentumIndex>),

    // Volatility & Channels
    Dc(Box<Dc>),
    Kc(Box<Kc>),
    Rvi(Box<Rvi>),
    Vr(Box<Vr>),
    TrueRange(Box<TrueRange>),
    ChaikinVolatility(Box<ChaikinVolatility>),
    Sqmom(Box<SqueezeMomentum>),
    Natr(Box<Natr>),
    ChoppinessIndex(Box<ChoppinessIndex>),
    UlcerIndex(Box<UlcerIndex>),
    MassIndex(Box<MassIndex>),
    VolatilityPercentileRankBands(Box<VolatilityPercentileRankBands>),
    Wvf(Box<Wvf>),
    HarRv(Box<HarRv>),
    BipowerVariance(Box<BipowerVariance>),
    RealizedQuarticity(Box<RealizedQuarticity>),
    HistoricalVolatilityC2C(Box<HistoricalVolatilityC2C>),
    AtrChannels(Box<AtrBands>),
    Kp(Box<Kp>),
    FuzzyCandlesticks(Box<FuzzyCandlesticks>),
    // Additional channel/volatility metrics
    BollingerMetrics(Box<BollingerMetrics>),
    Dcmetrics(Box<DonchianMetrics>),
    Kcmetrics(Box<KeltnerMetrics>),
    PercentB(Box<PercentB>),
    RbvJumpTest(Box<RbvJumpTest>),
    VolOfVolPercentile(Box<VolOfVolPercentile>),
    VolOfVolPercentileTrend(Box<VolOfVolPercentileTrend>),

    // Volume / Accumulation
    AccumulationDistribution(Box<AccumulationDistribution>),
    Mfi(Box<Mfi>),
    WilliamsAd(Box<WilliamsAd>),
    Vdelta(Box<VolumeDelta>),
    VolumeProfile(Box<VolumeProfile>),
    VolumePriceTrend(Box<VolumePriceTrend>),
    VolumeRateOfChange(Box<VolumeRateOfChange>),
    NviPvi(Box<NegativePositiveVolumeIndex>),
    Trin(Box<TrinIndicator>),
    Vpin(Box<Vpin>),
    Cmf(Box<ChaikinMoneyFlow>),
    EaseOfMovement(Box<EaseOfMovement>),
    ForceIndex(Box<ForceIndex>),
    Cho(Box<ChaikinOscillator>),
    IntradayIntensity(Box<IntradayIntensity>),
    AccumulativeSwingIndex(Box<AccumulativeSwingIndex>),
    DemandIndex(Box<DemandIndex>),
    Kvo(Box<Kvo>),
    // Additional volume indicators
    Pvo(Box<Pvo>),
    Pzo(Box<Pzo>),
    VolumeOscillator(Box<VolumeOscillator>),
    // Position/Levels indicators
    DistLevels(Box<DistanceToLevels>),
    AvwapDistance(Box<AvwapDistance>),
    AvwapMultiAnchorReversion(Box<AvwapMultiAnchorReversion>),
    AvwapTouchProbability(Box<AvwapTouchProbability>),
    // Trend indicators
    EfficiencyRatio(Box<EfficiencyRatio>),
    AdxSlope(Box<AdxSlope>),
    Supertrend(Box<Supertrend>),
    SslChannel(Box<SslChannel>),
    GmmaCompression(Box<GmmaCompression>),
    GannHilo(Box<GannHiLoActivator>),
    HaTrend(Box<HeikinAshiTrend>),
    Tii(Box<TrendIntensityIndex>),
    Ravi(Box<RaviTrend>),
    Didi(Box<DidiIndex>),
    EfficiencyRatioFullHistory(Box<EfficiencyRatioFullHistory>),
    EfficiencyRatioRingWindow(Box<EfficiencyRatioRingWindow>),
    SpreadAnalyzer(Box<SpreadAnalyzer>),
    VarianceRatio(Box<VarianceRatio>),
    VarianceRatioAggregate(Box<VarianceRatioAggregate>),
    ArchLmProxy(Box<ArchLmProxy>),
    KpssProxy(Box<KpssProxy>),
    AdfProxy(Box<AdfProxy>),
    Pacf(Box<Pacf>),
    LjungBox(Box<LjungBox>),
    // Trend Stops
    PSARStop(Box<PSARStop>),
    Supts(Box<SuperTrendStop>),
    Atrts(Box<ATRTrailingStop>),
    Chand(Box<ChandelierStop>),
    VolatilityStop(Box<VolatilityStop>),
    SwingStop(Box<SwingStop>),
    KeltnerStop(Box<KeltnerStop>),
    DonchianStop(Box<DonchianStop>),
    ChandeKrollStop(Box<ChandeKrollStop>),
    DonchianBreakout(Box<DonchianBreakout>),

    // Chaos (Williams)
    Alligator(Box<Alligator>),
    AwesomeOscillator(Box<AwesomeOscillator>),
    Ac(Box<AccelerationDeceleration>),
    MarketFacilitationIndex(Box<MarketFacilitationIndex>),

    // Signal Processing / Spectral
    FastFourierTransform(Box<FastFourierTransform>),
    Wave(Box<WaveletTransform>),
    Hilb(Box<HilbertTransform>),
    Sent(Box<SpectralEntropy>),
    SpectralFlatness(Box<SpectralFlatness>),
    SpectralRolloff(Box<SpectralRolloff>),
    Screst(Box<SpectralCrest>),
    SpectralEntropyRate(Box<SpectralEntropyRate>),
    Ser(Box<SpectralEnergyRatio>),
    SpectralSlope(Box<SpectralSlope>),
    SpectralSlopeZscore(Box<SpectralSlopeZscore>),
    SpectralLowMidPowerRatio(Box<SpectralLowMidPowerRatio>),
    SpectralHighMidPowerRatio(Box<SpectralHighMidPowerRatio>),
    SpectralCentroidFeature(Box<SpectralCentroidFeature>),
    SpectralBandwidthFeature(Box<SpectralBandwidthFeature>),
    SpectralBandpower(Box<SpectralBandpower>),
    ButterworthFilter(Box<ButterworthFilter>),
    ChebyshevFilter(Box<ChebyshevFilter>),
    Sg(Box<SavitzkyGolayFilter>),
    StftBandEnergyRatio(Box<StftBandEnergyRatio>),

    // Support/Resistance Pivots
    PivotPoints(Box<PivotPoints>),
    FloorTraderPivots(Box<FloorTraderPivots>),
    CamarillaPivots(Box<CamarillaPivots>),
    WoodiePivots(Box<WoodiePivots>),
    DeMarkPivots(Box<DeMarkPivots>),
    CentralPivotRange(Box<CentralPivotRange>),

    // Chaos / Fractal / Hurst / DFA
    FractalDimension(Box<FractalDimension>),
    Hurst(Box<HurstExponent>),
    HurstPct(Box<HurstPercentile>),
    ChaosOscillator(Box<ChaosOscillator>),
    Fractals(Box<WilliamsFractals>),
    Dfa(Box<Dfa>),
    DfaPercentile(Box<DfaPercentile>),

    // Z-Score & EWMAC
    RsiZscore(Box<RsiZscore>),
    MacdHistZscore(Box<MacdHistZscore>),
    PriceZScore(Box<PriceZScore>),
    PriceMadZscore(Box<PriceMadZscore>),
    MomentumZscore(Box<MomentumZscore>),
    Ewmac(Box<Ewmac>),

    // Ehlers & Filters
    Roof(Box<RoofingFilter>),
    Decyc(Box<Decycler>),
    Eit(Box<EhlersInstantaneousTrendline>),
    MesaAdaptiveMA(Box<MesaAdaptiveMA>),
    Ess(Box<EhlersSuperSmoother>),

    // RSI variants
    AtrRsi(Box<AtrRsi>),
    VolumeWeightedRsi(Box<VolumeWeightedRsi>),
    RsiOma(Box<RsiOma>),
    Tdi(Box<Tdi>),

    // Logic gates
    Thresh(Box<ThresholdGate>),
    AndGate(Box<AndGate>),
    OrGate(Box<OrGate>),
    XorGate(Box<XorGate>),
    SignCombiner(Box<SignCombiner>),

    // Realized vol / stats
    RealizedVol(Box<RealizedVol>),
    RealizedVolZscore(Box<RealizedVolZscore>),
    Autocorr(Box<Autocorr>),
    Hmom(Box<HigherMoments>),
    RocPercentile(Box<RocPercentile>),
    NrRange(Box<NrRange>),

    // Market regime & adaptive oscillators
    MarketRegimeFilter(Box<MarketRegimeFilter>),
    AdaptiveStochastic(Box<AdaptiveStochastic>),

    // Breakout/filters
    VolatilityBreakoutDetector(Box<VolatilityBreakoutDetector>),
    CusumFilter(Box<CusumFilter>),

    // Entropy / information
    MutualInformation(Box<MutualInformation>),
    TransferEntropy(Box<TransferEntropy>),
    KLDivergence(Box<KLDivergence>),
    JSDivergence(Box<JSDivergence>),
    RollingFisherInformation(Box<RollingFisherInformation>),
    InformationGain(Box<InformationGain>),
    Lz(Box<LempelZivComplexity>),

    // Logic/composite
    Hyst(Box<HysteresisGate>),
    WeightedComposite(Box<WeightedComposite>),

    // Candles/patterns
    SfpDetector(Box<SfpDetector>),

    // Momentum/trend/distance
    EhlersRocketRsi(Box<EhlersRocketRsi>),
    EhlersCc(Box<EhlersCyberCycle>),
    ZlSma(Box<ZlSma>),
    VwapDistance(Box<VwapDistance>),
    HlValueArea(Box<HlValueArea>),
    SweepReversionIndex(Box<SweepReversionIndex>),

    // Entropy/statistics
    Conden(Box<ConditionalEntropy>),
    HalfLifeMr(Box<HalfLifeMr>),
    ResidStat(Box<ResidualStationarity>),
    Coint(Box<CointegrationProxy>),
    EngleGrangerProxy(Box<EngleGrangerProxy>),
    EngleGrangerAdfProxy(Box<EngleGrangerAdfProxy>),
    EngleGrangerTrendProxy(Box<EngleGrangerTrendProxy>),

    // Momentum/trend extras
    MultiTimeframeMomentumDivergence(Box<MultiTimeframeMomentumDivergence>),

    // Filters/channels
    Hampel(Box<HampelFilter>),

    // Patterns/ML-style
    AdvancedPatternRecognition(Box<AdvancedPatternRecognition>),
    AdaptiveVolatilityRegime(Box<AdaptiveVolatilityRegime>),
    NeuralMomentumNetwork(Box<NeuralMomentumNetwork>),
    CandleAnatomy(Box<CandleAnatomy>),

    // Price action / structure helpers
    SwingStrengthScore(Box<SwingStrengthScore>),
    Liqgap(Box<LiquidityGapDensity>),
    WickSpike(Box<WickSpike>),
    SwingAge(Box<SwingAge>),

    // Regime/time effects and structures
    RangeCompressionBurst(Box<RangeCompressionBurst>),
    TimeEncoders(Box<TimeEncoders>),
    // Calendar/time effects are not part of universal OHLC constructor; excluded
    FvgDetector(Box<FvgDetector>),
    BosChochDetector(Box<BosChochDetector>),

    // ZigZag
    // ZigZag family requires index/time; excluded from universal OHLC path

    // Clusters / Orderflow / Book
    // Cluster/book/tick indicators require Bar/book; excluded

    // Regression models
    Arima(Box<Arima>),
    ArimaX(Box<ArimaX>),
    Garch(Box<Garch>),
    EGarch(Box<EGarch>),
    Var(Box<Var>),
    PolyReg(Box<PolynomialRegression>),

    // Momentum extras (Aroon family, extremes, pressure, MA cross, simple candle patterns)
    Aroon(Box<Aroon>),
    AroonOscillator(Box<AroonOscillator>),
    AroonUp(Box<AroonUp>),
    AroonDown(Box<AroonDown>),
    Highest(Box<Highest>),
    Lowest(Box<Lowest>),
    Pressure(Box<Pressure>),
    MaCross(Box<MaCross>),
    CandlePatterns(Box<CandlePatterns>),

    // Accumulation
    TwiggsMoneyFlow(Box<Tmf>),

    // Positioning
    // AvwapDistance needs timestamp; excluded

    // AutoFibo (index/time aware)
    // AutoFibo needs bar index/time; excluded
    // Ehlers / Cycles
    Esine(Box<EhlersSinewave>),
    Cyber(Box<CyberCycle>),
    HilbertDominantCycle(Box<HilbertDominantCycle>),

    // Kalman Filters
    BasicKalmanFilter(Box<BasicKalmanFilter>),
    ExtendedKalmanFilter(Box<ExtendedKalmanFilter>),
    UnscentedKalmanFilter(Box<UnscentedKalmanFilter>),
    ParticleFilter(Box<ParticleFilter>),

    // Adaptive
    AdaptiveMovingAverage(Box<AdaptiveMovingAverage>),
    KaufmanAdaptiveMA(Box<KaufmanAdaptiveMA>),
    Vidya(Box<VariableIndexDynamicAverage>),
    Framaadv(Box<FractalAdaptiveMovingAverage>),

    // Entropy
    ShannonEntropy(Box<ShannonEntropy>),
    Apen(Box<ApproximateEntropy>),
    SampleEntropy(Box<SampleEntropy>),
    PermutationEntropy(Box<PermutationEntropy>),

    // Regime composites
    RegimeCompositeV2(Box<RegimeCompositeV2>),
    RegimeCompositeV3(Box<RegimeCompositeV3>),
    RegimeCompositeV4(Box<RegimeCompositeV4>),
    RegimeComposite(Box<RegimeComposite>),

    // Anchored VWAPs & levels
    AnchoredVwap(Box<AnchoredVwap>),
    PivotAnchoredVwap(Box<PivotAnchoredVwap>),

    // Vol/Volatility helpers
    AtrPercentile(Box<AtrPercentileInd>),
    AtrBandwidth(Box<AtrBandwidth>),
    AtrZscore(Box<AtrZscore>),
    CloseVolPercentile(Box<CloseVolPercentile>),
    RelativeVolume(Box<RelativeVolume>),
    VolumeZscore(Box<VolumeZscore>),

    // Trend/levels helpers
    SlopeDirectionLine(Box<SlopeDirectionLine>),
    EmaSlope(Box<EmaSlope>),
    Rmid(Box<RollingMidline>),
    RangePercentile(Box<RangePercentile>),
    RangeToAtr(Box<RangeToAtr>),
    RollingQuartiles(Box<RollingQuartiles>),

    // ========== PHASE 2 ADDITIONS ==========
    // ACCUMULATION (2)
    Iip(Box<IntradayIntensityPercent>),
    Iir(Box<IntradayIntensityRatio>),

    // KALMAN (8)
    Abgfilter(Box<AlphaBetaGammaFilter>),
    Kcomp(Box<KalmanRegimeComposite>),
    Kregime(Box<KalmanTrendRegime>),
    Kscr(Box<KalmanRegimeScore>),
    Kslope(Box<KalmanTrendSlope>),
    Kslopez(Box<KalmanSlopeZscore>),

    // SIGNALPROCESSING (13: 6 new + 7 already present)
    // Already present: Hdc, Mrf, Scf, Sentr, Sflat, Sroll, Sslope, Tenc, Wcomp
    Screstp(Box<SpectralCrestPercentile>),
    Slmpr(Box<SpectralLowMidPowerRatio>),
    Sroll95(Box<SpectralRolloff95>),
    Srollp(Box<SpectralRolloffPercentile>),
    Sslopep(Box<SpectralSlopePercentile>),
    Ssloperp(Box<SpectralSlopeRobustPercentile>),

    // MOMENTUM (2)

    // STATISTICS (10)
    AdfKpss(Box<AdfKpssComposite>),
    ArchLmPval(Box<ArchLmPvalueProxy>),
    BpCusum(Box<BaiPerronCusum>),
    KpssTrend(Box<KpssTrendProxy>),
    KpssZ(Box<KpssZProxy>),
    Pp(Box<PhillipsPerronProxy>),
    PvCoherence(Box<PriceVolumeCoherenceProxy>),
    RSquared(Box<RSquared>),
    VrZAgg(Box<VarianceRatioZAggregate>),
    Za(Box<ZivotAndrewsProxy>),

    // VOLATILITY (4: 1 new + 3 already present)
    // Already present: Abb (AdaptiveBollingerBands), Cv (ChaikinVolatility), Ui (UlcerIndex)
    Dvr(Box<DynamicVolatilityRegime>),

    // CANDLES (14 patterns) - Batch 1
    Doji(Box<Doji>),
    Engulfing(Box<Engulfing>),
    Hammer(Box<Hammer>),
    Harami(Box<Harami>),
    Heikinashi(Box<HeikinAshi>),
    Marubozu(Box<Marubozu>),
    Morningstar(Box<MorningStar>),
    Piercingpattern(Box<PiercingPattern>),
    Shootingstar(Box<ShootingStar>),
    Threewhitesoldiers(Box<ThreeWhiteSoldiers>),
    Tweezer(Box<Tweezer>),
    Darkcloudcover(Box<DarkCloudCover>),
    Eveningstar(Box<EveningStar>),
    Threeblackcrows(Box<ThreeBlackCrows>),

    // ACCUMULATION (1) - Batch 1
    Di(Box<DemandIndex>),

    // Note: Legacy Hmafast/Wmafast/Amaring REMOVED - canonical names Hma/Wma/Ama used instead
    // OHLCV variants (15) - REMOVED
    // Replaced by MovingAverageWithField::new(MovingAverageType, period, OhlcvField)

    // VOLATILITY (1) - Batch 2 (genuinely new)
    Vov(Box<VolOfVol>),

    Cog(Box<CenterOfGravity>),
    // MOMENTUM (2) - Batch 2
    Pfe(Box<Pfe>),

    // ========================================
    // BATCH 3 - 5 INDICATORS (standard update_bar compatible)
    // ========================================
    // MOMENTUM (5)
    EwmacRobust(Box<EwmacRobust>),
    Gapo(Box<Gapo>),
    Gator(Box<GatorOscillator>),
    MacdHist(Box<MacdHistogram>),
    MacdSignal(Box<MacdSignal>),
    Ppo(Box<Ppo>),
    PpoSignal(Box<PpoSignal>),

    // ========================================
    // BATCH 4 ADDITIONS (11 indicators - only standard update_bar compatible)
    // ========================================
    // SIGNAL_PROCESSING (6)
    Scf(Box<SpectralCentroidFeature>),
    Sentr(Box<SpectralEntropyRate>),
    Sflat(Box<SpectralFlatness>),
    Sroll(Box<SpectralRolloff>),
    Sslope(Box<SpectralSlope>),
    Wcomp(Box<WeightedComposite>),
    // TREND (2)
    KamaSlope(Box<KamaSlope>),
    LrSlope(Box<LrSlope>),
    // KALMAN (1)
    Rts(Box<RtsSmoother>),
    // VOLUME (2)
    Vfi(Box<Vfi>),
    Vzo(Box<Vzo>),

    // ========================================
    // BATCH 5 ADDITIONS (13 divergence indicators)
    // ========================================
    // DIVERGENCE (13)
    RsiDiv(Box<RsiDivergence>),
    CciDiv(Box<CciDivergence>),
    MacdDiv(Box<MacdDivergence>),
    MacdHistDiv(Box<MacdHistogramDivergence>),
    StochDiv(Box<StochasticDivergence>),
    WilliamsDiv(Box<WilliamsDivergence>),
    ObvDiv(Box<ObvDivergence>),
    VolumeDiv(Box<VolumeDivergence>),
    ClassicDiv(Box<ClassicDivergence>),
    HiddenDiv(Box<HiddenDivergence>),
    DivStrength(Box<DivergenceStrength>),
    MultiDiv(Box<MultiDivergence>),
    MomentumDiv(Box<MomentumDivergence>),

    // ========================================
    // BATCH 6 ADDITIONS (3 timestamp-dependent indicators)
    // ========================================
    // POSITION (3) - timestamp-based
    HourDay(Box<HourOfDayEffect>),
    WeekMonth(Box<WeekInMonthEffect>),
    SoqEoq(Box<StartEndOfQuarterFlags>),
    // Phase 2 timestamp-based indicators
    Tenc(Box<TimeEncoders>),
    Weekday(Box<WeekdayEffect>),
    Session(Box<SessionEffect>),
    MonthQtr(Box<MonthQuarterEffect>),
    DomWoq(Box<DayOfMonthWeekOfQuarterEffect>),
    RelTrendPos(Box<RelativeTrendPosition>),
    MonthTurn(Box<MonthTurnEffect>),
    QtrTurn(Box<QuarterTurnEffect>),

    // ========================================
    // PHASE 3 ADDITIONS (5 medium complexity indicators)
    // ========================================
    // POSITION (4 timestamp-based)
    SomEom(Box<StartEndOfMonthFlags>),
    SowEow(Box<StartEndOfWeekFlags>),
    HolidayProx(Box<HolidayWeekendProximityEffect>),
    DayWeekMonth(Box<DayOfWeekInMonthEffect>),
    // VOLATILITY (1 struct unwrap)
    Vbd(Box<VolatilityBreakoutDetector>),


    // ========================================
    // PHASE 4 ADDITIONS (9 indicators - Poc + AutoFibo + 7 ZigZag variants)
    // ========================================
    // POC (1)
    Poc(Box<PocDetector>),
    // AUTO FIBO (1)
    AutoFibo(Box<AutoFibo>),
    // ZIGZAG FAMILY (6 variants)
    Zigzag(Box<ZigZagClassic>),  // Uses deviation parameter
    ZigzagClassic(Box<ZigZagClassic>),  // Uses threshold_percent/threshold_abs
    ZigzagAtr(Box<ZigZagAtr>),
    ZigzagCandle(Box<ZigZagCandle>),
    ZigzagLookahead(Box<ZigZagLookahead>),
    ZigzagTime(Box<ZigZagTime>),

    // ========================================
    // PHASE 5 ADDITIONS (3 FVG indicators - Final push to 100%)
    // ========================================
    Fvgalt(Box<FvgIntensityAltScore>),
    Fvgdur(Box<FvgDurationIntensityScore>),
    Fvgrev(Box<FvgReversionProbability>),
}

impl IndicatorInstance {
    // Helper: derive weekday (1..=7, Mon=1) from timestamp
    #[inline]
    fn derive_weekday(timestamp: Option<i64>) -> u8 {
        timestamp
            .map(|ts| {
                let wd = CalendarService::weekday_from_timestamp(ts); // 0=Mon
                (wd + 1) as u8 // Convert to 1=Mon
            })
            .unwrap_or(1) // Default Monday
    }

    // Helper: derive session (0..=3) from timestamp
    #[inline]
    fn derive_session(timestamp: Option<i64>) -> u8 {
        timestamp.map(|ts| {
            let secs = ts.rem_euclid(86_400);
            let hour = secs / 3600;
            if hour < 6 { 3 } else if hour < 12 { 0 } else if hour < 18 { 1 } else { 2 }
        }).unwrap_or(0)
    }

    // Helper: derive month (1..=12) from timestamp
    #[inline]
    fn derive_month(timestamp: Option<i64>) -> u8 {
        timestamp
            .map(|ts| {
                let (_year, month, _day) = CalendarService::ymd_from_timestamp(ts);
                month as u8
            })
            .unwrap_or(1)
    }

    // Helper: derive quarter (1..=4) from month
    #[inline]
    fn month_to_quarter(month: u8) -> u8 {
        ((month.clamp(1, 12) - 1) / 3) + 1
    }

    // Helper: derive (year, month, day) from timestamp
    #[inline]
    fn derive_ymd(timestamp: Option<i64>) -> (i32, u32, u32) {
        timestamp
            .map(CalendarService::ymd_from_timestamp)
            .unwrap_or((2024, 1, 1)) // Default date
    }

    // Helper: derive day_of_month from timestamp
    #[inline]
    fn derive_day_of_month(timestamp: Option<i64>) -> u8 {
        timestamp
            .map(|ts| {
                let (_year, _month, day) = CalendarService::ymd_from_timestamp(ts);
                day as u8
            })
            .unwrap_or(1)
    }

    // Helper: derive week_of_quarter from timestamp
    #[inline]
    fn derive_week_of_quarter(timestamp: Option<i64>) -> u8 {
        timestamp
            .map(|ts| CalendarService::week_of_quarter(ts) as u8)
            .unwrap_or(1)
    }

    /// Determine if an indicator uses the configurable source field
    ///
    /// Returns true if the indicator should use `config.source` (PriceOnly indicators),
    /// false if the indicator has its own internal logic for handling price/volume data.
    ///
    /// # Source Type Categories:
    /// - **PriceOnly**: Indicators that accept a configurable OHLCV field source
    /// - **VolumeOnly**: Indicators that only use volume (no source selection)
    /// - **PriceAndVolume**: Indicators that combine price+volume with internal formulas
    pub fn uses_configurable_source(id: BarIndicatorId) -> bool {
        use BarIndicatorId::*;

        // These indicators do NOT use config.source - they have internal price/volume logic
        !matches!(
            id,
            // PriceAndVolume indicators - use internal formulas combining price+volume
            Vwap | Vwma | Mfi | Cmf | Cho | Fi | Ii | Wad | Eom | Asi | Di | Kvo |
            Vwapchan | Vwapchanwidth | VwapLevels | VwapDist | Vfi | Vzo | Pzo |

            // VolumeOnly indicators - only use volume data
            Obv | MoObv | Vdelta | Vprofile | Vpt | Vroc | NviPvi | Trin | Vpin | TickVolume |
            Pvo | Pvt | Rvol | Vo | Vz | Poc |

            // Additional accumulation indicators that use volume internally
            Ad // Accumulation/Distribution uses volume

            // Note: Most other indicators are PriceOnly and will use config.source
        )
    }

    /// Create an indicator instance from configuration
    ///
    /// # Source Routing Logic
    ///
    /// This factory routes indicator creation based on the indicator's SourceType:
    ///
    /// - **PriceOnly indicators**: Use `config.source` to allow user-selected OHLCV field
    ///   - Examples: SMA, EMA, RSI, Bollinger Bands
    ///   - Created with: `.with_source(period, config.source)`
    ///
    /// - **VolumeOnly indicators**: Ignore `config.source`, use volume internally
    ///   - Examples: OBV, Volume Delta, Volume Profile
    ///   - Created with: `.new(period)` - no source parameter
    ///
    /// - **PriceAndVolume indicators**: Ignore `config.source`, use internal formulas
    ///   - Examples: VWAP, VWMA, MFI, Chaikin Money Flow
    ///   - Created with: `.new(period)` - internal logic combines price+volume
    ///
    /// The `uses_configurable_source()` helper determines which indicators accept source selection.
    pub fn create(config: &IndicatorConfig) -> Result<Self, String> {
        // Ensure minimal valid period for all periodized indicators
        let period = config.periods.first().copied().unwrap_or(14).clamp(2, 512);
        match config.id {
            // Average indicators
            BarIndicatorId::Sma => Ok(Self::Sma(Sma::with_source(period, config.source))),
            BarIndicatorId::Ema => Ok(Self::Ema(Ema::with_source(period, config.source))),
            BarIndicatorId::Rma => Ok(Self::Rma(Box::new(Rma::with_source(period, config.source)))),
            BarIndicatorId::Hma => Ok(Self::Hma(Box::new(Hma::with_source(period, config.source)))),
            BarIndicatorId::Wma => Ok(Self::Wma(Box::new(Wma::with_source(period, config.source)))),
            BarIndicatorId::Dema => Ok(Self::Dema(Box::new(Dema::with_source(period, config.source)))),
            BarIndicatorId::Vwma => Ok(Self::Vwma(Box::new(Vwma::new(period)))),
            BarIndicatorId::Vwap => Ok(Self::Vwap(Box::new(Vwap::new(period)))),
            BarIndicatorId::Trima => Ok(Self::Trima(Box::new(Trima::with_source(period, config.source)))),
            BarIndicatorId::Alma => {
                let offset = config.additional_params.get("offset").copied().unwrap_or(0.85);
                let sigma = config.additional_params.get("sigma").copied().unwrap_or(6.0);
                Ok(Self::Alma(Box::new(Alma::with_source(period, config.source, offset, sigma))))
            }
            BarIndicatorId::T3 => Ok(Self::T3(Box::new(T3::with_source(period, 0.7, config.source)))),
            BarIndicatorId::Mcginley => Ok(Self::McGinley(Box::new(McGinleyDynamic::with_source(period, config.source)))),
            BarIndicatorId::Ama => {
                let fast = config.periods.get(1).copied().unwrap_or(2);
                let slow = config.periods.get(2).copied().unwrap_or(30);
                Ok(Self::Ama(Box::new(Ama::with_source(period, fast, slow, config.source))))
            }
            BarIndicatorId::Tma => Ok(Self::Tma(Box::new(Tma::with_source(period, config.source)))),
            BarIndicatorId::Tema => Ok(Self::Tema(Box::new(Tema::with_source(period, config.source)))),
            BarIndicatorId::Frama => Ok(Self::AvFrama(Box::new(Frama::with_source(period, config.source)))),
            BarIndicatorId::AvFrama => Ok(Self::AvFrama(Box::new(Frama::with_source(period, config.source)))), // average/ version (adaptive/ has plain Frama)
            BarIndicatorId::Lr => Ok(Self::Lr(Box::new(Lr::with_source(period, config.source)))),
            BarIndicatorId::Jma => {
                let phase = config.additional_params.get("phase").copied().unwrap_or(0.0);
                Ok(Self::JurikMa(Box::new(JurikMa::with_source(period, phase, config.source))))
            }
            BarIndicatorId::AvVidya => {
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::AvVidya(Box::new(Vidya::with_source(period, ma_type, config.source))))
            }
            // VWMA handled above, leave here for completeness

            // Core momentum/volatility
            BarIndicatorId::Rsi => Ok(Self::Rsi(Rsi::with_source(period, MovingAverageType::RMA, config.source))),
            BarIndicatorId::Macd => {
                let fast = config.periods.first().copied().unwrap_or(12);
                let slow = config.periods.get(1).copied().unwrap_or(26);
                let signal = config.periods.get(2).copied().unwrap_or(9);

                let fast_ma_type = config.ma_types.get("fast_ma_type")
                    .copied()
                    .unwrap_or(MovingAverageType::EMA);
                let slow_ma_type = config.ma_types.get("slow_ma_type")
                    .copied()
                    .unwrap_or(MovingAverageType::EMA);
                let signal_ma_type = config.ma_types.get("signal_ma_type")
                    .copied()
                    .unwrap_or(MovingAverageType::EMA);

                // Get per-component sources from component_configs or fallback to main source
                let fast_source = config.component_configs.get("fast_ma")
                    .and_then(|c| c.source)
                    .unwrap_or(config.source);
                let slow_source = config.component_configs.get("slow_ma")
                    .and_then(|c| c.source)
                    .unwrap_or(config.source);

                Ok(Self::Macd(Macd::with_full_config(
                    fast, slow, signal,
                    fast_ma_type, slow_ma_type, signal_ma_type,
                    fast_source, slow_source
                )))
            }
            BarIndicatorId::Ppo => {
                let fast = config.periods.first().copied().unwrap_or(12);
                let slow = config.periods.get(1).copied().unwrap_or(26);
                let signal = config.periods.get(2).copied().unwrap_or(9);

                // Check if we have custom MA types
                let has_ma_types = config.ma_types.contains_key("fast_ma_type")
                    || config.ma_types.contains_key("slow_ma_type")
                    || config.ma_types.contains_key("signal_ma_type");

                if has_ma_types || config.source != OhlcvField::Close {
                    let fast_ma_type = config.ma_types.get("fast_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let slow_ma_type = config.ma_types.get("slow_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let signal_ma_type = config.ma_types.get("signal_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let source = config.source;

                    Ok(Self::Ppo(Box::new(Ppo::with_full_config(
                        fast, slow, signal, source,
                        fast_ma_type, slow_ma_type, signal_ma_type
                    ))))
                } else {
                    Ok(Self::Ppo(Box::new(Ppo::new(fast, slow, signal))))
                }
            }
            BarIndicatorId::BbPeriod => {
                let std_dev = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::BbPeriod(BbPeriod::new(period, std_dev, ma_type)))
            }
            BarIndicatorId::Atr => {
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::Atr(Atr::new(period, ma_type)))
            }
            BarIndicatorId::Tr => Ok(Self::TrueRange(Box::default())),
            BarIndicatorId::Bpv => {
                let n = config.periods.first().copied().unwrap_or(10).clamp(2, 512);
                let k = config.periods.get(1).copied().unwrap_or(10).clamp(2, 512);
                Ok(Self::ChaikinVolatility(Box::new(ChaikinVolatility::new(n, k))))
            }
            BarIndicatorId::Sqmom => {
                let bb = config.periods.first().copied().unwrap_or(20).clamp(2, 512);
                let kc = config.periods.get(1).copied().unwrap_or(20).clamp(2, 512);
                let mp = config.periods.get(2).copied().unwrap_or(20).clamp(2, 512);

                // Check if we have custom MA types or source
                let has_ma_types = config.ma_types.contains_key("bb_ma_type")
                    || config.ma_types.contains_key("kc_ma_type");

                if has_ma_types || config.source != OhlcvField::Close {
                    let bb_ma_type = config.ma_types.get("bb_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);
                    let kc_ma_type = config.ma_types.get("kc_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);
                    let source = config.source;

                    Ok(Self::Sqmom(Box::new(SqueezeMomentum::with_full_config(
                        bb, kc, mp, source, bb_ma_type, kc_ma_type
                    ))))
                } else {
                    Ok(Self::Sqmom(Box::new(SqueezeMomentum::new(bb, kc, mp))))
                }
            }
            BarIndicatorId::Natr => Ok(Self::Natr(Box::new(Natr::new(period)))),
            BarIndicatorId::Chop => Ok(Self::ChoppinessIndex(Box::new(ChoppinessIndex::with_period(period)))),
            BarIndicatorId::Ui => Ok(Self::UlcerIndex(Box::new(UlcerIndex::with_period(period)))),
            BarIndicatorId::VoMi => {
                let ema_p = config.periods.first().copied().unwrap_or(9);
                let sum_p = config.periods.get(1).copied().unwrap_or(25);
                Ok(Self::MassIndex(Box::new(MassIndex::with_params(ema_p, sum_p))))
            }

            // Channels / Adaptive
            BarIndicatorId::Adaptivebb => {
                let base_multiplier = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                let auto_mode = config.flags.get("auto_mode").copied().unwrap_or(true);

                if auto_mode {
                    // Auto mode: compute min/max ranges automatically
                    Ok(Self::AdaptiveBollingerBands(Box::new(
                        AdaptiveBollingerBands::from_base_params_with_source(period, base_multiplier, config.source)
                    )))
                } else {
                    // Manual mode: use explicit min/max parameters
                    let min_period = config.additional_params.get("min_period").map(|v| *v as usize).unwrap_or((period / 2).max(5));
                    let max_period = config.additional_params.get("max_period").map(|v| *v as usize).unwrap_or(period * 2);
                    let min_multiplier = config.additional_params.get("min_multiplier").copied().unwrap_or((base_multiplier / 2.0).max(0.5));
                    let max_multiplier = config.additional_params.get("max_multiplier").copied().unwrap_or(base_multiplier * 1.5);

                    Ok(Self::AdaptiveBollingerBands(Box::new(
                        AdaptiveBollingerBands::with_parameters_and_source(
                            period, min_period, max_period,
                            base_multiplier, min_multiplier, max_multiplier,
                            config.source
                        )
                    )))
                }
            }
            BarIndicatorId::Bb => {
                let std_dev = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                let ma_type = config.ma_types.get("ma_type")
                    .copied()
                    .unwrap_or(MovingAverageType::SMA);

                Ok(Self::BollingerBands(Box::new(
                    BollingerBands::with_source(
                        period,
                        std_dev,
                        crate::bar_indicators::channels::bollinger_bands::BollingerMode::Close,
                        ma_type,
                        config.source
                    )
                )))
            }
            BarIndicatorId::Dc => Ok(Self::DonchianChannel(Box::new(DonchianChannel::new(period)))),
            BarIndicatorId::Kc => {
                let multiplier = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                let ma_type = config.ma_types.get("ma_type")
                    .copied()
                    .unwrap_or(MovingAverageType::EMA);
                let atr_ma_type = config.ma_types.get("atr_ma_type")
                    .copied()
                    .unwrap_or(MovingAverageType::RMA);

                Ok(Self::KeltnerChannel(Box::new(
                    KeltnerChannel::with_source(
                        period,
                        multiplier,
                        crate::bar_indicators::channels::keltner_channel::KeltnerMode::Classic,
                        ma_type,
                        atr_ma_type,
                        config.source
                    )
                )))
            }
            BarIndicatorId::Dcwidth => Ok(Self::Dcwidth(Box::new(DonchianWidth::new(period)))),
            BarIndicatorId::Dcpos => Ok(Self::DonchianPosition(Box::new(DonchianPosition::new(period)))),
            BarIndicatorId::Keltbw => {
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                Ok(Self::KeltnerBandwidth(Box::new(KeltnerBandwidth::new(period, mult))))
            }
            BarIndicatorId::Keltpos => {
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                Ok(Self::Keltpos(Box::new(KeltnerPosition::new(period, mult))))
            }
            BarIndicatorId::Keltdist => {
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                Ok(Self::Keltdist(Box::new(KeltnerDistance::new(period, mult))))
            }
            BarIndicatorId::Pricechan => Ok(Self::PriceChannels(Box::new(PriceChannels::new_raw(period)))),
            BarIndicatorId::Pchwidth => Ok(Self::Pchwidth(Box::new(PriceChannelWidth::new(period)))),
            BarIndicatorId::Pchosc => Ok(Self::Pchosc(Box::new(PriceChannelOscillator::new(period)))),
            BarIndicatorId::Vwapchan => {
                let mult = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                Ok(Self::VwapChannels(Box::new(VwapChannels::new_standard(period, mult))))
            }
            BarIndicatorId::Vwapchanwidth => {
                let mult = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                Ok(Self::VwapChannelWidth(Box::new(VwapChannelWidth::new(period, mult))))
            }
            BarIndicatorId::Regchan => {
                let mult = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                Ok(Self::RegressionChannels(Box::new(RegressionChannels::with_source(period, mult, crate::bar_indicators::channels::regression_channels::RegressionChannelMode::Standard, config.source))))
            }
            BarIndicatorId::Regchanwidth => {
                let mult = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                Ok(Self::RegressionChannelWidth(Box::new(RegressionChannelWidth::new(period, mult))))
            }
            BarIndicatorId::Envelope => {
                let pct = config.additional_params.get("pct").copied().unwrap_or(2.5);
                Ok(Self::EnvelopeChannels(Box::new(EnvelopeChannels::with_source(period, pct, crate::bar_indicators::channels::envelope_channels::EnvelopeMode::Fixed, MovingAverageType::SMA, config.source))))
            }
            BarIndicatorId::Envbw => {
                let pct = config.additional_params.get("pct").copied().unwrap_or(2.5);
                Ok(Self::Envbw(Box::new(EnvelopeBandwidth::new(period, pct))))
            }
            BarIndicatorId::Stddevchan => Ok(Self::StandardDeviationChannels(Box::new(StandardDeviationChannels::with_source(period, 2.0, crate::bar_indicators::channels::standard_deviation_channels::StandardDeviationMode::Simple, crate::bar_indicators::channels::standard_deviation_channels::RegressionSource::Close, config.source)))) ,
            BarIndicatorId::Stddevwidth => {
                let mult = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                Ok(Self::StdDevChannelWidth(Box::new(StdDevChannelWidth::new(period, mult))))
            }
            BarIndicatorId::Starc => {
                let k = config.additional_params.get("k").copied().unwrap_or(2.0);
                Ok(Self::StarcBands(Box::new(StarcBands::with_source(period, period, k, MovingAverageType::SMA, config.source))))
            }
            BarIndicatorId::Ichimoku => Ok(Self::IchimokuCloud(Box::default())),
            BarIndicatorId::Ichimokuthick => Ok(Self::IchimokuCloudThickness(Box::default())),
            BarIndicatorId::Ichimokupos => Ok(Self::IchimokuCloudPosition(Box::default())),
            
            // Support/Resistance Levels (Pivots)
            BarIndicatorId::Pivot => {
                let calc_period = config.periods.first().copied().unwrap_or(1).max(1);
                Ok(Self::PivotPoints(Box::new(PivotPoints::with_period(calc_period))))
            }
            BarIndicatorId::Floorpivot => {
                let period = config.periods.first().copied().unwrap_or(24).max(1);
                Ok(Self::FloorTraderPivots(Box::new(FloorTraderPivots::with_period(period))))
            }
            BarIndicatorId::Camarilla => {
                let period = config.periods.first().copied().unwrap_or(24).max(1);
                Ok(Self::CamarillaPivots(Box::new(CamarillaPivots::with_period(period))))
            }
            BarIndicatorId::Woodie => {
                let period = config.periods.first().copied().unwrap_or(24).max(1);
                Ok(Self::WoodiePivots(Box::new(WoodiePivots::with_period(period))))
            }
            BarIndicatorId::Demark => {
                let calc_period = config.periods.first().copied().unwrap_or(1).max(1);
                Ok(Self::DeMarkPivots(Box::new(DeMarkPivots::with_period(calc_period))))
            }
            BarIndicatorId::Cpr => Ok(Self::CentralPivotRange(Box::default())),
            BarIndicatorId::Pivotchan => Ok(Self::Pivotchan(Box::default())),
            // MedianChannels requires period > 2 (asserts inside). Clamp to at least 3.
            BarIndicatorId::Medchan => Ok(Self::Medchan(Box::new(MedianChannels::with_source(period.max(3), crate::bar_indicators::channels::median_channels::MedianMode::Simple, crate::bar_indicators::channels::median_channels::MedianSource::Close, 1.4826, config.source)))),
            BarIndicatorId::Medchanpos => Ok(Self::Medchanpos(Box::new(MedianChannelPosition::new(period)))),
            BarIndicatorId::Qrchan => Ok(Self::QuantileRegressionChannels(Box::new(QuantileRegressionChannels::new(period, 2.0)))),
            BarIndicatorId::Trimabands => Ok(Self::TrimaBands(Box::new(TrimaBands::with_source(period, 2.0, config.source)))),
            BarIndicatorId::Dpobands => Ok(Self::DpoBands(Box::new(DpoBands::new(period, period.max(5), 2.0)))),
            BarIndicatorId::Percentilech => Ok(Self::Percentilech(Box::new(PercentileChannels::with_source(period, PercentileBasis::Close, 0.25, 0.75, config.source)))),
            // Duplicate short form removed; detailed arm below
            // Duplicate short form removed; detailed arm below
            BarIndicatorId::Fibochan => {
                let zigzag = config.periods.first().copied().unwrap_or(20);
                let atr_p = config.periods.get(1).copied().unwrap_or(14);
                let mult = config.additional_params.get("atr_multiplier").copied().unwrap_or(2.0);
                let mode_code = config.additional_params.get("mode").copied().unwrap_or(0.0) as i32;
                let mode = match mode_code { 1 => FibonacciChannelMode::Extension, 2 => FibonacciChannelMode::Combined, _ => FibonacciChannelMode::Retracement };
                Ok(Self::FibonacciChannels(Box::new(FibonacciChannels::new(zigzag, atr_p, mult, mode))))
            }
            ,
            // DarvasBox variant not present in BarIndicatorId; skip wiring here.
            BarIndicatorId::Adaptivechan => {
                let mode_code = config.additional_params.get("adaptation_mode").copied().unwrap_or(3.0) as i32; // Combined
                let mode = match mode_code { 0 => AdaptationMode::Volatility, 1 => AdaptationMode::Trend, 2 => AdaptationMode::Cycle, 4 => AdaptationMode::MachineLearning, _ => AdaptationMode::Combined };
                let center_code = config.additional_params.get("center_line_type").copied().unwrap_or(0.0) as i32; // KAMA
                let center = match center_code { 1 => CenterLineType::FastKAMA, 2 => CenterLineType::SlowKAMA, 3 => CenterLineType::AdaptiveLinReg, _ => CenterLineType::KAMA };
                let vlook = config.additional_params.get("volatility_lookback").copied().unwrap_or(50.0) as usize;
                let vlook = vlook.clamp(1, 512);
                let period_ac = period.clamp(1, 512);
                Ok(Self::AdaptiveChannels(Box::new(AdaptiveChannels::new_custom(period_ac, mode, center, vlook))))
            }
            ,
            BarIndicatorId::Volprofchan => {
                let mode_code = config.additional_params.get("mode").copied().unwrap_or(1.0) as i32; // AdaptiveBins
                let mode = match mode_code { 0 => VolumeProfileMode::FixedBins, 2 => VolumeProfileMode::TickBased, _ => VolumeProfileMode::AdaptiveBins };
                let per_code = config.additional_params.get("vp_period").copied().unwrap_or(1.0) as i32; // Daily
                let last_n = config.additional_params.get("last_n_bars").copied().unwrap_or(500.0) as usize;
                let period_vp = match per_code { 0 => VolumeProfilePeriod::Session, 2 => VolumeProfilePeriod::Weekly, 3 => VolumeProfilePeriod::Monthly, 4 => VolumeProfilePeriod::LastNBars(last_n.max(10)), _ => VolumeProfilePeriod::Daily };
                let num_bins = config.additional_params.get("num_bins").copied().unwrap_or(50.0) as usize;
                let num_bins = num_bins.clamp(6, 200);
                let va_pct = config.additional_params.get("value_area_percent").copied().unwrap_or(70.0);
                let va_pct = va_pct.clamp(50.1, 94.9);
                Ok(Self::VolumeProfileChannels(Box::new(VolumeProfileChannels::new_custom(mode, period_vp, num_bins, va_pct))))
            }
            ,

            // Momentum extras
            BarIndicatorId::Roc => {
                let use_log = config.additional_params.get("use_log").map(|&v| v != 0.0).unwrap_or(false);
                Ok(Self::Roc(Box::new(Roc::with_source(period, use_log, config.source))))
            }
            BarIndicatorId::Cci => {
                let scalar = config.additional_params.get("scalar").copied().unwrap_or(0.015);
                Ok(Self::Cci(Box::new(Cci::new(period, scalar, None))))
            }
            BarIndicatorId::Stoch => {
                let period_k = config.periods.first().copied().unwrap_or(14).clamp(2, 512);
                let period_d = config.periods.get(1).copied().unwrap_or(3).clamp(1, 512);
                Ok(Self::Stochastics(Box::new(Stochastics::new(period_k, period_d))))
            }
            BarIndicatorId::Obv => Ok(Self::Obv(Box::new(Obv::new()))),
            BarIndicatorId::MoObv => Ok(Self::Obv(Box::new(Obv::new()))), // momentum/ version (accumulation/ has plain Obv)
            BarIndicatorId::Vhf => Ok(Self::Vhf(Box::new(VhfSimple::new(period)))),
            BarIndicatorId::Swings => Ok(Self::Swings(Box::new(Swings::new(period)))),
            BarIndicatorId::Psl => Ok(Self::Psl(Box::new(Psl::new(period)))),
            // BOOK category (4 indicators)
            BarIndicatorId::BookImb => Ok(Self::BookImb(Box::default())),
            BarIndicatorId::BookSlope => Ok(Self::BookSlope(Box::default())),
            BarIndicatorId::Ofi => {
                let tick_size = config.additional_params.get("tick_size").copied().unwrap_or(0.01);
                Ok(Self::Ofi(Box::new(OrderFlowImbalance::new(period, tick_size))))
            }
            BarIndicatorId::QueueImb => Ok(Self::QueueImb(Box::default())),
            // CLUSTERS category (6 indicators)
            BarIndicatorId::ClQueueImb => Ok(Self::ClQueueImb(Box::default())),
            BarIndicatorId::MarketMicro => Ok(Self::MarketMicro(Box::new(MarketMicrostructure::new(period)))),
            BarIndicatorId::OrderBookSlope => Ok(Self::OrderBookSlope(Box::default())),
            BarIndicatorId::OrderFlowImb => {
                let tick_size = config.additional_params.get("tick_size").copied().unwrap_or(0.01);
                Ok(Self::OrderFlowImb(Box::new(OrderFlowImbalance::new(period, tick_size))))
            }
            BarIndicatorId::TickVolume => Ok(Self::TickVolume(Box::new(TickVolumeAnalyzer::new(period)))),
            BarIndicatorId::VwapLevels => {
                let price_precision = config.additional_params.get("price_precision").copied().unwrap_or(0.01);
                Ok(Self::VwapLevels(Box::new(VolumeWeightedPriceLevels::new(period, price_precision))))
            }
            // ENTROPY category (2 indicators)
            BarIndicatorId::Sampen => {
                let m = config.additional_params.get("pattern_length").copied().unwrap_or(2.0) as usize;
                let r = config.additional_params.get("threshold_ratio").copied().unwrap_or(0.15);
                Ok(Self::Sampen(Box::new(SampleEntropy::new(period, m, r))))
            }
            BarIndicatorId::Xmil => {
                let bins = config.additional_params.get("bins").copied().unwrap_or(10.0) as usize;
                let clip_abs = config.additional_params.get("clip_abs").copied().unwrap_or(3.0);
                let lags = vec![1, 2, 3, 5, 10]; // Default lags
                Ok(Self::Xmil(Box::new(CrossMutualInformationLags::new(period, &lags, bins, clip_abs))))
            }
            BarIndicatorId::Cmo => Ok(Self::Cmo(Box::new(Cmo::new(period, None)))),
            BarIndicatorId::Bias => Ok(Self::Bias(Box::new(Bias::new(period, None)))),
            BarIndicatorId::Amat => {
                let fast = config.periods.first().copied().unwrap_or(10);
                let slow = config.periods.get(1).copied().unwrap_or(21);
                let signal = config.periods.get(2).copied().unwrap_or(5);
                let fast_ma = config.ma_types.get("fast_ma").copied().unwrap_or(MovingAverageType::SMA);
                let slow_ma = config.ma_types.get("slow_ma").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::Amat(Box::new(Amat::new(fast, slow, signal, fast_ma, slow_ma))))
            }
            BarIndicatorId::Adx => Ok(Self::Adx(Box::new(AdxIndicator::new(period)))),
            BarIndicatorId::DiPlusMinus => Ok(Self::DiPlusMinus(Box::new(DiPlusMinus::with_period(period)))),
            BarIndicatorId::Dm => Ok(Self::Dm(Box::new(Dm::new(period)))),
            BarIndicatorId::VhfMa => {
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::VhfMa(Box::new(VhfMa::new(period, ma_type))))
            }
            BarIndicatorId::Stochkd => {
                let period_k = config.periods.first().copied().unwrap_or(14);
                let period_d = config.periods.get(1).copied().unwrap_or(3);
                Ok(Self::StochastikD(Box::new(StochastikD::new(period_k, period_d))))
            }
            BarIndicatorId::SwingsSoft => Ok(Self::SwingsSoft(Box::new(SwingsSoft::new(period)))),
            BarIndicatorId::WilliamsR => Ok(Self::WilliamsR(Box::new(WilliamsR::new(period)))),
            BarIndicatorId::Demarker => Ok(Self::Demarker(Box::new(Demarker::new(period)))),
            BarIndicatorId::Psar => {
                let af_start = config.additional_params.get("af_start").copied().unwrap_or(0.02);
                let af_inc = config.additional_params.get("af_increment").copied().unwrap_or(0.02);
                let af_max = config.additional_params.get("af_max").copied().unwrap_or(0.20);
                Ok(Self::ParabolicSAR(Box::new(ParabolicSAR::with_params(af_start, af_inc, af_max))))
            }
            BarIndicatorId::Uo => {
                // Ensure increasing default periods (7,14,28)
                Ok(Self::UltimateOscillator(Box::new(UltimateOscillator::with_periods(7, 14, 28))))
            }
            BarIndicatorId::UoSmooth => {
                let p1 = config.periods.first().copied().unwrap_or(7);
                let p2 = config.periods.get(1).copied().unwrap_or(14);
                let p3 = config.periods.get(2).copied().unwrap_or(28);
                let smooth = config.periods.get(3).copied().unwrap_or(9);
                Ok(Self::UltimateOscillatorSmooth(Box::new(UltimateOscillatorSmooth::new(p1,p2,p3,smooth))))
            }
            BarIndicatorId::Rwi => Ok(Self::Rwi(Box::new(Rwi::new(period)))),
            BarIndicatorId::Bop => Ok(Self::Bop(Box::default())),
            BarIndicatorId::Cfo => Ok(Self::Cfo(Box::new(Cfo::new(period)))),
            BarIndicatorId::Rmi => {
                let momentum_lookback = config.periods.first().copied().unwrap_or(5);
                let ema_period = config.periods.get(1).copied().unwrap_or(14);
                Ok(Self::Rmi(Box::new(Rmi::new(momentum_lookback, ema_period))))
            }
            BarIndicatorId::Qstick => Ok(Self::Qstick(Box::new(Qstick::new(period)))),
            BarIndicatorId::Coppock => {
                let r1 = config.periods.first().copied().unwrap_or(11);
                let r2 = config.periods.get(1).copied().unwrap_or(14);
                let wma = config.periods.get(2).copied().unwrap_or(10);

                // Check if we have custom source
                if config.source != OhlcvField::Close {
                    Ok(Self::CoppockCurve(Box::new(CoppockCurve::with_source(r1, r2, wma, config.source))))
                } else {
                    Ok(Self::CoppockCurve(Box::new(CoppockCurve::new(r1, r2, wma))))
                }
            }
            BarIndicatorId::Apo => {
                let fast = config.periods.first().copied().unwrap_or(12);
                let slow = config.periods.get(1).copied().unwrap_or(26);
                Ok(Self::Apo(Box::new(Apo::new(fast, slow))))
            }
            BarIndicatorId::Pmo => {
                let roc_p = config.periods.first().copied().unwrap_or(10);
                let s1 = config.periods.get(1).copied().unwrap_or(10);
                let s2 = config.periods.get(2).copied().unwrap_or(10);
                Ok(Self::Pmo(Box::new(Pmo::new(roc_p, s1, s2))))
            }
            BarIndicatorId::Tsi => {
                let p1 = config.periods.first().copied().unwrap_or(25);
                let p2 = config.periods.get(1).copied().unwrap_or(13);
                let sig = config.periods.get(2).copied().unwrap_or(13);

                // Check if we have custom MA types or source
                let has_ma_types = config.ma_types.contains_key("smoothing_ma_type")
                    || config.ma_types.contains_key("signal_ma_type");

                if has_ma_types || config.source != OhlcvField::Close {
                    let smoothing_ma_type = config.ma_types.get("smoothing_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let signal_ma_type = config.ma_types.get("signal_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let source = config.source;

                    Ok(Self::TrueStrengthIndex(Box::new(TrueStrengthIndex::with_full_config(
                        p1, p2, sig,
                        smoothing_ma_type, signal_ma_type, source
                    ))))
                } else {
                    Ok(Self::TrueStrengthIndex(Box::new(TrueStrengthIndex::with_params_default(p1, p2, sig))))
                }
            }
            BarIndicatorId::Dpo => {
                // Check if we have custom source
                if config.source != OhlcvField::Close {
                    Ok(Self::DetrendedPriceOscillator(Box::new(DetrendedPriceOscillator::with_source(period, config.source))))
                } else {
                    Ok(Self::DetrendedPriceOscillator(Box::new(DetrendedPriceOscillator::with_period(period))))
                }
            }
            BarIndicatorId::Kst => {
                // KST uses default params: roc=[10,15,20,30], sma=[10,10,10,15], signal=9
                let roc_periods = [
                    config.periods.first().copied().unwrap_or(10),
                    config.periods.get(1).copied().unwrap_or(15),
                    config.periods.get(2).copied().unwrap_or(20),
                    config.periods.get(3).copied().unwrap_or(30),
                ];
                let sma_periods = [
                    config.periods.get(4).copied().unwrap_or(10),
                    config.periods.get(5).copied().unwrap_or(10),
                    config.periods.get(6).copied().unwrap_or(10),
                    config.periods.get(7).copied().unwrap_or(15),
                ];
                let signal_period = config.periods.get(8).copied().unwrap_or(9);

                // Check if we have custom MA types or source
                let has_ma_types = config.ma_types.contains_key("roc_ma_type")
                    || config.ma_types.contains_key("signal_ma_type");

                if has_ma_types || config.source != OhlcvField::Close {
                    let roc_ma_type = config.ma_types.get("roc_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);
                    let signal_ma_type = config.ma_types.get("signal_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);
                    let source = config.source;

                    Ok(Self::KnowSureThing(Box::new(KnowSureThing::with_full_config(
                        roc_periods, sma_periods, signal_period,
                        roc_ma_type, signal_ma_type, source
                    ))))
                } else {
                    Ok(Self::KnowSureThing(Box::new(KnowSureThing::with_params_default(
                        roc_periods, sma_periods, signal_period
                    ))))
                }
            }
            BarIndicatorId::Rvgi => {
                let p = config.periods.first().copied().unwrap_or(14);
                let sig = config.periods.get(1).copied().unwrap_or(9);
                Ok(Self::Rvgi(Box::new(Rvgi::new(p, sig))))
            }
            BarIndicatorId::Smi => {
                let p = config.periods.first().copied().unwrap_or(14);
                let sig = config.periods.get(1).copied().unwrap_or(3);
                Ok(Self::Smi(Box::new(Smi::new(p, sig))))
            }
            BarIndicatorId::Stc => {
                let fast = config.periods.first().copied().unwrap_or(12);
                let slow = config.periods.get(1).copied().unwrap_or(26);
                let kp = config.periods.get(2).copied().unwrap_or(10);
                let dp = config.periods.get(3).copied().unwrap_or(3);
                Ok(Self::Stc(Box::new(Stc::new(fast, slow, kp, dp))))
            }
            BarIndicatorId::ElderImpulse => {
                let ema_period = config.periods.first().copied().unwrap_or(13);

                // Check if we have custom MA type or source
                let has_ma_type = config.ma_types.contains_key("ma_type");

                if has_ma_type || config.source != OhlcvField::Close {
                    let ma_type = config.ma_types.get("ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let source = config.source;

                    Ok(Self::ElderImpulse(Box::new(ElderImpulseSystem::with_full_config(
                        ema_period, ma_type, source
                    ))))
                } else {
                    Ok(Self::ElderImpulse(Box::new(ElderImpulseSystem::new(ema_period))))
                }
            }
            BarIndicatorId::ElderRay => Ok(Self::ElderRay(Box::new(ElderRay::new(period)))),
            BarIndicatorId::Vortex => Ok(Self::VortexIndicator(Box::new(VortexIndicator::with_period(period)))),
            BarIndicatorId::StochRsi => {
                let rsi_p = config.periods.first().copied().unwrap_or(14);
                let stoch_p = config.periods.get(1).copied().unwrap_or(14);
                let k_p = config.periods.get(2).copied().unwrap_or(3);
                let d_p = config.periods.get(3).copied().unwrap_or(3);

                // Check if we have custom MA types or source
                let has_ma_types = config.ma_types.contains_key("k_ma_type")
                    || config.ma_types.contains_key("d_ma_type");

                if has_ma_types || config.source != OhlcvField::Close {
                    let k_ma_type = config.ma_types.get("k_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);
                    let d_ma_type = config.ma_types.get("d_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);
                    let source = config.source;

                    Ok(Self::StochasticRsi(Box::new(StochasticRsi::with_full_config(
                        rsi_p, stoch_p, k_p, d_p,
                        k_ma_type, d_ma_type, source
                    ))))
                } else {
                    Ok(Self::StochasticRsi(Box::new(StochasticRsi::new(rsi_p, stoch_p, k_p, d_p))))
                }
            }
            BarIndicatorId::MoFisher => {
                let p = config.periods.first().copied().unwrap_or(10);
                let sp = config.periods.get(1).copied().unwrap_or(3);
                Ok(Self::FisherTransform(Box::new(FisherTransform::new(p, sp))))
            }
            BarIndicatorId::LaguerreRsi => Ok(Self::LaguerreRsi(Box::new(LaguerreRsi::new(period)))),
            BarIndicatorId::Rsx => Ok(Self::Rsx(Box::new(Rsx::new(period)))),
            BarIndicatorId::Qqe => {
                let p = config.periods.first().copied().unwrap_or(14);
                let sm = config.periods.get(1).copied().unwrap_or(5);
                let mult = config.additional_params.get("threshold_mult").copied().unwrap_or(1.5);
                Ok(Self::Qqe(Box::new(Qqe::new(p, sm, mult))))
            }
            BarIndicatorId::Kdj => {
                let k = config.periods.first().copied().unwrap_or(14);
                let d = config.periods.get(1).copied().unwrap_or(3);
                Ok(Self::Kdj(Box::new(Kdj::new(k, d))))
            }
            BarIndicatorId::ConnorsRsi => {
                let rsi_p = config.periods.first().copied().unwrap_or(3);
                let up_p = config.periods.get(1).copied().unwrap_or(2);
                let roc_p = config.periods.get(2).copied().unwrap_or(100);
                Ok(Self::ConnorsRsi(Box::new(ConnorsRsi::with_periods(rsi_p, up_p, roc_p))))
            }
            BarIndicatorId::Trix => {
                let p = config.periods.first().copied().unwrap_or(14);
                let sig = config.periods.get(1).copied().unwrap_or(9);

                // Check if we have custom MA types or source
                let has_ma_types = config.ma_types.contains_key("smoothing_ma_type")
                    || config.ma_types.contains_key("signal_ma_type");

                if has_ma_types || config.source != OhlcvField::Close {
                    let smoothing_ma_type = config.ma_types.get("smoothing_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let signal_ma_type = config.ma_types.get("signal_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let source = config.source;

                    Ok(Self::Trix(Box::new(Trix::with_full_config(
                        p, sig,
                        smoothing_ma_type, signal_ma_type, source
                    ))))
                } else {
                    Ok(Self::Trix(Box::new(Trix::with_params_default(p, sig))))
                }
            }
            BarIndicatorId::IftRsi => Ok(Self::IftRsi(Box::new(IftRsi::new(period)))),
            BarIndicatorId::DpoPct => Ok(Self::DpoPercent(Box::new(DpoPercent::new(period)))),
            BarIndicatorId::Dsp => Ok(Self::DetrendedSyntheticPrice(Box::new(DetrendedSyntheticPrice::new(period)))),
            BarIndicatorId::RsiPctRank => {
                let rsi_p = config.periods.first().copied().unwrap_or(14);
                let win = config.periods.get(1).copied().unwrap_or(200);
                Ok(Self::RsiPercentileRank(Box::new(RsiPercentileRank::new(rsi_p, win))))
            }
            BarIndicatorId::RsiPctBands => {
                let rsi_p = config.periods.first().copied().unwrap_or(14);
                let win = config.periods.get(1).copied().unwrap_or(200);
                Ok(Self::RsiPercentileBands(Box::new(RsiPercentileBands::new(rsi_p, win))))
            }
            BarIndicatorId::Dss => {
                let k = config.periods.first().copied().unwrap_or(13);
                let s = config.periods.get(1).copied().unwrap_or(8);
                Ok(Self::DssBressert(Box::new(DssBressert::new(k, s))))
            }
            BarIndicatorId::Imi => Ok(Self::IntradayMomentumIndex(Box::new(IntradayMomentumIndex::new(period)))),

            // Volatility & Channels
            BarIndicatorId::VoDc => Ok(Self::Dc(Box::new(Dc::new(period)))),
            BarIndicatorId::VoKc => {
                let k = config.additional_params.get("k").copied().unwrap_or(2.0);
                Ok(Self::Kc(Box::new(Kc::new(period, k))))
            }
            BarIndicatorId::Rvi => Ok(Self::Rvi(Box::new(Rvi::new(period)))),
            BarIndicatorId::VoVr => {
                let fast = config.periods.first().copied().unwrap_or(10).clamp(2, 512);
                let slow = config.periods.get(1).copied().unwrap_or(20).max(fast+1).min(512);
                Ok(Self::Vr(Box::new(Vr::new(fast, slow))))
            }
            BarIndicatorId::Har => {
                let d = config.periods.first().copied().unwrap_or(5);
                let w = config.periods.get(1).copied().unwrap_or(22);
                let m = config.periods.get(2).copied().unwrap_or(66);
                let ann = config.additional_params.get("annualize_factor").copied().unwrap_or(252.0_f64.sqrt());
                Ok(Self::HarRv(Box::new(HarRv::new(d, w, m, ann))))
            }
            BarIndicatorId::Atrpt => {
                let w = config.periods.first().copied().unwrap_or(30);
                Ok(Self::BipowerVariance(Box::new(BipowerVariance::new(w))))
            }
            BarIndicatorId::Rq => {
                let w = config.periods.first().copied().unwrap_or(30);
                Ok(Self::RealizedQuarticity(Box::new(RealizedQuarticity::new(w))))
            }
            BarIndicatorId::Hvc2c => {
                let w = config.periods.first().copied().unwrap_or(30);
                Ok(Self::HistoricalVolatilityC2C(Box::new(HistoricalVolatilityC2C::new(w))))
            }
            BarIndicatorId::Atrchan => {
                let ma_p = config.periods.first().copied().unwrap_or(20);
                let atr_p = config.periods.get(1).copied().unwrap_or(14);
                let k = config.additional_params.get("k").copied().unwrap_or(2.0);
                let center_ma = config.ma_types.get("center_ma").copied().unwrap_or(MovingAverageType::SMA);
                let atr_ma = config.ma_types.get("atr_ma").copied().unwrap_or(MovingAverageType::EMA);
                Ok(Self::AtrChannels(Box::new(AtrBands::new(ma_p, center_ma, atr_p, atr_ma, k))))
            }
            BarIndicatorId::Kp => {
                let p = config.periods.first().copied().unwrap_or(20);
                let k = config.additional_params.get("k").copied().unwrap_or(2.0);
                Ok(Self::Kp(Box::new(Kp::new(p, k))))
            }
            BarIndicatorId::Fuzzy => {
                let per = config.periods.first().copied().unwrap_or(50);
                let t1 = config.additional_params.get("t1").copied().unwrap_or(0.5);
                let t2 = config.additional_params.get("t2").copied().unwrap_or(1.0);
                let t3 = config.additional_params.get("t3").copied().unwrap_or(1.5);
                let t4 = config.additional_params.get("t4").copied().unwrap_or(2.0);
                Ok(Self::FuzzyCandlesticks(Box::new(FuzzyCandlesticks::new(per, t1, t2, t3, t4))))
            }

            // Accumulation/Volume/Misc
            BarIndicatorId::Ad => Ok(Self::AccumulationDistribution(Box::new(AccumulationDistribution::new()))),
            BarIndicatorId::Mfi => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::Mfi(Box::new(Mfi::new(p))))
            }
            BarIndicatorId::Wad => Ok(Self::WilliamsAd(Box::default())),
            BarIndicatorId::Vdelta => {
                let p = config.periods.first().copied().unwrap_or(50);
                Ok(Self::Vdelta(Box::new(VolumeDelta::new(p))))
            }
            BarIndicatorId::Vprofile => {
                let tick = config.additional_params.get("tick_size").copied().unwrap_or(0.01);
                let sess = config.additional_params.get("session_duration").copied().unwrap_or(0.0) as i64;
                Ok(Self::VolumeProfile(Box::new(VolumeProfile::new(tick.max(1e-6), sess))))
            }
            BarIndicatorId::Vpt => {
                let sp = config.periods.first().copied().unwrap_or(21);
                Ok(Self::VolumePriceTrend(Box::new(VolumePriceTrend::with_signal_period(sp))))
            }
            BarIndicatorId::Vroc => {
                let p = config.periods.first().copied().unwrap_or(14);
                let sp = config.periods.get(1).copied().unwrap_or(9);
                Ok(Self::VolumeRateOfChange(Box::new(VolumeRateOfChange::with_params(p, sp))))
            }
            BarIndicatorId::NviPvi => {
                let nvi_ma = config.periods.first().copied().unwrap_or(255);
                let pvi_ma = config.periods.get(1).copied().unwrap_or(255);
                Ok(Self::NviPvi(Box::new(NegativePositiveVolumeIndex::with_params(nvi_ma, pvi_ma))))
            }
            BarIndicatorId::Trin => {
                let smooth = config.periods.first().copied().unwrap_or(10);
                Ok(Self::Trin(Box::new(TrinIndicator::with_smoothing(smooth))))
            }
            BarIndicatorId::Vpin => {
                let buckets = config.periods.first().copied().unwrap_or(50);
                let bucket_volume = config.additional_params.get("bucket_volume").copied().unwrap_or(10_000.0);
                Ok(Self::Vpin(Box::new(Vpin::new(buckets, bucket_volume))))
            }
            BarIndicatorId::Cmf => {
                let p = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Cmf(Box::new(ChaikinMoneyFlow::new(p))))
            }
            BarIndicatorId::Eom => {
                let smooth = config.periods.first().copied().unwrap_or(14);
                let scale = config.additional_params.get("scale_factor").copied().unwrap_or(100_000_000.0);
                Ok(Self::EaseOfMovement(Box::new(EaseOfMovement::with_params(smooth, scale))))
            }
            BarIndicatorId::Fi => {
                let ema = config.periods.first().copied().unwrap_or(13);
                Ok(Self::ForceIndex(Box::new(ForceIndex::with_ema(ema))))
            }
            BarIndicatorId::Cho => {
                let fast = config.periods.first().copied().unwrap_or(3);
                let slow = config.periods.get(1).copied().unwrap_or(10);
                Ok(Self::Cho(Box::new(ChaikinOscillator::new(fast, slow))))
            }
            BarIndicatorId::Ii => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::IntradayIntensity(Box::new(IntradayIntensity::new(p))))
            }
            BarIndicatorId::Asi => Ok(Self::AccumulativeSwingIndex(Box::default())),
            BarIndicatorId::Di => Ok(Self::DemandIndex(Box::default())),
            BarIndicatorId::Kvo => {
                let fast = config.periods.first().copied().unwrap_or(34);
                let slow = config.periods.get(1).copied().unwrap_or(55);
                let signal = config.periods.get(2).copied().unwrap_or(13);
                Ok(Self::Kvo(Box::new(Kvo::new(fast, slow, signal))))
            }
            // Duplicate short form removed; detailed arm below
            BarIndicatorId::Supertrend => {
                let p = config.periods.first().copied().unwrap_or(10);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(3.0);
                Ok(Self::Supertrend(Box::new(Supertrend::with_params(p, mult))))
            }
            BarIndicatorId::Ssl => {
                let p = config.periods.first().copied().unwrap_or(10);

                // Check if we have custom MA type
                let has_ma_type = config.ma_types.contains_key("ma_type");

                if has_ma_type {
                    let ma_type = config.ma_types.get("ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::SMA);

                    Ok(Self::SslChannel(Box::new(SslChannel::new_with_ma_type(p, ma_type))))
                } else {
                    Ok(Self::SslChannel(Box::new(SslChannel::new(p))))
                }
            }
            BarIndicatorId::Gmma => Ok(Self::GmmaCompression(Box::new(GmmaCompression::new()))),
            BarIndicatorId::Tii => {
                let w = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Tii(Box::new(TrendIntensityIndex::new(w))))
            }
            BarIndicatorId::Ravi => {
                let fast = config.periods.first().copied().unwrap_or(7);
                let slow = config.periods.get(1).copied().unwrap_or(65);
                Ok(Self::Ravi(Box::new(RaviTrend::new(fast, slow))))
            }
            BarIndicatorId::Didi => {
                let s = config.periods.first().copied().unwrap_or(3);
                let m = config.periods.get(1).copied().unwrap_or(8);
                let l = config.periods.get(2).copied().unwrap_or(20);
                Ok(Self::Didi(Box::new(DidiIndex::new(s, m, l))))
            }
            BarIndicatorId::HaTrend => Ok(Self::HaTrend(Box::default())),
            BarIndicatorId::Er => {
                let p = config.periods.first().copied().unwrap_or(10);
                Ok(Self::EfficiencyRatioFullHistory(Box::new(EfficiencyRatioFullHistory::new(p))))
            }
            BarIndicatorId::ErRing => {
                let p = config.periods.first().copied().unwrap_or(10);
                Ok(Self::EfficiencyRatioRingWindow(Box::new(EfficiencyRatioRingWindow::new(p))))
            }
            BarIndicatorId::SpreadAnalyzer => {
                let p = config.periods.first().copied().unwrap_or(20);
                Ok(Self::SpreadAnalyzer(Box::new(SpreadAnalyzer::new(p))))
            }
            BarIndicatorId::Vr => {
                let w = config.periods.first().copied().unwrap_or(100);
                let m = config.periods.get(1).copied().unwrap_or(5);
                Ok(Self::VarianceRatio(Box::new(VarianceRatio::new(w, m))))
            }
            BarIndicatorId::VrAgg => {
                // simple three-configs default
                let w1 = config.periods.first().copied().unwrap_or(100);
                let m1 = config.periods.get(1).copied().unwrap_or(2);
                let w2 = config.periods.get(2).copied().unwrap_or(100);
                let m2 = config.periods.get(3).copied().unwrap_or(5);
                let w3 = config.periods.get(4).copied().unwrap_or(100);
                let m3 = config.periods.get(5).copied().unwrap_or(10);
                let cfgs = vec![(w1, m1), (w2, m2), (w3, m3)];
                Ok(Self::VarianceRatioAggregate(Box::new(VarianceRatioAggregate::new(&cfgs))))
            }
            BarIndicatorId::ArchLm => {
                let w = config.periods.first().copied().unwrap_or(200);
                let l = config.periods.get(1).copied().unwrap_or(5);
                Ok(Self::ArchLmProxy(Box::new(ArchLmProxy::new(w, l))))
            }
            BarIndicatorId::Kpss => {
                let w = config.periods.first().copied().unwrap_or(200);
                Ok(Self::KpssProxy(Box::new(KpssProxy::new(w))))
            }
            BarIndicatorId::Adf => {
                let w = config.periods.first().copied().unwrap_or(200);
                Ok(Self::AdfProxy(Box::new(AdfProxy::new(w))))
            }
            BarIndicatorId::Pacf => {
                let w = config.periods.first().copied().unwrap_or(200);
                let k = config.periods.get(1).copied().unwrap_or(5);
                Ok(Self::Pacf(Box::new(Pacf::new(w, k))))
            }
            BarIndicatorId::LjungBox => {
                let w = config.periods.first().copied().unwrap_or(200);
                let k = config.periods.get(1).copied().unwrap_or(10);
                Ok(Self::LjungBox(Box::new(LjungBox::new(w, k))))
            }
            BarIndicatorId::Psars => {
                let af_start = config.additional_params.get("af_start").copied().unwrap_or(0.02);
                let af_inc = config.additional_params.get("af_increment").copied().unwrap_or(0.02);
                let af_max = config.additional_params.get("af_max").copied().unwrap_or(0.20);
                Ok(Self::PSARStop(Box::new(PSARStop::with_params(af_start, af_inc, af_max))))
            }
            BarIndicatorId::Supts => {
                let p = config.periods.first().copied().unwrap_or(10);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(3.0);
                Ok(Self::Supts(Box::new(SuperTrendStop::with_params(p, mult))))
            }
            BarIndicatorId::Atrts => {
                let p = config.periods.first().copied().unwrap_or(14);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                Ok(Self::Atrts(Box::new(ATRTrailingStop::with_params(p, mult))))
            }
            BarIndicatorId::Chand => {
                let p = config.periods.first().copied().unwrap_or(22);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(3.0);
                Ok(Self::Chand(Box::new(ChandelierStop::with_params(p, mult))))
            }
            BarIndicatorId::Volts => {
                let p = config.periods.first().copied().unwrap_or(20);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                let vtype_code = config.additional_params.get("volatility_type").copied().unwrap_or(0.0) as i32;
                let vtype = match vtype_code { 1 => VolatilityType::ATR, 2 => VolatilityType::Range, _ => VolatilityType::StandardDeviation };
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::VolatilityStop(Box::new(VolatilityStop::with_params(p, mult, vtype, ma_type))))
            }
            BarIndicatorId::Kelts => {
                let p = config.periods.first().copied().unwrap_or(20);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                let center_ma = config.ma_types.get("center_ma").copied().unwrap_or(MovingAverageType::EMA);
                let atr_ma = config.ma_types.get("atr_ma").copied().unwrap_or(MovingAverageType::RMA);
                Ok(Self::KeltnerStop(Box::new(KeltnerStop::with_params(p, mult, center_ma, atr_ma))))
            }
            BarIndicatorId::Dons => {
                let up = config.periods.first().copied().unwrap_or(20);
                let lo = config.periods.get(1).copied().unwrap_or(up);
                let offset = config.additional_params.get("offset").copied().unwrap_or(0.0);
                let use_pct = config.additional_params.get("use_percentage").copied().unwrap_or(0.0) != 0.0;
                Ok(Self::DonchianStop(Box::new(DonchianStop::with_different_periods(up, lo, offset, use_pct))))
            }
            BarIndicatorId::Cks => {
                let atr_p = config.periods.first().copied().unwrap_or(14);
                let k = config.additional_params.get("k").copied().unwrap_or(1.5);
                let hh = config.periods.get(1).copied().unwrap_or(22);
                let ll = config.periods.get(2).copied().unwrap_or(22);
                Ok(Self::ChandeKrollStop(Box::new(ChandeKrollStop::new(atr_p, k, hh, ll))))
            }
            BarIndicatorId::Donbo => {
                let p = config.periods.first().copied().unwrap_or(20);
                Ok(Self::DonchianBreakout(Box::new(DonchianBreakout::new(p))))
            }

            // Chaos (Williams)
            BarIndicatorId::Alligator => Ok(Self::Alligator(Box::default())),
            BarIndicatorId::Ao => Ok(Self::AwesomeOscillator(Box::default())),
            BarIndicatorId::Ac => Ok(Self::Ac(Box::default())),
            BarIndicatorId::WilliamsMfi => Ok(Self::MarketFacilitationIndex(Box::default())),
            // Ehlers / Cycles
            BarIndicatorId::Esine => {
                let alpha = config.additional_params.get("alpha").copied().unwrap_or(0.2);
                Ok(Self::Esine(Box::new(EhlersSinewave::new(alpha))))
            }
            BarIndicatorId::Cyber => {
                let alpha = config.additional_params.get("alpha").copied().unwrap_or(0.2);
                Ok(Self::Cyber(Box::new(CyberCycle::new(alpha))))
            }

            // Alias match arms for indicators with value() methods
            BarIndicatorId::TsSwings => {
                let look = config.periods.first().copied().unwrap_or(5);
                let min_sz = config.additional_params.get("min_swing_size").copied().unwrap_or(0.0);
                let offset = config.additional_params.get("offset").copied().unwrap_or(0.0);
                let use_pct = config.additional_params.get("use_percentage").copied().unwrap_or(0.0) != 0.0;
                Ok(Self::SwingStop(Box::new(SwingStop::with_params(look, min_sz, offset, use_pct))))
            }
            BarIndicatorId::VoltsAtr => {
                let p = config.periods.first().copied().unwrap_or(20);
                let mult = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                let vtype = VolatilityType::ATR; // Hardcoded to ATR for VoltsAtr variant
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::VolatilityStop(Box::new(VolatilityStop::with_params(p, mult, vtype, ma_type))))
            }
            BarIndicatorId::Hdc => Ok(Self::HilbertDominantCycle(Box::new(HilbertDominantCycle::new()))),

            // Kalman Filters
            BarIndicatorId::Kalman => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let q = config.additional_params.get("process_noise").copied().unwrap_or(1e-3);
                let r = config.additional_params.get("measurement_noise").copied().unwrap_or(1e-2);
                Ok(Self::BasicKalmanFilter(Box::new(BasicKalmanFilter::new(dt, q, r))))
            }
            BarIndicatorId::Ekf => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let q_std = config.additional_params.get("process_noise_std").copied().unwrap_or(0.05);
                let r_std = config.additional_params.get("measurement_noise_std").copied().unwrap_or(0.1);
                let friction = config.additional_params.get("friction").copied().unwrap_or(0.01);
                let obs_code = config.additional_params.get("observation_type").copied().unwrap_or(0.0) as i32;
                let obs = match obs_code { 1 => ObservationType::Logarithmic, 2 => ObservationType::Square, 3 => ObservationType::Sigmoid, _ => ObservationType::Linear };
                Ok(Self::ExtendedKalmanFilter(Box::new(ExtendedKalmanFilter::new(dt, q_std, r_std, friction, obs))))
            }
            BarIndicatorId::Ukf => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let q_std = config.additional_params.get("process_noise_std").copied().unwrap_or(0.05);
                let r_std = config.additional_params.get("measurement_noise_std").copied().unwrap_or(0.1);
                let alpha = config.additional_params.get("ut_alpha").copied().unwrap_or(0.001);
                let beta = config.additional_params.get("ut_beta").copied().unwrap_or(2.0);
                let kappa = config.additional_params.get("ut_kappa").copied().unwrap_or(0.0);
                let ut = UnscentedTransformParams { alpha, beta, kappa };
                Ok(Self::UnscentedKalmanFilter(Box::new(UnscentedKalmanFilter::new(dt, q_std, r_std, Some(ut)))))
            }
            BarIndicatorId::Particle => {
                let n = config.periods.first().copied().unwrap_or(200).clamp(10, 1000);
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let q_std = config.additional_params.get("process_noise_std").copied().unwrap_or(0.05);
                let r_std = config.additional_params.get("measurement_noise_std").copied().unwrap_or(0.1);
                let rs_code = config.additional_params.get("resampling").copied().unwrap_or(0.0) as i32;
                let rs = match rs_code { 1 => ResamplingStrategy::Stratified, 2 => ResamplingStrategy::Residual, 3 => ResamplingStrategy::Multinomial, _ => ResamplingStrategy::Systematic };
                Ok(Self::ParticleFilter(Box::new(ParticleFilter::new(n, dt, q_std, r_std, rs, None))))
            }

            // Adaptive
            BarIndicatorId::Adaptivema => {
                let mode_code = config.additional_params.get("adaptation_mode").copied().unwrap_or(4.0) as i32; // Combined
                let eff_code = config.additional_params.get("efficiency_method").copied().unwrap_or(0.0) as i32; // Kaufman
                let mode = match mode_code { 0 => AmaAdaptationMode::Volatility, 1 => AmaAdaptationMode::Volume, 2 => AmaAdaptationMode::Trend, 3 => AmaAdaptationMode::Momentum, 5 => AmaAdaptationMode::Market, _ => AmaAdaptationMode::Combined };
                let eff = match eff_code { 1 => EfficiencyMethod::Fractal, 2 => EfficiencyMethod::DirectionalMovement, 3 => EfficiencyMethod::TrendStrength, 4 => EfficiencyMethod::NoiseRatio, _ => EfficiencyMethod::Kaufman };
                Ok(Self::AdaptiveMovingAverage(Box::new(AdaptiveMovingAverage::new(period, mode, eff))))
            }
            BarIndicatorId::Kama => {
                let er = config.periods.first().copied().unwrap_or(10);
                let fast = config.periods.get(1).copied().unwrap_or(2);
                let slow = config.periods.get(2).copied().unwrap_or(30);
                Ok(Self::KaufmanAdaptiveMA(Box::new(KaufmanAdaptiveMA::with_source(er, fast, slow, config.source))))
            }
            BarIndicatorId::Vidya => {
                let p = config.periods.first().copied().unwrap_or(14);
                let cmo_code = config.additional_params.get("cmo_ma_type").copied().unwrap_or(0.0) as i32;
                let cmo = match cmo_code { 1 => VidyaCmoMaType::Exponential, 2 => VidyaCmoMaType::Linear, 3 => VidyaCmoMaType::Triangular, _ => VidyaCmoMaType::Simple };
                Ok(Self::Vidya(Box::new(VariableIndexDynamicAverage::new(p, cmo))))
            }
            BarIndicatorId::Framaadv => {
                let p = config.periods.first().copied().unwrap_or(20);
                let fm_code = config.additional_params.get("fractal_method").copied().unwrap_or(0.0) as i32;
                let fm = match fm_code { 1 => FractalMethod::Improved, 2 => FractalMethod::Dynamic, 3 => FractalMethod::Robust, _ => FractalMethod::Standard };
                Ok(Self::Framaadv(Box::new(FractalAdaptiveMovingAverage::new(p, fm))))
            }

            // Entropy
            BarIndicatorId::Shannon => {
                let p = config.periods.first().copied().unwrap_or(100).min(512);
                let bins = config.additional_params.get("bins").copied().unwrap_or(20.0) as usize;
                Ok(Self::ShannonEntropy(Box::new(ShannonEntropy::new(p, bins))))
            }
            BarIndicatorId::Apen => {
                let p = config.periods.first().copied().unwrap_or(100).min(512);
                let m = config.additional_params.get("m").copied().unwrap_or(2.0) as usize;
                let r = config.additional_params.get("r").copied().unwrap_or(0.0);
                Ok(Self::Apen(Box::new(ApproximateEntropy::new(p, m, r))))
            }
            BarIndicatorId::Pe => {
                let p = config.periods.first().copied().unwrap_or(100).min(512);
                let m = config.additional_params.get("m").copied().unwrap_or(2.0) as usize;
                let r = config.additional_params.get("r").copied().unwrap_or(0.0);
                Ok(Self::SampleEntropy(Box::new(SampleEntropy::new(p, m, r))))
            }

            // Chaos / Fractal / Hurst / DFA
            BarIndicatorId::FractalDim => {
                let period = config.periods.first().copied().unwrap_or(128).min(512);
                let max_k = config.periods.get(1).copied().unwrap_or(period / 8);
                Ok(Self::FractalDimension(Box::new(FractalDimension::new(period, max_k))))
            }
            BarIndicatorId::Hurst => {
                let period = config.periods.first().copied().unwrap_or(128).min(512);
                Ok(Self::Hurst(Box::new(HurstExponent::new(period))))
            }
            BarIndicatorId::HurstPct => {
                let window = config.periods.first().copied().unwrap_or(200).min(1024);
                Ok(Self::HurstPct(Box::new(HurstPercentile::new(window))))
            }
            BarIndicatorId::ChaosOsc => {
                let period = config.periods.first().copied().unwrap_or(128).min(512);
                let w1 = config.additional_params.get("complexity_weight").copied().unwrap_or(0.4);
                let w2 = config.additional_params.get("persistence_weight").copied().unwrap_or(0.4);
                let w3 = config.additional_params.get("volatility_weight").copied().unwrap_or(0.2);
                Ok(Self::ChaosOscillator(Box::new(ChaosOscillator::new_with_weights(period, w1, w2, w3))))
            }
            BarIndicatorId::Fractals => Ok(Self::Fractals(Box::default())),
            BarIndicatorId::Dfa => {
                let s0 = config.periods.first().copied().unwrap_or(16);
                let s1 = config.periods.get(1).copied().unwrap_or(32);
                let s2 = config.periods.get(2).copied().unwrap_or(64);
                let s3 = config.periods.get(3).copied().unwrap_or(128);
                Ok(Self::Dfa(Box::new(Dfa::new([s0, s1, s2, s3]))))
            }
            BarIndicatorId::DfaPct => {
                let s0 = config.periods.first().copied().unwrap_or(16);
                let s1 = config.periods.get(1).copied().unwrap_or(32);
                let s2 = config.periods.get(2).copied().unwrap_or(64);
                let s3 = config.periods.get(3).copied().unwrap_or(128);
                let w = config.periods.get(4).copied().unwrap_or(200);
                Ok(Self::DfaPercentile(Box::new(DfaPercentile::new([s0, s1, s2, s3], w))))
            }

            // Z-Score & EWMAC
            BarIndicatorId::RsiZscore => {
                let rsi_p = config.periods.first().copied().unwrap_or(14);
                let win = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::RsiZscore(Box::new(RsiZscore::new(rsi_p, win))))
            }
            BarIndicatorId::MacdHistZ => {
                let fast = config.periods.first().copied().unwrap_or(12);
                let slow = config.periods.get(1).copied().unwrap_or(26);
                let signal = config.periods.get(2).copied().unwrap_or(9);
                let window = config.periods.get(3).copied().unwrap_or(100);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::EMA);
                Ok(Self::MacdHistZscore(Box::new(MacdHistZscore::new(fast, slow, signal, ma_type, window))))
            }
            BarIndicatorId::PriceZscore => {
                let p = config.periods.first().copied().unwrap_or(100).max(2);
                Ok(Self::PriceZScore(Box::new(PriceZScore::new(p))))
            }
            BarIndicatorId::Zmad => {
                let win = config.periods.first().copied().unwrap_or(100).max(3);
                Ok(Self::PriceMadZscore(Box::new(PriceMadZscore::new(win))))
            }
            BarIndicatorId::MomZscore => {
                let diff = config.periods.first().copied().unwrap_or(1).max(1);
                let win = config.periods.get(1).copied().unwrap_or(100).max(2);
                Ok(Self::MomentumZscore(Box::new(MomentumZscore::new(diff, win))))
            }
            BarIndicatorId::Ewmac => {
                let fast = config.periods.first().copied().unwrap_or(16).max(1);
                let slow = config.periods.get(1).copied().unwrap_or(64).max(2);
                Ok(Self::Ewmac(Box::new(Ewmac::new(fast, slow))))
            }

            // Ehlers & Filters
            BarIndicatorId::Roof => {
                let hp_alpha = config.additional_params.get("hp_alpha").copied().unwrap_or(0.2);
                let lp_alpha = config.additional_params.get("lp_alpha").copied().unwrap_or(0.1);
                Ok(Self::Roof(Box::new(RoofingFilter::new(hp_alpha, lp_alpha))))
            }
            BarIndicatorId::Decyc => {
                let alpha = config.additional_params.get("alpha").copied().unwrap_or(0.1);
                Ok(Self::Decyc(Box::new(Decycler::new(alpha))))
            }
            BarIndicatorId::Ehlersfa => {
                let p = config.periods.first().copied().unwrap_or(20);
                Ok(Self::EhlersFractalAdaptiveMa(Box::new(EhlersFractalAdaptiveMa::with_source(p, config.source))))
            }
            BarIndicatorId::Ehlersz => {
                let p = config.periods.first().copied().unwrap_or(21);
                Ok(Self::EhlersZeroLagEma(Box::new(EhlersZeroLagEma::with_source(p, config.source))))
            }
            BarIndicatorId::Eit => {
                let alpha = config.additional_params.get("alpha").copied().unwrap_or(0.07);
                Ok(Self::Eit(Box::new(EhlersInstantaneousTrendline::with_source(alpha, config.source))))
            }
            BarIndicatorId::Mama => Ok(Self::MesaAdaptiveMA(Box::new(MesaAdaptiveMA::with_source(8.0, 50.0, MovingAverageType::EMA, config.source)))),
            BarIndicatorId::Ess => {
                let period = config.additional_params.get("period").copied().unwrap_or(10.0);
                Ok(Self::Ess(Box::new(EhlersSuperSmoother::with_period(period))))
            }

            // RSI variants
            BarIndicatorId::AtrRsi => Ok(Self::AtrRsi(Box::new(AtrRsi::new()))),
            BarIndicatorId::Vwrsi => Ok(Self::VolumeWeightedRsi(Box::new(VolumeWeightedRsi::new()))),
            BarIndicatorId::Rsioma => {
                let rsi_p = config.periods.first().copied().unwrap_or(14);
                let ema_p = config.periods.get(1).copied().unwrap_or(9);
                Ok(Self::RsiOma(Box::new(RsiOma::new(rsi_p, ema_p))))
            }
            BarIndicatorId::Tdi => {
                let rsi_period = config.periods.first().copied().unwrap_or(13);
                let signal_period = config.periods.get(1).copied().unwrap_or(2);
                let band_period = config.periods.get(2).copied().unwrap_or(34);
                Ok(Self::Tdi(Box::new(Tdi::new(rsi_period, signal_period, band_period))))
            }

            // Logic gates / combiners
            BarIndicatorId::Thresh => {
                let lower = config.additional_params.get("lower").copied().unwrap_or(30.0);
                let upper = config.additional_params.get("upper").copied().unwrap_or(70.0);
                Ok(Self::Thresh(Box::new(ThresholdGate::new(lower, upper))))
            }
            BarIndicatorId::Logicand => Ok(Self::AndGate(Box::default())),
            BarIndicatorId::Logicor => Ok(Self::OrGate(Box::default())),
            BarIndicatorId::Logicxor => Ok(Self::XorGate(Box::default())),
            BarIndicatorId::Logicsign => Ok(Self::SignCombiner(Box::default())),

            // Realized Vol / Stats
            BarIndicatorId::Rv => {
                let win = config.periods.first().copied().unwrap_or(21);
                let ann = config.additional_params.get("annualize_factor").copied().unwrap_or(252.0_f64.sqrt());
                Ok(Self::RealizedVol(Box::new(RealizedVol::new(win, ann))))
            }
            BarIndicatorId::Rvz => {
                let vp = config.periods.first().copied().unwrap_or(21);
                let zw = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::RealizedVolZscore(Box::new(RealizedVolZscore::new(vp, zw))))
            }
            BarIndicatorId::Autocorr => {
                let lag = config.periods.first().copied().unwrap_or(1);
                let win = config.periods.get(1).copied().unwrap_or(50);
                Ok(Self::Autocorr(Box::new(Autocorr::new(lag, win))))
            }
            BarIndicatorId::Hmom => {
                let win = config.periods.first().copied().unwrap_or(50);
                Ok(Self::Hmom(Box::new(HigherMoments::new(win))))
            }
            BarIndicatorId::RocPct => {
                let p = config.periods.first().copied().unwrap_or(10);
                let win = config.periods.get(1).copied().unwrap_or(200);
                Ok(Self::RocPercentile(Box::new(RocPercentile::new(p, win))))
            }
            BarIndicatorId::Nr => {
                let win = config.periods.first().copied().unwrap_or(20);
                Ok(Self::NrRange(Box::new(NrRange::new(win))))
            }

            // Market regime & adaptive oscillators
            BarIndicatorId::Mrf => Ok(Self::MarketRegimeFilter(Box::new(MarketRegimeFilter::new()))),
            BarIndicatorId::AdaptiveStoch => Ok(Self::AdaptiveStochastic(Box::new(AdaptiveStochastic::new()))),

            // Breakout/filters
            BarIndicatorId::Vbexp => Ok(Self::VolatilityBreakoutDetector(Box::new(VolatilityBreakoutDetector::new()))),
            BarIndicatorId::Cusum => {
                let thr = config.additional_params.get("threshold").copied().unwrap_or(0.01);
                Ok(Self::CusumFilter(Box::new(CusumFilter::new(thr))))
            }
            BarIndicatorId::StCusum => {
                let thr = config.additional_params.get("threshold").copied().unwrap_or(0.01);
                Ok(Self::CusumFilter(Box::new(CusumFilter::new(thr)))) // statistics/ version
            }

            // Entropy / information
            BarIndicatorId::Mi => {
                let w = config.periods.first().copied().unwrap_or(50);
                let lag = config.periods.get(1).copied().unwrap_or(1);
                let bins = config.additional_params.get("bins").copied().unwrap_or(8.0) as usize;
                let clip = config.additional_params.get("clip_abs").copied().unwrap_or(0.15);
                Ok(Self::MutualInformation(Box::new(MutualInformation::new(w, lag, bins, clip))))
            }
            BarIndicatorId::Te => {
                let w = config.periods.first().copied().unwrap_or(50);
                let lag = config.periods.get(1).copied().unwrap_or(1);
                let bins = config.additional_params.get("bins").copied().unwrap_or(8.0) as usize;
                let clip = config.additional_params.get("clip_abs").copied().unwrap_or(0.15);
                Ok(Self::TransferEntropy(Box::new(TransferEntropy::new(w, lag, bins, clip))))
            }
            BarIndicatorId::Kld => {
                let w = config.periods.first().copied().unwrap_or(200);
                let bins = config.additional_params.get("bins").copied().unwrap_or(16.0) as usize;
                let clip = config.additional_params.get("clip_abs").copied().unwrap_or(0.05);
                Ok(Self::KLDivergence(Box::new(KLDivergence::new(w, bins, clip))))
            }
            BarIndicatorId::Jsd => {
                let w = config.periods.first().copied().unwrap_or(200);
                let bins = config.additional_params.get("bins").copied().unwrap_or(16.0) as usize;
                let clip = config.additional_params.get("clip_abs").copied().unwrap_or(0.05);
                Ok(Self::JSDivergence(Box::new(JSDivergence::new(w, bins, clip))))
            }
            BarIndicatorId::Fisher => {
                let w = config.periods.first().copied().unwrap_or(200);
                Ok(Self::RollingFisherInformation(Box::new(RollingFisherInformation::new(w))))
            }
            BarIndicatorId::Infog => {
                let w = config.periods.first().copied().unwrap_or(100);
                let bins = config.additional_params.get("bins").copied().unwrap_or(8.0) as usize;
                let clip = config.additional_params.get("clip_abs").copied().unwrap_or(0.05);
                Ok(Self::InformationGain(Box::new(InformationGain::new(w, bins, clip))))
            }
            BarIndicatorId::Lz => {
                let w = config.periods.first().copied().unwrap_or(64);
                Ok(Self::Lz(Box::new(LempelZivComplexity::new(w))))
            }

            // Logic/composites
            BarIndicatorId::Hyst => {
                let lower = config.additional_params.get("lower").copied().unwrap_or(30.0);
                let upper = config.additional_params.get("upper").copied().unwrap_or(70.0);
                Ok(Self::Hyst(Box::new(HysteresisGate::new(lower, upper))))
            }
            BarIndicatorId::Wcomp => {
                let w1 = config.additional_params.get("w1").copied().unwrap_or(1.0);
                let w2 = config.additional_params.get("w2").copied().unwrap_or(0.0);
                let w3 = config.additional_params.get("w3").copied().unwrap_or(0.0);
                let w4 = config.additional_params.get("w4").copied().unwrap_or(0.0);
                let norm = config.additional_params.get("normalize").copied().unwrap_or(1.0) != 0.0;
                Ok(Self::WeightedComposite(Box::new(WeightedComposite::new(w1, w2, w3, w4, norm))))
            }

            // Candles/patterns
            BarIndicatorId::Sfp => {
                let lb = config.periods.first().copied().unwrap_or(20);
                Ok(Self::SfpDetector(Box::new(SfpDetector::new(lb))))
            }

            // Momentum/trend/distance
            BarIndicatorId::EhlersRocket => Ok(Self::EhlersRocketRsi(Box::new(EhlersRocketRsi::new()))),
            BarIndicatorId::EhlersCc => {
                let alpha = config.additional_params.get("alpha").copied().unwrap_or(0.07);
                Ok(Self::EhlersCc(Box::new(EhlersCyberCycle::new(alpha))))
            }
            BarIndicatorId::Zlsma => {
                let w = config.periods.first().copied().unwrap_or(50);
                Ok(Self::ZlSma(Box::new(ZlSma::new(w))))
            }
            BarIndicatorId::VwapDist => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::VwapDistance(Box::new(VwapDistance::new(p))))
            }
            BarIndicatorId::Hlva => {
                let w = config.periods.first().copied().unwrap_or(50);
                Ok(Self::HlValueArea(Box::new(HlValueArea::new(w))))
            }
            BarIndicatorId::SweepRev => {
                let look = config.periods.first().copied().unwrap_or(40);
                let atr_p = config.periods.get(1).copied().unwrap_or(14);
                let quart = config.additional_params.get("close_quartile").copied().unwrap_or(0.35);
                let k = config.additional_params.get("weight_k").copied().unwrap_or(1.0);
                let confirm = config.additional_params.get("confirm_next_bar").copied().unwrap_or(0.0) != 0.0;
                let atr_ma = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::RMA);
                let params = SweepReversionParams { lookback_period: look, close_quartile: quart, atr_period: atr_p, weight_k: k, confirm_next_bar: confirm, atr_ma_type: atr_ma };
                Ok(Self::SweepReversionIndex(Box::new(SweepReversionIndex::new(params))))
            }

            // Entropy/statistics/econometrics
            BarIndicatorId::Conden => {
                let w = config.periods.first().copied().unwrap_or(100);
                let bins = config.additional_params.get("bins").copied().unwrap_or(8.0) as usize;
                let clip = config.additional_params.get("clip_abs").copied().unwrap_or(0.05);
                Ok(Self::Conden(Box::new(ConditionalEntropy::new(w, bins, clip))))
            }
            BarIndicatorId::HalfLifeMr => {
                let w = config.periods.first().copied().unwrap_or(100);
                Ok(Self::HalfLifeMr(Box::new(HalfLifeMr::new(w))))
            }
            BarIndicatorId::ResidStat => {
                let w = config.periods.first().copied().unwrap_or(100);
                Ok(Self::ResidStat(Box::new(ResidualStationarity::new(w))))
            }
            BarIndicatorId::Coint => {
                let w = config.periods.first().copied().unwrap_or(100);
                Ok(Self::Coint(Box::new(CointegrationProxy::new(w))))
            }
            BarIndicatorId::EgCoint => {
                let w = config.periods.first().copied().unwrap_or(100);
                Ok(Self::EngleGrangerProxy(Box::new(EngleGrangerProxy::new(w))))
            }
            BarIndicatorId::EgAdf => {
                let w = config.periods.first().copied().unwrap_or(100);
                let ma = config.periods.get(1).copied().unwrap_or(20);
                Ok(Self::EngleGrangerAdfProxy(Box::new(EngleGrangerAdfProxy::new(w, ma))))
            }
            BarIndicatorId::EgTrend => {
                let w = config.periods.first().copied().unwrap_or(100);
                Ok(Self::EngleGrangerTrendProxy(Box::new(EngleGrangerTrendProxy::new(w))))
            }

            // Momentum/trend extras
            BarIndicatorId::MarketCipher => Ok(Self::MultiTimeframeMomentumDivergence(Box::new(MultiTimeframeMomentumDivergence::new()))),
            BarIndicatorId::AdxSlope => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::AdxSlope(Box::new(AdxSlope::new(p))))
            }
            BarIndicatorId::GannHilo => {
                let p = config.periods.first().copied().unwrap_or(10);
                Ok(Self::GannHilo(Box::new(GannHiLoActivator::new(p))))
            }

            // Filters/channels
            BarIndicatorId::Hampel => {
                let w = config.periods.first().copied().unwrap_or(25);
                let k = config.additional_params.get("k").copied().unwrap_or(3.0);
                Ok(Self::Hampel(Box::new(HampelFilter::new(w, k))))
            }
            BarIndicatorId::Theilsenchan => {
                let w = config.periods.first().copied().unwrap_or(50);
                let k = config.additional_params.get("k").copied().unwrap_or(2.0);
                Ok(Self::TheilSenChannels(Box::new(TheilSenChannels::new(w, k))))
            }
            BarIndicatorId::Projbands => {
                let w = config.periods.first().copied().unwrap_or(50);
                let k = config.additional_params.get("k").copied().unwrap_or(2.0);
                Ok(Self::ProjectionBands(Box::new(ProjectionBands::with_source(w, k, config.source))))
            }

            // Patterns/ML-style
            BarIndicatorId::Patternrec => Ok(Self::AdvancedPatternRecognition(Box::default())),
            BarIndicatorId::Avr => Ok(Self::AdaptiveVolatilityRegime(Box::new(AdaptiveVolatilityRegime::new()))),
            BarIndicatorId::NeuralMom => Ok(Self::NeuralMomentumNetwork(Box::new(NeuralMomentumNetwork::new()))),
            BarIndicatorId::Candleanatomy => {
                let thr = config.additional_params.get("long_wick_ratio_threshold").copied().unwrap_or(0.6);
                Ok(Self::CandleAnatomy(Box::new(CandleAnatomy::new(thr))))
            }

            // Price action / structure helpers
            BarIndicatorId::Swingstr => {
                let left = config.periods.first().copied().unwrap_or(3);
                let right = config.periods.get(1).copied().unwrap_or(3);
                Ok(Self::SwingStrengthScore(Box::new(SwingStrengthScore::new(left, right))))
            }
            BarIndicatorId::Liqgap => {
                let win = config.periods.first().copied().unwrap_or(50);
                let thr = config.additional_params.get("threshold").copied().unwrap_or(0.003);
                Ok(Self::Liqgap(Box::new(LiquidityGapDensity::new(win, thr))))
            }
            BarIndicatorId::Wickspike => {
                let win = config.periods.first().copied().unwrap_or(50);
                Ok(Self::WickSpike(Box::new(WickSpike::new(win))))
            }
            BarIndicatorId::SwingAge => {
                let look = config.periods.first().copied().unwrap_or(20);
                Ok(Self::SwingAge(Box::new(SwingAge::new(look))))
            }

            // Regime/time effects and structures
            BarIndicatorId::Rcb => {
                let win = config.periods.first().copied().unwrap_or(20);
                Ok(Self::RangeCompressionBurst(Box::new(RangeCompressionBurst::new(win))))
            }
            BarIndicatorId::Stft => {
                let window = config.periods.first().copied().unwrap_or(64);
                let split = config.periods.get(1).copied().unwrap_or(4);
                Ok(Self::StftBandEnergyRatio(Box::new(StftBandEnergyRatio::new(window, split))))
            }
            // Calendar/time effects excluded from universal constructor
            BarIndicatorId::Fvg => Ok(Self::FvgDetector(Box::default())),
            BarIndicatorId::Bos => {
                let lb = config.periods.first().copied().unwrap_or(20);
                Ok(Self::BosChochDetector(Box::new(BosChochDetector::new(lb))))
            }

            // ZigZag
            // ZigZag family excluded from universal constructor

            // Clusters / Orderflow / Book (require Bar/book feeds upstream; neutral in OHLC path)
            // Cluster/book/tick indicators excluded from universal constructor

            // Regression models (operate on close stream in update_bar)
            BarIndicatorId::Arima => {
                let p = config.periods.first().copied().unwrap_or(1);
                let d = config.periods.get(1).copied().unwrap_or(1);
                let q = config.periods.get(2).copied().unwrap_or(1);
                Ok(Self::Arima(Box::new(Arima::new(p, d, q))))
            }
            BarIndicatorId::Arimax => {
                let p = config.periods.first().copied().unwrap_or(1);
                let d = config.periods.get(1).copied().unwrap_or(1);
                let q = config.periods.get(2).copied().unwrap_or(1);
                let exog = config.periods.get(3).copied().unwrap_or(0);
                Ok(Self::ArimaX(Box::new(ArimaX::new(p, d, q, exog))))
            }
            BarIndicatorId::Garch => {
                let p = config.periods.first().copied().unwrap_or(1);
                let q = config.periods.get(1).copied().unwrap_or(1);
                Ok(Self::Garch(Box::new(Garch::new(p, q))))
            }
            BarIndicatorId::Egarch => {
                let p = config.periods.first().copied().unwrap_or(1);
                let q = config.periods.get(1).copied().unwrap_or(1);
                Ok(Self::EGarch(Box::new(EGarch::new(p, q))))
            }
            BarIndicatorId::Var => {
                let p = config.periods.first().copied().unwrap_or(1);
                // VAR uses [close, volume] so n_vars=2
                Ok(Self::Var(Box::new(Var::new(p, 2))))
            }
            BarIndicatorId::PolyReg => {
                let degree = config.periods.first().copied().unwrap_or(2);
                Ok(Self::PolyReg(Box::new(PolynomialRegression::with_source(degree, config.source))))
            }

            // Momentum extras & helpers
            BarIndicatorId::Aroon => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::Aroon(Box::new(Aroon::new(p))))
            }
            BarIndicatorId::AroonOsc => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::AroonOscillator(Box::new(AroonOscillator::new(p))))
            }
            BarIndicatorId::AroonUp => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::AroonUp(Box::new(AroonUp::new(p))))
            }
            BarIndicatorId::AroonDown => {
                let p = config.periods.first().copied().unwrap_or(14);
                Ok(Self::AroonDown(Box::new(AroonDown::new(p))))
            }
            BarIndicatorId::Highest => {
                let p = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Highest(Box::new(Highest::new(p))))
            }
            BarIndicatorId::Lowest => {
                let p = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Lowest(Box::new(Lowest::new(p))))
            }
            BarIndicatorId::Pressure => {
                let p = config.periods.first().copied().unwrap_or(14);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::SMA);
                Ok(Self::Pressure(Box::new(Pressure::new(p, ma_type))))
            }
            BarIndicatorId::MaCross => {
                let fast = config.periods.first().copied().unwrap_or(9);
                let slow = config.periods.get(1).copied().unwrap_or(21);
                let fast_ma = config.ma_types.get("fast_ma").copied().unwrap_or(MovingAverageType::EMA);
                let slow_ma = config.ma_types.get("slow_ma").copied().unwrap_or(MovingAverageType::EMA);
                Ok(Self::MaCross(Box::new(MaCross::new(fast, slow, fast_ma, slow_ma))))
            }
            BarIndicatorId::CandlePatterns => {
                let min_pct = config.additional_params.get("min_candle_size_pct").copied().unwrap_or(0.5);
                Ok(Self::CandlePatterns(Box::new(CandlePatterns::new(min_pct))))
            }

            // Accumulation
            BarIndicatorId::Tmf => {
                let p = config.periods.first().copied().unwrap_or(21);
                Ok(Self::TwiggsMoneyFlow(Box::new(Tmf::new(p))))
            }

            // Positioning
            // AvwapDistance excluded from universal constructor

            // AutoFibo (time/index aware; neutral in OHLC path)
            // AutoFibo excluded from universal constructor

            // Regime composites
            BarIndicatorId::Rc2 => {
                let win = config.periods.first().copied().unwrap_or(128);
                let roll_target = config.additional_params.get("roll_target").copied().unwrap_or(0.85);
                let atr_p = config.periods.get(1).copied().unwrap_or(14);
                let atr_ma = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::RMA);
                let atr_win = config.periods.get(2).copied().unwrap_or(100);
                let bins = config.additional_params.get("shannon_bins").copied().unwrap_or(20.0) as usize;
                Ok(Self::RegimeCompositeV2(Box::new(RegimeCompositeV2::new(win, roll_target, atr_p, atr_ma, atr_win, bins))))
            }
            BarIndicatorId::Rc3 => {
                let win = config.periods.first().copied().unwrap_or(128);
                let low_cut = config.additional_params.get("low_cut_fraction").copied().unwrap_or(0.1);
                let vov_win = config.periods.get(1).copied().unwrap_or(100);
                let bins = config.additional_params.get("shannon_bins").copied().unwrap_or(20.0) as usize;
                Ok(Self::RegimeCompositeV3(Box::new(RegimeCompositeV3::new(win, low_cut, vov_win, bins))))
            }
            BarIndicatorId::Rc4 => {
                let hurst_w = config.periods.first().copied().unwrap_or(100);
                let dfa1 = config.periods.get(1).copied().unwrap_or(4);
                let dfa2 = config.periods.get(2).copied().unwrap_or(8);
                let dfa3 = config.periods.get(3).copied().unwrap_or(16);
                let dfa4 = config.periods.get(4).copied().unwrap_or(32);
                let fft_w = config.periods.get(5).copied().unwrap_or(128);
                let ser_low = config.additional_params.get("ser_low_cut").copied().unwrap_or(0.1);
                let vov_win = config.periods.get(6).copied().unwrap_or(100);
                let perc_win = config.periods.get(7).copied().unwrap_or(100);
                let atr_p = config.periods.get(8).copied().unwrap_or(14);
                Ok(Self::RegimeCompositeV4(Box::new(RegimeCompositeV4::new(hurst_w, [dfa1, dfa2, dfa3, dfa4], fft_w, ser_low, vov_win, perc_win, atr_p))))
            }
            BarIndicatorId::Rc => {
                let params = RegimeCompositeParams::default();
                Ok(Self::RegimeComposite(Box::new(RegimeComposite::new(params))))
            }

            // Anchored VWAPs & levels
            BarIndicatorId::Avwap => {
                let params = AnchoredVwapParams::default();
                Ok(Self::AnchoredVwap(Box::new(AnchoredVwap::new(params))))
            }
            BarIndicatorId::Pivavwap => {
                let look = config.periods.first().copied().unwrap_or(20);
                Ok(Self::PivotAnchoredVwap(Box::new(PivotAnchoredVwap::new(look))))
            }

            // Vol/volatility helpers
            BarIndicatorId::Atrp => {
                let atr_p = config.periods.first().copied().unwrap_or(14);
                let win = config.periods.get(1).copied().unwrap_or(100);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::RMA);
                Ok(Self::AtrPercentile(Box::new(AtrPercentileInd::new(atr_p, ma_type, win))))
            }
            BarIndicatorId::Atrbw => {
                let atr_p = config.periods.first().copied().unwrap_or(14);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::RMA);
                Ok(Self::AtrBandwidth(Box::new(AtrBandwidth::new(atr_p, ma_type))))
            }
            BarIndicatorId::Atrz => {
                let atr_p = config.periods.first().copied().unwrap_or(14);
                let win = config.periods.get(1).copied().unwrap_or(100);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::RMA);
                Ok(Self::AtrZscore(Box::new(AtrZscore::new(atr_p, ma_type, win))))
            }
            BarIndicatorId::C2cvp => {
                let vol_win = config.periods.first().copied().unwrap_or(50);
                let perc_win = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::CloseVolPercentile(Box::new(CloseVolPercentile::new(vol_win, perc_win))))
            }
            BarIndicatorId::Pvt => {
                let win = config.periods.first().copied().unwrap_or(50);
                Ok(Self::RelativeVolume(Box::new(RelativeVolume::new(win))))
            }
            BarIndicatorId::Vz => {
                let win = config.periods.first().copied().unwrap_or(100);
                Ok(Self::VolumeZscore(Box::new(VolumeZscore::new(win))))
            }

            // Trend/levels helpers
            BarIndicatorId::Sdl => {
                Ok(Self::SlopeDirectionLine(Box::new(SlopeDirectionLine::new(period))))
            }
            BarIndicatorId::EmaSlope => {
                let ema_p = config.periods.first().copied().unwrap_or(21);
                let look = config.periods.get(1).copied().unwrap_or(20);
                Ok(Self::EmaSlope(Box::new(EmaSlope::new(ema_p, look))))
            }
            BarIndicatorId::Rmid => {
                Ok(Self::Rmid(Box::new(RollingMidline::new(period))))
            }
            BarIndicatorId::Pgry => {
                let win = config.periods.first().copied().unwrap_or(100);
                Ok(Self::RangePercentile(Box::new(RangePercentile::new(win))))
            }
            BarIndicatorId::RangeAtr => {
                let atr_p = config.periods.first().copied().unwrap_or(14);
                let ma_type = config.ma_types.get("ma_type").copied().unwrap_or(MovingAverageType::RMA);
                Ok(Self::RangeToAtr(Box::new(RangeToAtr::new(atr_p, ma_type))))
            }
            BarIndicatorId::Rquart => {
                let win = config.periods.first().copied().unwrap_or(100);
                Ok(Self::RollingQuartiles(Box::new(RollingQuartiles::new(win))))
            }

            // Signal Processing / Spectral
            BarIndicatorId::Fft => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 256);
                let sr = config.additional_params.get("sampling_rate").copied().unwrap_or(1.0);
                Ok(Self::FastFourierTransform(Box::new(FastFourierTransform::new(win, sr))))
            }
            BarIndicatorId::Wave => {
                let max_scales = config.periods.first().copied().unwrap_or(16).clamp(1, 32);
                let wtype_code = config.additional_params.get("wavelet_type").copied().unwrap_or(0.0) as i32;
                let wtype = match wtype_code { 1 => WaveletType::Daubechies4, 2 => WaveletType::Daubechies6, 3 => WaveletType::Morlet, 4 => WaveletType::Mexican, 5 => WaveletType::Biorthogonal, _ => WaveletType::Haar };
                Ok(Self::Wave(Box::new(WaveletTransform::new(wtype, max_scales))))
            }
            BarIndicatorId::Hilb => {
                let win = config.periods.first().copied().unwrap_or(64).clamp(16, 256);
                let sr = config.additional_params.get("sampling_rate").copied().unwrap_or(1.0);
                Ok(Self::Hilb(Box::new(HilbertTransform::new(win, sr))))
            }
            BarIndicatorId::Sent => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 256);
                Ok(Self::Sent(Box::new(SpectralEntropy::new(win))))
            }
            BarIndicatorId::Shmpr => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 256);
                let target = config.additional_params.get("target_fraction").copied().unwrap_or(0.85);
                Ok(Self::SpectralRolloff(Box::new(SpectralRolloff::new(win, target))))
            }
            BarIndicatorId::Screst => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 256);
                Ok(Self::Screst(Box::new(SpectralCrest::new(win))))
            }
            BarIndicatorId::Sentent => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 256);
                let alpha = config.additional_params.get("smoothing_alpha").copied().unwrap_or(0.2);
                Ok(Self::SpectralEntropyRate(Box::new(SpectralEntropyRate::new(win, alpha))))
            }
            BarIndicatorId::Ser => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(32, 256);
                let low_cut = config.additional_params.get("low_cut_fraction").copied().unwrap_or(0.1);
                Ok(Self::Ser(Box::new(SpectralEnergyRatio::new(win, low_cut))))
            }
            BarIndicatorId::Srollrp => {
                let win = config.periods.first().copied().unwrap_or(256).clamp(32, 512);
                Ok(Self::SpectralSlope(Box::new(SpectralSlope::new(win))))
            }
            BarIndicatorId::Sslopez => {
                let fft_win = config.periods.first().copied().unwrap_or(256).clamp(32, 512);
                let z_win = config.periods.get(1).copied().unwrap_or(256).clamp(20, 2048);
                Ok(Self::SpectralSlopeZscore(Box::new(SpectralSlopeZscore::new(fft_win, z_win))))
            }
            BarIndicatorId::Sflux => {
                let win = config.periods.first().copied().unwrap_or(256).clamp(32, 1024);
                let low = config.additional_params.get("low_cut_fraction").copied().unwrap_or(0.1);
                let high = config.additional_params.get("high_cut_fraction").copied().unwrap_or(0.25);
                Ok(Self::SpectralLowMidPowerRatio(Box::new(SpectralLowMidPowerRatio::new(win, low, high))))
            }
            BarIndicatorId::Sflatp => {
                let win = config.periods.first().copied().unwrap_or(256).clamp(32, 1024);
                let low = config.additional_params.get("low_cut_fraction").copied().unwrap_or(0.1);
                let high = config.additional_params.get("high_cut_fraction").copied().unwrap_or(0.25);
                Ok(Self::SpectralHighMidPowerRatio(Box::new(SpectralHighMidPowerRatio::new(win, low, high))))
            }
            BarIndicatorId::Sbprhl => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 512);
                Ok(Self::SpectralCentroidFeature(Box::new(SpectralCentroidFeature::new(win))))
            }
            BarIndicatorId::Sbwf => {
                let win = config.periods.first().copied().unwrap_or(128).clamp(16, 512);
                Ok(Self::SpectralBandwidthFeature(Box::new(SpectralBandwidthFeature::new(win))))
            }
            BarIndicatorId::Sbp => {
                let win = config.periods.first().copied().unwrap_or(256).clamp(32, 512);
                let low = config.additional_params.get("low_cut_fraction").copied().unwrap_or(0.1);
                let high = config.additional_params.get("high_cut_fraction").copied().unwrap_or(0.3);
                Ok(Self::SpectralBandpower(Box::new(SpectralBandpower::new(win, low, high))))
            }
            BarIndicatorId::Butter => {
                let order = config.periods.first().copied().unwrap_or(2);
                let cutoff_hz = config.additional_params.get("cutoff_hz").copied().unwrap_or(0.1);
                let sr = config.additional_params.get("sampling_rate").copied().unwrap_or(1.0);
                let t = match config.additional_params.get("filter_type").copied().unwrap_or(0.0) as i32 { 1 => ButterType::HighPass, 2 => ButterType::BandPass, 3 => ButterType::BandStop, _ => ButterType::LowPass };
                let instance = match t {
                    ButterType::LowPass | ButterType::HighPass => ButterworthFilter::new(t, order, cutoff_hz, sr),
                    ButterType::BandPass | ButterType::BandStop => {
                        let low = config.additional_params.get("low_cut_hz").copied().unwrap_or(0.05);
                        let high = config.additional_params.get("high_cut_hz").copied().unwrap_or(0.2);
                        ButterworthFilter::new_band_filter(t, order, low, high, sr)
                    }
                };
                Ok(Self::ButterworthFilter(Box::new(instance)))
            }
            BarIndicatorId::Cheby => {
                let order = config.periods.first().copied().unwrap_or(2);
                let cutoff = config.additional_params.get("cutoff_fraction").copied().unwrap_or(0.1);
                let ripple = config.additional_params.get("ripple_db").copied().unwrap_or(1.0);
                let ctype = match config.additional_params.get("cheby_type").copied().unwrap_or(1.0) as i32 { 2 => ChebyshevType::Type2, _ => ChebyshevType::Type1 };
                let ftype = match config.additional_params.get("filter_type").copied().unwrap_or(0.0) as i32 { 1 => ChebyType::HighPass, 2 => ChebyType::BandPass, 3 => ChebyType::BandStop, _ => ChebyType::LowPass };
                let inst = ChebyshevFilter::new(ctype, ftype, order, cutoff, ripple);
                Ok(Self::ChebyshevFilter(Box::new(inst)))
            }
            BarIndicatorId::Sg => {
                let win = config.periods.first().copied().unwrap_or(9);
                let poly = config.periods.get(1).copied().unwrap_or(3);
                let d = match config.additional_params.get("derivative").copied().unwrap_or(0.0) as i32 { 1 => SgDerivativeOrder::FirstDerivative, 2 => SgDerivativeOrder::SecondDerivative, 3 => SgDerivativeOrder::ThirdDerivative, _ => SgDerivativeOrder::Smoothing };
                Ok(Self::Sg(Box::new(SavitzkyGolayFilter::new(win, poly, d))))
            }


            // ========== PHASE 2 ADDITIONS ==========
            // ACCUMULATION (2)
            BarIndicatorId::Iip => {
                Ok(Self::Iip(Box::default()))
            }
            BarIndicatorId::Iir => {
                let window = config.periods.first().copied().unwrap_or(14);
                Ok(Self::Iir(Box::new(IntradayIntensityRatio::new(window))))
            }

            // KALMAN (8)
            BarIndicatorId::Abgfilter => {
                let period = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Abgfilter(Box::new(AlphaBetaGammaFilter::new(period))))
            }
            BarIndicatorId::Kcomp => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let q = config.additional_params.get("q").copied().unwrap_or(0.01);
                let r = config.additional_params.get("r").copied().unwrap_or(0.1);
                let k_window = config.periods.first().copied().unwrap_or(20);
                let decay = config.additional_params.get("decay").copied().unwrap_or(0.95);
                let atr_period = config.periods.get(1).copied().unwrap_or(14);
                let atr_ma = config.ma_types.get("atr_ma").copied().unwrap_or(MovingAverageType::SMA);
                let atr_pct_window = config.periods.get(2).copied().unwrap_or(100);
                let vov_vol_window = config.periods.get(3).copied().unwrap_or(20);
                let vov_pct_window = config.periods.get(4).copied().unwrap_or(100);
                let w_regime = config.additional_params.get("w_regime").copied().unwrap_or(0.5);
                let w_atr = config.additional_params.get("w_atr").copied().unwrap_or(0.3);
                let w_vov = config.additional_params.get("w_vov").copied().unwrap_or(0.2);
                Ok(Self::Kcomp(Box::new(KalmanRegimeComposite::new(dt, q, r, k_window, decay, atr_period, atr_ma, atr_pct_window, vov_vol_window, vov_pct_window, w_regime, w_atr, w_vov))))
            }
            BarIndicatorId::Kregime => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let process_noise = config.additional_params.get("process_noise").copied().unwrap_or(0.01);
                let measurement_noise = config.additional_params.get("measurement_noise").copied().unwrap_or(0.1);
                let z_window = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Kregime(Box::new(KalmanTrendRegime::new(dt, process_noise, measurement_noise, z_window))))
            }
            BarIndicatorId::Kscr => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let process_noise = config.additional_params.get("process_noise").copied().unwrap_or(0.01);
                let measurement_noise = config.additional_params.get("measurement_noise").copied().unwrap_or(0.1);
                let window = config.periods.first().copied().unwrap_or(20);
                let decay = config.additional_params.get("decay").copied().unwrap_or(0.95);
                Ok(Self::Kscr(Box::new(KalmanRegimeScore::new(dt, process_noise, measurement_noise, window, decay))))
            }
            BarIndicatorId::Kslope => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let process_noise = config.additional_params.get("process_noise").copied().unwrap_or(0.01);
                let measurement_noise = config.additional_params.get("measurement_noise").copied().unwrap_or(0.1);
                let window = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Kslope(Box::new(KalmanTrendSlope::new(dt, process_noise, measurement_noise, window))))
            }
            BarIndicatorId::Kslopez => {
                let dt = config.additional_params.get("dt").copied().unwrap_or(1.0);
                let process_noise = config.additional_params.get("process_noise").copied().unwrap_or(0.01);
                let measurement_noise = config.additional_params.get("measurement_noise").copied().unwrap_or(0.1);
                let window = config.periods.first().copied().unwrap_or(20);
                Ok(Self::Kslopez(Box::new(KalmanSlopeZscore::new(dt, process_noise, measurement_noise, window))))
            }

            // SIGNALPROCESSING (6 new)
            BarIndicatorId::Screstp => {
                let fft_window = config.periods.first().copied().unwrap_or(32);
                let pct_window = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::Screstp(Box::new(SpectralCrestPercentile::new(fft_window, pct_window))))
            }
            BarIndicatorId::Slmpr => {
                let window = config.periods.first().copied().unwrap_or(32);
                let low_cut_fraction = config.additional_params.get("low_cut_fraction").copied().unwrap_or(0.1);
                let high_cut_fraction = config.additional_params.get("high_cut_fraction").copied().unwrap_or(0.5);
                Ok(Self::Slmpr(Box::new(SpectralLowMidPowerRatio::new(window, low_cut_fraction, high_cut_fraction))))
            }
            BarIndicatorId::Sroll95 => {
                let window = config.periods.first().copied().unwrap_or(32);
                Ok(Self::Sroll95(Box::new(SpectralRolloff95::new(window))))
            }
            BarIndicatorId::Srollp => {
                let fft_window = config.periods.first().copied().unwrap_or(32);
                let pct_window = config.periods.get(1).copied().unwrap_or(100);
                let rolloff_percent = config.additional_params.get("rolloff_percent").copied().unwrap_or(0.85);
                Ok(Self::Srollp(Box::new(SpectralRolloffPercentile::new(fft_window, pct_window, rolloff_percent))))
            }
            BarIndicatorId::Sslopep => {
                let fft_window = config.periods.first().copied().unwrap_or(32);
                let pct_window = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::Sslopep(Box::new(SpectralSlopePercentile::new(fft_window, pct_window))))
            }
            BarIndicatorId::Ssloperp => {
                let fft_window = config.periods.first().copied().unwrap_or(32);
                let pct_window = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::Ssloperp(Box::new(SpectralSlopeRobustPercentile::new(fft_window, pct_window))))
            }


            // STATISTICS (10)
            BarIndicatorId::AdfKpss => {
                let window = config.periods.first().copied().unwrap_or(50);
                let ma_period = config.periods.get(1).copied().unwrap_or(20);
                let use_trend = config.additional_params.get("use_trend").copied().unwrap_or(0.0) > 0.5;
                Ok(Self::AdfKpss(Box::new(AdfKpssComposite::new(window, ma_period, use_trend))))
            }
            BarIndicatorId::ArchLmPval => {
                let window = config.periods.first().copied().unwrap_or(50);
                let lags = config.periods.get(1).copied().unwrap_or(5);
                Ok(Self::ArchLmPval(Box::new(ArchLmPvalueProxy::new(window, lags))))
            }
            BarIndicatorId::BpCusum => {
                let threshold = config.additional_params.get("threshold").copied().unwrap_or(1.5);
                let kappa = config.additional_params.get("kappa").copied().unwrap_or(0.5);
                let seg_window = config.periods.first().copied().unwrap_or(20);
                Ok(Self::BpCusum(Box::new(BaiPerronCusum::new(threshold, kappa, seg_window))))
            }
            BarIndicatorId::KpssTrend => {
                let window = config.periods.first().copied().unwrap_or(50);
                Ok(Self::KpssTrend(Box::new(KpssTrendProxy::new(window))))
            }
            BarIndicatorId::KpssZ => {
                let window_stat = config.periods.first().copied().unwrap_or(50);
                let window_z = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::KpssZ(Box::new(KpssZProxy::new(window_stat, window_z))))
            }
            BarIndicatorId::Pp => {
                let window = config.periods.first().copied().unwrap_or(50);
                Ok(Self::Pp(Box::new(PhillipsPerronProxy::new(window))))
            }
            BarIndicatorId::PvCoherence => {
                let window = config.periods.first().copied().unwrap_or(50);
                Ok(Self::PvCoherence(Box::new(PriceVolumeCoherenceProxy::new(window))))
            }
            BarIndicatorId::RSquared => {
                let window = config.periods.first().copied().unwrap_or(20);
                Ok(Self::RSquared(Box::new(RSquared::new(window))))
            }
            BarIndicatorId::VrZAgg => {
                let configs_vec: Vec<(usize, usize)> = vec![(2, 10), (5, 10), (10, 20)]; // Default configs
                let z_window = config.periods.first().copied().unwrap_or(100);
                Ok(Self::VrZAgg(Box::new(VarianceRatioZAggregate::new(&configs_vec, z_window))))
            }
            BarIndicatorId::Za => {
                let window = config.periods.first().copied().unwrap_or(50);
                Ok(Self::Za(Box::new(ZivotAndrewsProxy::new(window))))
            }

            // VOLATILITY (1)
            BarIndicatorId::Dvr => {
                Ok(Self::Dvr(Box::new(DynamicVolatilityRegime::new())))
            }

            // CHANNELS - Additional metrics
            BarIndicatorId::Bbmetrics => {
                let std_dev = config.additional_params.get("std_dev").copied().unwrap_or(2.0);
                Ok(Self::BollingerMetrics(Box::new(BollingerMetrics::new(period, std_dev))))
            }
            BarIndicatorId::Dcmetrics => {
                Ok(Self::Dcmetrics(Box::new(DonchianMetrics::new(period))))
            }
            BarIndicatorId::Kcmetrics => {
                let atr_mult = config.additional_params.get("atr_mult").copied().unwrap_or(2.0);
                Ok(Self::Kcmetrics(Box::new(KeltnerMetrics::new(period, atr_mult))))
            }
            BarIndicatorId::Percentb => {
                let std_mult = config.additional_params.get("std_mult").copied().unwrap_or(2.0);
                Ok(Self::PercentB(Box::new(PercentB::new(period, std_mult))))
            }

            // VOLATILITY - Additional indicators
            BarIndicatorId::Atrc => {
                let multiplier = config.additional_params.get("multiplier").copied().unwrap_or(2.0);
                let _mode = AtrChannelMode::Close;
                let _ma_type = MovingAverageType::SMA;
                let _atr_ma_type = MovingAverageType::RMA;
                Ok(Self::AtrChannels(Box::new(AtrBands::new(period, MovingAverageType::SMA, period, MovingAverageType::RMA, multiplier))))
            }
            BarIndicatorId::Rbvj => {
                let annualize_factor = config.additional_params.get("annualize_factor").copied().unwrap_or(252.0);
                Ok(Self::RbvJumpTest(Box::new(RbvJumpTest::new(period, annualize_factor))))
            }
            BarIndicatorId::Vovp => {
                let vov_window = config.periods.first().copied().unwrap_or(20);
                let percentile_window = config.periods.get(1).copied().unwrap_or(100);
                Ok(Self::VolOfVolPercentile(Box::new(VolOfVolPercentile::new(None, vov_window, percentile_window))))
            }
            BarIndicatorId::Vovpt => {
                let vov_window = config.periods.first().copied().unwrap_or(20);
                let pct_window = config.periods.get(1).copied().unwrap_or(100);
                let alpha = config.additional_params.get("alpha").copied().unwrap_or(0.05);
                Ok(Self::VolOfVolPercentileTrend(Box::new(VolOfVolPercentileTrend::new(None, vov_window, pct_window, alpha))))
            }

            // VOLUME indicators
            BarIndicatorId::Pvo => {
                let fast = config.periods.first().copied().unwrap_or(12);
                let slow = config.periods.get(1).copied().unwrap_or(26);
                let signal = config.periods.get(2).copied().unwrap_or(9);

                // Check if we have custom MA types (PVO doesn't use source, it's volume-based)
                let has_ma_types = config.ma_types.contains_key("fast_ma_type")
                    || config.ma_types.contains_key("slow_ma_type")
                    || config.ma_types.contains_key("signal_ma_type");

                if has_ma_types {
                    let fast_ma_type = config.ma_types.get("fast_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let slow_ma_type = config.ma_types.get("slow_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let signal_ma_type = config.ma_types.get("signal_ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);

                    Ok(Self::Pvo(Box::new(Pvo::with_ma_types(
                        fast, slow, signal,
                        fast_ma_type, slow_ma_type, signal_ma_type
                    ))))
                } else {
                    Ok(Self::Pvo(Box::new(Pvo::new(fast, slow, signal))))
                }
            }
            BarIndicatorId::Pzo => {
                Ok(Self::Pzo(Box::new(Pzo::new(period))))
            }
            BarIndicatorId::Rvol => {
                Ok(Self::RelativeVolume(Box::new(RelativeVolume::new(period))))
            }
            BarIndicatorId::Vo => {
                let short = config.periods.first().copied().unwrap_or(5);
                let long = config.periods.get(1).copied().unwrap_or(10);
                Ok(Self::VolumeOscillator(Box::new(VolumeOscillator::new(short, long))))
            }

            // POSITION indicators
            BarIndicatorId::DistLevels => {
                let mid_window = config.periods.first().copied().unwrap_or(50);
                let pct_window = config.periods.get(1).copied().unwrap_or(50);
                let low_pct = config.additional_params.get("low_pct").copied().unwrap_or(0.1);
                let high_pct = config.additional_params.get("high_pct").copied().unwrap_or(0.9);
                Ok(Self::DistLevels(Box::new(DistanceToLevels::new(mid_window, pct_window, low_pct, high_pct))))
            }
            BarIndicatorId::AvwapDist => {
                Ok(Self::AvwapDistance(Box::default()))
            }

            // LEVELS indicators  
            BarIndicatorId::Avwaprev => {
                let z_window = config.periods.first().copied().unwrap_or(100);
                let params = vec![AnchoredVwapParams::default()];
                Ok(Self::AvwapMultiAnchorReversion(Box::new(AvwapMultiAnchorReversion::new(params, z_window))))
            }
            BarIndicatorId::Avwaptouch => {
                let prob_window = config.periods.first().copied().unwrap_or(100);
                let touch_threshold = config.additional_params.get("touch_threshold").copied().unwrap_or(0.001);
                let params = vec![AnchoredVwapParams::default()];
                Ok(Self::AvwapTouchProbability(Box::new(AvwapTouchProbability::new(params, prob_window, touch_threshold))))
            }

            // TREND indicators
            BarIndicatorId::TrEr => {
                Ok(Self::EfficiencyRatio(Box::new(EfficiencyRatio::new(period))))
            }

            // CANDLES (11 patterns) - Batch 1
            BarIndicatorId::Doji => {
                let body_ratio_max = config.additional_params.get("body_ratio_max").copied().unwrap_or(0.1);
                Ok(Self::Doji(Box::new(Doji::new(body_ratio_max))))
            }
            BarIndicatorId::Engulfing => {
                let min_size_ratio = config.additional_params.get("min_size_ratio").copied().unwrap_or(1.2);
                Ok(Self::Engulfing(Box::new(Engulfing::new(min_size_ratio))))
            }
            BarIndicatorId::Hammer => {
                let shadow_ratio_min = config.additional_params.get("shadow_ratio_min").copied().unwrap_or(2.0);
                let opposite_shadow_max = config.additional_params.get("opposite_shadow_max").copied().unwrap_or(0.1);
                Ok(Self::Hammer(Box::new(Hammer::new(shadow_ratio_min, opposite_shadow_max))))
            }
            BarIndicatorId::Harami => {
                let min_first_body_ratio = config.additional_params.get("min_first_body_ratio").copied().unwrap_or(0.6);
                Ok(Self::Harami(Box::new(Harami::new(min_first_body_ratio))))
            }
            BarIndicatorId::Heikinashi => {
                Ok(Self::Heikinashi(Box::new(HeikinAshi::new())))
            }
            BarIndicatorId::Marubozu => {
                let body_ratio_min = config.additional_params.get("body_ratio_min").copied().unwrap_or(0.9);
                Ok(Self::Marubozu(Box::new(Marubozu::new(body_ratio_min))))
            }
            BarIndicatorId::Morningstar => {
                let max_star_ratio = config.additional_params.get("max_star_ratio").copied().unwrap_or(0.3);
                Ok(Self::Morningstar(Box::new(MorningStar::new(max_star_ratio))))
            }
            BarIndicatorId::Piercingpattern => {
                let min_penetration = config.additional_params.get("min_penetration").copied().unwrap_or(0.5);
                Ok(Self::Piercingpattern(Box::new(PiercingPattern::new(min_penetration))))
            }
            BarIndicatorId::Shootingstar => {
                let shadow_ratio_min = config.additional_params.get("shadow_ratio_min").copied().unwrap_or(2.0);
                let opposite_shadow_max = config.additional_params.get("opposite_shadow_max").copied().unwrap_or(0.1);
                Ok(Self::Shootingstar(Box::new(ShootingStar::new(shadow_ratio_min, opposite_shadow_max))))
            }
            BarIndicatorId::Threewhitesoldiers => {
                let min_body_ratio = config.additional_params.get("min_body_ratio").copied().unwrap_or(0.6);
                Ok(Self::Threewhitesoldiers(Box::new(ThreeWhiteSoldiers::new(min_body_ratio))))
            }
            BarIndicatorId::Tweezer => {
                let max_diff_ratio = config.additional_params.get("max_diff_ratio").copied().unwrap_or(0.01);
                Ok(Self::Tweezer(Box::new(Tweezer::new(max_diff_ratio))))
            }
            BarIndicatorId::Darkcloudcover => {
                Ok(Self::Darkcloudcover(Box::default()))
            }
            BarIndicatorId::Eveningstar => {
                Ok(Self::Eveningstar(Box::default()))
            }
            BarIndicatorId::Threeblackcrows => {
                Ok(Self::Threeblackcrows(Box::default()))
            }

            // ACCUMULATION (1) - Batch 1

            // AVERAGE - Hma/Wma defined earlier (lines ~1571-1572)
            // OHLCV MA variants (15) - REMOVED
            // All replaced by MovingAverageWithField::new(MovingAverageType, period, OhlcvField)

            // VOLATILITY (7 aliases + 1 new) - Batch 2
            BarIndicatorId::Abb => Ok(Self::AdaptiveBollingerBands(Box::new(AdaptiveBollingerBands::new()))),
            BarIndicatorId::Cv => {
                let n = config.periods.first().copied().unwrap_or(10).clamp(2, 512);
                let k = config.periods.get(1).copied().unwrap_or(10).clamp(2, 512);
                Ok(Self::ChaikinVolatility(Box::new(ChaikinVolatility::new(n, k))))
            }
            BarIndicatorId::Rp => {
                let win = config.periods.first().copied().unwrap_or(20).max(2);
                Ok(Self::RangePercentile(Box::new(RangePercentile::new(win))))
            }
            BarIndicatorId::Vov => {
                let source = VoVSource::AbsReturn;
                let window = config.periods.first().copied().unwrap_or(20).max(2);
                Ok(Self::Vov(Box::new(VolOfVol::new(source, window))))
            }
            BarIndicatorId::Vprb => {
                let atr_period = config.periods.first().copied().unwrap_or(14).max(2);
                let rank_window = config.periods.get(1).copied().unwrap_or(100).max(10);
                Ok(Self::VolatilityPercentileRankBands(Box::new(VolatilityPercentileRankBands::new(atr_period, rank_window))))
            }
            BarIndicatorId::Wvf => {
                let lookback = config.periods.first().copied().unwrap_or(22).max(2);
                Ok(Self::Wvf(Box::new(Wvf::new(lookback))))
            }

            // CHANNELS (2 aliases) - Batch 2
            BarIndicatorId::Darvas => {
                let lookback = config.periods.first().copied().unwrap_or(5).max(2);
                Ok(Self::DarvasBox(Box::new(DarvasBox::new(lookback))))
            }

            // MOMENTUM (2) - Batch 2
            BarIndicatorId::Cog => {
                let period = config.periods.first().copied().unwrap_or(10).max(2);
                Ok(Self::Cog(Box::new(CenterOfGravity::new(period))))
            }
            BarIndicatorId::Pfe => {
                let window = config.periods.first().copied().unwrap_or(10).max(2);
                Ok(Self::Pfe(Box::new(Pfe::new(window))))
            }

            // ========================================
            // BATCH 3 - 5 INDICATORS (standard update_bar compatible)
            // ========================================
            // MOMENTUM (5)
            BarIndicatorId::EwmacRobust => {
                let fast_period = config.periods.first().copied().unwrap_or(32).max(2);
                let slow_period = config.periods.get(1).copied().unwrap_or(128).max(2);
                let robust_window = config.periods.get(2).copied().unwrap_or(252).max(2);
                Ok(Self::EwmacRobust(Box::new(EwmacRobust::new(fast_period, slow_period, robust_window))))
            }
            BarIndicatorId::Gapo => {
                let window = config.periods.first().copied().unwrap_or(10).max(2);
                Ok(Self::Gapo(Box::new(Gapo::new(window))))
            }
            BarIndicatorId::Gator => {
                let fast_period = config.periods.first().copied().unwrap_or(5).max(2);
                let slow_period = config.periods.get(1).copied().unwrap_or(8).max(2);

                // Check if we have custom MA type or source
                let has_ma_type = config.ma_types.contains_key("ma_type");

                if has_ma_type || config.source != OhlcvField::Close {
                    let ma_type = config.ma_types.get("ma_type")
                        .copied()
                        .unwrap_or(MovingAverageType::EMA);
                    let source = config.source;

                    Ok(Self::Gator(Box::new(GatorOscillator::with_full_config(
                        fast_period, slow_period, source, ma_type
                    ))))
                } else {
                    Ok(Self::Gator(Box::new(GatorOscillator::new(fast_period, slow_period))))
                }
            }
            BarIndicatorId::MacdHist => {
                let fast = config.periods.first().copied().unwrap_or(12).max(1);
                let slow = config.periods.get(1).copied().unwrap_or(26).max(1);
                let signal = config.periods.get(2).copied().unwrap_or(9).max(1);
                Ok(Self::MacdHist(Box::new(MacdHistogram::new(fast, slow, signal))))
            }
            BarIndicatorId::MacdSignal => {
                let fast = config.periods.first().copied().unwrap_or(12).max(2);
                let slow = config.periods.get(1).copied().unwrap_or(26).max(2);
                let signal = config.periods.get(2).copied().unwrap_or(9).max(2);
                Ok(Self::MacdSignal(Box::new(MacdSignal::new(fast, slow, signal))))
            }
            BarIndicatorId::PpoSignal => {
                let fast = config.periods.first().copied().unwrap_or(12).max(2);
                let slow = config.periods.get(1).copied().unwrap_or(26).max(2);
                let signal = config.periods.get(2).copied().unwrap_or(9).max(2);
                Ok(Self::PpoSignal(Box::new(PpoSignal::new(fast, slow, signal))))
            }

            // ========================================
            // BATCH 4 - 12 INDICATORS (standard update_bar compatible)
            // ========================================
            // SIGNAL_PROCESSING (7)
            BarIndicatorId::Scf => {
                let window = config.periods.first().copied().unwrap_or(64).max(16);
                Ok(Self::Scf(Box::new(SpectralCentroidFeature::new(window))))
            }
            BarIndicatorId::Sentr => {
                let window = config.periods.first().copied().unwrap_or(64).max(16);
                let smoothing_alpha = config.additional_params.get("smoothing_alpha").copied().unwrap_or(0.2);
                Ok(Self::Sentr(Box::new(SpectralEntropyRate::new(window, smoothing_alpha))))
            }
            BarIndicatorId::Sflat => {
                let window = config.periods.first().copied().unwrap_or(64).max(16);
                Ok(Self::Sflat(Box::new(SpectralFlatness::new(window))))
            }
            BarIndicatorId::Sroll => {
                let window = config.periods.first().copied().unwrap_or(64).max(16);
                let target_fraction = config.additional_params.get("target_fraction").copied().unwrap_or(0.85);
                Ok(Self::Sroll(Box::new(SpectralRolloff::new(window, target_fraction))))
            }
            BarIndicatorId::Sslope => {
                let window = config.periods.first().copied().unwrap_or(64).max(32);
                Ok(Self::Sslope(Box::new(SpectralSlope::new(window))))
            }

            // TREND (2)
            BarIndicatorId::KamaSlope => {
                let period = config.periods.first().copied().unwrap_or(10).max(2);
                Ok(Self::KamaSlope(Box::new(KamaSlope::new(period))))
            }
            BarIndicatorId::LrSlope => {
                let window = config.periods.first().copied().unwrap_or(14).max(2);
                Ok(Self::LrSlope(Box::new(LrSlope::new(window))))
            }

            // KALMAN (1)
            BarIndicatorId::Rts => Ok(Self::Rts(Box::default())),

            // VOLUME (2)
            BarIndicatorId::Vfi => {
                let window = config.periods.first().copied().unwrap_or(130).max(1);
                Ok(Self::Vfi(Box::new(Vfi::new(window))))
            }
            BarIndicatorId::Vzo => {
                let period = config.periods.first().copied().unwrap_or(14).max(2);
                Ok(Self::Vzo(Box::new(Vzo::new(period))))
            }

            // ========================================
            // BATCH 5 ADDITIONS (13 divergence indicators)
            // ========================================
            // DIVERGENCE (13)
            BarIndicatorId::RsiDiv => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::RsiDiv(Box::new(RsiDivergence::new(period, lookback))))
            }
            BarIndicatorId::CciDiv => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::CciDiv(Box::new(CciDivergence::new(period, lookback))))
            }
            BarIndicatorId::MacdDiv => {
                let fast = config.periods.first().copied().unwrap_or(12).max(1);
                let slow = config.periods.get(1).copied().unwrap_or(26).max(2);
                let lookback = config.periods.get(2).copied().unwrap_or(14).max(5);
                Ok(Self::MacdDiv(Box::new(MacdDivergence::new(fast, slow, lookback))))
            }
            BarIndicatorId::MacdHistDiv => {
                let fast = config.periods.first().copied().unwrap_or(12).max(1);
                let slow = config.periods.get(1).copied().unwrap_or(26).max(2);
                let signal = config.periods.get(2).copied().unwrap_or(9).max(1);
                let lookback = config.periods.get(3).copied().unwrap_or(14).max(5);
                Ok(Self::MacdHistDiv(Box::new(MacdHistogramDivergence::new(fast, slow, signal, lookback))))
            }
            BarIndicatorId::StochDiv => {
                let period_k = config.periods.first().copied().unwrap_or(14).max(1);
                let period_d = config.periods.get(1).copied().unwrap_or(3).max(1);
                let lookback = config.periods.get(2).copied().unwrap_or(14).max(5);
                Ok(Self::StochDiv(Box::new(StochasticDivergence::new(period_k, period_d, lookback))))
            }
            BarIndicatorId::WilliamsDiv => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::WilliamsDiv(Box::new(WilliamsDivergence::new(period, lookback))))
            }
            BarIndicatorId::ObvDiv => {
                let lookback = config.periods.first().copied().unwrap_or(14).max(5);
                Ok(Self::ObvDiv(Box::new(ObvDivergence::new(lookback))))
            }
            BarIndicatorId::VolDiv => {
                let lookback = config.periods.first().copied().unwrap_or(14).max(5);
                Ok(Self::VolumeDiv(Box::new(VolumeDivergence::new(lookback))))
            }
            BarIndicatorId::ClassicDiv => {
                let lookback = config.periods.first().copied().unwrap_or(14).max(5);
                Ok(Self::ClassicDiv(Box::new(ClassicDivergence::new(lookback))))
            }
            BarIndicatorId::HiddenDiv => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::HiddenDiv(Box::new(HiddenDivergence::new(period, lookback))))
            }
            BarIndicatorId::DivStrength => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::DivStrength(Box::new(DivergenceStrength::new(period, lookback))))
            }
            BarIndicatorId::MultiDiv => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::MultiDiv(Box::new(MultiDivergence::new(period, lookback))))
            }
            BarIndicatorId::MtfMomDiv => {
                let period = config.periods.first().copied().unwrap_or(14).max(1);
                let lookback = config.periods.get(1).copied().unwrap_or(14).max(5);
                Ok(Self::MomentumDiv(Box::new(MomentumDivergence::new(period, lookback))))
            }

            // ========================================
            // BATCH 6 - Timestamp-dependent indicators
            // ========================================
            BarIndicatorId::HourDay => {
                Ok(Self::HourDay(Box::default()))
            }
            BarIndicatorId::WeekMonth => {
                Ok(Self::WeekMonth(Box::default()))
            }
            BarIndicatorId::SoqEoq => {
                let window_days = config.periods.first().copied().unwrap_or(7) as u32;
                Ok(Self::SoqEoq(Box::new(StartEndOfQuarterFlags::new(window_days))))
            }

            // ========================================
            // Phase 2 - Additional timestamp indicators
            // ========================================
            BarIndicatorId::Tenc => Ok(Self::Tenc(Box::default())),
            BarIndicatorId::Weekday => Ok(Self::Weekday(Box::default())),
            BarIndicatorId::Session => Ok(Self::Session(Box::default())),
            BarIndicatorId::MonthQtr => Ok(Self::MonthQtr(Box::default())),
            BarIndicatorId::DomWoq => Ok(Self::DomWoq(Box::default())),
            BarIndicatorId::RelTrendPos => {
                let sma_period = config.periods.first().copied().unwrap_or(200);
                Ok(Self::RelTrendPos(Box::new(RelativeTrendPosition::new(sma_period))))
            }
            BarIndicatorId::MonthTurn => {
                let window = config.periods.first().copied().unwrap_or(5) as u32;
                Ok(Self::MonthTurn(Box::new(MonthTurnEffect::new(window))))
            }
            BarIndicatorId::QtrTurn => {
                let window = config.periods.first().copied().unwrap_or(10) as u32;
                Ok(Self::QtrTurn(Box::new(QuarterTurnEffect::new(window))))
            }

            // ========================================
            // PHASE 3 ADDITIONS
            // ========================================
            BarIndicatorId::SomEom => {
                let window = config.periods.first().copied().unwrap_or(5) as u32;
                Ok(Self::SomEom(Box::new(StartEndOfMonthFlags::new(window))))
            }
            BarIndicatorId::SowEow => {
                let window = config.periods.first().copied().unwrap_or(3) as u32;
                Ok(Self::SowEow(Box::new(StartEndOfWeekFlags::new(window))))
            }
            BarIndicatorId::HolidayProx => {
                let window = config.periods.first().copied().unwrap_or(2) as u32;
                Ok(Self::HolidayProx(Box::new(HolidayWeekendProximityEffect::new(window))))
            }
            BarIndicatorId::Vbd => {
                let mild = config.additional_params.get("mild_threshold").copied().unwrap_or(1.2);
                let strong = config.additional_params.get("strong_threshold").copied().unwrap_or(1.8);
                let extreme = config.additional_params.get("extreme_threshold").copied().unwrap_or(2.5);
                let squeeze = config.additional_params.get("squeeze_threshold").copied().unwrap_or(0.7);
                Ok(Self::Vbd(Box::new(VolatilityBreakoutDetector::with_parameters(mild, strong, extreme, squeeze))))
            }

            // ========================================
            // PHASE 4 ADDITIONS (9 indicators)
            // ========================================
            BarIndicatorId::Poc => {
                let price_precision = config.periods.first().copied().unwrap_or(2) as u32;
                let min_volume_threshold = config.additional_params.get("min_volume_threshold").copied()
                    .unwrap_or(100.0);
                let rolling_period = config.periods.get(1).copied().unwrap_or(20);

                Ok(Self::Poc(Box::new(
                    PocDetector::new(price_precision, min_volume_threshold, rolling_period)
                )))
            }

            BarIndicatorId::AutoFibo => {
                let zigzag_period = config.periods.first().copied().unwrap_or(20);
                let atr_period = config.periods.get(1).copied().unwrap_or(14);
                let atr_multiplier = config.additional_params.get("atr_multiplier").copied()
                    .unwrap_or(1.5);

                Ok(Self::AutoFibo(Box::new(
                    AutoFibo::new(zigzag_period, atr_period, atr_multiplier)
                )))
            }

            BarIndicatorId::Zigzag => {
                // Zigzag uses deviation parameter (0.01-1.0, converted to percent)
                let period = config.periods.first().copied().unwrap_or(20);
                let deviation = config.additional_params.get("deviation")
                    .copied()
                    .unwrap_or(0.05);
                // Convert deviation (0.05 = 5%) to threshold_percent
                let threshold_percent = Some(deviation * 100.0);

                Ok(Self::Zigzag(Box::new(
                    ZigZagClassic::new(period, threshold_percent, None)
                )))
            }

            BarIndicatorId::ZigzagClassic => {
                // ZigzagClassic uses threshold_percent or threshold_abs directly
                let period = config.periods.first().copied().unwrap_or(20);
                let threshold_percent = config.additional_params.get("threshold_percent")
                    .copied();
                let threshold_abs = config.additional_params.get("threshold_abs")
                    .copied();

                // If both None, use default threshold_percent
                let threshold_percent = if threshold_percent.is_none() && threshold_abs.is_none() {
                    Some(5.0)
                } else {
                    threshold_percent
                };

                Ok(Self::ZigzagClassic(Box::new(
                    ZigZagClassic::new(period, threshold_percent, threshold_abs)
                )))
            }

            BarIndicatorId::ZigzagAtr => {
                let zigzag_period = config.periods.first().copied().unwrap_or(20);
                let atr_period = config.periods.get(1).copied().unwrap_or(14);
                let atr_multiplier = config.additional_params.get("atr_multiplier").copied()
                    .unwrap_or(1.5);

                Ok(Self::ZigzagAtr(Box::new(
                    ZigZagAtr::new(zigzag_period, atr_period, atr_multiplier)
                )))
            }

            BarIndicatorId::ZigzagCandle => {
                let period = config.periods.first().copied().unwrap_or(20);
                let swing_bars = config.periods.get(1).copied().unwrap_or(3);

                Ok(Self::ZigzagCandle(Box::new(
                    ZigZagCandle::new(period, swing_bars)
                )))
            }

            BarIndicatorId::ZigzagLookahead => {
                let period = config.periods.first().copied().unwrap_or(20);
                let lookahead = config.periods.get(1).copied().unwrap_or(5);

                Ok(Self::ZigzagLookahead(Box::new(
                    ZigZagLookahead::new(period, lookahead)
                )))
            }

            BarIndicatorId::ZigzagTime => {
                let period = config.periods.first().copied().unwrap_or(20);
                let min_bars = config.periods.get(1).copied().unwrap_or(10);

                Ok(Self::ZigzagTime(Box::new(
                    ZigZagTime::new(period, min_bars)
                )))
            }

            BarIndicatorId::ZigzagDiv => {
                // FIXME: ZigzagDiv implementation not found - using ZigzagClassic as fallback
                let period = config.periods.first().copied().unwrap_or(20);
                Ok(Self::ZigzagClassic(Box::new(
                    ZigZagClassic::new(period, Some(5.0), None)
                )))
            }

            // === Phase 5: Final 3 FVG indicators ===

            BarIndicatorId::Fvgalt => {
                let alpha = config.additional_params.get("alpha")
                    .copied()
                    .unwrap_or(0.1);

                Ok(Self::Fvgalt(Box::new(
                    FvgIntensityAltScore::new(alpha)
                )))
            }

            BarIndicatorId::Fvgdur => {
                let lookback = config.periods.first().copied().unwrap_or(20);
                let agg_window = config.periods.get(1).copied().unwrap_or(50);

                Ok(Self::Fvgdur(Box::new(
                    FvgDurationIntensityScore::new(lookback, agg_window)
                )))
            }

            BarIndicatorId::Fvgrev => {
                let horizon = config.periods.first().copied().unwrap_or(10);

                Ok(Self::Fvgrev(Box::new(
                    FvgReversionProbability::new(horizon)
                )))
            }
            BarIndicatorId::DayWeekMonth => Ok(Self::DayWeekMonth(Box::default())),
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64, timestamp: Option<i64>) -> IndicatorValue {
        match self {
            Self::Abgfilter(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ac(x) => {
                let price = (high + low) / 2.0; x.update(price);
                x.value()
            }
            Self::AccumulationDistribution(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AccumulativeSwingIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdaptiveBollingerBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdaptiveChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdaptiveMovingAverage(x) => {
                x.update(close, high, low, volume);
                x.value()
            }
            Self::AdaptiveStochastic(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdaptiveVolatilityRegime(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Adx(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DiPlusMinus(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdxSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdvancedPatternRecognition(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Alligator(x) => {
                x.update_bar(open, high, low, close, volume)
            }
            Self::Alma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ama(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Amat(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AnchoredVwap(x) => {
                if let Some(ts) = timestamp { x.update_bar(open, high, low, close, volume, ts); }
                x.value()
            }
            Self::AndGate(x) => {
                x.update_bar(open, high, low, close, volume);
                IndicatorValue::Flag(x.value())
            }
            Self::Apen(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Apo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ArchLmProxy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ArchLmPval(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Aroon(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AroonDown(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AroonOscillator(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AroonUp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Atr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AtrBandwidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AtrPercentile(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AtrRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Atrts(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AtrZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AtrChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Autocorr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AvFrama(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AvVidya(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AwesomeOscillator(x) => {
                let price = (high + low) / 2.0; x.update(price);
                x.value()
            }
            Self::BbPeriod(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Bias(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::BipowerVariance(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::BollingerBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::BollingerMetrics(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::BookImb(x) => {
                let bid = low; let ask = high; x.update_book(bid, ask);
                x.value()
            }
            Self::BookSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Bop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::BpCusum(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::BosChochDetector(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ButterworthFilter(x) => {
                x.update(close);
                x.value()
            }
            Self::CandleAnatomy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::CandlePatterns(x) => {
                let pattern = x.update_bar(open, high, low, close, volume);
                // Convert CandlePattern to numeric: 0=None, 1=BullishEngulfing, -1=BearishEngulfing, 2=Hammer, -2=ShootingStar
                let v = match pattern {
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::None => 0.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::BullishEngulfing => 1.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::BearishEngulfing => -1.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::Hammer => 2.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::ShootingStar => -2.0,
                };
                IndicatorValue::Single(v)
            }
            Self::Cci(x) => {
                x.update_bar(high, low, close, volume);
                x.value()
            }
            Self::CciDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::CentralPivotRange(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ChaosOscillator(x) => {
                x.update_bar(open, high, low, close, volume)
            }
            Self::Cfo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ChaikinVolatility(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Chand(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ChandeKrollStop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ChebyshevFilter(x) => {
                x.update(close);
                x.value()
            }
            Self::Cho(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ChoppinessIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ClassicDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::CloseVolPercentile(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ClQueueImb(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Cmf(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Cmo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Cog(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Conden(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ConnorsRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::CoppockCurve(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::CusumFilter(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Cyber(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Darkcloudcover(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DarvasBox(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Dc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Dcmetrics(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Dcwidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Decyc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Dema(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DemandIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Demarker(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DeMarkPivots(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DetrendedPriceOscillator(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DetrendedSyntheticPrice(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Di(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Didi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DistLevels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DivStrength(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Dm(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Doji(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DonchianBreakout(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DonchianChannel(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DonchianPosition(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DonchianStop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DpoBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DpoPercent(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DssBressert(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EaseOfMovement(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EfficiencyRatio(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EfficiencyRatioFullHistory(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EfficiencyRatioRingWindow(x) => {
                x.update_raw(close);
                x.value()
            }
            Self::EhlersCc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EhlersFractalAdaptiveMa(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EhlersRocketRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EhlersZeroLagEma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Eit(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ElderImpulse(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ElderRay(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ema(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EmaSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Engulfing(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EngleGrangerProxy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EngleGrangerTrendProxy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Envbw(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EnvelopeChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Esine(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ess(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Eveningstar(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ewmac(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EwmacRobust(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::FibonacciChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::FisherTransform(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ForceIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::FractalDimension(x) => {
                x.update_bar(open, high, low, close, volume)
            }
            Self::Framaadv(x) => {
                x.update_ohlc(open, high, low, close);
                x.value()
            }
            Self::FuzzyCandlesticks(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::GannHilo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Gapo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Gator(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::GmmaCompression(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Hammer(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Hampel(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Harami(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::HarRv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::HaTrend(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Heikinashi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::HiddenDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Highest(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Hilb(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::HilbertDominantCycle(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::HistoricalVolatilityC2C(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::HlValueArea(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Hma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Hmom(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Hyst(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::IchimokuCloud(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::IchimokuCloudPosition(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::IchimokuCloudThickness(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::IftRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Iip(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Iir(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::InformationGain(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::IntradayIntensity(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::IntradayMomentumIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::JSDivergence(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::JurikMa(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KamaSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KaufmanAdaptiveMA(x) => {
                x.update(close);
                x.value()
            }
            Self::Kc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kdj(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Keltdist(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KeltnerBandwidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KeltnerChannel(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KeltnerStop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Keltpos(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KLDivergence(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KnowSureThing(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kvo(x) => {
                x.update_bar(high, low, close, volume);
                x.value()
            }
            Self::Kcomp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kregime(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kscr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kslope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kslopez(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Kcmetrics(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::LaguerreRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::LjungBox(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Liqgap(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Lowest(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Lr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::LrSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Lz(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Macd(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MacdDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MacdHist(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MacdHistDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MacdHistZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MacdSignal(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MaCross(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MarketFacilitationIndex(x) => {
                x.update(high, low, volume);
                x.value()
            }
            Self::MarketRegimeFilter(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MarketMicro(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Marubozu(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MassIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::McGinley(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Medchan(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Medchanpos(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MesaAdaptiveMA(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Mfi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MomentumDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MomentumZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Morningstar(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MovingAverage(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MultiDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MultiTimeframeMomentumDivergence(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::MutualInformation(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Natr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::NeuralMomentumNetwork(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::NrRange(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::NviPvi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Obv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ObvDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ofi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::OrderBookSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::OrderFlowImb(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::OrGate(x) => {
                x.update_bar(open, high, low, close, volume);
                IndicatorValue::Flag(x.value())
            }
            Self::Pacf(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ParabolicSAR(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pchosc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pchwidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PercentB(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Percentilech(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PermutationEntropy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pfe(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Piercingpattern(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PivotAnchoredVwap(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pivotchan(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pmo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ppo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PpoSignal(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pressure(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PriceChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PriceMadZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PriceZScore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ProjectionBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PSARStop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Psl(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pvo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Pzo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Qqe(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Qstick(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::QuantileRegressionChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::QueueImb(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RangePercentile(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RangeCompressionBurst(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RangeToAtr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ravi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RbvJumpTest(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RealizedQuarticity(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RealizedVol(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RealizedVolZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RegimeComposite(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RegimeCompositeV2(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RegimeCompositeV3(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RegimeCompositeV4(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RegressionChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RegressionChannelWidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RelativeVolume(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rmi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rmid(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Roc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RocPercentile(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RollingFisherInformation(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Roof(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RsiDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RsiOma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RsiPercentileBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RsiPercentileRank(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RsiZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RSquared(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rsx(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rts(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rvgi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rvi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Rwi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sampen(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SampleEntropy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Scf(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Screst(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Screstp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sent(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sentr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ser(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sflat(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SfpDetector(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sg(x) => {
                x.update(close);
                x.value()
            }
            Self::StftBandEnergyRatio(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ShannonEntropy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Shootingstar(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SignCombiner(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Slmpr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SlopeDirectionLine(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Smi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralBandpower(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralBandwidthFeature(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralCentroidFeature(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralEntropyRate(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralFlatness(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralHighMidPowerRatio(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralLowMidPowerRatio(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralRolloff(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralSlope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpectralSlopeZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SpreadAnalyzer(x) => {
                x.update_bar(low, high);
                x.value()
            }
            Self::Sqmom(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sroll(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sroll95(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Srollp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SslChannel(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sslope(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Sslopep(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Ssloperp(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::StandardDeviationChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::StarcBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Stc(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::StdDevChannelWidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::StochasticRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::StochDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::StochastikD(x) => {
                x.update_bar(high, low, close, volume);
                x.value()
            }
            Self::Supertrend(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Supts(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SweepReversionIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SwingStop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::T3(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Tema(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TheilSenChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Threeblackcrows(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Threewhitesoldiers(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Thresh(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TickVolume(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Tii(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Tma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TransferEntropy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Trima(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TrimaBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Trin(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Trix(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TrueRange(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TrueStrengthIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Tweezer(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::TwiggsMoneyFlow(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::UlcerIndex(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::UltimateOscillator(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::UltimateOscillatorSmooth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VarianceRatio(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vbd(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vfi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vhf(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VhfMa(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vidya(x) => {
                x.update(close);
                x.value()
            }
            Self::VolatilityBreakoutDetector(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolatilityPercentileRankBands(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolatilityStop(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumeDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumeOscillator(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumePriceTrend(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumeProfileChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumeRateOfChange(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumeWeightedRsi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolumeZscore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vov(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VolOfVolPercentile(x) => {
                x.update_bar(open, high, low, close, volume);
                IndicatorValue::Single(x.value())
            }
            Self::VolOfVolPercentileTrend(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vdelta(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vpin(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VrZAgg(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VarianceRatioAggregate(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vwap(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VwapChannels(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VwapChannelWidth(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VwapDistance(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::VwapLevels(x) => {
                // VwapLevels uses update_volume_bar with Bar struct
                if let Some(ts) = timestamp {
                    let bar = crate::types::Bar {
                        time: ts, open, high, low, close, volume,
                    };
                    x.update_volume_bar(&bar);
                }
                x.value()
            }
            Self::Vwma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Vzo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::WeightedComposite(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::WilliamsAd(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::WilliamsDiv(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::WilliamsR(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::WickSpike(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Wave(x) => {
                x.update_bar(open, high, low, close, volume)
            }
            Self::Wma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Wvf(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::XorGate(x) => {
                x.update_bar(open, high, low, close, volume);
                IndicatorValue::Flag(x.value())
            }
            Self::Za(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ZlSma(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // ========================================
            // TIME-BASED INDICATORS (10)
            // ========================================
            Self::HourDay(x) => {
                if let Some(ts) = timestamp { x.update_with_timestamp(ts); }
                x.value()
            }
            Self::WeekMonth(x) => {
                if let Some(ts) = timestamp { x.update_with_timestamp(ts); }
                x.value()
            }
            Self::SoqEoq(x) => {
                if let Some(ts) = timestamp { x.update_with_timestamp(ts); }
                x.value()
            }
            Self::Tenc(x) => {
                if let Some(ts) = timestamp { x.update_with_timestamp(ts); }
                x.value() // Returns Triple(dow, month, session)
            }
            Self::HolidayProx(x) => {
                if let Some(ts) = timestamp {
                    let weekday = Self::derive_weekday(Some(ts)) as u32;
                    x.update_bar(open, high, low, close, volume, weekday);
                }
                x.value()
            }
            Self::SowEow(x) => {
                if let Some(ts) = timestamp {
                    let weekday = Self::derive_weekday(Some(ts)) as u32;
                    x.update_bar(open, high, low, close, volume, weekday);
                }
                x.value()
            }
            Self::SomEom(x) => {
                if let Some(ts) = timestamp {
                    let (year, month, day) = Self::derive_ymd(Some(ts));
                    x.update_bar(open, high, low, close, volume, year, month, day);
                }
                x.value()
            }
            Self::MonthTurn(x) => {
                if let Some(ts) = timestamp {
                    let (year, month, day) = Self::derive_ymd(Some(ts));
                    x.update_bar(open, high, low, close, volume, year, month, day);
                }
                x.value()
            }
            Self::QtrTurn(x) => {
                if let Some(ts) = timestamp {
                    let (year, month, day) = Self::derive_ymd(Some(ts));
                    x.update_bar(open, high, low, close, volume, year, month, day);
                }
                x.value()
            }
            Self::DayWeekMonth(x) => {
                if let Some(ts) = timestamp {
                    let (year, month, day) = Self::derive_ymd(Some(ts));
                    let weekday = Self::derive_weekday(Some(ts)) as u32;
                    x.update_bar(open, high, low, close, volume, year, month, day, weekday);
                }
                x.value()
            }
            // ========================================
            // FVG INDICATORS (4)
            // ========================================
            Self::FvgDetector(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Fvgalt(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Fvgdur(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Fvgrev(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // ========================================
            // PHASE 1: Simple update(close) indicators
            // ========================================
            Self::Hurst(x) => {
                x.update(close);
                x.value()
            }
            Self::HurstPct(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Arima(x) => {
                x.update(close);
                x.value()
            }
            Self::ArimaX(x) => {
                // ArimaX needs exogenous variables; use close as single exog
                x.update(close, &[volume]);
                x.value()
            }
            Self::Garch(x) => {
                x.update(close);
                x.value()
            }
            Self::EGarch(x) => {
                x.update(close);
                x.value()
            }
            Self::Var(x) => {
                // VAR needs multiple series; use [close, volume] as proxy
                x.update(&[close, volume]);
                x.value()
            }
            Self::PolyReg(x) => {
                x.update(close);
                x.value()
            }
            Self::BasicKalmanFilter(x) => {
                x.update(close);
                x.value()
            }
            Self::ExtendedKalmanFilter(x) => {
                x.update(close);
                x.value()
            }
            Self::UnscentedKalmanFilter(x) => {
                x.update(close);
                x.value()
            }
            Self::ParticleFilter(x) => {
                x.update(close);
                x.value()
            }
            Self::HalfLifeMr(x) => {
                let hl = x.update_bar(open, high, low, close, volume);
                IndicatorValue::Single(if hl.is_finite() { hl } else { 0.0 })
            }
            Self::Coint(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Dfa(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::DfaPercentile(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // ========================================
            // PHASE 2: update_bar(h, l, c) indicators
            // ========================================
            Self::Swings(x) => {
                x.update_bar(high, low, close, volume);
                x.value()
            }
            Self::SwingsSoft(x) => {
                x.update_bar(high, low, close, volume);
                x.value()
            }
            Self::Fractals(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SwingStrengthScore(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::SwingAge(x) => {
                let (age_high, age_low) = x.update_bar(open, high, low, close, volume);
                IndicatorValue::Double(age_high as f64, age_low as f64)
            }
            // ========================================
            // PHASE 3: OHLCV indicators
            // ========================================
            Self::Tdi(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PivotPoints(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::FloorTraderPivots(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::CamarillaPivots(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::WoodiePivots(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RelTrendPos(x) => {
                // RelTrendPos needs timestamp
                let ts = timestamp.unwrap_or(0);
                x.update_bar(open, high, low, close, volume, ts);
                x.value()
            }
            // ========================================
            // PHASE 4: Timestamp-dependent indicators
            // ========================================
            Self::Weekday(x) => {
                let weekday = Self::derive_weekday(timestamp);
                x.update_with_weekday(close, weekday);
                x.value()
            }
            Self::Session(x) => {
                let session = Self::derive_session(timestamp);
                x.update_with_session(close, session);
                x.value()
            }
            Self::MonthQtr(x) => {
                let month = Self::derive_month(timestamp);
                let quarter = Self::month_to_quarter(month);
                x.update_with_calendar(close, month, quarter);
                x.value()
            }
            Self::DomWoq(x) => {
                let day_of_month = Self::derive_day_of_month(timestamp);
                let week_of_quarter = Self::derive_week_of_quarter(timestamp);
                x.update_with_calendar(close, day_of_month, week_of_quarter);
                x.value()
            }
            Self::VolumeProfile(x) => {
                // VolumeProfile needs Bar struct; create minimal proxy
                if let Some(ts) = timestamp {
                    let bar = crate::types::Bar {
                        time: ts, open, high, low, close, volume,
                    };
                    x.update(&bar);
                }
                // VolumeProfile returns POC price as main value
                let poc = x.get_poc().map(|pl| pl.price).unwrap_or(close);
                IndicatorValue::Single(poc)
            }
            Self::Poc(x) => {
                // Poc needs Bar struct
                if let Some(ts) = timestamp {
                    let bar = crate::types::Bar {
                        time: ts, open, high, low, close, volume,
                    };
                    x.update(&bar);
                }
                // Return POC price from analysis
                let poc_price = x.get_current_poc().map(|poc| poc.price).unwrap_or(close);
                IndicatorValue::Single(poc_price)
            }
            Self::AutoFibo(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // ========================================
            // PHASE 5: ZigZag indicators with idx
            // ========================================
            Self::Zigzag(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ZigzagClassic(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ZigzagAtr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ZigzagCandle(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ZigzagLookahead(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ZigzagTime(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // ========== PHASE 6: Previously missing indicators ==========
            Self::Stochastics(x) => {
                x.update_bar(high, low, close, volume);
                x.value()
            }
            Self::VortexIndicator(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdfProxy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KpssProxy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KpssTrend(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::KpssZ(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AdfKpss(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::EngleGrangerAdfProxy(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // ========================================
            // PHASE 7: More missing NOT READY indicators
            // ========================================
            Self::FastFourierTransform(x) => {
                x.update(close);
                x.value()
            }
            Self::Dvr(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::PvCoherence(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::ResidStat(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::RollingQuartiles(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::Xmil(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AvwapDistance(x) => {
                let ts = timestamp.unwrap_or(0);
                x.update_bar(open, high, low, close, volume, ts);
                x.value()
            }
            Self::AvwapMultiAnchorReversion(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            Self::AvwapTouchProbability(x) => {
                x.update_bar(open, high, low, close, volume);
                x.value()
            }
            // Catch-all for remaining legacy indicators
            _ => IndicatorValue::Single(0.0),
        }
    }

    pub fn value(&self) -> IndicatorValue {
        match self {
            Self::Abgfilter(ind) => ind.value(),
            Self::Ac(ind) => ind.value(),
            Self::AccumulationDistribution(ind) => ind.value(),
            Self::AccumulativeSwingIndex(ind) => ind.value(),
            Self::AdaptiveBollingerBands(ind) => ind.value(),
            Self::AdaptiveChannels(ind) => ind.value(),
            Self::AdaptiveMovingAverage(ind) => ind.value(),
            Self::AdaptiveStochastic(ind) => ind.value(),
            Self::Adx(ind) => ind.value(),
            Self::AdxSlope(ind) => ind.value(),
            Self::AdfKpss(ind) => ind.value(),
            Self::AdfProxy(ind) => ind.value(),
            Self::Alligator(ind) => ind.value(),
            Self::Alma(ind) => ind.value(),
            Self::Ama(ind) => ind.value(),
            Self::Amat(ind) => ind.value(),
            Self::AnchoredVwap(ind) => ind.value(),
            Self::AndGate(ind) => IndicatorValue::Flag(ind.value()),
            Self::Apen(ind) => ind.value(),
            Self::Apo(ind) => ind.value(),
            Self::Aroon(ind) => ind.value(),
            Self::AroonDown(ind) => ind.value(),
            Self::AroonOscillator(ind) => ind.value(),
            Self::AroonUp(ind) => ind.value(),
            Self::Atr(ind) => ind.value(),
            Self::AtrBandwidth(ind) => ind.value(),
            Self::AtrChannels(ind) => ind.value(),
            Self::AtrPercentile(ind) => ind.value(),
            Self::AtrRsi(ind) => ind.value(),
            Self::Atrts(ind) => ind.value(),
            Self::AtrZscore(ind) => ind.value(),
            Self::Autocorr(ind) => ind.value(),
            Self::AvFrama(ind) => ind.value(),
            Self::AvVidya(ind) => ind.value(),
            Self::AwesomeOscillator(ind) => ind.value(),
            Self::BbPeriod(ind) => ind.value(),
            Self::Bias(ind) => ind.value(),
            Self::BipowerVariance(ind) => ind.value(),
            Self::BollingerBands(ind) => ind.value(),
            Self::BollingerMetrics(ind) => ind.value(),
            Self::BookImb(ind) => ind.value(),
            Self::BookSlope(ind) => ind.value(),
            Self::Bop(ind) => ind.value(),
            Self::ButterworthFilter(ind) => ind.value(),
            Self::CandleAnatomy(ind) => ind.value(),
            Self::Cci(ind) => ind.value(),
            Self::CciDiv(ind) => ind.value(),
            Self::CentralPivotRange(ind) => ind.value(),
            Self::Cfo(ind) => ind.value(),
            Self::ChaikinVolatility(ind) => ind.value(),
            Self::Chand(ind) => ind.value(),
            Self::ChandeKrollStop(ind) => ind.value(),
            Self::ChebyshevFilter(ind) => ind.value(),
            Self::Cho(ind) => ind.value(),
            Self::ChoppinessIndex(ind) => ind.value(),
            Self::ClassicDiv(ind) => ind.value(),
            Self::CloseVolPercentile(ind) => ind.value(),
            Self::ClQueueImb(ind) => ind.value(),
            Self::Cmf(ind) => ind.value(),
            Self::Cmo(ind) => ind.value(),
            Self::Cog(ind) => ind.value(),
            Self::Coint(ind) => ind.value(),
            Self::Conden(ind) => ind.value(),
            Self::ConnorsRsi(ind) => ind.value(),
            Self::CoppockCurve(ind) => ind.value(),
            Self::CusumFilter(ind) => ind.value(),
            Self::Cyber(ind) => ind.value(),
            Self::Darkcloudcover(ind) => ind.value(),
            Self::DarvasBox(ind) => ind.value(),
            Self::Dc(ind) => ind.value(),
            Self::Dcwidth(ind) => ind.value(),
            Self::Decyc(ind) => ind.value(),
            Self::Dema(ind) => ind.value(),
            Self::DemandIndex(ind) => ind.value(),
            Self::Demarker(ind) => ind.value(),
            Self::DeMarkPivots(ind) => ind.value(),
            Self::DetrendedPriceOscillator(ind) => ind.value(),
            Self::DetrendedSyntheticPrice(ind) => ind.value(),
            Self::Di(ind) => ind.value(),
            Self::DiPlusMinus(ind) => ind.value(),
            Self::Didi(ind) => ind.value(),
            Self::DivStrength(ind) => ind.value(),
            Self::Dm(ind) => ind.value(),
            Self::Doji(ind) => ind.value(),
            Self::DonchianBreakout(ind) => ind.value(),
            Self::DonchianChannel(ind) => ind.value(),
            Self::DonchianPosition(ind) => ind.value(),
            Self::DonchianStop(ind) => ind.value(),
            Self::DpoBands(ind) => ind.value(),
            Self::DpoPercent(ind) => ind.value(),
            Self::DssBressert(ind) => ind.value(),
            Self::EaseOfMovement(ind) => ind.value(),
            Self::EfficiencyRatioFullHistory(ind) => ind.value(),
            Self::EfficiencyRatioRingWindow(ind) => ind.value(),
            Self::EhlersCc(ind) => ind.value(),
            Self::EhlersFractalAdaptiveMa(ind) => ind.value(),
            Self::EhlersRocketRsi(ind) => ind.value(),
            Self::EhlersZeroLagEma(ind) => ind.value(),
            Self::Eit(ind) => ind.value(),
            Self::ElderImpulse(ind) => ind.value(),
            Self::ElderRay(ind) => ind.value(),
            Self::Ema(ind) => ind.value(),
            Self::EmaSlope(ind) => ind.value(),
            Self::Engulfing(ind) => ind.value(),
            Self::Envbw(ind) => ind.value(),
            Self::EnvelopeChannels(ind) => ind.value(),
            Self::Esine(ind) => ind.value(),
            Self::Ess(ind) => ind.value(),
            Self::Eveningstar(ind) => ind.value(),
            Self::Ewmac(ind) => ind.value(),
            Self::FibonacciChannels(ind) => ind.value(),
            Self::FisherTransform(ind) => ind.value(),
            Self::ForceIndex(ind) => ind.value(),
            Self::Framaadv(ind) => ind.value(),
            Self::FuzzyCandlesticks(ind) => ind.value(),
            Self::GannHilo(ind) => ind.value(),
            Self::Gapo(ind) => ind.value(),
            Self::Gator(ind) => ind.value(),
            Self::GmmaCompression(ind) => ind.value(),
            Self::Hammer(ind) => ind.value(),
            Self::Hampel(ind) => ind.value(),
            Self::Harami(ind) => ind.value(),
            Self::HarRv(ind) => ind.value(),
            Self::HaTrend(ind) => ind.value(),
            Self::Heikinashi(ind) => ind.value(),
            Self::HiddenDiv(ind) => ind.value(),
            Self::Highest(ind) => ind.value(),
            Self::HistoricalVolatilityC2C(ind) => ind.value(),
            Self::Hilb(ind) => ind.value(),
            Self::HlValueArea(ind) => ind.value(),
            Self::Hma(ind) => ind.value(),
            Self::Hmom(ind) => ind.value(),
            Self::Hyst(ind) => ind.value(),
            Self::IchimokuCloud(ind) => ind.value(),
            Self::IchimokuCloudPosition(ind) => ind.value(),
            Self::IchimokuCloudThickness(ind) => ind.value(),
            Self::IftRsi(ind) => ind.value(),
            Self::Iip(ind) => ind.value(),
            Self::Iir(ind) => ind.value(),
            Self::IntradayIntensity(ind) => ind.value(),
            Self::IntradayMomentumIndex(ind) => ind.value(),
            Self::JSDivergence(ind) => ind.value(),
            Self::JurikMa(ind) => ind.value(),
            Self::KamaSlope(ind) => ind.value(),
            Self::KaufmanAdaptiveMA(ind) => ind.value(),
            Self::Kc(ind) => ind.value(),
            Self::Kdj(ind) => ind.value(),
            Self::Keltdist(ind) => ind.value(),
            Self::KeltnerBandwidth(ind) => ind.value(),
            Self::KeltnerChannel(ind) => ind.value(),
            Self::KeltnerStop(ind) => ind.value(),
            Self::Keltpos(ind) => ind.value(),
            Self::KLDivergence(ind) => ind.value(),
            Self::KnowSureThing(ind) => ind.value(),
            Self::Kp(ind) => ind.value(),
            Self::Kvo(ind) => ind.value(),
            Self::KpssProxy(ind) => ind.value(),
            Self::LaguerreRsi(ind) => ind.value(),
            Self::LjungBox(ind) => ind.value(),
            Self::Lowest(ind) => ind.value(),
            Self::Lr(ind) => ind.value(),
            Self::LrSlope(ind) => ind.value(),
            Self::Lz(ind) => ind.value(),
            Self::Macd(ind) => ind.value(),
            Self::MacdDiv(ind) => ind.value(),
            Self::MacdHist(ind) => ind.value(),
            Self::MacdHistDiv(ind) => ind.value(),
            Self::MacdHistZscore(ind) => ind.value(),
            Self::MacdSignal(ind) => ind.value(),
            Self::MaCross(ind) => ind.value(),
            Self::MarketFacilitationIndex(ind) => ind.value(),
            Self::MarketRegimeFilter(ind) => ind.value(),
            Self::Marubozu(ind) => ind.value(),
            Self::MassIndex(ind) => ind.value(),
            Self::McGinley(ind) => ind.value(),
            Self::Medchan(ind) => ind.value(),
            Self::Medchanpos(ind) => ind.value(),
            Self::MesaAdaptiveMA(ind) => ind.value(),
            Self::Mfi(ind) => ind.value(),
            Self::MomentumDiv(ind) => ind.value(),
            Self::MomentumZscore(ind) => ind.value(),
            Self::Morningstar(ind) => ind.value(),
            Self::MovingAverage(ind) => ind.value(),
            Self::MultiDiv(ind) => ind.value(),
            Self::MultiTimeframeMomentumDivergence(ind) => ind.value(),
            Self::MutualInformation(ind) => ind.value(),
            Self::Natr(ind) => ind.value(),
            Self::NrRange(ind) => ind.value(),
            Self::NviPvi(ind) => ind.value(),
            Self::Obv(ind) => ind.value(),
            Self::ObvDiv(ind) => ind.value(),
            // Note: OHLCV indicators (15 variants) REMOVED - use MovingAverageWithField instead
            Self::OrderBookSlope(ind) => ind.value(),
            Self::OrGate(ind) => IndicatorValue::Flag(ind.value()),
            Self::Pacf(ind) => ind.value(),
            Self::ParabolicSAR(ind) => ind.value(),
            Self::Pchosc(ind) => ind.value(),
            Self::Pchwidth(ind) => ind.value(),
            Self::PercentB(ind) => ind.value(),
            Self::Percentilech(ind) => ind.value(),
            Self::PermutationEntropy(ind) => ind.value(),
            Self::Pfe(ind) => ind.value(),
            Self::Piercingpattern(ind) => ind.value(),
            Self::PivotAnchoredVwap(ind) => ind.value(),
            Self::Pivotchan(ind) => ind.value(),
            Self::Pmo(ind) => ind.value(),
            Self::Pp(ind) => ind.value(),
            Self::Ppo(ind) => ind.value(),
            Self::PpoSignal(ind) => ind.value(),
            Self::Pressure(ind) => ind.value(),
            Self::PriceChannels(ind) => ind.value(),
            Self::PriceMadZscore(ind) => ind.value(),
            Self::PriceZScore(ind) => ind.value(),
            Self::ProjectionBands(ind) => ind.value(),
            Self::PSARStop(ind) => ind.value(),
            Self::Psl(ind) => ind.value(),
            Self::Pvo(ind) => ind.value(),
            Self::Pzo(ind) => ind.value(),
            Self::Qqe(ind) => ind.value(),
            Self::Qstick(ind) => ind.value(),
            Self::QuantileRegressionChannels(ind) => ind.value(),
            Self::QueueImb(ind) => ind.value(),
            Self::RangePercentile(ind) => ind.value(),
            Self::RangeToAtr(ind) => ind.value(),
            Self::Ravi(ind) => ind.value(),
            Self::RbvJumpTest(ind) => ind.value(),
            Self::RealizedQuarticity(ind) => ind.value(),
            Self::RealizedVol(ind) => ind.value(),
            Self::RealizedVolZscore(ind) => ind.value(),
            Self::RegimeComposite(ind) => ind.value(),
            Self::RegimeCompositeV2(ind) => ind.value(),
            Self::RegimeCompositeV3(ind) => ind.value(),
            Self::RegimeCompositeV4(ind) => ind.value(),
            Self::RegressionChannels(ind) => ind.value(),
            Self::RegressionChannelWidth(ind) => ind.value(),
            Self::RelativeVolume(ind) => ind.value(),
            Self::Rma(ind) => ind.value(),
            Self::Rmi(ind) => ind.value(),
            Self::Rmid(ind) => ind.value(),
            Self::Roc(ind) => ind.value(),
            Self::RocPercentile(ind) => ind.value(),
            Self::RollingFisherInformation(ind) => ind.value(),
            Self::Roof(ind) => ind.value(),
            Self::Rsi(ind) => ind.value(),
            Self::RsiDiv(ind) => ind.value(),
            Self::RsiOma(ind) => ind.value(),
            Self::RsiPercentileBands(ind) => ind.value(),
            Self::RsiPercentileRank(ind) => ind.value(),
            Self::RsiZscore(ind) => ind.value(),
            Self::RSquared(ind) => ind.value(),
            Self::Rsx(ind) => ind.value(),
            Self::Rts(ind) => ind.value(),
            Self::Rvgi(ind) => ind.value(),
            Self::Rvi(ind) => ind.value(),
            Self::Rwi(ind) => ind.value(),
            Self::Sampen(ind) => ind.value(),
            Self::SampleEntropy(ind) => ind.value(),
            Self::Scf(ind) => ind.value(),
            Self::Screst(ind) => ind.value(),
            Self::Screstp(ind) => ind.value(),
            Self::Sent(ind) => ind.value(),
            Self::Sentr(ind) => ind.value(),
            Self::Ser(ind) => ind.value(),
            Self::Sflat(ind) => ind.value(),
            Self::Sg(ind) => ind.value(),
            Self::StftBandEnergyRatio(ind) => ind.value(),
            Self::ShannonEntropy(ind) => ind.value(),
            Self::Shootingstar(ind) => ind.value(),
            Self::SignCombiner(ind) => ind.value(),
            Self::Slmpr(ind) => ind.value(),
            Self::SlopeDirectionLine(ind) => ind.value(),
            Self::Sma(ind) => ind.value(),
            Self::Smi(ind) => ind.value(),
            Self::SpectralBandpower(ind) => ind.value(),
            Self::SpectralBandwidthFeature(ind) => ind.value(),
            Self::SpectralCentroidFeature(ind) => ind.value(),
            Self::SpectralEntropyRate(ind) => ind.value(),
            Self::SpectralFlatness(ind) => ind.value(),
            Self::SpectralHighMidPowerRatio(ind) => ind.value(),
            Self::SpectralLowMidPowerRatio(ind) => ind.value(),
            Self::SpectralRolloff(ind) => ind.value(),
            Self::SpectralSlope(ind) => ind.value(),
            Self::SpectralSlopeZscore(ind) => ind.value(),
            Self::SpreadAnalyzer(ind) => ind.value(),
            Self::Sqmom(ind) => ind.value(),
            Self::Sroll(ind) => ind.value(),
            Self::Sroll95(ind) => ind.value(),
            Self::Srollp(ind) => ind.value(),
            Self::SslChannel(ind) => ind.value(),
            Self::Sslope(ind) => ind.value(),
            Self::Sslopep(ind) => ind.value(),
            Self::Ssloperp(ind) => ind.value(),
            Self::StandardDeviationChannels(ind) => ind.value(),
            Self::StarcBands(ind) => ind.value(),
            Self::Stc(ind) => ind.value(),
            Self::StdDevChannelWidth(ind) => ind.value(),
            Self::StochastikD(ind) => ind.value(),
            Self::StochasticRsi(ind) => ind.value(),
            Self::StochDiv(ind) => ind.value(),
            Self::Supertrend(ind) => ind.value(),
            Self::Supts(ind) => ind.value(),
            Self::SweepReversionIndex(ind) => ind.value(),
            Self::Swings(ind) => ind.value(),
            Self::SwingsSoft(ind) => ind.value(),
            Self::SwingStop(ind) => ind.value(),
            Self::T3(ind) => ind.value(),
            Self::Tema(ind) => ind.value(),
            Self::TheilSenChannels(ind) => ind.value(),
            Self::Threeblackcrows(ind) => ind.value(),
            Self::Threewhitesoldiers(ind) => ind.value(),
            Self::Thresh(ind) => ind.value(),
            Self::Tii(ind) => ind.value(),
            Self::Tma(ind) => ind.value(),
            Self::Tdi(ind) => ind.value(),
            Self::TransferEntropy(ind) => ind.value(),
            Self::Trima(ind) => ind.value(),
            Self::TrimaBands(ind) => ind.value(),
            Self::Trin(ind) => ind.value(),
            Self::Trix(ind) => ind.value(),
            Self::TrueRange(ind) => ind.value(),
            Self::TrueStrengthIndex(ind) => ind.value(),
            Self::Tweezer(ind) => ind.value(),
            Self::TwiggsMoneyFlow(ind) => ind.value(),
            Self::UlcerIndex(ind) => ind.value(),
            Self::UltimateOscillator(ind) => ind.value(),
            Self::UltimateOscillatorSmooth(ind) => ind.value(),
            Self::VarianceRatio(ind) => ind.value(),
            Self::Vbd(ind) => ind.value(),
            Self::Vfi(ind) => ind.value(),
            Self::Vhf(ind) => ind.value(),
            Self::VhfMa(ind) => ind.value(),
            Self::Vidya(ind) => ind.value(),
            Self::VortexIndicator(ind) => ind.value(),
            Self::VolatilityBreakoutDetector(ind) => ind.value(),
            Self::VolatilityPercentileRankBands(ind) => ind.value(),
            Self::VolatilityStop(ind) => ind.value(),
            Self::VolumeDiv(ind) => ind.value(),
            Self::VolumeOscillator(ind) => ind.value(),
            Self::VolumePriceTrend(ind) => ind.value(),
            Self::VolumeProfileChannels(ind) => ind.value(),
            Self::VolumeRateOfChange(ind) => ind.value(),
            Self::VolumeWeightedRsi(ind) => ind.value(),
            Self::VolumeZscore(ind) => ind.value(),
            Self::Vov(ind) => ind.value(),
            Self::Vpin(ind) => ind.value(),
            Self::Vr(ind) => ind.value(),
            Self::Vwap(ind) => ind.value(),
            Self::VwapChannels(ind) => ind.value(),
            Self::VwapChannelWidth(ind) => ind.value(),
            Self::VwapDistance(ind) => ind.value(),
            Self::Vwma(ind) => ind.value(),
            Self::Vzo(ind) => ind.value(),
            Self::WeightedComposite(ind) => ind.value(),
            Self::WilliamsAd(ind) => ind.value(),
            Self::WilliamsDiv(ind) => ind.value(),
            Self::WilliamsR(ind) => ind.value(),
            Self::Wma(ind) => ind.value(),
            Self::Wvf(ind) => ind.value(),
            Self::XorGate(ind) => IndicatorValue::Flag(ind.value()),
            Self::Za(ind) => ind.value(),
            Self::Zigzag(ind) => ind.value(),
            Self::ZigzagClassic(ind) => ind.value(),
            Self::ZlSma(ind) => ind.value(),
            Self::Stochastics(ind) => ind.value(),
            Self::PivotPoints(ind) => ind.value(),
            Self::RollingQuartiles(ind) => ind.value(),
            // TIME-BASED INDICATORS
            Self::HourDay(ind) => ind.value(),
            Self::WeekMonth(ind) => ind.value(),
            Self::SoqEoq(ind) => ind.value(),
            Self::Tenc(ind) => ind.value(),
            Self::HolidayProx(ind) => ind.value(),
            Self::SowEow(ind) => ind.value(),
            Self::SomEom(ind) => ind.value(),
            Self::MonthTurn(ind) => ind.value(),
            Self::QtrTurn(ind) => ind.value(),
            Self::DayWeekMonth(ind) => ind.value(),
            // FVG INDICATORS
            Self::FvgDetector(ind) => ind.value(),
            Self::Fvgalt(ind) => ind.value(),
            Self::Fvgdur(ind) => ind.value(),
            Self::Fvgrev(ind) => ind.value(),
            // ========================================
            // MISSING VALUE() HANDLERS - BATCH FIX
            // ========================================
            Self::AdaptiveVolatilityRegime(ind) => ind.value(),
            Self::AdvancedPatternRecognition(ind) => ind.value(),
            Self::ArchLmProxy(ind) => ind.value(),
            Self::ArchLmPval(ind) => ind.value(),
            Self::Arima(ind) => ind.value(),
            Self::ArimaX(ind) => ind.value(),
            Self::AutoFibo(ind) => ind.value(),
            Self::AvwapDistance(ind) => ind.value(),
            Self::AvwapMultiAnchorReversion(ind) => ind.value(),
            Self::AvwapTouchProbability(ind) => ind.value(),
            Self::BasicKalmanFilter(ind) => ind.value(),
            Self::BosChochDetector(ind) => ind.value(),
            Self::BpCusum(ind) => ind.value(),
            Self::CamarillaPivots(ind) => ind.value(),
            Self::CandlePatterns(ind) => {
                let pattern = ind.value();
                let v = match pattern {
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::None => 0.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::BullishEngulfing => 1.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::BearishEngulfing => -1.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::Hammer => 2.0,
                    crate::bar_indicators::momentum::candle_patterns::CandlePattern::ShootingStar => -2.0,
                };
                IndicatorValue::Single(v)
            }
            Self::ChaosOscillator(ind) => ind.value(),
            Self::Dcmetrics(ind) => ind.value(),
            Self::Dfa(ind) => ind.value(),
            Self::DfaPercentile(ind) => ind.value(),
            Self::DistLevels(ind) => ind.value(),
            Self::DomWoq(ind) => ind.value(),
            Self::Dvr(ind) => ind.value(),
            Self::EfficiencyRatio(ind) => ind.value(),
            Self::EGarch(ind) => ind.value(),
            Self::EngleGrangerAdfProxy(ind) => ind.value(),
            Self::EngleGrangerProxy(ind) => ind.value(),
            Self::EngleGrangerTrendProxy(ind) => ind.value(),
            Self::EwmacRobust(ind) => ind.value(),
            Self::ExtendedKalmanFilter(ind) => ind.value(),
            Self::FastFourierTransform(ind) => ind.value(),
            Self::FloorTraderPivots(ind) => ind.value(),
            Self::FractalDimension(ind) => ind.value(),
            Self::Fractals(ind) => ind.value(),
            Self::Garch(ind) => ind.value(),
            Self::HalfLifeMr(ind) => ind.value(),
            Self::HilbertDominantCycle(ind) => ind.value(),
            Self::Hurst(ind) => ind.value(),
            Self::HurstPct(ind) => ind.value(),
            Self::InformationGain(ind) => ind.value(),
            Self::Kcmetrics(ind) => ind.value(),
            Self::Kcomp(ind) => ind.value(),
            Self::KpssTrend(ind) => ind.value(),
            Self::KpssZ(ind) => ind.value(),
            Self::Kregime(ind) => ind.value(),
            Self::Kscr(ind) => ind.value(),
            Self::Kslope(ind) => ind.value(),
            Self::Kslopez(ind) => ind.value(),
            Self::Liqgap(ind) => ind.value(),
            Self::MarketMicro(ind) => ind.value(),
            Self::MonthQtr(ind) => ind.value(),
            Self::NeuralMomentumNetwork(ind) => ind.value(),
            Self::Ofi(ind) => ind.value(),
            Self::OrderFlowImb(ind) => ind.value(),
            Self::ParticleFilter(ind) => ind.value(),
            Self::Poc(ind) => {
                let poc_price = ind.get_current_poc().map(|p| p.price).unwrap_or(0.0);
                IndicatorValue::Single(poc_price)
            }
            Self::PolyReg(ind) => ind.value(),
            Self::PvCoherence(ind) => ind.value(),
            Self::RangeCompressionBurst(ind) => ind.value(),
            Self::RelTrendPos(ind) => ind.value(),
            Self::ResidStat(ind) => ind.value(),
            Self::Session(ind) => ind.value(),
            Self::SfpDetector(ind) => ind.value(),
            Self::SwingAge(ind) => ind.value(),
            Self::SwingStrengthScore(ind) => ind.value(),
            Self::TickVolume(ind) => ind.value(),
            Self::TimeEncoders(ind) => ind.value(),
            Self::UnscentedKalmanFilter(ind) => ind.value(),
            Self::Var(ind) => ind.value(),
            Self::VarianceRatioAggregate(ind) => ind.value(),
            Self::Vdelta(ind) => ind.value(),
            Self::VolOfVolPercentile(ind) => IndicatorValue::Single(ind.value()),
            Self::VolOfVolPercentileTrend(ind) => ind.value(),
            Self::VolumeProfile(ind) => {
                let poc = ind.get_poc().map(|pl| pl.price).unwrap_or(0.0);
                IndicatorValue::Single(poc)
            }
            Self::VrZAgg(ind) => ind.value(),
            Self::VwapLevels(ind) => ind.value(),
            Self::Wave(ind) => ind.value(),
            Self::Wcomp(ind) => ind.value(),
            Self::Weekday(ind) => ind.value(),
            Self::WickSpike(ind) => ind.value(),
            Self::WoodiePivots(ind) => ind.value(),
            Self::Xmil(ind) => ind.value(),
            Self::ZigzagAtr(ind) => ind.value(),
            Self::ZigzagCandle(ind) => ind.value(),
            Self::ZigzagLookahead(ind) => ind.value(),
            Self::ZigzagTime(ind) => ind.value(),
        }
    }

    /// Check if indicator is ready (has enough data)
    pub fn is_ready(&self) -> bool {
        match self {
            Self::Abgfilter(ind) => ind.is_ready(),
            Self::Ac(ind) => ind.is_ready(),
            Self::AccumulationDistribution(ind) => ind.is_ready(),
            Self::AccumulativeSwingIndex(ind) => ind.is_ready(),
            Self::AdaptiveBollingerBands(ind) => ind.is_ready(),
            Self::AdaptiveChannels(ind) => ind.is_ready(),
            Self::AdaptiveMovingAverage(ind) => ind.is_ready(),
            Self::AdaptiveStochastic(ind) => ind.is_ready(),
            Self::AdaptiveVolatilityRegime(ind) => ind.is_ready(),
            Self::AdfKpss(ind) => ind.is_ready(),
            Self::AdfProxy(ind) => ind.is_ready(),
            Self::AdvancedPatternRecognition(ind) => ind.is_ready(),
            Self::Adx(ind) => ind.is_ready(),
            Self::DiPlusMinus(ind) => ind.is_ready(),
            Self::AdxSlope(ind) => ind.is_ready(),
            Self::Alligator(ind) => ind.is_ready(),
            Self::Alma(ind) => ind.is_ready(),
            Self::Ama(ind) => ind.is_ready(),
            Self::Amat(ind) => ind.is_ready(),
            Self::AnchoredVwap(ind) => ind.is_ready(),
            Self::AndGate(ind) => ind.is_ready(),
            Self::Apen(ind) => ind.is_ready(),
            Self::Apo(ind) => ind.is_ready(),
            Self::ArchLmProxy(ind) => ind.is_ready(),
            Self::ArchLmPval(ind) => ind.is_ready(),
            Self::Arima(ind) => ind.is_ready(),
            Self::ArimaX(ind) => ind.is_ready(),
            Self::Aroon(ind) => ind.is_ready(),
            Self::AroonDown(ind) => ind.is_ready(),
            Self::AroonOscillator(ind) => ind.is_ready(),
            Self::AroonUp(ind) => ind.is_ready(),
            Self::Atr(ind) => ind.is_ready(),
            Self::AtrBandwidth(ind) => ind.is_ready(),
            Self::AtrChannels(ind) => ind.is_ready(),
            Self::AtrPercentile(ind) => ind.is_ready(),
            Self::AtrRsi(ind) => ind.is_ready(),
            Self::Atrts(ind) => ind.is_ready(),
            Self::AtrZscore(ind) => ind.is_ready(),
            Self::Autocorr(ind) => ind.is_ready(),
            Self::AutoFibo(ind) => ind.is_ready(),
            Self::AvFrama(ind) => ind.is_ready(),
            Self::AvVidya(ind) => ind.is_ready(),
            Self::AvwapDistance(ind) => ind.is_ready(),
            Self::AvwapMultiAnchorReversion(ind) => ind.is_ready(),
            Self::AvwapTouchProbability(ind) => ind.is_ready(),
            Self::AwesomeOscillator(ind) => ind.is_ready(),
            Self::BasicKalmanFilter(ind) => ind.is_ready(),
            Self::BbPeriod(ind) => ind.is_ready(),
            Self::Bias(ind) => ind.is_ready(),
            Self::BipowerVariance(ind) => ind.is_ready(),
            Self::BollingerBands(ind) => ind.is_ready(),
            Self::BollingerMetrics(ind) => ind.is_ready(),
            Self::BookImb(ind) => ind.is_ready(),
            Self::BookSlope(ind) => ind.is_ready(),
            Self::Bop(ind) => ind.is_ready(),
            Self::BosChochDetector(ind) => ind.is_ready(),
            Self::BpCusum(ind) => ind.is_ready(),
            Self::ButterworthFilter(ind) => ind.is_ready(),
            Self::CamarillaPivots(ind) => ind.is_ready(),
            Self::CandleAnatomy(ind) => ind.is_ready(),
            Self::CandlePatterns(ind) => ind.is_ready(),
            Self::Cci(ind) => ind.is_ready(),
            Self::CciDiv(ind) => ind.is_ready(),
            Self::CentralPivotRange(ind) => ind.is_ready(),
            Self::Cfo(ind) => ind.is_ready(),
            Self::ChaikinVolatility(ind) => ind.is_ready(),
            Self::Chand(ind) => ind.is_ready(),
            Self::ChandeKrollStop(ind) => ind.is_ready(),
            Self::ChaosOscillator(ind) => ind.is_ready(),
            Self::ChebyshevFilter(ind) => ind.is_ready(),
            Self::Cho(ind) => ind.is_ready(),
            Self::ChoppinessIndex(ind) => ind.is_ready(),
            Self::ClassicDiv(ind) => ind.is_ready(),
            Self::CloseVolPercentile(ind) => ind.is_ready(),
            Self::ClQueueImb(ind) => ind.is_ready(),
            Self::Cmf(ind) => ind.is_ready(),
            Self::Cmo(ind) => ind.is_ready(),
            Self::Cog(ind) => ind.is_ready(),
            Self::Coint(ind) => ind.is_ready(),
            Self::Conden(ind) => ind.is_ready(),
            Self::ConnorsRsi(ind) => ind.is_ready(),
            Self::CoppockCurve(ind) => ind.is_ready(),
            Self::CusumFilter(ind) => ind.is_ready(),
            Self::Cyber(ind) => ind.is_ready(),
            Self::Darkcloudcover(ind) => ind.is_ready(),
            Self::DarvasBox(ind) => ind.is_ready(),
            Self::DayWeekMonth(ind) => ind.is_ready(),
            Self::Dc(ind) => ind.is_ready(),
            Self::Dcmetrics(ind) => ind.is_ready(),
            Self::Dcwidth(ind) => ind.is_ready(),
            Self::Decyc(ind) => ind.is_ready(),
            Self::Dema(ind) => ind.is_ready(),
            Self::DemandIndex(ind) => ind.is_ready(),
            Self::Demarker(ind) => ind.is_ready(),
            Self::DeMarkPivots(ind) => ind.is_ready(),
            Self::DetrendedPriceOscillator(ind) => ind.is_ready(),
            Self::DetrendedSyntheticPrice(ind) => ind.is_ready(),
            Self::Dfa(ind) => ind.is_ready(),
            Self::DfaPercentile(ind) => ind.is_ready(),
            Self::Di(ind) => ind.is_ready(),
            Self::Didi(ind) => ind.is_ready(),
            Self::DistLevels(ind) => ind.is_ready(),
            Self::DivStrength(ind) => ind.is_ready(),
            Self::Dm(ind) => ind.is_ready(),
            Self::Doji(ind) => ind.is_ready(),
            Self::DomWoq(ind) => ind.is_ready(),
            Self::DonchianBreakout(ind) => ind.is_ready(),
            Self::DonchianChannel(ind) => ind.is_ready(),
            Self::DonchianPosition(ind) => ind.is_ready(),
            Self::DonchianStop(ind) => ind.is_ready(),
            Self::DpoBands(ind) => ind.is_ready(),
            Self::DpoPercent(ind) => ind.is_ready(),
            Self::DssBressert(ind) => ind.is_ready(),
            Self::Dvr(ind) => ind.is_ready(),
            Self::EaseOfMovement(ind) => ind.is_ready(),
            Self::EfficiencyRatio(ind) => ind.is_ready(),
            Self::EfficiencyRatioFullHistory(ind) => ind.is_ready(),
            Self::EfficiencyRatioRingWindow(ind) => ind.is_ready(),
            Self::EGarch(ind) => ind.is_ready(),
            Self::EhlersCc(ind) => ind.is_ready(),
            Self::EhlersFractalAdaptiveMa(ind) => ind.is_ready(),
            Self::EhlersRocketRsi(ind) => ind.is_ready(),
            Self::EhlersZeroLagEma(ind) => ind.is_ready(),
            Self::Eit(ind) => ind.is_ready(),
            Self::ElderImpulse(ind) => ind.is_ready(),
            Self::ElderRay(ind) => ind.is_ready(),
            Self::Ema(ind) => ind.is_ready(),
            Self::EmaSlope(ind) => ind.is_ready(),
            Self::EngleGrangerAdfProxy(ind) => ind.is_ready(),
            Self::EngleGrangerProxy(ind) => ind.is_ready(),
            Self::EngleGrangerTrendProxy(ind) => ind.is_ready(),
            Self::Engulfing(ind) => ind.is_ready(),
            Self::Envbw(ind) => ind.is_ready(),
            Self::EnvelopeChannels(ind) => ind.is_ready(),
            Self::Esine(ind) => ind.is_ready(),
            Self::Ess(ind) => ind.is_ready(),
            Self::Eveningstar(ind) => ind.is_ready(),
            Self::Ewmac(ind) => ind.is_ready(),
            Self::EwmacRobust(ind) => ind.is_ready(),
            Self::ExtendedKalmanFilter(ind) => ind.is_ready(),
            Self::FastFourierTransform(ind) => ind.is_ready(),
            Self::FibonacciChannels(ind) => ind.is_ready(),
            Self::FisherTransform(ind) => ind.is_ready(),
            Self::FloorTraderPivots(ind) => ind.is_ready(),
            Self::ForceIndex(ind) => ind.is_ready(),
            Self::FractalDimension(ind) => ind.is_ready(),
            Self::Fractals(ind) => ind.is_ready(),
            Self::Framaadv(ind) => ind.is_ready(),
            Self::FuzzyCandlesticks(ind) => ind.is_ready(),
            Self::Fvgalt(ind) => ind.is_ready(),
            Self::FvgDetector(ind) => ind.is_ready(),
            Self::Fvgdur(ind) => ind.is_ready(),
            Self::Fvgrev(ind) => ind.is_ready(),
            Self::GannHilo(ind) => ind.is_ready(),
            Self::Gapo(ind) => ind.is_ready(),
            Self::Garch(ind) => ind.is_ready(),
            Self::Gator(ind) => ind.is_ready(),
            Self::GmmaCompression(ind) => ind.is_ready(),
            Self::HalfLifeMr(ind) => ind.is_ready(),
            Self::Hammer(ind) => ind.is_ready(),
            Self::Hampel(ind) => ind.is_ready(),
            Self::Harami(ind) => ind.is_ready(),
            Self::HarRv(ind) => ind.is_ready(),
            Self::HaTrend(ind) => ind.is_ready(),
            Self::Heikinashi(ind) => ind.is_ready(),
            Self::HiddenDiv(ind) => ind.is_ready(),
            Self::Highest(ind) => ind.is_ready(),
            Self::Hilb(ind) => ind.is_ready(),
            Self::HilbertDominantCycle(ind) => ind.is_ready(),
            Self::HistoricalVolatilityC2C(ind) => ind.is_ready(),
            Self::HlValueArea(ind) => ind.is_ready(),
            Self::Hma(ind) => ind.is_ready(),
            Self::Hmom(ind) => ind.is_ready(),
            Self::HolidayProx(ind) => ind.is_ready(),
            Self::HourDay(ind) => ind.is_ready(),
            Self::Hurst(ind) => ind.is_ready(),
            Self::HurstPct(ind) => ind.is_ready(),
            Self::Hyst(ind) => ind.is_ready(),
            Self::IchimokuCloud(ind) => ind.is_ready(),
            Self::IchimokuCloudPosition(ind) => ind.is_ready(),
            Self::IchimokuCloudThickness(ind) => ind.is_ready(),
            Self::IftRsi(ind) => ind.is_ready(),
            Self::Iip(ind) => ind.is_ready(),
            Self::Iir(ind) => ind.is_ready(),
            Self::InformationGain(ind) => ind.is_ready(),
            Self::IntradayIntensity(ind) => ind.is_ready(),
            Self::IntradayMomentumIndex(ind) => ind.is_ready(),
            Self::JSDivergence(ind) => ind.is_ready(),
            Self::JurikMa(ind) => ind.is_ready(),
            Self::KamaSlope(ind) => ind.is_ready(),
            Self::KaufmanAdaptiveMA(ind) => ind.is_ready(),
            Self::Kc(ind) => ind.is_ready(),
            Self::Kcmetrics(ind) => ind.is_ready(),
            Self::Kcomp(ind) => ind.is_ready(),
            Self::Kdj(ind) => ind.is_ready(),
            Self::Keltdist(ind) => ind.is_ready(),
            Self::KeltnerBandwidth(ind) => ind.is_ready(),
            Self::KeltnerChannel(ind) => ind.is_ready(),
            Self::KeltnerStop(ind) => ind.is_ready(),
            Self::Keltpos(ind) => ind.is_ready(),
            Self::KLDivergence(ind) => ind.is_ready(),
            Self::KnowSureThing(ind) => ind.is_ready(),
            Self::Kp(ind) => ind.is_ready(),
            Self::KpssProxy(ind) => ind.is_ready(),
            Self::KpssTrend(ind) => ind.is_ready(),
            Self::KpssZ(ind) => ind.is_ready(),
            Self::Kregime(ind) => ind.is_ready(),
            Self::Kscr(ind) => ind.is_ready(),
            Self::Kslope(ind) => ind.is_ready(),
            Self::Kslopez(ind) => ind.is_ready(),
            Self::Kvo(ind) => ind.is_ready(),
            Self::LaguerreRsi(ind) => ind.is_ready(),
            Self::Liqgap(ind) => ind.is_ready(),
            Self::LjungBox(ind) => ind.is_ready(),
            Self::Lowest(ind) => ind.is_ready(),
            Self::Lr(ind) => ind.is_ready(),
            Self::LrSlope(ind) => ind.is_ready(),
            Self::Lz(ind) => ind.is_ready(),
            Self::Macd(ind) => ind.is_ready(),
            Self::MacdDiv(ind) => ind.is_ready(),
            Self::MacdHist(ind) => ind.is_ready(),
            Self::MacdHistDiv(ind) => ind.is_ready(),
            Self::MacdHistZscore(ind) => ind.is_ready(),
            Self::MacdSignal(ind) => ind.is_ready(),
            Self::MaCross(ind) => ind.is_ready(),
            Self::MarketFacilitationIndex(ind) => ind.is_ready(),
            Self::MarketMicro(ind) => ind.is_ready(),
            Self::MarketRegimeFilter(ind) => ind.is_ready(),
            Self::Marubozu(ind) => ind.is_ready(),
            Self::MassIndex(ind) => ind.is_ready(),
            Self::McGinley(ind) => ind.is_ready(),
            Self::Medchan(ind) => ind.is_ready(),
            Self::Medchanpos(ind) => ind.is_ready(),
            Self::MesaAdaptiveMA(ind) => ind.is_ready(),
            Self::Mfi(ind) => ind.is_ready(),
            Self::MomentumDiv(ind) => ind.is_ready(),
            Self::MomentumZscore(ind) => ind.is_ready(),
            Self::MonthQtr(ind) => ind.is_ready(),
            Self::MonthTurn(ind) => ind.is_ready(),
            Self::Morningstar(ind) => ind.is_ready(),
            Self::MovingAverage(ind) => ind.is_ready(),
            Self::MultiDiv(ind) => ind.is_ready(),
            Self::MultiTimeframeMomentumDivergence(ind) => ind.is_ready(),
            Self::MutualInformation(ind) => ind.is_ready(),
            Self::Natr(ind) => ind.is_ready(),
            Self::NeuralMomentumNetwork(ind) => ind.is_ready(),
            Self::NrRange(ind) => ind.is_ready(),
            Self::NviPvi(ind) => ind.is_ready(),
            Self::Obv(ind) => ind.is_ready(),
            Self::ObvDiv(ind) => ind.is_ready(),
            Self::Ofi(ind) => ind.is_ready(),
            // Note: OHLCV indicators (15 variants) REMOVED - use MovingAverageWithField instead
            Self::OrderBookSlope(ind) => ind.is_ready(),
            Self::OrderFlowImb(ind) => ind.is_ready(),
            Self::OrGate(ind) => ind.is_ready(),
            Self::Pacf(ind) => ind.is_ready(),
            Self::ParabolicSAR(ind) => ind.is_ready(),
            Self::ParticleFilter(ind) => ind.is_ready(),
            Self::Pchosc(ind) => ind.is_ready(),
            Self::Pchwidth(ind) => ind.is_ready(),
            Self::PercentB(ind) => ind.is_ready(),
            Self::Percentilech(ind) => ind.is_ready(),
            Self::PermutationEntropy(ind) => ind.is_ready(),
            Self::Pfe(ind) => ind.is_ready(),
            Self::Piercingpattern(ind) => ind.is_ready(),
            Self::PivotAnchoredVwap(ind) => ind.is_ready(),
            Self::Pivotchan(ind) => ind.is_ready(),
            Self::PivotPoints(ind) => ind.is_ready(),
            Self::Pmo(ind) => ind.is_ready(),
            Self::Poc(ind) => ind.is_ready(),
            Self::PolyReg(ind) => ind.is_ready(),
            Self::Pp(ind) => ind.is_ready(),
            Self::Ppo(ind) => ind.is_ready(),
            Self::PpoSignal(ind) => ind.is_ready(),
            Self::Pressure(ind) => ind.is_ready(),
            Self::PriceChannels(ind) => ind.is_ready(),
            Self::PriceMadZscore(ind) => ind.is_ready(),
            Self::PriceZScore(ind) => ind.is_ready(),
            Self::ProjectionBands(ind) => ind.is_ready(),
            Self::PSARStop(ind) => ind.is_ready(),
            Self::Psl(ind) => ind.is_ready(),
            Self::PvCoherence(ind) => ind.is_ready(),
            Self::Pvo(ind) => ind.is_ready(),
            Self::Pzo(ind) => ind.is_ready(),
            Self::Qqe(ind) => ind.is_ready(),
            Self::Qstick(ind) => ind.is_ready(),
            Self::QtrTurn(ind) => ind.is_ready(),
            Self::QuantileRegressionChannels(ind) => ind.is_ready(),
            Self::QueueImb(ind) => ind.is_ready(),
            Self::RangeCompressionBurst(ind) => ind.is_ready(),
            Self::RangePercentile(ind) => ind.is_ready(),
            Self::RangeToAtr(ind) => ind.is_ready(),
            Self::Ravi(ind) => ind.is_ready(),
            Self::RbvJumpTest(ind) => ind.is_ready(),
            Self::RealizedQuarticity(ind) => ind.is_ready(),
            Self::RealizedVol(ind) => ind.is_ready(),
            Self::RealizedVolZscore(ind) => ind.is_ready(),
            Self::RegimeComposite(ind) => ind.is_ready(),
            Self::RegimeCompositeV2(ind) => ind.is_ready(),
            Self::RegimeCompositeV3(ind) => ind.is_ready(),
            Self::RegimeCompositeV4(ind) => ind.is_ready(),
            Self::RegressionChannels(ind) => ind.is_ready(),
            Self::RegressionChannelWidth(ind) => ind.is_ready(),
            Self::RelativeVolume(ind) => ind.is_ready(),
            Self::RelTrendPos(ind) => ind.is_ready(),
            Self::ResidStat(ind) => ind.is_ready(),
            Self::Rma(ind) => ind.is_ready(),
            Self::Rmi(ind) => ind.is_ready(),
            Self::Rmid(ind) => ind.is_ready(),
            Self::Roc(ind) => ind.is_ready(),
            Self::RocPercentile(ind) => ind.is_ready(),
            Self::RollingFisherInformation(ind) => ind.is_ready(),
            Self::RollingQuartiles(ind) => ind.is_ready(),
            Self::Roof(ind) => ind.is_ready(),
            Self::Rsi(ind) => ind.is_ready(),
            Self::RsiDiv(ind) => ind.is_ready(),
            Self::RsiOma(ind) => ind.is_ready(),
            Self::RsiPercentileBands(ind) => ind.is_ready(),
            Self::RsiPercentileRank(ind) => ind.is_ready(),
            Self::RsiZscore(ind) => ind.is_ready(),
            Self::RSquared(ind) => ind.is_ready(),
            Self::Rsx(ind) => ind.is_ready(),
            Self::Rts(ind) => ind.is_ready(),
            Self::Rvgi(ind) => ind.is_ready(),
            Self::Rvi(ind) => ind.is_ready(),
            Self::Rwi(ind) => ind.is_ready(),
            Self::Sampen(ind) => ind.is_ready(),
            Self::SampleEntropy(ind) => ind.is_ready(),
            Self::Scf(ind) => ind.is_ready(),
            Self::Screst(ind) => ind.is_ready(),
            Self::Screstp(ind) => ind.is_ready(),
            Self::Sent(ind) => ind.is_ready(),
            Self::Sentr(ind) => ind.is_ready(),
            Self::Ser(ind) => ind.is_ready(),
            Self::Session(ind) => ind.is_ready(),
            Self::Sflat(ind) => ind.is_ready(),
            Self::SfpDetector(ind) => ind.is_ready(),
            Self::Sg(ind) => ind.is_ready(),
            Self::StftBandEnergyRatio(ind) => ind.is_ready(),
            Self::ShannonEntropy(ind) => ind.is_ready(),
            Self::Shootingstar(ind) => ind.is_ready(),
            Self::SignCombiner(ind) => ind.is_ready(),
            Self::Slmpr(ind) => ind.is_ready(),
            Self::SlopeDirectionLine(ind) => ind.is_ready(),
            Self::Sma(ind) => ind.is_ready(),
            Self::Smi(ind) => ind.is_ready(),
            Self::SomEom(ind) => ind.is_ready(),
            Self::SoqEoq(ind) => ind.is_ready(),
            Self::SowEow(ind) => ind.is_ready(),
            Self::SpectralBandpower(ind) => ind.is_ready(),
            Self::SpectralBandwidthFeature(ind) => ind.is_ready(),
            Self::SpectralCentroidFeature(ind) => ind.is_ready(),
            Self::SpectralEntropyRate(ind) => ind.is_ready(),
            Self::SpectralFlatness(ind) => ind.is_ready(),
            Self::SpectralHighMidPowerRatio(ind) => ind.is_ready(),
            Self::SpectralLowMidPowerRatio(ind) => ind.is_ready(),
            Self::SpectralRolloff(ind) => ind.is_ready(),
            Self::SpectralSlope(ind) => ind.is_ready(),
            Self::SpectralSlopeZscore(ind) => ind.is_ready(),
            Self::SpreadAnalyzer(ind) => ind.is_ready(),
            Self::Sqmom(ind) => ind.is_ready(),
            Self::Sroll(ind) => ind.is_ready(),
            Self::Sroll95(ind) => ind.is_ready(),
            Self::Srollp(ind) => ind.is_ready(),
            Self::SslChannel(ind) => ind.is_ready(),
            Self::Sslope(ind) => ind.is_ready(),
            Self::Sslopep(ind) => ind.is_ready(),
            Self::Ssloperp(ind) => ind.is_ready(),
            Self::StandardDeviationChannels(ind) => ind.is_ready(),
            Self::StarcBands(ind) => ind.is_ready(),
            Self::Stc(ind) => ind.is_ready(),
            Self::StdDevChannelWidth(ind) => ind.is_ready(),
            Self::StochasticRsi(ind) => ind.is_ready(),
            Self::Stochastics(ind) => ind.is_ready(),
            Self::StochastikD(ind) => ind.is_ready(),
            Self::StochDiv(ind) => ind.is_ready(),
            Self::Supertrend(ind) => ind.is_ready(),
            Self::Supts(ind) => ind.is_ready(),
            Self::SweepReversionIndex(ind) => ind.is_ready(),
            Self::SwingAge(ind) => ind.is_ready(),
            Self::Swings(ind) => ind.is_ready(),
            Self::SwingsSoft(ind) => ind.is_ready(),
            Self::SwingStop(ind) => ind.is_ready(),
            Self::SwingStrengthScore(ind) => ind.is_ready(),
            Self::T3(ind) => ind.is_ready(),
            Self::Tdi(ind) => ind.is_ready(),
            Self::Tema(ind) => ind.is_ready(),
            Self::Tenc(ind) => ind.is_ready(),
            Self::TheilSenChannels(ind) => ind.is_ready(),
            Self::Threeblackcrows(ind) => ind.is_ready(),
            Self::Threewhitesoldiers(ind) => ind.is_ready(),
            Self::Thresh(ind) => ind.is_ready(),
            Self::TickVolume(ind) => ind.is_ready(),
            Self::Tii(ind) => ind.is_ready(),
            Self::TimeEncoders(ind) => ind.is_ready(),
            Self::Tma(ind) => ind.is_ready(),
            Self::TransferEntropy(ind) => ind.is_ready(),
            Self::Trima(ind) => ind.is_ready(),
            Self::TrimaBands(ind) => ind.is_ready(),
            Self::Trin(ind) => ind.is_ready(),
            Self::Trix(ind) => ind.is_ready(),
            Self::TrueRange(ind) => ind.is_ready(),
            Self::TrueStrengthIndex(ind) => ind.is_ready(),
            Self::Tweezer(ind) => ind.is_ready(),
            Self::TwiggsMoneyFlow(ind) => ind.is_ready(),
            Self::UlcerIndex(ind) => ind.is_ready(),
            Self::UltimateOscillator(ind) => ind.is_ready(),
            Self::UltimateOscillatorSmooth(ind) => ind.is_ready(),
            Self::UnscentedKalmanFilter(ind) => ind.is_ready(),
            Self::Var(ind) => ind.is_ready(),
            Self::VarianceRatio(ind) => ind.is_ready(),
            Self::VarianceRatioAggregate(ind) => ind.is_ready(),
            Self::Vbd(ind) => ind.is_ready(),
            Self::Vdelta(ind) => ind.is_ready(),
            Self::Vfi(ind) => ind.is_ready(),
            Self::Vhf(ind) => ind.is_ready(),
            Self::VhfMa(ind) => ind.is_ready(),
            Self::Vidya(ind) => ind.is_ready(),
            Self::VolatilityBreakoutDetector(ind) => ind.is_ready(),
            Self::VolatilityPercentileRankBands(ind) => ind.is_ready(),
            Self::VolatilityStop(ind) => ind.is_ready(),
            Self::VolOfVolPercentile(ind) => ind.is_ready(),
            Self::VolOfVolPercentileTrend(ind) => ind.is_ready(),
            Self::VolumeDiv(ind) => ind.is_ready(),
            Self::VolumeOscillator(ind) => ind.is_ready(),
            Self::VolumePriceTrend(ind) => ind.is_ready(),
            Self::VolumeProfile(ind) => ind.is_ready(),
            Self::VolumeProfileChannels(ind) => ind.is_ready(),
            Self::VolumeRateOfChange(ind) => ind.is_ready(),
            Self::VolumeWeightedRsi(ind) => ind.is_ready(),
            Self::VolumeZscore(ind) => ind.is_ready(),
            Self::VortexIndicator(ind) => ind.is_ready(),
            Self::Vov(ind) => ind.is_ready(),
            Self::Vpin(ind) => ind.is_ready(),
            Self::Vr(ind) => ind.is_ready(),
            Self::VrZAgg(ind) => ind.is_ready(),
            Self::Vwap(ind) => ind.is_ready(),
            Self::VwapChannels(ind) => ind.is_ready(),
            Self::VwapChannelWidth(ind) => ind.is_ready(),
            Self::VwapDistance(ind) => ind.is_ready(),
            Self::VwapLevels(ind) => ind.is_ready(),
            Self::Vwma(ind) => ind.is_ready(),
            Self::Vzo(ind) => ind.is_ready(),
            Self::Wave(ind) => ind.is_ready(),
            Self::Wcomp(ind) => ind.is_ready(),
            Self::Weekday(ind) => ind.is_ready(),
            Self::WeekMonth(ind) => ind.is_ready(),
            Self::WeightedComposite(ind) => ind.is_ready(),
            Self::WickSpike(ind) => ind.is_ready(),
            Self::WilliamsAd(ind) => ind.is_ready(),
            Self::WilliamsDiv(ind) => ind.is_ready(),
            Self::WilliamsR(ind) => ind.is_ready(),
            Self::Wma(ind) => ind.is_ready(),
            Self::WoodiePivots(ind) => ind.is_ready(),
            Self::Wvf(ind) => ind.is_ready(),
            Self::Xmil(ind) => ind.is_ready(),
            Self::XorGate(ind) => ind.is_ready(),
            Self::Za(ind) => ind.is_ready(),
            Self::Zigzag(ind) => ind.is_ready(),
            Self::ZigzagAtr(ind) => ind.is_ready(),
            Self::ZigzagCandle(ind) => ind.is_ready(),
            Self::ZigzagClassic(ind) => ind.is_ready(),
            Self::ZigzagLookahead(ind) => ind.is_ready(),
            Self::ZigzagTime(ind) => ind.is_ready(),
            Self::ZlSma(ind) => ind.is_ready(),
        }
    }

    /// Reset indicator state
    pub fn reset(&mut self) {
        match self {
            Self::Abgfilter(ind) => ind.reset(),
            Self::Ac(ind) => ind.reset(),
            Self::AccumulationDistribution(ind) => ind.reset(),
            Self::AccumulativeSwingIndex(ind) => ind.reset(),
            Self::AdaptiveBollingerBands(ind) => ind.reset(),
            Self::AdaptiveChannels(ind) => ind.reset(),
            Self::AdaptiveMovingAverage(ind) => ind.reset(),
            Self::AdaptiveStochastic(ind) => ind.reset(),
            Self::AdaptiveVolatilityRegime(ind) => ind.reset(),
            Self::AdfKpss(ind) => ind.reset(),
            Self::AdfProxy(ind) => ind.reset(),
            Self::AdvancedPatternRecognition(ind) => ind.reset(),
            Self::Adx(ind) => ind.reset(),
            Self::DiPlusMinus(ind) => ind.reset(),
            Self::AdxSlope(ind) => ind.reset(),
            Self::Alligator(ind) => ind.reset(),
            Self::Alma(ind) => ind.reset(),
            Self::Ama(ind) => ind.reset(),
            Self::Amat(ind) => ind.reset(),
            Self::AnchoredVwap(ind) => ind.reset(),
            Self::AndGate(ind) => ind.reset(),
            Self::Apen(ind) => ind.reset(),
            Self::Apo(ind) => ind.reset(),
            Self::ArchLmProxy(ind) => ind.reset(),
            Self::ArchLmPval(ind) => ind.reset(),
            Self::Arima(ind) => ind.reset(),
            Self::ArimaX(ind) => ind.reset(),
            Self::Aroon(ind) => ind.reset(),
            Self::AroonDown(ind) => ind.reset(),
            Self::AroonOscillator(ind) => ind.reset(),
            Self::AroonUp(ind) => ind.reset(),
            Self::Atr(ind) => ind.reset(),
            Self::AtrBandwidth(ind) => ind.reset(),
            Self::AtrChannels(ind) => ind.reset(),
            Self::AtrPercentile(ind) => ind.reset(),
            Self::AtrRsi(ind) => ind.reset(),
            Self::Atrts(ind) => ind.reset(),
            Self::AtrZscore(ind) => ind.reset(),
            Self::Autocorr(ind) => ind.reset(),
            Self::AutoFibo(ind) => ind.reset(),
            Self::AvFrama(ind) => ind.reset(),
            Self::AvVidya(ind) => ind.reset(),
            Self::AvwapDistance(ind) => ind.reset(),
            Self::AvwapMultiAnchorReversion(ind) => ind.reset(),
            Self::AvwapTouchProbability(ind) => ind.reset(),
            Self::AwesomeOscillator(ind) => ind.reset(),
            Self::BasicKalmanFilter(ind) => ind.reset(),
            Self::BbPeriod(ind) => ind.reset(),
            Self::Bias(ind) => ind.reset(),
            Self::BipowerVariance(ind) => ind.reset(),
            Self::BollingerBands(ind) => ind.reset(),
            Self::BollingerMetrics(ind) => ind.reset(),
            Self::BookImb(ind) => ind.reset(),
            Self::BookSlope(ind) => ind.reset(),
            Self::Bop(ind) => ind.reset(),
            Self::BosChochDetector(ind) => ind.reset(),
            Self::BpCusum(ind) => ind.reset(),
            Self::ButterworthFilter(ind) => ind.reset(),
            Self::CamarillaPivots(ind) => ind.reset(),
            Self::CandleAnatomy(ind) => ind.reset(),
            Self::CandlePatterns(ind) => ind.reset(),
            Self::Cci(ind) => ind.reset(),
            Self::CciDiv(ind) => ind.reset(),
            Self::CentralPivotRange(ind) => ind.reset(),
            Self::Cfo(ind) => ind.reset(),
            Self::ChaikinVolatility(ind) => ind.reset(),
            Self::Chand(ind) => ind.reset(),
            Self::ChandeKrollStop(ind) => ind.reset(),
            Self::ChaosOscillator(ind) => ind.reset(),
            Self::ChebyshevFilter(ind) => ind.reset(),
            Self::Cho(ind) => ind.reset(),
            Self::ChoppinessIndex(ind) => ind.reset(),
            Self::ClassicDiv(ind) => ind.reset(),
            Self::CloseVolPercentile(ind) => ind.reset(),
            Self::ClQueueImb(ind) => ind.reset(),
            Self::Cmf(ind) => ind.reset(),
            Self::Cmo(ind) => ind.reset(),
            Self::Cog(ind) => ind.reset(),
            Self::Coint(ind) => ind.reset(),
            Self::Conden(ind) => ind.reset(),
            Self::ConnorsRsi(ind) => ind.reset(),
            Self::CoppockCurve(ind) => ind.reset(),
            Self::CusumFilter(ind) => ind.reset(),
            Self::Cyber(ind) => ind.reset(),
            Self::Darkcloudcover(ind) => ind.reset(),
            Self::DarvasBox(ind) => ind.reset(),
            Self::DayWeekMonth(ind) => ind.reset(),
            Self::Dc(ind) => ind.reset(),
            Self::Dcmetrics(ind) => ind.reset(),
            Self::Dcwidth(ind) => ind.reset(),
            Self::Decyc(ind) => ind.reset(),
            Self::Dema(ind) => ind.reset(),
            Self::DemandIndex(ind) => ind.reset(),
            Self::Demarker(ind) => ind.reset(),
            Self::DeMarkPivots(ind) => ind.reset(),
            Self::DetrendedPriceOscillator(ind) => ind.reset(),
            Self::DetrendedSyntheticPrice(ind) => ind.reset(),
            Self::Dfa(ind) => ind.reset(),
            Self::DfaPercentile(ind) => ind.reset(),
            Self::Di(ind) => ind.reset(),
            Self::Didi(ind) => ind.reset(),
            Self::DistLevels(ind) => ind.reset(),
            Self::DivStrength(ind) => ind.reset(),
            Self::Dm(ind) => ind.reset(),
            Self::Doji(ind) => ind.reset(),
            Self::DomWoq(ind) => ind.reset(),
            Self::DonchianBreakout(ind) => ind.reset(),
            Self::DonchianChannel(ind) => ind.reset(),
            Self::DonchianPosition(ind) => ind.reset(),
            Self::DonchianStop(ind) => ind.reset(),
            Self::DpoBands(ind) => ind.reset(),
            Self::DpoPercent(ind) => ind.reset(),
            Self::DssBressert(ind) => ind.reset(),
            Self::Dvr(ind) => ind.reset(),
            Self::EaseOfMovement(ind) => ind.reset(),
            Self::EfficiencyRatio(ind) => ind.reset(),
            Self::EfficiencyRatioFullHistory(ind) => ind.reset(),
            Self::EfficiencyRatioRingWindow(ind) => ind.reset(),
            Self::EGarch(ind) => ind.reset(),
            Self::EhlersCc(ind) => ind.reset(),
            Self::EhlersFractalAdaptiveMa(ind) => ind.reset(),
            Self::EhlersRocketRsi(ind) => ind.reset(),
            Self::EhlersZeroLagEma(ind) => ind.reset(),
            Self::Eit(ind) => ind.reset(),
            Self::ElderImpulse(ind) => ind.reset(),
            Self::ElderRay(ind) => ind.reset(),
            Self::Ema(ind) => ind.reset(),
            Self::EmaSlope(ind) => ind.reset(),
            Self::EngleGrangerAdfProxy(ind) => ind.reset(),
            Self::EngleGrangerProxy(ind) => ind.reset(),
            Self::EngleGrangerTrendProxy(ind) => ind.reset(),
            Self::Engulfing(ind) => ind.reset(),
            Self::Envbw(ind) => ind.reset(),
            Self::EnvelopeChannels(ind) => ind.reset(),
            Self::Esine(ind) => ind.reset(),
            Self::Ess(ind) => ind.reset(),
            Self::Eveningstar(ind) => ind.reset(),
            Self::Ewmac(ind) => ind.reset(),
            Self::EwmacRobust(ind) => ind.reset(),
            Self::ExtendedKalmanFilter(ind) => ind.reset(),
            Self::FastFourierTransform(ind) => ind.reset(),
            Self::FibonacciChannels(ind) => ind.reset(),
            Self::FisherTransform(ind) => ind.reset(),
            Self::FloorTraderPivots(ind) => ind.reset(),
            Self::ForceIndex(ind) => ind.reset(),
            Self::FractalDimension(ind) => ind.reset(),
            Self::Fractals(ind) => ind.reset(),
            Self::Framaadv(ind) => ind.reset(),
            Self::FuzzyCandlesticks(ind) => ind.reset(),
            Self::Fvgalt(ind) => ind.reset(),
            Self::FvgDetector(ind) => ind.reset(),
            Self::Fvgdur(ind) => ind.reset(),
            Self::Fvgrev(ind) => ind.reset(),
            Self::GannHilo(ind) => ind.reset(),
            Self::Gapo(ind) => ind.reset(),
            Self::Garch(ind) => ind.reset(),
            Self::Gator(ind) => ind.reset(),
            Self::GmmaCompression(ind) => ind.reset(),
            Self::HalfLifeMr(ind) => ind.reset(),
            Self::Hammer(ind) => ind.reset(),
            Self::Hampel(ind) => ind.reset(),
            Self::Harami(ind) => ind.reset(),
            Self::HarRv(ind) => ind.reset(),
            Self::HaTrend(ind) => ind.reset(),
            Self::Heikinashi(ind) => ind.reset(),
            Self::HiddenDiv(ind) => ind.reset(),
            Self::Highest(ind) => ind.reset(),
            Self::Hilb(ind) => ind.reset(),
            Self::HilbertDominantCycle(ind) => ind.reset(),
            Self::HistoricalVolatilityC2C(ind) => ind.reset(),
            Self::HlValueArea(ind) => ind.reset(),
            Self::Hma(ind) => ind.reset(),
            Self::Hmom(ind) => ind.reset(),
            Self::HolidayProx(ind) => ind.reset(),
            Self::HourDay(ind) => ind.reset(),
            Self::Hurst(ind) => ind.reset(),
            Self::HurstPct(ind) => ind.reset(),
            Self::Hyst(ind) => ind.reset(),
            Self::IchimokuCloud(ind) => ind.reset(),
            Self::IchimokuCloudPosition(ind) => ind.reset(),
            Self::IchimokuCloudThickness(ind) => ind.reset(),
            Self::IftRsi(ind) => ind.reset(),
            Self::Iip(ind) => ind.reset(),
            Self::Iir(ind) => ind.reset(),
            Self::InformationGain(ind) => ind.reset(),
            Self::IntradayIntensity(ind) => ind.reset(),
            Self::IntradayMomentumIndex(ind) => ind.reset(),
            Self::JSDivergence(ind) => ind.reset(),
            Self::JurikMa(ind) => ind.reset(),
            Self::KamaSlope(ind) => ind.reset(),
            Self::KaufmanAdaptiveMA(ind) => ind.reset(),
            Self::Kc(ind) => ind.reset(),
            Self::Kcmetrics(ind) => ind.reset(),
            Self::Kcomp(ind) => ind.reset(),
            Self::Kdj(ind) => ind.reset(),
            Self::Keltdist(ind) => ind.reset(),
            Self::KeltnerBandwidth(ind) => ind.reset(),
            Self::KeltnerChannel(ind) => ind.reset(),
            Self::KeltnerStop(ind) => ind.reset(),
            Self::Keltpos(ind) => ind.reset(),
            Self::KLDivergence(ind) => ind.reset(),
            Self::KnowSureThing(ind) => ind.reset(),
            Self::Kp(ind) => ind.reset(),
            Self::KpssProxy(ind) => ind.reset(),
            Self::KpssTrend(ind) => ind.reset(),
            Self::KpssZ(ind) => ind.reset(),
            Self::Kregime(ind) => ind.reset(),
            Self::Kscr(ind) => ind.reset(),
            Self::Kslope(ind) => ind.reset(),
            Self::Kslopez(ind) => ind.reset(),
            Self::Kvo(ind) => ind.reset(),
            Self::LaguerreRsi(ind) => ind.reset(),
            Self::Liqgap(ind) => ind.reset(),
            Self::LjungBox(ind) => ind.reset(),
            Self::Lowest(ind) => ind.reset(),
            Self::Lr(ind) => ind.reset(),
            Self::LrSlope(ind) => ind.reset(),
            Self::Lz(ind) => ind.reset(),
            Self::Macd(ind) => ind.reset(),
            Self::MacdDiv(ind) => ind.reset(),
            Self::MacdHist(ind) => ind.reset(),
            Self::MacdHistDiv(ind) => ind.reset(),
            Self::MacdHistZscore(ind) => ind.reset(),
            Self::MacdSignal(ind) => ind.reset(),
            Self::MaCross(ind) => ind.reset(),
            Self::MarketFacilitationIndex(ind) => ind.reset(),
            Self::MarketMicro(ind) => ind.reset(),
            Self::MarketRegimeFilter(ind) => ind.reset(),
            Self::Marubozu(ind) => ind.reset(),
            Self::MassIndex(ind) => ind.reset(),
            Self::McGinley(ind) => ind.reset(),
            Self::Medchan(ind) => ind.reset(),
            Self::Medchanpos(ind) => ind.reset(),
            Self::MesaAdaptiveMA(ind) => ind.reset(),
            Self::Mfi(ind) => ind.reset(),
            Self::MomentumDiv(ind) => ind.reset(),
            Self::MomentumZscore(ind) => ind.reset(),
            Self::MonthQtr(ind) => ind.reset(),
            Self::MonthTurn(ind) => ind.reset(),
            Self::Morningstar(ind) => ind.reset(),
            Self::MovingAverage(ind) => ind.reset(),
            Self::MultiDiv(ind) => ind.reset(),
            Self::MultiTimeframeMomentumDivergence(ind) => ind.reset(),
            Self::MutualInformation(ind) => ind.reset(),
            Self::Natr(ind) => ind.reset(),
            Self::NeuralMomentumNetwork(ind) => ind.reset(),
            Self::NrRange(ind) => ind.reset(),
            Self::NviPvi(ind) => ind.reset(),
            Self::Obv(ind) => ind.reset(),
            Self::ObvDiv(ind) => ind.reset(),
            Self::Ofi(ind) => ind.reset(),
            // Note: OHLCV indicators (15 variants) REMOVED - use MovingAverageWithField instead
            Self::OrderBookSlope(ind) => ind.reset(),
            Self::OrderFlowImb(ind) => ind.reset(),
            Self::OrGate(ind) => ind.reset(),
            Self::Pacf(ind) => ind.reset(),
            Self::ParabolicSAR(ind) => ind.reset(),
            Self::ParticleFilter(ind) => ind.reset(),
            Self::Pchosc(ind) => ind.reset(),
            Self::Pchwidth(ind) => ind.reset(),
            Self::PercentB(ind) => ind.reset(),
            Self::Percentilech(ind) => ind.reset(),
            Self::PermutationEntropy(ind) => ind.reset(),
            Self::Pfe(ind) => ind.reset(),
            Self::Piercingpattern(ind) => ind.reset(),
            Self::PivotAnchoredVwap(ind) => ind.reset(),
            Self::Pivotchan(ind) => ind.reset(),
            Self::PivotPoints(ind) => ind.reset(),
            Self::Pmo(ind) => ind.reset(),
            Self::Poc(ind) => ind.reset(),
            Self::PolyReg(ind) => ind.reset(),
            Self::Pp(ind) => ind.reset(),
            Self::Ppo(ind) => ind.reset(),
            Self::PpoSignal(ind) => ind.reset(),
            Self::Pressure(ind) => ind.reset(),
            Self::PriceChannels(ind) => ind.reset(),
            Self::PriceMadZscore(ind) => ind.reset(),
            Self::PriceZScore(ind) => ind.reset(),
            Self::ProjectionBands(ind) => ind.reset(),
            Self::PSARStop(ind) => ind.reset(),
            Self::Psl(ind) => ind.reset(),
            Self::PvCoherence(ind) => ind.reset(),
            Self::Pvo(ind) => ind.reset(),
            Self::Pzo(ind) => ind.reset(),
            Self::Qqe(ind) => ind.reset(),
            Self::Qstick(ind) => ind.reset(),
            Self::QtrTurn(ind) => ind.reset(),
            Self::QuantileRegressionChannels(ind) => ind.reset(),
            Self::QueueImb(ind) => ind.reset(),
            Self::RangeCompressionBurst(ind) => ind.reset(),
            Self::RangePercentile(ind) => ind.reset(),
            Self::RangeToAtr(ind) => ind.reset(),
            Self::Ravi(ind) => ind.reset(),
            Self::RbvJumpTest(ind) => ind.reset(),
            Self::RealizedQuarticity(ind) => ind.reset(),
            Self::RealizedVol(ind) => ind.reset(),
            Self::RealizedVolZscore(ind) => ind.reset(),
            Self::RegimeComposite(ind) => ind.reset(),
            Self::RegimeCompositeV2(ind) => ind.reset(),
            Self::RegimeCompositeV3(ind) => ind.reset(),
            Self::RegimeCompositeV4(ind) => ind.reset(),
            Self::RegressionChannels(ind) => ind.reset(),
            Self::RegressionChannelWidth(ind) => ind.reset(),
            Self::RelativeVolume(ind) => ind.reset(),
            Self::RelTrendPos(ind) => ind.reset(),
            Self::ResidStat(ind) => ind.reset(),
            Self::Rma(ind) => ind.reset(),
            Self::Rmi(ind) => ind.reset(),
            Self::Rmid(ind) => ind.reset(),
            Self::Roc(ind) => ind.reset(),
            Self::RocPercentile(ind) => ind.reset(),
            Self::RollingFisherInformation(ind) => ind.reset(),
            Self::RollingQuartiles(ind) => ind.reset(),
            Self::Roof(ind) => ind.reset(),
            Self::Rsi(ind) => ind.reset(),
            Self::RsiDiv(ind) => ind.reset(),
            Self::RsiOma(ind) => ind.reset(),
            Self::RsiPercentileBands(ind) => ind.reset(),
            Self::RsiPercentileRank(ind) => ind.reset(),
            Self::RsiZscore(ind) => ind.reset(),
            Self::RSquared(ind) => ind.reset(),
            Self::Rsx(ind) => ind.reset(),
            Self::Rts(ind) => ind.reset(),
            Self::Rvgi(ind) => ind.reset(),
            Self::Rvi(ind) => ind.reset(),
            Self::Rwi(ind) => ind.reset(),
            Self::Sampen(ind) => ind.reset(),
            Self::SampleEntropy(ind) => ind.reset(),
            Self::Scf(ind) => ind.reset(),
            Self::Screst(ind) => ind.reset(),
            Self::Screstp(ind) => ind.reset(),
            Self::Sent(ind) => ind.reset(),
            Self::Sentr(ind) => ind.reset(),
            Self::Ser(ind) => ind.reset(),
            Self::Session(ind) => ind.reset(),
            Self::Sflat(ind) => ind.reset(),
            Self::SfpDetector(ind) => ind.reset(),
            Self::Sg(ind) => ind.reset(),
            Self::StftBandEnergyRatio(ind) => ind.reset(),
            Self::ShannonEntropy(ind) => ind.reset(),
            Self::Shootingstar(ind) => ind.reset(),
            Self::SignCombiner(ind) => ind.reset(),
            Self::Slmpr(ind) => ind.reset(),
            Self::SlopeDirectionLine(ind) => ind.reset(),
            Self::Sma(ind) => ind.reset(),
            Self::Smi(ind) => ind.reset(),
            Self::SomEom(ind) => ind.reset(),
            Self::SoqEoq(ind) => ind.reset(),
            Self::SowEow(ind) => ind.reset(),
            Self::SpectralBandpower(ind) => ind.reset(),
            Self::SpectralBandwidthFeature(ind) => ind.reset(),
            Self::SpectralCentroidFeature(ind) => ind.reset(),
            Self::SpectralEntropyRate(ind) => ind.reset(),
            Self::SpectralFlatness(ind) => ind.reset(),
            Self::SpectralHighMidPowerRatio(ind) => ind.reset(),
            Self::SpectralLowMidPowerRatio(ind) => ind.reset(),
            Self::SpectralRolloff(ind) => ind.reset(),
            Self::SpectralSlope(ind) => ind.reset(),
            Self::SpectralSlopeZscore(ind) => ind.reset(),
            Self::SpreadAnalyzer(ind) => ind.reset(),
            Self::Sqmom(ind) => ind.reset(),
            Self::Sroll(ind) => ind.reset(),
            Self::Sroll95(ind) => ind.reset(),
            Self::Srollp(ind) => ind.reset(),
            Self::SslChannel(ind) => ind.reset(),
            Self::Sslope(ind) => ind.reset(),
            Self::Sslopep(ind) => ind.reset(),
            Self::Ssloperp(ind) => ind.reset(),
            Self::StandardDeviationChannels(ind) => ind.reset(),
            Self::StarcBands(ind) => ind.reset(),
            Self::Stc(ind) => ind.reset(),
            Self::StdDevChannelWidth(ind) => ind.reset(),
            Self::StochasticRsi(ind) => ind.reset(),
            Self::Stochastics(ind) => ind.reset(),
            Self::StochastikD(ind) => ind.reset(),
            Self::StochDiv(ind) => ind.reset(),
            Self::Supertrend(ind) => ind.reset(),
            Self::Supts(ind) => ind.reset(),
            Self::SweepReversionIndex(ind) => ind.reset(),
            Self::SwingAge(ind) => ind.reset(),
            Self::Swings(ind) => ind.reset(),
            Self::SwingsSoft(ind) => ind.reset(),
            Self::SwingStop(ind) => ind.reset(),
            Self::SwingStrengthScore(ind) => ind.reset(),
            Self::T3(ind) => ind.reset(),
            Self::Tdi(ind) => ind.reset(),
            Self::Tema(ind) => ind.reset(),
            Self::Tenc(ind) => ind.reset(),
            Self::TheilSenChannels(ind) => ind.reset(),
            Self::Threeblackcrows(ind) => ind.reset(),
            Self::Threewhitesoldiers(ind) => ind.reset(),
            Self::Thresh(ind) => ind.reset(),
            Self::TickVolume(ind) => ind.reset(),
            Self::Tii(ind) => ind.reset(),
            Self::TimeEncoders(ind) => ind.reset(),
            Self::Tma(ind) => ind.reset(),
            Self::TransferEntropy(ind) => ind.reset(),
            Self::Trima(ind) => ind.reset(),
            Self::TrimaBands(ind) => ind.reset(),
            Self::Trin(ind) => ind.reset(),
            Self::Trix(ind) => ind.reset(),
            Self::TrueRange(ind) => ind.reset(),
            Self::TrueStrengthIndex(ind) => ind.reset(),
            Self::Tweezer(ind) => ind.reset(),
            Self::TwiggsMoneyFlow(ind) => ind.reset(),
            Self::UlcerIndex(ind) => ind.reset(),
            Self::UltimateOscillator(ind) => ind.reset(),
            Self::UltimateOscillatorSmooth(ind) => ind.reset(),
            Self::UnscentedKalmanFilter(ind) => ind.reset(),
            Self::Var(ind) => ind.reset(),
            Self::VarianceRatio(ind) => ind.reset(),
            Self::VarianceRatioAggregate(ind) => ind.reset(),
            Self::Vbd(ind) => ind.reset(),
            Self::Vdelta(ind) => ind.reset(),
            Self::Vfi(ind) => ind.reset(),
            Self::Vhf(ind) => ind.reset(),
            Self::VhfMa(ind) => ind.reset(),
            Self::Vidya(ind) => ind.reset(),
            Self::VolatilityBreakoutDetector(ind) => ind.reset(),
            Self::VolatilityPercentileRankBands(ind) => ind.reset(),
            Self::VolatilityStop(ind) => ind.reset(),
            Self::VolOfVolPercentile(ind) => ind.reset(),
            Self::VolOfVolPercentileTrend(ind) => ind.reset(),
            Self::VolumeDiv(ind) => ind.reset(),
            Self::VolumeOscillator(ind) => ind.reset(),
            Self::VolumePriceTrend(ind) => ind.reset(),
            Self::VolumeProfile(ind) => ind.reset(),
            Self::VolumeProfileChannels(ind) => ind.reset(),
            Self::VolumeRateOfChange(ind) => ind.reset(),
            Self::VolumeWeightedRsi(ind) => ind.reset(),
            Self::VolumeZscore(ind) => ind.reset(),
            Self::VortexIndicator(ind) => ind.reset(),
            Self::Vov(ind) => ind.reset(),
            Self::Vpin(ind) => ind.reset(),
            Self::Vr(ind) => ind.reset(),
            Self::VrZAgg(ind) => ind.reset(),
            Self::Vwap(ind) => ind.reset(),
            Self::VwapChannels(ind) => ind.reset(),
            Self::VwapChannelWidth(ind) => ind.reset(),
            Self::VwapDistance(ind) => ind.reset(),
            Self::VwapLevels(ind) => ind.reset(),
            Self::Vwma(ind) => ind.reset(),
            Self::Vzo(ind) => ind.reset(),
            Self::Wave(ind) => ind.reset(),
            Self::Wcomp(ind) => ind.reset(),
            Self::Weekday(ind) => ind.reset(),
            Self::WeekMonth(ind) => ind.reset(),
            Self::WeightedComposite(ind) => ind.reset(),
            Self::WickSpike(ind) => ind.reset(),
            Self::WilliamsAd(ind) => ind.reset(),
            Self::WilliamsDiv(ind) => ind.reset(),
            Self::WilliamsR(ind) => ind.reset(),
            Self::Wma(ind) => ind.reset(),
            Self::WoodiePivots(ind) => ind.reset(),
            Self::Wvf(ind) => ind.reset(),
            Self::Xmil(ind) => ind.reset(),
            Self::XorGate(ind) => ind.reset(),
            Self::Za(ind) => ind.reset(),
            Self::Zigzag(ind) => ind.reset(),
            Self::ZigzagAtr(ind) => ind.reset(),
            Self::ZigzagCandle(ind) => ind.reset(),
            Self::ZigzagClassic(ind) => ind.reset(),
            Self::ZigzagLookahead(ind) => ind.reset(),
            Self::ZigzagTime(ind) => ind.reset(),
            Self::ZlSma(ind) => ind.reset(),
        }
    }
}
