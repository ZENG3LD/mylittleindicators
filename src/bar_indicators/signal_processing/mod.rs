//! Signal Processing Module
//! Цифровая обработка сигналов для анализа временных рядов
//! Включает FFT, вейвлеты, фильтры и преобразования

pub mod fft;
pub mod wavelet;
pub mod hilbert;
pub mod butterworth;
pub mod chebyshev;
pub mod savitzky_golay;
pub mod hilbert_dominant_cycle;
pub mod ehlers_super_smoother;

pub use fft::{FastFourierTransform, FrequencyDomain};
pub use wavelet::{WaveletTransform, WaveletType};
pub use hilbert::{HilbertTransform, AnalyticSignal};
pub use butterworth::{ButterworthFilter, FilterType};
pub use chebyshev::{ChebyshevFilter, ChebyshevType, FilterType as ChebyshevFilterType};
pub use savitzky_golay::{SavitzkyGolayFilter, DerivativeOrder};
pub use hilbert_dominant_cycle::*;
pub use ehlers_super_smoother::{EhlersSuperSmoother, SuperSmootherResult};

// signal_processing: Advanced Signal Processing Indicators
// Современные индикаторы для анализа рыночных сигналов и режимов

pub mod market_regime_filter;
pub mod autocorr;
pub mod cusum_filter;
pub mod cyber_cycle;
pub mod decycler;
pub mod ehlers_sinewave;
pub mod hampel_filter;
pub mod higher_moments;
pub mod hysteresis_gate;
pub mod lempel_ziv;
pub mod logic_gates;
pub mod regime_composite;
pub mod regime_composite_v2;
pub mod regime_composite_v3;
pub mod regime_composite_v4;
pub mod roofing_filter;
pub mod spectral_bandpower;
pub mod spectral_bandpower_ratio_hl;
pub mod spectral_bandwidth_feature;
pub mod spectral_centroid_feature;
pub mod spectral_crest;
pub mod spectral_crest_percentile;
pub mod spectral_energy_ratio;
pub mod spectral_entropy;
pub mod spectral_entropy_of_entropy;
pub mod spectral_entropy_rate;
pub mod spectral_flatness;
pub mod spectral_flatness_percentile;
pub mod spectral_flux_proxy;
pub mod spectral_high_mid_power_ratio;
pub mod spectral_low_mid_power_ratio;
pub mod spectral_rolloff;
pub mod spectral_rolloff_95;
pub mod spectral_rolloff_percentile;
pub mod spectral_rolloff_robust_percentile;
pub mod spectral_slope;
pub mod spectral_slope_percentile;
pub mod spectral_slope_robust_percentile;
pub mod spectral_slope_zscore;
pub mod stft_features;
pub mod threshold_gate;
pub mod time_encoders;
pub mod weighted_composite;
pub mod zscore_price_mad;

// Indicator catalog
pub mod signal_processing_catalog;

// Re-export main types
pub use market_regime_filter::*;
pub use spectral_energy_ratio::*;
pub use spectral_slope::*; 






















