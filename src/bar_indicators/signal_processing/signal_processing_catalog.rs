//! signal_processing_catalog.rs: Auto-generated indicator catalog for signal processing indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 52 signal processing indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::SignalProcessing;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Autocorrelation
pub fn signature_autocorr() -> IndicatorSignature {
    IndicatorSignature::builder("AUTOCORR", CATEGORY)
        .name("Autocorrelation")
        .description("Measures correlation of signal with delayed version of itself")
        .add_constraint(ParamConstraint::period(2, 100, 20))
        .add_constraint(
            ParamConstraint::new("lag", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .metadata("category", "correlation")
        .machine_id(BarIndicatorId::Autocorr)
        // Note: "AUTOCORR" is already the main ID, no need for alias
        .alias("Autocorr")
        .alias("autocorr")
        .alias("AUTOCORRELATION")
        .alias("Autocorrelation")
        .alias("autocorrelation")
        .build()
}

/// Butterworth Filter
pub fn signature_butterworth() -> IndicatorSignature {
    IndicatorSignature::builder("BUTTER", CATEGORY)
        .name("Butterworth Filter")
        .description("Low-pass/high-pass/band-pass digital filter with maximally flat frequency response")
        .add_constraint(ParamConstraint::period(2, 100, 10))
        .add_constraint(
            ParamConstraint::new("order", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .metadata("author", "Stephen Butterworth")
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Butter) // TODO: Add to enum
        // Note: "BUTTER" is already the main ID, no need for alias
        .alias("Butter")
        .alias("butter")
        .alias("BUTTERWORTHFILTER")
        .alias("ButterworthFilter")
        .alias("butterworthfilter")
        .alias("butterworth_filter")
        .alias("BUTTERWORTH_FILTER")
        .alias("Butterworth_Filter")
        .build()
}

/// Chebyshev Filter
pub fn signature_chebyshev() -> IndicatorSignature {
    IndicatorSignature::builder("CHEBY", CATEGORY)
        .name("Chebyshev Filter")
        .description("Digital filter with steeper roll-off than Butterworth")
        .add_constraint(ParamConstraint::period(2, 100, 10))
        .add_constraint(
            ParamConstraint::new("order", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("ripple", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(0.5))
                .required()
        )
        .metadata("author", "Pafnuty Chebyshev")
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Cheby) // TODO: Add to enum
        // Note: "CHEBY" is already the main ID, no need for alias
        .alias("Cheby")
        .alias("cheby")
        .alias("CHEBYSHEVFILTER")
        .alias("ChebyshevFilter")
        .alias("chebyshevfilter")
        .alias("chebyshev_filter")
        .alias("CHEBYSHEV_FILTER")
        .alias("Chebyshev_Filter")
        .build()
}

/// CUSUM Filter
pub fn signature_cusum_filter() -> IndicatorSignature {
    IndicatorSignature::builder("CUSUM", CATEGORY)
        .name("CUSUM Filter")
        .description("Cumulative sum filter for detecting changes in signal")
        .add_constraint(
            ParamConstraint::new("threshold", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .metadata("category", "change_detection")
        .machine_id(BarIndicatorId::Cusum) // TODO: Add to enum
        // Note: "CUSUM" is already the main ID, no need for alias
        .alias("Cusum")
        .alias("cusum")
        .alias("CUSUMFILTER")
        .alias("CUSUMFilter")
        .alias("cusumfilter")
        .alias("cusum_filter")
        .alias("CUSUM_FILTER")
        .alias("Cusum_Filter")
        .build()
}

/// Cyber Cycle
pub fn signature_cyber_cycle() -> IndicatorSignature {
    IndicatorSignature::builder("CYBER", CATEGORY)
        .name("Cyber Cycle")
        .description("Ehlers Cyber Cycle indicator for cycle detection")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("author", "John Ehlers")
        .metadata("category", "cycle")
        .machine_id(BarIndicatorId::Cyber) // TODO: Add to enum
        // Note: "CYBER" is already the main ID, no need for alias
        .alias("Cyber")
        .alias("cyber")
        .alias("CYBERCYCLE")
        .alias("CyberCycle")
        .alias("cybercycle")
        .alias("cyber_cycle")
        .alias("CYBER_CYCLE")
        .alias("Cyber_Cycle")
        .build()
}

/// Decycler
pub fn signature_decycler() -> IndicatorSignature {
    IndicatorSignature::builder("DECYC", CATEGORY)
        .name("Decycler")
        .description("Ehlers Decycler - removes cyclic components")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("author", "John Ehlers")
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Decyc) // TODO: Add to enum
        // Note: "DECYC" is already the main ID, no need for alias
        .alias("Decyc")
        .alias("decyc")
        .alias("DECYCLER")
        .alias("Decycler")
        .alias("decycler")
        .build()
}

/// Ehlers Sinewave
pub fn signature_ehlers_sinewave() -> IndicatorSignature {
    IndicatorSignature::builder("ESINE", CATEGORY)
        .name("Ehlers Sinewave")
        .description("Ehlers Sinewave indicator for cycle analysis")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("author", "John Ehlers")
        .metadata("category", "cycle")
        .machine_id(BarIndicatorId::Esine) // TODO: Add to enum
        // Note: "ESINE" is already the main ID, no need for alias
        .alias("Esine")
        .alias("esine")
        .alias("EHLERSSINEWAVE")
        .alias("EhlersSinewave")
        .alias("ehlerssinewave")
        .alias("ehlers_sinewave")
        .alias("EHLERS_SINEWAVE")
        .alias("Ehlers_Sinewave")
        .build()
}

/// Ehlers Super Smoother
pub fn signature_ehlers_super_smoother() -> IndicatorSignature {
    IndicatorSignature::builder("ESS", CATEGORY)
        .name("Ehlers Super Smoother")
        .description("Zero-lag low-pass filter by John Ehlers")
        .add_constraint(ParamConstraint::period(2, 100, 10))
        .metadata("author", "John Ehlers")
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Ess) // TODO: Add to enum
        // Note: "ESS" is already the main ID, no need for alias
        .alias("Ess")
        .alias("ess")
        .alias("EHLERSSUPERSMOOTHER")
        .alias("EhlersSuperSmoother")
        .alias("ehlerssupersmoother")
        .alias("ehlers_super_smoother")
        .alias("EHLERS_SUPER_SMOOTHER")
        .alias("Ehlers_Super_Smoother")
        .build()
}

/// Fast Fourier Transform
pub fn signature_fft() -> IndicatorSignature {
    IndicatorSignature::builder("FFT", CATEGORY)
        .name("Fast Fourier Transform")
        .description("Frequency domain analysis of price data")
        .add_constraint(ParamConstraint::period(8, 256, 64))
        .metadata("category", "transform")
        .metadata("complexity", "O(n log n)")
        .machine_id(BarIndicatorId::Fft) // TODO: Add to enum
        // Note: "FFT" is already the main ID, no need for alias
        .alias("Fft")
        .alias("fft")
        .alias("FASTFOURIERTRANSFORM")
        .alias("FastFourierTransform")
        .alias("fastfouriertransform")
        .alias("fast_fourier_transform")
        .alias("FAST_FOURIER_TRANSFORM")
        .alias("Fast_Fourier_Transform")
        .build()
}

/// Hampel Filter
pub fn signature_hampel_filter() -> IndicatorSignature {
    IndicatorSignature::builder("HAMPEL", CATEGORY)
        .name("Hampel Filter")
        .description("Robust outlier detection and filtering")
        .add_constraint(ParamConstraint::period(3, 50, 7))
        .add_constraint(
            ParamConstraint::new("n_sigma", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(3.0))
                .required()
        )
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Hampel) // TODO: Add to enum
        // Note: "HAMPEL" is already the main ID, no need for alias
        .alias("Hampel")
        .alias("hampel")
        .alias("HAMPELFILTER")
        .alias("HampelFilter")
        .alias("hampelfilter")
        .alias("hampel_filter")
        .alias("HAMPEL_FILTER")
        .alias("Hampel_Filter")
        .build()
}

/// Higher Moments
pub fn signature_higher_moments() -> IndicatorSignature {
    IndicatorSignature::builder("HMOM", CATEGORY)
        .name("Higher Moments")
        .description("Skewness and kurtosis of price distribution")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .metadata("category", "statistical")
        .machine_id(BarIndicatorId::Hmom) // TODO: Add to enum
        // Note: "HMOM" is already the main ID, no need for alias
        .alias("Hmom")
        .alias("hmom")
        .alias("HIGHERMOMENTS")
        .alias("HigherMoments")
        .alias("highermoments")
        .alias("higher_moments")
        .alias("HIGHER_MOMENTS")
        .alias("Higher_Moments")
        .build()
}

/// Hilbert Transform
pub fn signature_hilbert() -> IndicatorSignature {
    IndicatorSignature::builder("HILB", CATEGORY)
        .name("Hilbert Transform")
        .description("Analytic signal decomposition for phase and amplitude")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("category", "transform")
        .machine_id(BarIndicatorId::Hilb) // TODO: Add to enum
        // Note: "HILB" is already the main ID, no need for alias
        .alias("Hilb")
        .alias("hilb")
        .alias("HILBERTTRANSFORM")
        .alias("HilbertTransform")
        .alias("hilberttransform")
        .alias("hilbert_transform")
        .alias("HILBERT_TRANSFORM")
        .alias("Hilbert_Transform")
        .build()
}

/// Hilbert Dominant Cycle
pub fn signature_hilbert_dominant_cycle() -> IndicatorSignature {
    IndicatorSignature::builder("HDC", CATEGORY)
        .name("Hilbert Dominant Cycle")
        .description("Ehlers dominant cycle period detector using Hilbert Transform")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("author", "John Ehlers")
        .metadata("category", "cycle")
        .machine_id(BarIndicatorId::Hdc) // TODO: Add to enum
        // Note: "HDC" is already the main ID, no need for alias
        .alias("Hdc")
        .alias("hdc")
        .alias("HILBERTDOMINANTCYCLE")
        .alias("HilbertDominantCycle")
        .alias("hilbertdominantcycle")
        .alias("hilbert_dominant_cycle")
        .alias("HILBERT_DOMINANT_CYCLE")
        .alias("Hilbert_Dominant_Cycle")
        .build()
}

/// Hysteresis Gate
pub fn signature_hysteresis_gate() -> IndicatorSignature {
    IndicatorSignature::builder("HYST", CATEGORY)
        .name("Hysteresis Gate")
        .description("Binary signal with hysteresis to reduce noise")
        .add_constraint(
            ParamConstraint::new("upper_threshold", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(1.0))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("lower_threshold", ParamType::F64)
                .with_min(ParamValue::F64(-5.0))
                .with_max(ParamValue::F64(-0.1))
                .with_default(ParamValue::F64(-1.0))
                .required()
        )
        .metadata("category", "gate")
        .machine_id(BarIndicatorId::Hyst) // TODO: Add to enum
        // Note: "HYST" is already the main ID, no need for alias
        .alias("Hyst")
        .alias("hyst")
        .alias("HYSTERESISGATE")
        .alias("HysteresisGate")
        .alias("hysteresisgate")
        .alias("hysteresis_gate")
        .alias("HYSTERESIS_GATE")
        .alias("Hysteresis_Gate")
        .build()
}

/// Lempel-Ziv Complexity
pub fn signature_lempel_ziv() -> IndicatorSignature {
    IndicatorSignature::builder("LZ", CATEGORY)
        .name("Lempel-Ziv Complexity")
        .description("Measures signal complexity and randomness")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .metadata("category", "complexity")
        .machine_id(BarIndicatorId::Lz) // TODO: Add to enum
        // Note: "LZ" is already the main ID, no need for alias
        .alias("Lz")
        .alias("lz")
        .alias("LEMPELZIVCOMPLEXITY")
        .alias("LempelZivComplexity")
        .alias("lempelzivcomplexity")
        .alias("lempel_ziv_complexity")
        .alias("LEMPEL_ZIV_COMPLEXITY")
        .alias("Lempel_Ziv_Complexity")
        .build()
}

/// AND Logic Gate
pub fn signature_and_gate() -> IndicatorSignature {
    IndicatorSignature::builder("LOGICAND", CATEGORY)
        .name("AND Logic Gate")
        .description("Boolean AND operation: outputs true only if both inputs are true")
        .metadata("category", "gate")
        .metadata("operation", "AND")
        .machine_id(BarIndicatorId::Logicand)
        // Note: "LOGICAND" is already the main ID, no need for alias
        .alias("Logicand")
        .alias("logicand")
        .alias("ANDGATE")
        .alias("AndGate")
        .alias("andgate")
        .alias("and_gate")
        .alias("AND_GATE")
        .build()
}

/// OR Logic Gate
pub fn signature_or_gate() -> IndicatorSignature {
    IndicatorSignature::builder("LOGICOR", CATEGORY)
        .name("OR Logic Gate")
        .description("Boolean OR operation: outputs true if at least one input is true")
        .metadata("category", "gate")
        .metadata("operation", "OR")
        .machine_id(BarIndicatorId::Logicor)
        // Note: "LOGICOR" is already the main ID, no need for alias
        .alias("Logicor")
        .alias("logicor")
        .alias("ORGATE")
        .alias("OrGate")
        .alias("orgate")
        .alias("or_gate")
        .alias("OR_GATE")
        .build()
}

/// XOR Logic Gate
pub fn signature_xor_gate() -> IndicatorSignature {
    IndicatorSignature::builder("LOGICXOR", CATEGORY)
        .name("XOR Logic Gate")
        .description("Boolean XOR operation: outputs true if exactly one input is true")
        .metadata("category", "gate")
        .metadata("operation", "XOR")
        .machine_id(BarIndicatorId::Logicxor)
        // Note: "LOGICXOR" is already the main ID, no need for alias
        .alias("Logicxor")
        .alias("logicxor")
        .alias("XORGATE")
        .alias("XorGate")
        .alias("xorgate")
        .alias("xor_gate")
        .alias("XOR_GATE")
        .build()
}

/// Sign Combiner Logic Gate
pub fn signature_sign_combiner() -> IndicatorSignature {
    IndicatorSignature::builder("LOGICSIGN", CATEGORY)
        .name("Sign Combiner")
        .description("Combines two signals {-1,0,1} and clamps result to {-1,0,1}")
        .metadata("category", "gate")
        .metadata("operation", "SignCombiner")
        .machine_id(BarIndicatorId::Logicsign)
        // Note: "LOGICSIGN" is already the main ID, no need for alias
        .alias("Logicsign")
        .alias("logicsign")
        .alias("SIGNCOMBINER")
        .alias("SignCombiner")
        .alias("signcombiner")
        .alias("sign_combiner")
        .alias("SIGN_COMBINER")
        .build()
}

/// Market Regime Filter
pub fn signature_market_regime_filter() -> IndicatorSignature {
    IndicatorSignature::builder("MRF", CATEGORY)
        .name("Market Regime Filter")
        .description("Adaptive filter for detecting market regimes")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .metadata("category", "regime")
        .machine_id(BarIndicatorId::Mrf) // TODO: Add to enum
        // Note: "MRF" is already the main ID, no need for alias
        .alias("Mrf")
        .alias("mrf")
        .alias("MARKETREGIMEFILTER")
        .alias("MarketRegimeFilter")
        .alias("marketregimefilter")
        .alias("market_regime_filter")
        .alias("MARKET_REGIME_FILTER")
        .alias("Market_Regime_Filter")
        .build()
}

/// Regime Composite v1
pub fn signature_regime_composite() -> IndicatorSignature {
    IndicatorSignature::builder("RC", CATEGORY)
        .name("Regime Composite")
        .description("Composite indicator for regime detection")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .metadata("category", "regime")
        .machine_id(BarIndicatorId::Rc) // TODO: Add to enum
        // Note: "RC" is already the main ID, no need for alias
        .alias("Rc")
        .alias("rc")
        .alias("REGIMECOMPOSITE")
        .alias("RegimeComposite")
        .alias("regimecomposite")
        .alias("regime_composite")
        .alias("REGIME_COMPOSITE")
        .alias("Regime_Composite")
        .build()
}

/// Regime Composite v2
pub fn signature_regime_composite_v2() -> IndicatorSignature {
    IndicatorSignature::builder("RC2", CATEGORY)
        .name("Regime Composite v2")
        .description("Enhanced composite indicator for regime detection")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .metadata("category", "regime")
        .metadata("note", "Uses ATR with configurable MA type (default Wilder)")
        .machine_id(BarIndicatorId::Rc2) // TODO: Add to enum
        // Note: "RC2" is already the main ID, no need for alias
        .alias("Rc2")
        .alias("rc2")
        .alias("REGIMECOMPOSITEV2")
        .alias("RegimeCompositev2")
        .alias("regimecompositev2")
        .alias("regime_composite_v2")
        .alias("REGIME_COMPOSITE_V2")
        .alias("Regime_Composite_V2")
        .build()
}

/// Regime Composite v3
pub fn signature_regime_composite_v3() -> IndicatorSignature {
    IndicatorSignature::builder("RC3", CATEGORY)
        .name("Regime Composite v3")
        .description("Advanced composite indicator for regime detection")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .metadata("category", "regime")
        .machine_id(BarIndicatorId::Rc3) // TODO: Add to enum
        // Note: "RC3" is already the main ID, no need for alias
        .alias("Rc3")
        .alias("rc3")
        .alias("REGIMECOMPOSITEV3")
        .alias("RegimeCompositev3")
        .alias("regimecompositev3")
        .alias("regime_composite_v3")
        .alias("REGIME_COMPOSITE_V3")
        .alias("Regime_Composite_V3")
        .build()
}

/// Regime Composite v4
pub fn signature_regime_composite_v4() -> IndicatorSignature {
    IndicatorSignature::builder("RC4", CATEGORY)
        .name("Regime Composite v4")
        .description("Latest composite indicator for regime detection")
        .add_constraint(ParamConstraint::period(10, 100, 20))
        .metadata("category", "regime")
        .machine_id(BarIndicatorId::Rc4) // TODO: Add to enum
        // Note: "RC4" is already the main ID, no need for alias
        .alias("Rc4")
        .alias("rc4")
        .alias("REGIMECOMPOSITEV4")
        .alias("RegimeCompositev4")
        .alias("regimecompositev4")
        .alias("regime_composite_v4")
        .alias("REGIME_COMPOSITE_V4")
        .alias("Regime_Composite_V4")
        .build()
}

/// Roofing Filter
pub fn signature_roofing_filter() -> IndicatorSignature {
    IndicatorSignature::builder("ROOF", CATEGORY)
        .name("Roofing Filter")
        .description("Ehlers Roofing Filter - bandpass filter for cycles")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("author", "John Ehlers")
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Roof) // TODO: Add to enum
        // Note: "ROOF" is already the main ID, no need for alias
        .alias("Roof")
        .alias("roof")
        .alias("ROOFINGFILTER")
        .alias("RoofingFilter")
        .alias("roofingfilter")
        .alias("roofing_filter")
        .alias("ROOFING_FILTER")
        .alias("Roofing_Filter")
        .build()
}

/// Savitzky-Golay Filter
pub fn signature_savitzky_golay() -> IndicatorSignature {
    IndicatorSignature::builder("SG", CATEGORY)
        .name("Savitzky-Golay Filter")
        .description("Polynomial smoothing filter preserving signal features")
        .add_constraint(ParamConstraint::period(5, 51, 11))
        .add_constraint(
            ParamConstraint::new("poly_order", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(6))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .metadata("author", "Abraham Savitzky and Marcel J.E. Golay")
        .metadata("category", "filter")
        .machine_id(BarIndicatorId::Sg) // TODO: Add to enum
        // Note: "SG" is already the main ID, no need for alias
        .alias("Sg")
        .alias("sg")
        .alias("SAVITZKYGOLAYFILTER")
        .alias("SavitzkyGolayFilter")
        .alias("savitzkygolayfilter")
        .alias("savitzky_golay_filter")
        .alias("SAVITZKY_GOLAY_FILTER")
        .alias("Savitzky_Golay_Filter")
        .build()
}

/// Spectral Bandpower
pub fn signature_spectral_bandpower() -> IndicatorSignature {
    IndicatorSignature::builder("SBP", CATEGORY)
        .name("Spectral Bandpower")
        .description("Power in specific frequency bands")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sbp) // TODO: Add to enum
        // Note: "SBP" is already the main ID, no need for alias
        .alias("Sbp")
        .alias("sbp")
        .alias("SPECTRALBANDPOWER")
        .alias("SpectralBandpower")
        .alias("spectralbandpower")
        .alias("spectral_bandpower")
        .alias("SPECTRAL_BANDPOWER")
        .alias("Spectral_Bandpower")
        .build()
}

/// Spectral Bandpower Ratio HL
pub fn signature_spectral_bandpower_ratio_hl() -> IndicatorSignature {
    IndicatorSignature::builder("SBPRHL", CATEGORY)
        .name("Spectral Bandpower Ratio HL")
        .description("Ratio of high to low frequency bandpower")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sbprhl) // TODO: Add to enum
        // Note: "SBPRHL" is already the main ID, no need for alias
        .alias("Sbprhl")
        .alias("sbprhl")
        .alias("SPECTRALBANDPOWERRATIOHL")
        .alias("SpectralBandpowerRatioHL")
        .alias("spectralbandpowerratiohl")
        .alias("spectral_bandpower_ratio_hl")
        .alias("SPECTRAL_BANDPOWER_RATIO_HL")
        .alias("Spectral_Bandpower_Ratio_Hl")
        .build()
}

/// Spectral Bandwidth Feature
pub fn signature_spectral_bandwidth_feature() -> IndicatorSignature {
    IndicatorSignature::builder("SBWF", CATEGORY)
        .name("Spectral Bandwidth Feature")
        .description("Bandwidth of frequency spectrum")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sbwf) // TODO: Add to enum
        // Note: "SBWF" is already the main ID, no need for alias
        .alias("Sbwf")
        .alias("sbwf")
        .alias("SPECTRALBANDWIDTHFEATURE")
        .alias("SpectralBandwidthFeature")
        .alias("spectralbandwidthfeature")
        .alias("spectral_bandwidth_feature")
        .alias("SPECTRAL_BANDWIDTH_FEATURE")
        .alias("Spectral_Bandwidth_Feature")
        .build()
}

/// Spectral Centroid Feature
pub fn signature_spectral_centroid_feature() -> IndicatorSignature {
    IndicatorSignature::builder("SCF", CATEGORY)
        .name("Spectral Centroid Feature")
        .description("Center of mass of frequency spectrum")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Scf) // TODO: Add to enum
        // Note: "SCF" is already the main ID, no need for alias
        .alias("Scf")
        .alias("scf")
        .alias("SPECTRALCENTROIDFEATURE")
        .alias("SpectralCentroidFeature")
        .alias("spectralcentroidfeature")
        .alias("spectral_centroid_feature")
        .alias("SPECTRAL_CENTROID_FEATURE")
        .alias("Spectral_Centroid_Feature")
        .build()
}

/// Spectral Crest
pub fn signature_spectral_crest() -> IndicatorSignature {
    IndicatorSignature::builder("SCREST", CATEGORY)
        .name("Spectral Crest")
        .description("Peak-to-average ratio in frequency domain")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Screst) // TODO: Add to enum
        // Note: "SCREST" is already the main ID, no need for alias
        .alias("Screst")
        .alias("screst")
        .alias("SPECTRALCREST")
        .alias("SpectralCrest")
        .alias("spectralcrest")
        .alias("spectral_crest")
        .alias("SPECTRAL_CREST")
        .alias("Spectral_Crest")
        .build()
}

/// Spectral Crest Percentile
pub fn signature_spectral_crest_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("SCRESTP", CATEGORY)
        .name("Spectral Crest Percentile")
        .description("Percentile rank of spectral crest")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Screstp) // TODO: Add to enum
        // Note: "SCRESTP" is already the main ID, no need for alias
        .alias("Screstp")
        .alias("screstp")
        .alias("SPECTRALCRESTPERCENTILE")
        .alias("SpectralCrestPercentile")
        .alias("spectralcrestpercentile")
        .alias("spectral_crest_percentile")
        .alias("SPECTRAL_CREST_PERCENTILE")
        .alias("Spectral_Crest_Percentile")
        .build()
}

/// Spectral Energy Ratio
pub fn signature_spectral_energy_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("SER", CATEGORY)
        .name("Spectral Energy Ratio")
        .description("Ratio of energy in different frequency bands")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Ser) // TODO: Add to enum
        // Note: "SER" is already the main ID, no need for alias
        .alias("Ser")
        .alias("ser")
        .alias("SPECTRALENERGYRATIO")
        .alias("SpectralEnergyRatio")
        .alias("spectralenergyratio")
        .alias("spectral_energy_ratio")
        .alias("SPECTRAL_ENERGY_RATIO")
        .alias("Spectral_Energy_Ratio")
        .build()
}

/// Spectral Entropy
pub fn signature_spectral_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("SENT", CATEGORY)
        .name("Spectral Entropy")
        .description("Entropy of frequency spectrum")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sent) // TODO: Add to enum
        // Note: "SENT" is already the main ID, no need for alias
        .alias("Sent")
        .alias("sent")
        .alias("SPECTRALENTROPY")
        .alias("SpectralEntropy")
        .alias("spectralentropy")
        .alias("spectral_entropy")
        .alias("SPECTRAL_ENTROPY")
        .alias("Spectral_Entropy")
        .build()
}

/// Spectral Entropy of Entropy
pub fn signature_spectral_entropy_of_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("SENTENT", CATEGORY)
        .name("Spectral Entropy of Entropy")
        .description("Second-order entropy of spectrum")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sentent) // TODO: Add to enum
        // Note: "SENTENT" is already the main ID, no need for alias
        .alias("Sentent")
        .alias("sentent")
        .alias("SPECTRALENTROPYOFENTROPY")
        .alias("SpectralEntropyofEntropy")
        .alias("spectralentropyofentropy")
        .alias("spectral_entropy_of_entropy")
        .alias("SPECTRAL_ENTROPY_OF_ENTROPY")
        .alias("Spectral_Entropy_Of_Entropy")
        .build()
}

/// Spectral Entropy Rate
pub fn signature_spectral_entropy_rate() -> IndicatorSignature {
    IndicatorSignature::builder("SENTR", CATEGORY)
        .name("Spectral Entropy Rate")
        .description("Rate of change of spectral entropy")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sentr) // TODO: Add to enum
        // Note: "SENTR" is already the main ID, no need for alias
        .alias("Sentr")
        .alias("sentr")
        .alias("SPECTRALENTROPYRATE")
        .alias("SpectralEntropyRate")
        .alias("spectralentropyrate")
        .alias("spectral_entropy_rate")
        .alias("SPECTRAL_ENTROPY_RATE")
        .alias("Spectral_Entropy_Rate")
        .build()
}

/// Spectral Flatness
pub fn signature_spectral_flatness() -> IndicatorSignature {
    IndicatorSignature::builder("SFLAT", CATEGORY)
        .name("Spectral Flatness")
        .description("Measure of noise-like vs tone-like spectrum")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sflat) // TODO: Add to enum
        // Note: "SFLAT" is already the main ID, no need for alias
        .alias("Sflat")
        .alias("sflat")
        .alias("SPECTRALFLATNESS")
        .alias("SpectralFlatness")
        .alias("spectralflatness")
        .alias("spectral_flatness")
        .alias("SPECTRAL_FLATNESS")
        .alias("Spectral_Flatness")
        .build()
}

/// Spectral Flatness Percentile
pub fn signature_spectral_flatness_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("SFLATP", CATEGORY)
        .name("Spectral Flatness Percentile")
        .description("Percentile rank of spectral flatness")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sflatp) // TODO: Add to enum
        // Note: "SFLATP" is already the main ID, no need for alias
        .alias("Sflatp")
        .alias("sflatp")
        .alias("SPECTRALFLATNESSPERCENTILE")
        .alias("SpectralFlatnessPercentile")
        .alias("spectralflatnesspercentile")
        .alias("spectral_flatness_percentile")
        .alias("SPECTRAL_FLATNESS_PERCENTILE")
        .alias("Spectral_Flatness_Percentile")
        .build()
}

/// Spectral Flux Proxy
pub fn signature_spectral_flux_proxy() -> IndicatorSignature {
    IndicatorSignature::builder("SFLUX", CATEGORY)
        .name("Spectral Flux Proxy")
        .description("Rate of change of spectrum over time")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sflux) // TODO: Add to enum
        // Note: "SFLUX" is already the main ID, no need for alias
        .alias("Sflux")
        .alias("sflux")
        .alias("SPECTRALFLUXPROXY")
        .alias("SpectralFluxProxy")
        .alias("spectralfluxproxy")
        .alias("spectral_flux_proxy")
        .alias("SPECTRAL_FLUX_PROXY")
        .alias("Spectral_Flux_Proxy")
        .build()
}

/// Spectral High-Mid Power Ratio
pub fn signature_spectral_high_mid_power_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("SHMPR", CATEGORY)
        .name("Spectral High-Mid Power Ratio")
        .description("Ratio of high to mid frequency power")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Shmpr) // TODO: Add to enum
        // Note: "SHMPR" is already the main ID, no need for alias
        .alias("Shmpr")
        .alias("shmpr")
        .alias("SPECTRALHIGHMIDPOWERRATIO")
        .alias("SpectralHighMidPowerRatio")
        .alias("spectralhighmidpowerratio")
        .alias("spectral_high_mid_power_ratio")
        .alias("SPECTRAL_HIGH_MID_POWER_RATIO")
        .alias("Spectral_High_Mid_Power_Ratio")
        .build()
}

/// Spectral Low-Mid Power Ratio
pub fn signature_spectral_low_mid_power_ratio() -> IndicatorSignature {
    IndicatorSignature::builder("SLMPR", CATEGORY)
        .name("Spectral Low-Mid Power Ratio")
        .description("Ratio of low to mid frequency power")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Slmpr) // TODO: Add to enum
        // Note: "SLMPR" is already the main ID, no need for alias
        .alias("Slmpr")
        .alias("slmpr")
        .alias("SPECTRALLOWMIDPOWERRATIO")
        .alias("SpectralLowMidPowerRatio")
        .alias("spectrallowmidpowerratio")
        .alias("spectral_low_mid_power_ratio")
        .alias("SPECTRAL_LOW_MID_POWER_RATIO")
        .alias("Spectral_Low_Mid_Power_Ratio")
        .build()
}

/// Spectral Rolloff
pub fn signature_spectral_rolloff() -> IndicatorSignature {
    IndicatorSignature::builder("SROLL", CATEGORY)
        .name("Spectral Rolloff")
        .description("Frequency below which specified percentage of energy lies")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sroll) // TODO: Add to enum
        // Note: "SROLL" is already the main ID, no need for alias
        .alias("Sroll")
        .alias("sroll")
        .alias("SPECTRALROLLOFF")
        .alias("SpectralRolloff")
        .alias("spectralrolloff")
        .alias("spectral_rolloff")
        .alias("SPECTRAL_ROLLOFF")
        .alias("Spectral_Rolloff")
        .build()
}

/// Spectral Rolloff 95%
pub fn signature_spectral_rolloff_95() -> IndicatorSignature {
    IndicatorSignature::builder("SROLL95", CATEGORY)
        .name("Spectral Rolloff 95%")
        .description("Frequency below which 95% of energy lies")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sroll95) // TODO: Add to enum
        // Note: "SROLL95" is already the main ID, no need for alias
        .alias("Sroll95")
        .alias("sroll95")
        .alias("SPECTRALROLLOFF95%")
        .alias("SpectralRolloff95%")
        .alias("spectralrolloff95%")
        .alias("spectral_rolloff_95%")
        .alias("SPECTRAL_ROLLOFF_95%")
        .alias("Spectral_Rolloff_95%")
        .build()
}

/// Spectral Rolloff Percentile
pub fn signature_spectral_rolloff_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("SROLLP", CATEGORY)
        .name("Spectral Rolloff Percentile")
        .description("Percentile rank of spectral rolloff")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Srollp) // TODO: Add to enum
        // Note: "SROLLP" is already the main ID, no need for alias
        .alias("Srollp")
        .alias("srollp")
        .alias("SPECTRALROLLOFFPERCENTILE")
        .alias("SpectralRolloffPercentile")
        .alias("spectralrolloffpercentile")
        .alias("spectral_rolloff_percentile")
        .alias("SPECTRAL_ROLLOFF_PERCENTILE")
        .alias("Spectral_Rolloff_Percentile")
        .build()
}

/// Spectral Rolloff Robust Percentile
pub fn signature_spectral_rolloff_robust_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("SROLLRP", CATEGORY)
        .name("Spectral Rolloff Robust Percentile")
        .description("Robust percentile rank of spectral rolloff")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Srollrp) // TODO: Add to enum
        // Note: "SROLLRP" is already the main ID, no need for alias
        .alias("Srollrp")
        .alias("srollrp")
        .alias("SPECTRALROLLOFFROBUSTPERCENTILE")
        .alias("SpectralRolloffRobustPercentile")
        .alias("spectralrolloffrobustpercentile")
        .alias("spectral_rolloff_robust_percentile")
        .alias("SPECTRAL_ROLLOFF_ROBUST_PERCENTILE")
        .alias("Spectral_Rolloff_Robust_Percentile")
        .build()
}

/// Spectral Slope
pub fn signature_spectral_slope() -> IndicatorSignature {
    IndicatorSignature::builder("SSLOPE", CATEGORY)
        .name("Spectral Slope")
        .description("Linear regression slope of frequency spectrum")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sslope) // TODO: Add to enum
        // Note: "SSLOPE" is already the main ID, no need for alias
        .alias("Sslope")
        .alias("sslope")
        .alias("SPECTRALSLOPE")
        .alias("SpectralSlope")
        .alias("spectralslope")
        .alias("spectral_slope")
        .alias("SPECTRAL_SLOPE")
        .alias("Spectral_Slope")
        .build()
}

/// Spectral Slope Percentile
pub fn signature_spectral_slope_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("SSLOPEP", CATEGORY)
        .name("Spectral Slope Percentile")
        .description("Percentile rank of spectral slope")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sslopep) // TODO: Add to enum
        // Note: "SSLOPEP" is already the main ID, no need for alias
        .alias("Sslopep")
        .alias("sslopep")
        .alias("SPECTRALSLOPEPERCENTILE")
        .alias("SpectralSlopePercentile")
        .alias("spectralslopepercentile")
        .alias("spectral_slope_percentile")
        .alias("SPECTRAL_SLOPE_PERCENTILE")
        .alias("Spectral_Slope_Percentile")
        .build()
}

/// Spectral Slope Robust Percentile
pub fn signature_spectral_slope_robust_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("SSLOPERP", CATEGORY)
        .name("Spectral Slope Robust Percentile")
        .description("Robust percentile rank of spectral slope")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Ssloperp) // TODO: Add to enum
        // Note: "SSLOPERP" is already the main ID, no need for alias
        .alias("Ssloperp")
        .alias("ssloperp")
        .alias("SPECTRALSLOPEROBUSTPERCENTILE")
        .alias("SpectralSlopeRobustPercentile")
        .alias("spectralsloperobustpercentile")
        .alias("spectral_slope_robust_percentile")
        .alias("SPECTRAL_SLOPE_ROBUST_PERCENTILE")
        .alias("Spectral_Slope_Robust_Percentile")
        .build()
}

/// Spectral Slope Z-Score
pub fn signature_spectral_slope_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("SSLOPEZ", CATEGORY)
        .name("Spectral Slope Z-Score")
        .description("Standardized spectral slope")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("lookback", ParamType::USize)
                .with_min(ParamValue::USize(10))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(50))
                .required()
        )
        .metadata("category", "spectral")
        .machine_id(BarIndicatorId::Sslopez) // TODO: Add to enum
        // Note: "SSLOPEZ" is already the main ID, no need for alias
        .alias("Sslopez")
        .alias("sslopez")
        .alias("SPECTRALSLOPEZSCORE")
        .alias("SpectralSlopeZScore")
        .alias("spectralslopezscore")
        .alias("spectral_slope_z_score")
        .alias("SPECTRAL_SLOPE_Z_SCORE")
        .alias("Spectral_Slope_Z_Score")
        .build()
}

/// STFT Features
pub fn signature_stft_features() -> IndicatorSignature {
    IndicatorSignature::builder("STFT", CATEGORY)
        .name("Short-Time Fourier Transform Features")
        .description("Time-frequency analysis features")
        .add_constraint(ParamConstraint::period(16, 256, 64))
        .add_constraint(
            ParamConstraint::new("hop_size", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(64))
                .with_default(ParamValue::USize(8))
                .required()
        )
        .metadata("category", "transform")
        .machine_id(BarIndicatorId::Stft) // TODO: Add to enum
        // Note: "STFT" is already the main ID, no need for alias
        .alias("Stft")
        .alias("stft")
        .alias("SHORTTIMEFOURIERTRANSFORMFEATURES")
        .alias("ShortTimeFourierTransformFeatures")
        .alias("shorttimefouriertransformfeatures")
        .alias("short_time_fourier_transform_features")
        .alias("SHORT_TIME_FOURIER_TRANSFORM_FEATURES")
        .alias("Short_Time_Fourier_Transform_Features")
        .build()
}

/// Threshold Gate
pub fn signature_threshold_gate() -> IndicatorSignature {
    IndicatorSignature::builder("THRESH", CATEGORY)
        .name("Threshold Gate")
        .description("Binary signal based on threshold")
        .add_constraint(
            ParamConstraint::new("threshold", ParamType::F64)
                .with_min(ParamValue::F64(-10.0))
                .with_max(ParamValue::F64(10.0))
                .with_default(ParamValue::F64(0.0))
                .required()
        )
        .metadata("category", "gate")
        .machine_id(BarIndicatorId::Thresh) // TODO: Add to enum
        // Note: "THRESH" is already the main ID, no need for alias
        .alias("Thresh")
        .alias("thresh")
        .alias("THRESHOLDGATE")
        .alias("ThresholdGate")
        .alias("thresholdgate")
        .alias("threshold_gate")
        .alias("THRESHOLD_GATE")
        .alias("Threshold_Gate")
        .build()
}

/// Time Encoders
pub fn signature_time_encoders() -> IndicatorSignature {
    IndicatorSignature::builder("TENC", CATEGORY)
        .name("Time Encoders")
        .description("Cyclic time encoding (hour, day, week)")
        .metadata("category", "encoding")
        .machine_id(BarIndicatorId::Tenc) // TODO: Add to enum
        // Note: "TENC" is already the main ID, no need for alias
        .alias("Tenc")
        .alias("tenc")
        .alias("TIMEENCODERS")
        .alias("TimeEncoders")
        .alias("timeencoders")
        .alias("time_encoders")
        .alias("TIME_ENCODERS")
        .alias("Time_Encoders")
        .build()
}

/// Wavelet Transform
pub fn signature_wavelet() -> IndicatorSignature {
    IndicatorSignature::builder("WAVE", CATEGORY)
        .name("Wavelet Transform")
        .description("Multi-resolution time-frequency analysis")
        .add_constraint(ParamConstraint::period(8, 128, 32))
        .add_constraint(
            ParamConstraint::new("levels", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(8))
                .with_default(ParamValue::USize(4))
                .required()
        )
        .metadata("category", "transform")
        .machine_id(BarIndicatorId::Wave) // TODO: Add to enum
        // Note: "WAVE" is already the main ID, no need for alias
        .alias("Wave")
        .alias("wave")
        .alias("WAVELETTRANSFORM")
        .alias("WaveletTransform")
        .alias("wavelettransform")
        .alias("wavelet_transform")
        .alias("WAVELET_TRANSFORM")
        .alias("Wavelet_Transform")
        .build()
}

/// Weighted Composite
pub fn signature_weighted_composite() -> IndicatorSignature {
    IndicatorSignature::builder("WCOMP", CATEGORY)
        .name("Weighted Composite")
        .description("Weighted combination of multiple signals")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("category", "composite")
        .machine_id(BarIndicatorId::Wcomp) // TODO: Add to enum
        // Note: "WCOMP" is already the main ID, no need for alias
        .alias("Wcomp")
        .alias("wcomp")
        .alias("WEIGHTEDCOMPOSITE")
        .alias("WeightedComposite")
        .alias("weightedcomposite")
        .alias("weighted_composite")
        .alias("WEIGHTED_COMPOSITE")
        .alias("Weighted_Composite")
        .build()
}

/// Z-Score Price MAD
pub fn signature_zscore_price_mad() -> IndicatorSignature {
    IndicatorSignature::builder("ZMAD", CATEGORY)
        .name("Z-Score Price MAD")
        .description("Z-score using median absolute deviation")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .metadata("category", "statistical")
        .machine_id(BarIndicatorId::Zmad) // TODO: Add to enum
        // Note: "ZMAD" is already the main ID, no need for alias
        .alias("Zmad")
        .alias("zmad")
        .alias("ZSCOREPRICEMAD")
        .alias("ZScorePriceMAD")
        .alias("zscorepricemad")
        .alias("z_score_price_mad")
        .alias("Z_SCORE_PRICE_MAD")
        .alias("Z_Score_Price_Mad")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all signal processing indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("AUTOCORR", signature_autocorr as fn() -> IndicatorSignature),
    ("BUTTER", signature_butterworth as fn() -> IndicatorSignature),
    ("CHEBY", signature_chebyshev as fn() -> IndicatorSignature),
    ("CUSUM", signature_cusum_filter as fn() -> IndicatorSignature),
    ("CYBER", signature_cyber_cycle as fn() -> IndicatorSignature),
    ("DECYC", signature_decycler as fn() -> IndicatorSignature),
    ("ESINE", signature_ehlers_sinewave as fn() -> IndicatorSignature),
    ("ESS", signature_ehlers_super_smoother as fn() -> IndicatorSignature),
    ("FFT", signature_fft as fn() -> IndicatorSignature),
    ("HAMPEL", signature_hampel_filter as fn() -> IndicatorSignature),
    ("HMOM", signature_higher_moments as fn() -> IndicatorSignature),
    ("HILB", signature_hilbert as fn() -> IndicatorSignature),
    ("HDC", signature_hilbert_dominant_cycle as fn() -> IndicatorSignature),
    ("HYST", signature_hysteresis_gate as fn() -> IndicatorSignature),
    ("LZ", signature_lempel_ziv as fn() -> IndicatorSignature),
    ("LOGICAND", signature_and_gate as fn() -> IndicatorSignature),
    ("LOGICOR", signature_or_gate as fn() -> IndicatorSignature),
    ("LOGICXOR", signature_xor_gate as fn() -> IndicatorSignature),
    ("LOGICSIGN", signature_sign_combiner as fn() -> IndicatorSignature),
    ("MRF", signature_market_regime_filter as fn() -> IndicatorSignature),
    ("RC", signature_regime_composite as fn() -> IndicatorSignature),
    ("RC2", signature_regime_composite_v2 as fn() -> IndicatorSignature),
    ("RC3", signature_regime_composite_v3 as fn() -> IndicatorSignature),
    ("RC4", signature_regime_composite_v4 as fn() -> IndicatorSignature),
    ("ROOF", signature_roofing_filter as fn() -> IndicatorSignature),
    ("SG", signature_savitzky_golay as fn() -> IndicatorSignature),
    ("SBP", signature_spectral_bandpower as fn() -> IndicatorSignature),
    ("SBPRHL", signature_spectral_bandpower_ratio_hl as fn() -> IndicatorSignature),
    ("SBWF", signature_spectral_bandwidth_feature as fn() -> IndicatorSignature),
    ("SCF", signature_spectral_centroid_feature as fn() -> IndicatorSignature),
    ("SCREST", signature_spectral_crest as fn() -> IndicatorSignature),
    ("SCRESTP", signature_spectral_crest_percentile as fn() -> IndicatorSignature),
    ("SER", signature_spectral_energy_ratio as fn() -> IndicatorSignature),
    ("SENT", signature_spectral_entropy as fn() -> IndicatorSignature),
    ("SENTENT", signature_spectral_entropy_of_entropy as fn() -> IndicatorSignature),
    ("SENTR", signature_spectral_entropy_rate as fn() -> IndicatorSignature),
    ("SFLAT", signature_spectral_flatness as fn() -> IndicatorSignature),
    ("SFLATP", signature_spectral_flatness_percentile as fn() -> IndicatorSignature),
    ("SFLUX", signature_spectral_flux_proxy as fn() -> IndicatorSignature),
    ("SHMPR", signature_spectral_high_mid_power_ratio as fn() -> IndicatorSignature),
    ("SLMPR", signature_spectral_low_mid_power_ratio as fn() -> IndicatorSignature),
    ("SROLL", signature_spectral_rolloff as fn() -> IndicatorSignature),
    ("SROLL95", signature_spectral_rolloff_95 as fn() -> IndicatorSignature),
    ("SROLLP", signature_spectral_rolloff_percentile as fn() -> IndicatorSignature),
    ("SROLLRP", signature_spectral_rolloff_robust_percentile as fn() -> IndicatorSignature),
    ("SSLOPE", signature_spectral_slope as fn() -> IndicatorSignature),
    ("SSLOPEP", signature_spectral_slope_percentile as fn() -> IndicatorSignature),
    ("SSLOPERP", signature_spectral_slope_robust_percentile as fn() -> IndicatorSignature),
    ("SSLOPEZ", signature_spectral_slope_zscore as fn() -> IndicatorSignature),
    ("STFT", signature_stft_features as fn() -> IndicatorSignature),
    ("THRESH", signature_threshold_gate as fn() -> IndicatorSignature),
    ("TENC", signature_time_encoders as fn() -> IndicatorSignature),
    ("WAVE", signature_wavelet as fn() -> IndicatorSignature),
    ("WCOMP", signature_weighted_composite as fn() -> IndicatorSignature),
    ("ZMAD", signature_zscore_price_mad as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static SIGNAL_PROCESSING_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();

    for &(main_id, func) in BASE_CATALOG {
        // Call function once to get signature with aliases
        let sig = func();

        // Insert main ID
        m.insert(main_id.to_string(), func);

        // Auto-insert all aliases from signature
        for alias in &sig.aliases {
            m.insert(alias.clone(), func);
        }
    }

    m
});

// ============================================================================
// Public API
// ============================================================================

/// Get indicator signature by ID
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    SIGNAL_PROCESSING_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators
pub fn count() -> usize {
    BASE_CATALOG.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_fft_signature() {
        let sig = get_signature("FFT").unwrap();
        assert_eq!(sig.id, "FFT");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_butterworth_signature() {
        let sig = get_signature("BUTTER").unwrap();
        assert_eq!(sig.id, "BUTTER");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }

    #[test]
    fn test_count() {
        assert_eq!(count(), 55);
    }
}
