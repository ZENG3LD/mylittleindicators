//! Rendering catalog for all indicators
//!
//! Contains rendering metadata for all 480+ indicators in the catalog.
//! Each indicator has rendering specifications: overlay/sub-pane, output types,
//! colors, bounds, reference lines, etc.

use super::rendering::{
    RenderingMetadata, OutputSpec, ReferenceLine,
    LineStyle, HistogramStyle, ValueExtractor, ChannelPart, MacdPart, IchimokuPart,
    DoublePart, TriplePart, HilbertPart,
    CandleAnatomyPart,
};
use crate::BarIndicatorId;
use once_cell::sync::Lazy;
use std::collections::HashMap;

// ============================================================================
// Color Constants
// ============================================================================

// Primary colors
const COLOR_BLUE: &str = "#2196F3";
const COLOR_RED: &str = "#F44336";
const COLOR_GREEN: &str = "#4CAF50";
const COLOR_ORANGE: &str = "#FF9800";
const COLOR_PURPLE: &str = "#9C27B0";
const COLOR_CYAN: &str = "#00BCD4";
const COLOR_PINK: &str = "#E91E63";
const COLOR_TEAL: &str = "#009688";
const COLOR_INDIGO: &str = "#3F51B5";

// Secondary colors
const COLOR_DEEP_ORANGE: &str = "#FF5722";
const COLOR_LIME: &str = "#CDDC39";
const COLOR_AMBER: &str = "#FFC107";
const COLOR_GRAY: &str = "#9E9E9E";
const COLOR_BLUE_GRAY: &str = "#607D8B";

// Reference line colors
const COLOR_OVERBOUGHT: &str = "#FF5722";
const COLOR_OVERSOLD: &str = "#4CAF50";
const COLOR_ZERO_LINE: &str = "#9E9E9E";

// Channel colors (with transparency for fills)
const COLOR_CHANNEL_UPPER: &str = "#F44336";
const COLOR_CHANNEL_MIDDLE: &str = "#607D8B";
const COLOR_CHANNEL_LOWER: &str = "#4CAF50";
const COLOR_CHANNEL_FILL: &str = "#607D8B40";

// ============================================================================
// Rendering Metadata Catalog
// ============================================================================

/// Static catalog of all rendering metadata
pub static RENDERING_CATALOG: Lazy<HashMap<BarIndicatorId, RenderingMetadata>> = Lazy::new(|| {
    let mut catalog = HashMap::new();

    // ========================================================================
    // AVERAGE INDICATORS (22) - All overlay
    // ========================================================================
    register_average_indicators(&mut catalog);

    // ========================================================================
    // MOMENTUM INDICATORS (90+) - Mostly sub-pane
    // ========================================================================
    register_momentum_indicators(&mut catalog);

    // ========================================================================
    // CHANNEL INDICATORS (42) - Mostly overlay
    // ========================================================================
    register_channel_indicators(&mut catalog);

    // ========================================================================
    // VOLATILITY INDICATORS (30+) - Mostly sub-pane
    // ========================================================================
    register_volatility_indicators(&mut catalog);

    // ========================================================================
    // VOLUME INDICATORS (25+) - Sub-pane
    // ========================================================================
    register_volume_indicators(&mut catalog);

    // ========================================================================
    // TREND INDICATORS (30+) - Mixed overlay/sub-pane
    // ========================================================================
    register_trend_indicators(&mut catalog);

    // ========================================================================
    // LEVELS INDICATORS (15+) - Overlay
    // ========================================================================
    register_levels_indicators(&mut catalog);

    // ========================================================================
    // ENTROPY INDICATORS (10+) - Sub-pane
    // ========================================================================
    register_entropy_indicators(&mut catalog);

    // ========================================================================
    // KALMAN INDICATORS (10+) - Overlay
    // ========================================================================
    register_kalman_indicators(&mut catalog);

    // ========================================================================
    // SIGNAL PROCESSING (20+) - Sub-pane
    // ========================================================================
    register_signal_processing_indicators(&mut catalog);

    // ========================================================================
    // CHAOS INDICATORS (10+) - Sub-pane
    // ========================================================================
    register_chaos_indicators(&mut catalog);

    // ========================================================================
    // REGRESSION INDICATORS (10+) - Overlay
    // ========================================================================
    register_regression_indicators(&mut catalog);

    // ========================================================================
    // ADAPTIVE INDICATORS (15+) - Overlay
    // ========================================================================
    register_adaptive_indicators(&mut catalog);

    // ========================================================================
    // ACCUMULATION INDICATORS (10+) - Sub-pane
    // ========================================================================
    register_accumulation_indicators(&mut catalog);

    // ========================================================================
    // BOOK INDICATORS (5+) - Sub-pane
    // ========================================================================
    register_book_indicators(&mut catalog);

    // ========================================================================
    // CANDLE INDICATORS (15+) - Mixed
    // ========================================================================
    register_candle_indicators(&mut catalog);

    // ========================================================================
    // CLUSTER INDICATORS (10+) - Sub-pane
    // ========================================================================
    register_cluster_indicators(&mut catalog);

    // ========================================================================
    // DIVERGENCE INDICATORS (10+) - Sub-pane
    // ========================================================================
    register_divergence_indicators(&mut catalog);

    // ========================================================================
    // RATIO INDICATORS (10+) - Sub-pane
    // ========================================================================
    register_ratio_indicators(&mut catalog);

    // ========================================================================
    // TREND STOP INDICATORS (10+) - Overlay
    // ========================================================================
    register_trend_stop_indicators(&mut catalog);

    // ========================================================================
    // POSITION INDICATORS (5+) - Sub-pane
    // ========================================================================
    register_position_indicators(&mut catalog);

    // ========================================================================
    // STATISTICS INDICATORS (20+) - Sub-pane
    // ========================================================================
    register_statistics_indicators(&mut catalog);

    // ========================================================================
    // ZIGZAG INDICATORS (5) - Overlay
    // ========================================================================
    register_zigzag_indicators(&mut catalog);

    // ========================================================================
    // MISSING INDICATORS (catch-all for any remaining)
    // ========================================================================
    register_missing_indicators(&mut catalog);

    catalog
});

// ============================================================================
// AVERAGE INDICATORS
// ============================================================================

fn register_average_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Simple Moving Average
    catalog.insert(BarIndicatorId::Sma,
        RenderingMetadata::builder("SMA")
            .overlay()
            .line_output("sma", "SMA", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Exponential Moving Average
    catalog.insert(BarIndicatorId::Ema,
        RenderingMetadata::builder("EMA")
            .overlay()
            .line_output("ema", "EMA", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Weighted Moving Average
    catalog.insert(BarIndicatorId::Wma,
        RenderingMetadata::builder("WMA")
            .overlay()
            .line_output("wma", "WMA", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Hull Moving Average
    catalog.insert(BarIndicatorId::Hma,
        RenderingMetadata::builder("HMA")
            .overlay()
            .line_output("hma", "HMA", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Double EMA
    catalog.insert(BarIndicatorId::Dema,
        RenderingMetadata::builder("DEMA")
            .overlay()
            .line_output("dema", "DEMA", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Triple EMA
    catalog.insert(BarIndicatorId::Tema,
        RenderingMetadata::builder("TEMA")
            .overlay()
            .line_output("tema", "TEMA", COLOR_INDIGO)
            .precision(4)
            .build()
    );

    // Running Moving Average (Wilder's)
    catalog.insert(BarIndicatorId::Rma,
        RenderingMetadata::builder("RMA")
            .overlay()
            .line_output("rma", "RMA", COLOR_PINK)
            .precision(4)
            .build()
    );

    // VWAP
    catalog.insert(BarIndicatorId::Vwap,
        RenderingMetadata::builder("VWAP")
            .overlay()
            .line_output("vwap", "VWAP", COLOR_DEEP_ORANGE)
            .precision(4)
            .build()
    );

    // Triangular Moving Average
    catalog.insert(BarIndicatorId::Tma,
        RenderingMetadata::builder("TMA")
            .overlay()
            .line_output("tma", "TMA", COLOR_LIME)
            .precision(4)
            .build()
    );

    // Adaptive Moving Average (Kaufman's)
    catalog.insert(BarIndicatorId::Ama,
        RenderingMetadata::builder("AMA")
            .overlay()
            .line_output("ama", "AMA", COLOR_AMBER)
            .precision(4)
            .build()
    );

    // Fractal Adaptive MA
    catalog.insert(BarIndicatorId::AvFrama,
        RenderingMetadata::builder("AV_FRAMA")
            .overlay()
            .line_output("frama", "FRAMA", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // FRAMA Advanced
    catalog.insert(BarIndicatorId::Framaadv,
        RenderingMetadata::builder("FRAMAADV")
            .overlay()
            .line_output("framaadv", "FRAMA Advanced", COLOR_INDIGO)
            .precision(4)
            .build()
    );

    // Linear Regression
    catalog.insert(BarIndicatorId::Lr,
        RenderingMetadata::builder("LR")
            .overlay()
            .line_output("lr", "Linear Regression", COLOR_BLUE_GRAY)
            .precision(4)
            .build()
    );

    // Ehlers Fractal Adaptive MA
    catalog.insert(BarIndicatorId::Ehlersfa,
        RenderingMetadata::builder("EHLERSFA")
            .overlay()
            .line_output("ehlersfa", "Ehlers FA", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Ehlers Zero Lag EMA
    catalog.insert(BarIndicatorId::Ehlersz,
        RenderingMetadata::builder("EHLERSZ")
            .overlay()
            .line_output("ehlersz", "Ehlers ZL", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // ALMA
    catalog.insert(BarIndicatorId::Alma,
        RenderingMetadata::builder("ALMA")
            .overlay()
            .line_output("alma", "ALMA", COLOR_PINK)
            .precision(4)
            .build()
    );

    // Jurik Moving Average
    catalog.insert(BarIndicatorId::Jma,
        RenderingMetadata::builder("JMA")
            .overlay()
            .line_output("jma", "JMA", COLOR_GREEN)
            .precision(4)
            .build()
    );

    // McGinley Dynamic
    catalog.insert(BarIndicatorId::Mcginley,
        RenderingMetadata::builder("MCGINLEY")
            .overlay()
            .line_output("mcginley", "McGinley", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // T3 Moving Average
    catalog.insert(BarIndicatorId::T3,
        RenderingMetadata::builder("T3")
            .overlay()
            .line_output("t3", "T3", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // TRIMA
    catalog.insert(BarIndicatorId::Trima,
        RenderingMetadata::builder("TRIMA")
            .overlay()
            .line_output("trima", "TRIMA", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // VIDYA
    catalog.insert(BarIndicatorId::AvVidya,
        RenderingMetadata::builder("AV_VIDYA")
            .overlay()
            .line_output("vidya", "VIDYA", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Volume Weighted MA
    catalog.insert(BarIndicatorId::Vwma,
        RenderingMetadata::builder("VWMA")
            .overlay()
            .line_output("vwma", "VWMA", COLOR_AMBER)
            .precision(4)
            .build()
    );
}

// ============================================================================
// MOMENTUM INDICATORS
// ============================================================================

fn register_momentum_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // RSI - bounded 0-100 oscillator
    catalog.insert(BarIndicatorId::Rsi,
        RenderingMetadata::builder("RSI")
            .sub_pane()
            .line_output("rsi", "RSI", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // MACD - zero-baseline with histogram
    catalog.insert(BarIndicatorId::Macd,
        RenderingMetadata::builder("MACD")
            .sub_pane()
            .output(OutputSpec::line("macd", "MACD", COLOR_BLUE, 2.0, ValueExtractor::Macd(MacdPart::Line)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Macd(MacdPart::Signal)))
            .output(OutputSpec::histogram("histogram", "Histogram", COLOR_GREEN, ValueExtractor::Macd(MacdPart::Histogram)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // Stochastic Oscillator
    catalog.insert(BarIndicatorId::Stoch,
        RenderingMetadata::builder("STOCH")
            .sub_pane()
            .output(OutputSpec::line("k", "%K", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("d", "%D", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(0.0, 100.0)
            .overbought_oversold(80.0, 20.0)
            .precision(2)
            .build()
    );

    // Stochastic RSI
    catalog.insert(BarIndicatorId::StochRsi,
        RenderingMetadata::builder("STOCHRSI")
            .sub_pane()
            .output(OutputSpec::line("k", "%K", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("d", "%D", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.8, COLOR_OVERBOUGHT).with_label("Overbought"))
            .reference_line(ReferenceLine::new(0.2, COLOR_OVERSOLD).with_label("Oversold"))
            .precision(4)
            .build()
    );

    // ADX - Average Directional Index (returns Single - just ADX value)
    // Note: +DI/-DI are available via DiPlusMinus indicator
    catalog.insert(BarIndicatorId::Adx,
        RenderingMetadata::builder("ADX")
            .sub_pane()
            .line_output("adx", "ADX", COLOR_BLUE)
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(25.0, COLOR_GRAY).with_label("Trend"))
            .precision(2)
            .build()
    );

    // CCI - Commodity Channel Index
    catalog.insert(BarIndicatorId::Cci,
        RenderingMetadata::builder("CCI")
            .sub_pane()
            .line_output("cci", "CCI", COLOR_PURPLE)
            .zero_baseline()
            .reference_line(ReferenceLine::new(100.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-100.0, COLOR_OVERSOLD))
            .precision(2)
            .build()
    );

    // Williams %R

    // ROC - Rate of Change
    catalog.insert(BarIndicatorId::Roc,
        RenderingMetadata::builder("ROC")
            .sub_pane()
            .line_output("roc", "ROC", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Momentum

    // PPO - Percentage Price Oscillator (returns Macd type: line, signal, histogram)
    catalog.insert(BarIndicatorId::Ppo,
        RenderingMetadata::builder("PPO")
            .sub_pane()
            .output(OutputSpec::line("ppo", "PPO", COLOR_BLUE, 2.0, ValueExtractor::Macd(MacdPart::Line)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Macd(MacdPart::Signal)))
            .output(OutputSpec::histogram("histogram", "Histogram", COLOR_GRAY, ValueExtractor::Macd(MacdPart::Histogram)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // APO - Absolute Price Oscillator
    catalog.insert(BarIndicatorId::Apo,
        RenderingMetadata::builder("APO")
            .sub_pane()
            .line_output("apo", "APO", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // MFI - Money Flow Index
    catalog.insert(BarIndicatorId::Mfi,
        RenderingMetadata::builder("MFI")
            .sub_pane()
            .line_output("mfi", "MFI", COLOR_TEAL)
            .bounds(0.0, 100.0)
            .overbought_oversold(80.0, 20.0)
            .precision(2)
            .build()
    );

    // CMO - Chande Momentum Oscillator
    catalog.insert(BarIndicatorId::Cmo,
        RenderingMetadata::builder("CMO")
            .sub_pane()
            .line_output("cmo", "CMO", COLOR_PURPLE)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-50.0, COLOR_OVERSOLD))
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // Aroon - returns Triple(up, down, osc)
    catalog.insert(BarIndicatorId::Aroon,
        RenderingMetadata::builder("AROON")
            .sub_pane()
            .output(OutputSpec::line("aroon_up", "Aroon Up", COLOR_GREEN, 2.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("aroon_down", "Aroon Down", COLOR_RED, 2.0, ValueExtractor::Triple(TriplePart::Second)))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Aroon Oscillator
    catalog.insert(BarIndicatorId::AroonOsc,
        RenderingMetadata::builder("AROON_OSC")
            .sub_pane()
            .line_output("aroon_osc", "Aroon Oscillator", COLOR_BLUE)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // Ultimate Oscillator
    catalog.insert(BarIndicatorId::Uo,
        RenderingMetadata::builder("UO")
            .sub_pane()
            .line_output("uo", "Ultimate Osc", COLOR_INDIGO)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // TSI - True Strength Index
    catalog.insert(BarIndicatorId::Tsi,
        RenderingMetadata::builder("TSI")
            .sub_pane()
            .output(OutputSpec::line("tsi", "TSI", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(25.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-25.0, COLOR_OVERSOLD))
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // Awesome Oscillator
    catalog.insert(BarIndicatorId::Ao,
        RenderingMetadata::builder("AO")
            .sub_pane()
            .output(OutputSpec::histogram("ao", "Awesome Oscillator", COLOR_GREEN, ValueExtractor::Main))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // Accelerator Oscillator
    catalog.insert(BarIndicatorId::Ac,
        RenderingMetadata::builder("AC")
            .sub_pane()
            .output(OutputSpec::histogram("ac", "Accelerator", COLOR_CYAN, ValueExtractor::Main))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // Bias
    catalog.insert(BarIndicatorId::Bias,
        RenderingMetadata::builder("BIAS")
            .sub_pane()
            .line_output("bias", "Bias", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // BOP - Balance of Power
    catalog.insert(BarIndicatorId::Bop,
        RenderingMetadata::builder("BOP")
            .sub_pane()
            .line_output("bop", "Balance of Power", COLOR_BLUE)
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // DPO - Detrended Price Oscillator
    catalog.insert(BarIndicatorId::Dpo,
        RenderingMetadata::builder("DPO")
            .sub_pane()
            .line_output("dpo", "DPO", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // TRIX
    catalog.insert(BarIndicatorId::Trix,
        RenderingMetadata::builder("TRIX")
            .sub_pane()
            .line_output("trix", "TRIX", COLOR_BLUE)
            .zero_baseline()
            .precision(6)
            .build()
    );

    // Coppock Curve
    catalog.insert(BarIndicatorId::Coppock,
        RenderingMetadata::builder("COPPOCK")
            .sub_pane()
            .line_output("coppock", "Coppock", COLOR_TEAL)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Vortex Indicator
    catalog.insert(BarIndicatorId::Vortex,
        RenderingMetadata::builder("VORTEX")
            .sub_pane()
            .output(OutputSpec::line("vi_plus", "VI+", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("vi_minus", "VI-", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .reference_line(ReferenceLine::new(1.0, COLOR_GRAY))
            .precision(4)
            .build()
    );

    // KST - Know Sure Thing
    catalog.insert(BarIndicatorId::Kst,
        RenderingMetadata::builder("KST")
            .sub_pane()
            .output(OutputSpec::line("kst", "KST", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Mass Index

    // Elder Ray - Bull/Bear Power
    catalog.insert(BarIndicatorId::ElderRay,
        RenderingMetadata::builder("ELDER_RAY")
            .sub_pane()
            .output(OutputSpec::histogram("bull", "Bull Power", COLOR_GREEN, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::histogram("bear", "Bear Power", COLOR_RED, ValueExtractor::Double(DoublePart::Second)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // Chaikin Oscillator

    // Fisher Transform
    // Fisher Information (entropy-based) - returns Single
    catalog.insert(BarIndicatorId::Fisher,
        RenderingMetadata::builder("FISHER")
            .sub_pane()
            .line_output("fisher_info", "Fisher Information", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Connors RSI

    // QStick
    catalog.insert(BarIndicatorId::Qstick,
        RenderingMetadata::builder("QSTICK")
            .sub_pane()
            .line_output("qstick", "QStick", COLOR_TEAL)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Relative Volatility Index (RVI returns single value 0-100)
    catalog.insert(BarIndicatorId::Rvi,
        RenderingMetadata::builder("RVI")
            .sub_pane()
            .output(OutputSpec::line("rvi", "RVI", COLOR_GREEN, 2.0, ValueExtractor::Main))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // SMI - Stochastic Momentum Index
    catalog.insert(BarIndicatorId::Smi,
        RenderingMetadata::builder("SMI")
            .sub_pane()
            .output(OutputSpec::line("smi", "SMI", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(40.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-40.0, COLOR_OVERSOLD))
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // Inertia

    // Squeeze Momentum

    // ========================================================================
    // MISSING MOMENTUM INDICATORS (from checklist)
    // ========================================================================

    // Adaptive Stochastic
    catalog.insert(BarIndicatorId::AdaptiveStoch,
        RenderingMetadata::builder("ADAPTIVE_STOCH")
            .sub_pane()
            .output(OutputSpec::line("k", "%K", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("d", "%D", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(0.0, 100.0)
            .overbought_oversold(80.0, 20.0)
            .precision(2)
            .build()
    );

    // AMAT - Absolute Momentum & Trend
    catalog.insert(BarIndicatorId::Amat,
        RenderingMetadata::builder("AMAT")
            .sub_pane()
            .line_output("amat", "AMAT", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Aroon Down (sub-component)
    catalog.insert(BarIndicatorId::AroonDown,
        RenderingMetadata::builder("AROON_DOWN")
            .sub_pane()
            .line_output("aroon_down", "Aroon Down", COLOR_RED)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Aroon Up (sub-component)
    catalog.insert(BarIndicatorId::AroonUp,
        RenderingMetadata::builder("AROON_UP")
            .sub_pane()
            .line_output("aroon_up", "Aroon Up", COLOR_GREEN)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // ATR RSI
    catalog.insert(BarIndicatorId::AtrRsi,
        RenderingMetadata::builder("ATR_RSI")
            .sub_pane()
            .line_output("atr_rsi", "ATR RSI", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // Auto Fibonacci
    catalog.insert(BarIndicatorId::AutoFibo,
        RenderingMetadata::builder("AUTO_FIBO")
            .overlay()
            .line_output("fibo", "Auto Fibo", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // BB Period
    catalog.insert(BarIndicatorId::BbPeriod,
        RenderingMetadata::builder("BB_PERIOD")
            .sub_pane()
            .line_output("bb_period", "BB Period", COLOR_BLUE)
            .precision(0)
            .build()
    );

    // Candle Patterns
    catalog.insert(BarIndicatorId::CandlePatterns,
        RenderingMetadata::builder("CANDLE_PATTERNS")
            .sub_pane()
            .line_output("pattern", "Pattern", COLOR_PURPLE)
            .precision(0)
            .build()
    );

    // CFO - Chande Forecast Oscillator
    catalog.insert(BarIndicatorId::Cfo,
        RenderingMetadata::builder("CFO")
            .sub_pane()
            .line_output("cfo", "CFO", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // COG - Center of Gravity
    catalog.insert(BarIndicatorId::Cog,
        RenderingMetadata::builder("COG")
            .overlay()
            .line_output("cog", "COG", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Connors RSI
    catalog.insert(BarIndicatorId::ConnorsRsi,
        RenderingMetadata::builder("CONNORS_RSI")
            .sub_pane()
            .line_output("crsi", "Connors RSI", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // DeMarker
    catalog.insert(BarIndicatorId::Demarker,
        RenderingMetadata::builder("DEMARKER")
            .sub_pane()
            .line_output("demarker", "DeMarker", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.7, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(0.3, COLOR_OVERSOLD))
            .precision(4)
            .build()
    );

    // DM - Directional Movement - returns Triple(+DI, -DI, ADX)
    catalog.insert(BarIndicatorId::Dm,
        RenderingMetadata::builder("DM")
            .sub_pane()
            .output(OutputSpec::line("plus_di", "+DI", COLOR_GREEN, 2.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("minus_di", "-DI", COLOR_RED, 2.0, ValueExtractor::Triple(TriplePart::Second)))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // DPO Percent
    catalog.insert(BarIndicatorId::DpoPct,
        RenderingMetadata::builder("DPO_PCT")
            .sub_pane()
            .line_output("dpo_pct", "DPO %", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // DSP - Detrended Synthetic Price
    catalog.insert(BarIndicatorId::Dsp,
        RenderingMetadata::builder("DSP")
            .sub_pane()
            .line_output("dsp", "DSP", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // DSS - Double Smoothed Stochastic
    catalog.insert(BarIndicatorId::Dss,
        RenderingMetadata::builder("DSS")
            .sub_pane()
            .line_output("dss", "DSS", COLOR_BLUE)
            .bounds(0.0, 100.0)
            .overbought_oversold(80.0, 20.0)
            .precision(2)
            .build()
    );

    // Ehlers Cyber Cycle
    catalog.insert(BarIndicatorId::EhlersCc,
        RenderingMetadata::builder("EHLERS_CC")
            .sub_pane()
            .line_output("cyber", "Cyber Cycle", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Ehlers Rocket RSI
    catalog.insert(BarIndicatorId::EhlersRocket,
        RenderingMetadata::builder("EHLERS_ROCKET")
            .sub_pane()
            .line_output("rocket", "Rocket RSI", COLOR_ORANGE)
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Elder Impulse System
    catalog.insert(BarIndicatorId::ElderImpulse,
        RenderingMetadata::builder("ELDER_IMPULSE")
            .sub_pane()
            .line_output("impulse", "Impulse", COLOR_GREEN)
            .precision(0)
            .build()
    );

    // EMA Slope
    catalog.insert(BarIndicatorId::EmaSlope,
        RenderingMetadata::builder("EMA_SLOPE")
            .sub_pane()
            .line_output("slope", "EMA Slope", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // EWMAC - Exponentially Weighted Moving Average Crossover
    catalog.insert(BarIndicatorId::Ewmac,
        RenderingMetadata::builder("EWMAC")
            .sub_pane()
            .line_output("ewmac", "EWMAC", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // EWMAC Robust
    catalog.insert(BarIndicatorId::EwmacRobust,
        RenderingMetadata::builder("EWMAC_ROBUST")
            .sub_pane()
            .line_output("ewmac", "EWMAC Robust", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // GAPO - Gopalakrishnan Range Index
    catalog.insert(BarIndicatorId::Gapo,
        RenderingMetadata::builder("GAPO")
            .sub_pane()
            .line_output("gapo", "GAPO", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Gator Oscillator - returns Single (spread between fast and slow)
    catalog.insert(BarIndicatorId::Gator,
        RenderingMetadata::builder("GATOR")
            .sub_pane()
            .line_output("gator", "Gator", COLOR_GREEN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Highest
    catalog.insert(BarIndicatorId::Highest,
        RenderingMetadata::builder("HIGHEST")
            .overlay()
            .line_output("highest", "Highest", COLOR_GREEN)
            .precision(4)
            .build()
    );

    // IFT RSI - Inverse Fisher Transform RSI
    catalog.insert(BarIndicatorId::IftRsi,
        RenderingMetadata::builder("IFT_RSI")
            .sub_pane()
            .line_output("ift_rsi", "IFT RSI", COLOR_PURPLE)
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // IMI - Intraday Momentum Index
    catalog.insert(BarIndicatorId::Imi,
        RenderingMetadata::builder("IMI")
            .sub_pane()
            .line_output("imi", "IMI", COLOR_TEAL)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // KDJ
    catalog.insert(BarIndicatorId::Kdj,
        RenderingMetadata::builder("KDJ")
            .sub_pane()
            .output(OutputSpec::line("k", "K", COLOR_BLUE, 2.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("d", "D", COLOR_ORANGE, 1.0, ValueExtractor::Triple(TriplePart::Second)))
            .output(OutputSpec::line("j", "J", COLOR_PURPLE, 1.0, ValueExtractor::Triple(TriplePart::Third)))
            .bounds(0.0, 100.0)
            .overbought_oversold(80.0, 20.0)
            .precision(2)
            .build()
    );

    // Laguerre RSI
    catalog.insert(BarIndicatorId::LaguerreRsi,
        RenderingMetadata::builder("LAGUERRE_RSI")
            .sub_pane()
            .line_output("lrsi", "Laguerre RSI", COLOR_PURPLE)
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.8, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(0.2, COLOR_OVERSOLD))
            .precision(4)
            .build()
    );

    // Lowest
    catalog.insert(BarIndicatorId::Lowest,
        RenderingMetadata::builder("LOWEST")
            .overlay()
            .line_output("lowest", "Lowest", COLOR_RED)
            .precision(4)
            .build()
    );

    // MA Cross
    catalog.insert(BarIndicatorId::MaCross,
        RenderingMetadata::builder("MA_CROSS")
            .sub_pane()
            .line_output("cross", "MA Cross", COLOR_BLUE)
            .precision(0)
            .build()
    );

    // MACD Histogram (standalone)
    catalog.insert(BarIndicatorId::MacdHist,
        RenderingMetadata::builder("MACD_HIST")
            .sub_pane()
            .output(OutputSpec::histogram("histogram", "MACD Hist", COLOR_GREEN, ValueExtractor::Main))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // MACD Histogram Z-Score
    catalog.insert(BarIndicatorId::MacdHistZ,
        RenderingMetadata::builder("MACD_HIST_Z")
            .sub_pane()
            .line_output("z_score", "MACD Hist Z", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // MACD Signal (standalone)
    catalog.insert(BarIndicatorId::MacdSignal,
        RenderingMetadata::builder("MACD_SIGNAL")
            .sub_pane()
            .line_output("signal", "MACD Signal", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Market Cipher
    catalog.insert(BarIndicatorId::MarketCipher,
        RenderingMetadata::builder("MARKET_CIPHER")
            .sub_pane()
            .line_output("cipher", "Market Cipher", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // MO Fisher
    catalog.insert(BarIndicatorId::MoFisher,
        RenderingMetadata::builder("MO_FISHER")
            .sub_pane()
            .line_output("fisher", "MO Fisher", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // MO OBV
    catalog.insert(BarIndicatorId::MoObv,
        RenderingMetadata::builder("MO_OBV")
            .sub_pane()
            .line_output("mo_obv", "MO OBV", COLOR_TEAL)
            .precision(0)
            .build()
    );

    // Momentum Z-Score
    catalog.insert(BarIndicatorId::MomZscore,
        RenderingMetadata::builder("MOM_ZSCORE")
            .sub_pane()
            .line_output("z_score", "Mom Z-Score", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // MTF Momentum Divergence
    catalog.insert(BarIndicatorId::MtfMomDiv,
        RenderingMetadata::builder("MTF_MOM_DIV")
            .sub_pane()
            .line_output("div", "MTF Mom Div", COLOR_PURPLE)
            .precision(0)
            .build()
    );

    // Neural Momentum
    catalog.insert(BarIndicatorId::NeuralMom,
        RenderingMetadata::builder("NEURAL_MOM")
            .sub_pane()
            .line_output("neural", "Neural Mom", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // PFE - Polarized Fractal Efficiency
    catalog.insert(BarIndicatorId::Pfe,
        RenderingMetadata::builder("PFE")
            .sub_pane()
            .line_output("pfe", "PFE", COLOR_PURPLE)
            .bounds(-100.0, 100.0)
            .zero_baseline()
            .precision(2)
            .build()
    );

    // PMO - Price Momentum Oscillator
    catalog.insert(BarIndicatorId::Pmo,
        RenderingMetadata::builder("PMO")
            .sub_pane()
            .output(OutputSpec::line("pmo", "PMO", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .zero_baseline()
            .precision(4)
            .build()
    );

    // PPO Signal (standalone)
    catalog.insert(BarIndicatorId::PpoSignal,
        RenderingMetadata::builder("PPO_SIGNAL")
            .sub_pane()
            .line_output("signal", "PPO Signal", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Pressure
    catalog.insert(BarIndicatorId::Pressure,
        RenderingMetadata::builder("PRESSURE")
            .sub_pane()
            .line_output("pressure", "Pressure", COLOR_TEAL)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // PSL - Psychological Line
    catalog.insert(BarIndicatorId::Psl,
        RenderingMetadata::builder("PSL")
            .sub_pane()
            .line_output("psl", "PSL", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, COLOR_GRAY))
            .precision(2)
            .build()
    );

    // RMI - Relative Momentum Index
    catalog.insert(BarIndicatorId::Rmi,
        RenderingMetadata::builder("RMI")
            .sub_pane()
            .line_output("rmi", "RMI", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // ROC Percent
    catalog.insert(BarIndicatorId::RocPct,
        RenderingMetadata::builder("ROC_PCT")
            .sub_pane()
            .line_output("roc_pct", "ROC %", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // RSI Percentile Bands
    catalog.insert(BarIndicatorId::RsiPctBands,
        RenderingMetadata::builder("RSI_PCT_BANDS")
            .sub_pane()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("rsi", "RSI", COLOR_PURPLE, 2.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // RSI Percentile Rank
    catalog.insert(BarIndicatorId::RsiPctRank,
        RenderingMetadata::builder("RSI_PCT_RANK")
            .sub_pane()
            .line_output("rank", "RSI % Rank", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // RSI Z-Score
    catalog.insert(BarIndicatorId::RsiZscore,
        RenderingMetadata::builder("RSI_ZSCORE")
            .sub_pane()
            .line_output("z_score", "RSI Z-Score", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // RSIOMA
    catalog.insert(BarIndicatorId::Rsioma,
        RenderingMetadata::builder("RSIOMA")
            .sub_pane()
            .line_output("rsioma", "RSIOMA", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // RSX - Relative Strength Xtra
    catalog.insert(BarIndicatorId::Rsx,
        RenderingMetadata::builder("RSX")
            .sub_pane()
            .line_output("rsx", "RSX", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // RVGI - Relative Vigor Index
    catalog.insert(BarIndicatorId::Rvgi,
        RenderingMetadata::builder("RVGI")
            .sub_pane()
            .output(OutputSpec::line("rvgi", "RVGI", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .zero_baseline()
            .precision(4)
            .build()
    );

    // RWI - Random Walk Index
    catalog.insert(BarIndicatorId::Rwi,
        RenderingMetadata::builder("RWI")
            .sub_pane()
            .output(OutputSpec::line("high", "RWI High", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("low", "RWI Low", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .reference_line(ReferenceLine::new(1.0, COLOR_GRAY))
            .precision(4)
            .build()
    );

    // Stochastic K-D
    catalog.insert(BarIndicatorId::Stochkd,
        RenderingMetadata::builder("STOCHKD")
            .sub_pane()
            .output(OutputSpec::line("k", "%K", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("d", "%D", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(0.0, 100.0)
            .overbought_oversold(80.0, 20.0)
            .precision(2)
            .build()
    );

    // Sweep Reversal
    catalog.insert(BarIndicatorId::SweepRev,
        RenderingMetadata::builder("SWEEP_REV")
            .sub_pane()
            .line_output("sweep", "Sweep Rev", COLOR_PURPLE)
            .precision(0)
            .build()
    );

    // Swing Age
    catalog.insert(BarIndicatorId::SwingAge,
        RenderingMetadata::builder("SWING_AGE")
            .sub_pane()
            .line_output("age", "Swing Age", COLOR_BLUE)
            .precision(0)
            .build()
    );

    // Swings
    catalog.insert(BarIndicatorId::Swings,
        RenderingMetadata::builder("SWINGS")
            .overlay()
            .output(OutputSpec::line("high", "Swing High", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("low", "Swing Low", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Swings Soft
    catalog.insert(BarIndicatorId::SwingsSoft,
        RenderingMetadata::builder("SWINGS_SOFT")
            .overlay()
            .output(OutputSpec::line("high", "Swing High", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("low", "Swing Low", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // TDI - Traders Dynamic Index
    catalog.insert(BarIndicatorId::Tdi,
        RenderingMetadata::builder("TDI")
            .sub_pane()
            .output(OutputSpec::line("rsi", "RSI", COLOR_GREEN, 2.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_RED, 1.0, ValueExtractor::Triple(TriplePart::Second)))
            .output(OutputSpec::line("baseline", "Baseline", COLOR_BLUE, 1.0, ValueExtractor::Triple(TriplePart::Third)))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Ultimate Oscillator Smooth
    catalog.insert(BarIndicatorId::UoSmooth,
        RenderingMetadata::builder("UO_SMOOTH")
            .sub_pane()
            .line_output("uo", "UO Smooth", COLOR_INDIGO)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // VHF - Vertical Horizontal Filter
    catalog.insert(BarIndicatorId::Vhf,
        RenderingMetadata::builder("VHF")
            .sub_pane()
            .line_output("vhf", "VHF", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // VHF MA
    catalog.insert(BarIndicatorId::VhfMa,
        RenderingMetadata::builder("VHF_MA")
            .sub_pane()
            .line_output("vhf_ma", "VHF MA", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Volume Weighted RSI
    catalog.insert(BarIndicatorId::Vwrsi,
        RenderingMetadata::builder("VWRSI")
            .sub_pane()
            .line_output("vwrsi", "VWRSI", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(70.0, 30.0)
            .precision(2)
            .build()
    );

    // Williams %R (alias)
    catalog.insert(BarIndicatorId::WilliamsR,
        RenderingMetadata::builder("WILLIAMS_R")
            .sub_pane()
            .line_output("willr", "Williams %R", COLOR_CYAN)
            .bounds(-100.0, 0.0)
            .reference_line(ReferenceLine::new(-20.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-80.0, COLOR_OVERSOLD))
            .precision(2)
            .build()
    );

    // Zigzag
    catalog.insert(BarIndicatorId::Zigzag,
        RenderingMetadata::builder("ZIGZAG")
            .overlay()
            .line_output("zigzag", "Zigzag", COLOR_PURPLE)
            .precision(4)
            .build()
    );
}

// ============================================================================
// CHANNEL INDICATORS
// ============================================================================

fn register_channel_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Bollinger Bands
    catalog.insert(BarIndicatorId::Bb,
        RenderingMetadata::builder("BB")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .output(OutputSpec::band("fill", "Band Fill", COLOR_CHANNEL_FILL))
            .precision(4)
            .build()
    );

    // Keltner Channel
    catalog.insert(BarIndicatorId::Kc,
        RenderingMetadata::builder("KC")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .output(OutputSpec::band("fill", "Band Fill", COLOR_CHANNEL_FILL))
            .precision(4)
            .build()
    );

    // Donchian Channel
    catalog.insert(BarIndicatorId::Dc,
        RenderingMetadata::builder("DC")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Ichimoku Cloud
    catalog.insert(BarIndicatorId::Ichimoku,
        RenderingMetadata::builder("ICHIMOKU")
            .overlay()
            .output(OutputSpec::line("tenkan", "Tenkan-sen", COLOR_RED, 1.0, ValueExtractor::Ichimoku(IchimokuPart::Tenkan)))
            .output(OutputSpec::line("kijun", "Kijun-sen", COLOR_BLUE, 2.0, ValueExtractor::Ichimoku(IchimokuPart::Kijun)))
            .output(OutputSpec::line("senkou_a", "Senkou A", COLOR_GREEN, 1.0, ValueExtractor::Ichimoku(IchimokuPart::SenkouA)))
            .output(OutputSpec::line("senkou_b", "Senkou B", COLOR_RED, 1.0, ValueExtractor::Ichimoku(IchimokuPart::SenkouB)))
            .output(OutputSpec::line("chikou", "Chikou", COLOR_PURPLE, 1.0, ValueExtractor::Ichimoku(IchimokuPart::Chikou)).with_style(LineStyle::Dotted))
            .output(OutputSpec::cloud("cloud", "Kumo", COLOR_CHANNEL_FILL))
            .precision(4)
            .build()
    );

    // Adaptive Bollinger Bands
    catalog.insert(BarIndicatorId::Adaptivebb,
        RenderingMetadata::builder("ADAPTIVEBB")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // ATR Channels
    catalog.insert(BarIndicatorId::Atrchan,
        RenderingMetadata::builder("ATRCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // STARC Bands
    catalog.insert(BarIndicatorId::Starc,
        RenderingMetadata::builder("STARC")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Envelope
    catalog.insert(BarIndicatorId::Envelope,
        RenderingMetadata::builder("ENVELOPE")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Percent B (sub-pane)
    catalog.insert(BarIndicatorId::Percentb,
        RenderingMetadata::builder("PERCENTB")
            .sub_pane()
            .line_output("percentb", "%B", COLOR_PURPLE)
            .reference_line(ReferenceLine::new(1.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(0.5, COLOR_GRAY))
            .reference_line(ReferenceLine::new(0.0, COLOR_OVERSOLD))
            .precision(4)
            .build()
    );

    // Bollinger Metrics - %B and Bandwidth (sub-pane)
    catalog.insert(BarIndicatorId::Bbmetrics,
        RenderingMetadata::builder("BBMETRICS")
            .sub_pane()
            .output(OutputSpec::line("percent_b", "%B", COLOR_PURPLE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("bandwidth", "Bandwidth", COLOR_BLUE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Price Channels
    catalog.insert(BarIndicatorId::Pricechan,
        RenderingMetadata::builder("PRICECHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Regression Channels
    catalog.insert(BarIndicatorId::Regchan,
        RenderingMetadata::builder("REGCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Standard Deviation Channels
    catalog.insert(BarIndicatorId::Stddevchan,
        RenderingMetadata::builder("STDDEVCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Median Channels
    catalog.insert(BarIndicatorId::Medchan,
        RenderingMetadata::builder("MEDCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // VWAP Channels
    catalog.insert(BarIndicatorId::Vwapchan,
        RenderingMetadata::builder("VWAPCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("vwap", "VWAP", COLOR_BLUE, 2.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Darvas Box
    catalog.insert(BarIndicatorId::Darvas,
        RenderingMetadata::builder("DARVAS")
            .overlay()
            .output(OutputSpec::line("upper", "Box Top", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("lower", "Box Bottom", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Fibonacci Channels
    catalog.insert(BarIndicatorId::Fibochan,
        RenderingMetadata::builder("FIBOCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_CHANNEL_UPPER, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_CHANNEL_MIDDLE, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_CHANNEL_LOWER, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Channel position metrics (sub-pane)
    for id in [BarIndicatorId::Dcpos, BarIndicatorId::Keltpos, BarIndicatorId::Medchanpos] {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("position", "Position", COLOR_PURPLE)
                .bounds(0.0, 1.0)
                .reference_line(ReferenceLine::new(0.5, COLOR_GRAY))
                .precision(4)
                .build()
        );
    }

    // Channel width metrics (sub-pane)
    for id in [BarIndicatorId::Dcwidth, BarIndicatorId::Pchwidth, BarIndicatorId::Regchanwidth, BarIndicatorId::Stddevwidth, BarIndicatorId::Keltbw, BarIndicatorId::Envbw, BarIndicatorId::Vwapchanwidth] {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("width", "Width", COLOR_BLUE)
                .precision(4)
                .build()
        );
    }

    // Keltner Distance
    catalog.insert(BarIndicatorId::Keltdist,
        RenderingMetadata::builder("KELTDIST")
            .sub_pane()
            .line_output("distance", "Distance", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Channel oscillators (sub-pane)
    catalog.insert(BarIndicatorId::Pchosc,
        RenderingMetadata::builder("PCHOSC")
            .sub_pane()
            .line_output("oscillator", "Price Channel Osc", COLOR_PURPLE)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // Ichimoku position/thickness (sub-pane)
    catalog.insert(BarIndicatorId::Ichimokupos,
        RenderingMetadata::builder("ICHIMOKUPOS")
            .sub_pane()
            .line_output("position", "Cloud Position", COLOR_PURPLE)
            .precision(2)
            .build()
    );

    catalog.insert(BarIndicatorId::Ichimokuthick,
        RenderingMetadata::builder("ICHIMOKUTHICK")
            .sub_pane()
            .line_output("thickness", "Cloud Thickness", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // ========================================================================
    // MISSING CHANNEL INDICATORS (from checklist)
    // ========================================================================

    // Adaptive Channels
    catalog.insert(BarIndicatorId::Adaptivechan,
        RenderingMetadata::builder("ADAPTIVECHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Donchian Metrics
    catalog.insert(BarIndicatorId::Dcmetrics,
        RenderingMetadata::builder("DCMETRICS")
            .sub_pane()
            .line_output("metrics", "DC Metrics", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // DPO Bands
    catalog.insert(BarIndicatorId::Dpobands,
        RenderingMetadata::builder("DPOBANDS")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Keltner Metrics
    catalog.insert(BarIndicatorId::Kcmetrics,
        RenderingMetadata::builder("KCMETRICS")
            .sub_pane()
            .line_output("metrics", "KC Metrics", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Percentile Channels
    catalog.insert(BarIndicatorId::Percentilech,
        RenderingMetadata::builder("PERCENTILECH")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Pivot Channels
    catalog.insert(BarIndicatorId::Pivotchan,
        RenderingMetadata::builder("PIVOTCHAN")
            .overlay()
            .output(OutputSpec::line("r1", "R1", COLOR_RED, 1.0, ValueExtractor::Main))
            .output(OutputSpec::line("pivot", "Pivot", COLOR_GRAY, 1.0, ValueExtractor::Main))
            .output(OutputSpec::line("s1", "S1", COLOR_GREEN, 1.0, ValueExtractor::Main))
            .precision(4)
            .build()
    );

    // Projection Bands
    catalog.insert(BarIndicatorId::Projbands,
        RenderingMetadata::builder("PROJBANDS")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Quantile Regression Channels
    catalog.insert(BarIndicatorId::Qrchan,
        RenderingMetadata::builder("QRCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Theil-Sen Channels
    catalog.insert(BarIndicatorId::Theilsenchan,
        RenderingMetadata::builder("THEILSENCHAN")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // TRIMA Bands
    catalog.insert(BarIndicatorId::Trimabands,
        RenderingMetadata::builder("TRIMABANDS")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Volume Profile Channels
    catalog.insert(BarIndicatorId::Volprofchan,
        RenderingMetadata::builder("VOLPROFCHAN")
            .overlay()
            .output(OutputSpec::line("vah", "VAH", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("poc", "POC", COLOR_PURPLE, 2.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("val", "VAL", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // ========== CHANNEL WIDTH/POSITION INDICATORS ==========

    // Donchian Channel Position (0-1 normalized)
    catalog.insert(BarIndicatorId::Dcpos,
        RenderingMetadata::builder("DCPOS")
            .sub_pane()
            .line_output("dcpos", "DC Position", COLOR_BLUE)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Donchian Channel Width
    catalog.insert(BarIndicatorId::Dcwidth,
        RenderingMetadata::builder("DCWIDTH")
            .sub_pane()
            .line_output("dcwidth", "DC Width", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Envelope Bandwidth
    catalog.insert(BarIndicatorId::Envbw,
        RenderingMetadata::builder("ENVBW")
            .sub_pane()
            .line_output("envbw", "Envelope BW", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Keltner Bandwidth
    catalog.insert(BarIndicatorId::Keltbw,
        RenderingMetadata::builder("KELTBW")
            .sub_pane()
            .line_output("keltbw", "Keltner BW", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Keltner Position (0-1 normalized)
    catalog.insert(BarIndicatorId::Keltpos,
        RenderingMetadata::builder("KELTPOS")
            .sub_pane()
            .line_output("keltpos", "Keltner Position", COLOR_CYAN)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Median Channel Position (0-1 normalized)
    catalog.insert(BarIndicatorId::Medchanpos,
        RenderingMetadata::builder("MEDCHANPOS")
            .sub_pane()
            .line_output("medchanpos", "Median Chan Position", COLOR_GREEN)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Price Channel Width
    catalog.insert(BarIndicatorId::Pchwidth,
        RenderingMetadata::builder("PCHWIDTH")
            .sub_pane()
            .line_output("pchwidth", "Price Chan Width", COLOR_RED)
            .precision(4)
            .build()
    );

    // Regression Channel Width
    catalog.insert(BarIndicatorId::Regchanwidth,
        RenderingMetadata::builder("REGCHANWIDTH")
            .sub_pane()
            .line_output("regchanwidth", "Reg Chan Width", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // StdDev Channel Width
    catalog.insert(BarIndicatorId::Stddevwidth,
        RenderingMetadata::builder("STDDEVWIDTH")
            .sub_pane()
            .line_output("stddevwidth", "StdDev Width", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // VWAP Channel Width
    catalog.insert(BarIndicatorId::Vwapchanwidth,
        RenderingMetadata::builder("VWAPCHANWIDTH")
            .sub_pane()
            .line_output("vwapchanwidth", "VWAP Chan Width", COLOR_ORANGE)
            .precision(4)
            .build()
    );
}

// ============================================================================
// VOLATILITY INDICATORS
// ============================================================================

fn register_volatility_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // ATR - Average True Range
    catalog.insert(BarIndicatorId::Atr,
        RenderingMetadata::builder("ATR")
            .sub_pane()
            .line_output("atr", "ATR", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // True Range
    catalog.insert(BarIndicatorId::Tr,
        RenderingMetadata::builder("TR")
            .sub_pane()
            .line_output("tr", "True Range", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Standard Deviation

    // Historical Volatility

    // Normalized ATR
    catalog.insert(BarIndicatorId::Natr,
        RenderingMetadata::builder("NATR")
            .sub_pane()
            .line_output("natr", "NATR", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Chaikin Volatility

    // Ulcer Index

    // Variance
    catalog.insert(BarIndicatorId::Var,
        RenderingMetadata::builder("VAR")
            .sub_pane()
            .line_output("var", "Variance", COLOR_PURPLE)
            .precision(6)
            .build()
    );

    // RVI - Relative Volatility Index
    catalog.insert(BarIndicatorId::Rvol,
        RenderingMetadata::builder("RVOL")
            .sub_pane()
            .line_output("rvol", "Rel Volatility", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Garman-Klass Volatility

    // Parkinson Volatility

    // Rogers-Satchell Volatility

    // Yang-Zhang Volatility

    // GARCH
    catalog.insert(BarIndicatorId::Garch,
        RenderingMetadata::builder("GARCH")
            .sub_pane()
            .line_output("garch", "GARCH", COLOR_BLUE)
            .precision(6)
            .build()
    );

    // EGARCH
    catalog.insert(BarIndicatorId::Egarch,
        RenderingMetadata::builder("EGARCH")
            .sub_pane()
            .line_output("egarch", "EGARCH", COLOR_PURPLE)
            .precision(6)
            .build()
    );

    // ========================================================================
    // MISSING VOLATILITY INDICATORS (from checklist)
    // ========================================================================

    // ABB - Adaptive Bollinger Bandwidth
    catalog.insert(BarIndicatorId::Abb,
        RenderingMetadata::builder("ABB")
            .sub_pane()
            .line_output("abb", "Adaptive BB", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // ATR Bandwidth
    catalog.insert(BarIndicatorId::Atrbw,
        RenderingMetadata::builder("ATRBW")
            .sub_pane()
            .line_output("atrbw", "ATR Bandwidth", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // ATR Channels
    catalog.insert(BarIndicatorId::Atrc,
        RenderingMetadata::builder("ATRC")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("middle", "Middle", COLOR_GRAY, 1.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // ATR Percent
    catalog.insert(BarIndicatorId::Atrp,
        RenderingMetadata::builder("ATRP")
            .sub_pane()
            .line_output("atrp", "ATR %", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // ATR Percent Trailing
    catalog.insert(BarIndicatorId::Atrpt,
        RenderingMetadata::builder("ATRPT")
            .overlay()
            .line_output("stop", "ATR Trail", COLOR_RED)
            .precision(4)
            .build()
    );

    // ATR Z-Score
    catalog.insert(BarIndicatorId::Atrz,
        RenderingMetadata::builder("ATRZ")
            .sub_pane()
            .line_output("atrz", "ATR Z", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Average Volatility Range
    catalog.insert(BarIndicatorId::Avr,
        RenderingMetadata::builder("AVR")
            .sub_pane()
            .line_output("avr", "Avg Vol Range", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Bipower Variance
    catalog.insert(BarIndicatorId::Bpv,
        RenderingMetadata::builder("BPV")
            .sub_pane()
            .line_output("bpv", "Bipower Var", COLOR_PURPLE)
            .precision(6)
            .build()
    );

    // Close-to-Close Volatility Percentile
    catalog.insert(BarIndicatorId::C2cvp,
        RenderingMetadata::builder("C2CVP")
            .sub_pane()
            .line_output("c2cvp", "C2C Vol %", COLOR_TEAL)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Coefficient of Variation
    catalog.insert(BarIndicatorId::Cv,
        RenderingMetadata::builder("CV")
            .sub_pane()
            .line_output("cv", "Coef of Var", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Dynamic Volatility Regime
    catalog.insert(BarIndicatorId::Dvr,
        RenderingMetadata::builder("DVR")
            .sub_pane()
            .line_output("dvr", "Dyn Vol Regime", COLOR_PURPLE)
            .precision(2)
            .build()
    );

    // Fuzzy Candlesticks
    catalog.insert(BarIndicatorId::Fuzzy,
        RenderingMetadata::builder("FUZZY")
            .sub_pane()
            .line_output("fuzzy", "Fuzzy", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // HAR - Heterogeneous Autoregressive
    catalog.insert(BarIndicatorId::Har,
        RenderingMetadata::builder("HAR")
            .sub_pane()
            .line_output("har", "HAR", COLOR_BLUE)
            .precision(6)
            .build()
    );

    // Historical Volatility C2C
    catalog.insert(BarIndicatorId::Hvc2c,
        RenderingMetadata::builder("HVC2C")
            .sub_pane()
            .line_output("hvc2c", "HV C2C", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Kase Peak
    catalog.insert(BarIndicatorId::Kp,
        RenderingMetadata::builder("KP")
            .sub_pane()
            .line_output("kp", "Kase Peak", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Narrow Range
    catalog.insert(BarIndicatorId::Nr,
        RenderingMetadata::builder("NR")
            .sub_pane()
            .line_output("nr", "Narrow Range", COLOR_ORANGE)
            .precision(0)
            .build()
    );

    // Parkinson/Garman-Klass/Yang-Zhang volatility
    catalog.insert(BarIndicatorId::Pgry,
        RenderingMetadata::builder("PGRY")
            .sub_pane()
            .line_output("pgry", "PGRY Vol", COLOR_PURPLE)
            .precision(6)
            .build()
    );

    // Realized Bipower Jumps
    catalog.insert(BarIndicatorId::Rbvj,
        RenderingMetadata::builder("RBVJ")
            .sub_pane()
            .line_output("rbvj", "Bipower Jumps", COLOR_RED)
            .precision(6)
            .build()
    );

    // Range Compression Breakout
    catalog.insert(BarIndicatorId::Rcb,
        RenderingMetadata::builder("RCB")
            .sub_pane()
            .line_output("rcb", "Range Comp", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Realized Parkinson
    catalog.insert(BarIndicatorId::Rp,
        RenderingMetadata::builder("RP")
            .sub_pane()
            .line_output("rp", "Realized Park", COLOR_PURPLE)
            .precision(6)
            .build()
    );

    // Realized Quarticity
    catalog.insert(BarIndicatorId::Rq,
        RenderingMetadata::builder("RQ")
            .sub_pane()
            .line_output("rq", "Realized Quart", COLOR_TEAL)
            .precision(6)
            .build()
    );

    // Realized Volatility
    catalog.insert(BarIndicatorId::Rv,
        RenderingMetadata::builder("RV")
            .sub_pane()
            .line_output("rv", "Realized Vol", COLOR_BLUE)
            .precision(6)
            .build()
    );

    // Realized Volatility Z-Score
    catalog.insert(BarIndicatorId::Rvz,
        RenderingMetadata::builder("RVZ")
            .sub_pane()
            .line_output("rvz", "RV Z-Score", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Squeeze Momentum
    catalog.insert(BarIndicatorId::Sqmom,
        RenderingMetadata::builder("SQMOM")
            .sub_pane()
            .output(OutputSpec::histogram("sqmom", "Squeeze Mom", COLOR_GREEN, ValueExtractor::Main))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // Ulcer Index (alias)
    catalog.insert(BarIndicatorId::Ui,
        RenderingMetadata::builder("UI")
            .sub_pane()
            .line_output("ui", "Ulcer Index", COLOR_RED)
            .precision(4)
            .build()
    );

    // Volatility Breakout Detection
    catalog.insert(BarIndicatorId::Vbd,
        RenderingMetadata::builder("VBD")
            .sub_pane()
            .line_output("vbd", "Vol Breakout", COLOR_ORANGE)
            .precision(0)
            .build()
    );

    // Volatility Breakout Expansion
    catalog.insert(BarIndicatorId::Vbexp,
        RenderingMetadata::builder("VBEXP")
            .sub_pane()
            .line_output("vbexp", "Vol Expansion", COLOR_RED)
            .precision(4)
            .build()
    );

    // Volatility of Donchian Channel
    catalog.insert(BarIndicatorId::VoDc,
        RenderingMetadata::builder("VO_DC")
            .sub_pane()
            .line_output("vo_dc", "Vol DC", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Volatility of Keltner Channel
    catalog.insert(BarIndicatorId::VoKc,
        RenderingMetadata::builder("VO_KC")
            .sub_pane()
            .line_output("vo_kc", "Vol KC", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Volatility of Market Index
    catalog.insert(BarIndicatorId::VoMi,
        RenderingMetadata::builder("VO_MI")
            .sub_pane()
            .line_output("vo_mi", "Vol MI", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Volatility of Volatility Ratio
    catalog.insert(BarIndicatorId::VoVr,
        RenderingMetadata::builder("VO_VR")
            .sub_pane()
            .line_output("vo_vr", "Vol Ratio", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Volatility of Volatility
    catalog.insert(BarIndicatorId::Vov,
        RenderingMetadata::builder("VOV")
            .sub_pane()
            .line_output("vov", "Vol of Vol", COLOR_PURPLE)
            .precision(6)
            .build()
    );

    // Volatility of Volatility Percentile
    catalog.insert(BarIndicatorId::Vovp,
        RenderingMetadata::builder("VOVP")
            .sub_pane()
            .line_output("vovp", "VoV Percentile", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Volatility of Volatility Percentile Trailing
    catalog.insert(BarIndicatorId::Vovpt,
        RenderingMetadata::builder("VOVPT")
            .sub_pane()
            .line_output("vovpt", "VoV % Trail", COLOR_PURPLE)
            .precision(2)
            .build()
    );

    // Volatility Percentile Rank Bands
    catalog.insert(BarIndicatorId::Vprb,
        RenderingMetadata::builder("VPRB")
            .sub_pane()
            .output(OutputSpec::line("upper", "Upper", COLOR_RED, 1.0, ValueExtractor::Channel(ChannelPart::Upper)))
            .output(OutputSpec::line("vol", "Vol", COLOR_BLUE, 2.0, ValueExtractor::Channel(ChannelPart::Middle)))
            .output(OutputSpec::line("lower", "Lower", COLOR_GREEN, 1.0, ValueExtractor::Channel(ChannelPart::Lower)))
            .precision(4)
            .build()
    );

    // Williams Vix Fix
    catalog.insert(BarIndicatorId::Wvf,
        RenderingMetadata::builder("WVF")
            .sub_pane()
            .line_output("wvf", "Williams VixFix", COLOR_RED)
            .precision(4)
            .build()
    );
}

// ============================================================================
// VOLUME INDICATORS
// ============================================================================

fn register_volume_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Volume

    // OBV - On Balance Volume
    catalog.insert(BarIndicatorId::Obv,
        RenderingMetadata::builder("OBV")
            .sub_pane()
            .line_output("obv", "OBV", COLOR_BLUE)
            .precision(0)
            .build()
    );

    // A/D Line - Accumulation/Distribution
    catalog.insert(BarIndicatorId::Ad,
        RenderingMetadata::builder("AD")
            .sub_pane()
            .line_output("ad", "A/D Line", COLOR_GREEN)
            .precision(0)
            .build()
    );

    // CMF - Chaikin Money Flow
    catalog.insert(BarIndicatorId::Cmf,
        RenderingMetadata::builder("CMF")
            .sub_pane()
            .line_output("cmf", "CMF", COLOR_TEAL)
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // Force Index
    catalog.insert(BarIndicatorId::Fi,
        RenderingMetadata::builder("FI")
            .sub_pane()
            .line_output("fi", "Force Index", COLOR_PURPLE)
            .zero_baseline()
            .precision(0)
            .build()
    );

    // Ease of Movement

    // Volume Rate of Change
    catalog.insert(BarIndicatorId::Vroc,
        RenderingMetadata::builder("VROC")
            .sub_pane()
            .line_output("vroc", "Vol ROC", COLOR_ORANGE)
            .zero_baseline()
            .precision(2)
            .build()
    );

    // Positive/Negative Volume Index


    // VWAP-based indicators

    // Volume Profile

    // Klinger Volume Oscillator
    catalog.insert(BarIndicatorId::Kvo,
        RenderingMetadata::builder("KVO")
            .sub_pane()
            .output(OutputSpec::line("kvo", "KVO", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("signal", "Signal", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .zero_baseline()
            .precision(0)
            .build()
    );

    // Elder Force Index

    // Volume Weighted MACD

    // Twiggs Money Flow
    catalog.insert(BarIndicatorId::Tmf,
        RenderingMetadata::builder("TMF")
            .sub_pane()
            .line_output("tmf", "Twiggs MF", COLOR_PURPLE)
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // === MISSING VOLUME INDICATORS ===

    // NVI/PVI Combined
    catalog.insert(BarIndicatorId::NviPvi,
        RenderingMetadata::builder("NVI_PVI")
            .sub_pane()
            .output(OutputSpec::line("nvi", "NVI", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("pvi", "PVI", COLOR_GREEN, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Price Volume Oscillator
    catalog.insert(BarIndicatorId::Pvo,
        RenderingMetadata::builder("PVO")
            .sub_pane()
            .line_output("pvo", "PVO", COLOR_PURPLE)
            .zero_baseline()
            .precision(2)
            .build()
    );

    // Price Zone Oscillator
    catalog.insert(BarIndicatorId::Pzo,
        RenderingMetadata::builder("PZO")
            .sub_pane()
            .line_output("pzo", "PZO", COLOR_BLUE)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // TRIN (Arms Index)
    catalog.insert(BarIndicatorId::Trin,
        RenderingMetadata::builder("TRIN")
            .sub_pane()
            .line_output("trin", "TRIN", COLOR_ORANGE)
            .reference_line(ReferenceLine::new(1.0, COLOR_GRAY))
            .precision(2)
            .build()
    );

    // Volume Delta
    catalog.insert(BarIndicatorId::Vdelta,
        RenderingMetadata::builder("VDELTA")
            .sub_pane()
            .output(OutputSpec::histogram("delta", "Vol Delta", COLOR_TEAL, ValueExtractor::Main))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Volume Flow Indicator
    catalog.insert(BarIndicatorId::Vfi,
        RenderingMetadata::builder("VFI")
            .sub_pane()
            .line_output("vfi", "VFI", COLOR_CYAN)
            .zero_baseline()
            .precision(2)
            .build()
    );

    // Volume Oscillator
    catalog.insert(BarIndicatorId::Vo,
        RenderingMetadata::builder("VO")
            .sub_pane()
            .line_output("vo", "Vol Osc", COLOR_PURPLE)
            .zero_baseline()
            .precision(2)
            .build()
    );

    // VPIN
    catalog.insert(BarIndicatorId::Vpin,
        RenderingMetadata::builder("VPIN")
            .sub_pane()
            .line_output("vpin", "VPIN", COLOR_RED)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Volume Profile
    catalog.insert(BarIndicatorId::Vprofile,
        RenderingMetadata::builder("VPROFILE")
            .sub_pane()
            .output(OutputSpec::histogram("profile", "Vol Profile", COLOR_BLUE_GRAY, ValueExtractor::Main))
            .histogram_style(HistogramStyle::FromBottom)
            .precision(0)
            .build()
    );

    // Volume Price Trend
    catalog.insert(BarIndicatorId::Vpt,
        RenderingMetadata::builder("VPT")
            .sub_pane()
            .line_output("vpt", "VPT", COLOR_BLUE)
            .precision(0)
            .build()
    );

    // Volume Z-Score
    catalog.insert(BarIndicatorId::Vz,
        RenderingMetadata::builder("VZ")
            .sub_pane()
            .line_output("vz", "Vol Z", COLOR_ORANGE)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .reference_line(ReferenceLine::new(2.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-2.0, COLOR_OVERSOLD))
            .precision(2)
            .build()
    );

    // Volume Zone Oscillator
    catalog.insert(BarIndicatorId::Vzo,
        RenderingMetadata::builder("VZO")
            .sub_pane()
            .line_output("vzo", "VZO", COLOR_TEAL)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );
}

// ============================================================================
// TREND INDICATORS
// ============================================================================

fn register_trend_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Supertrend (overlay)
    catalog.insert(BarIndicatorId::Supertrend,
        RenderingMetadata::builder("SUPERTREND")
            .overlay()
            .line_output("supertrend", "Supertrend", COLOR_GREEN)
            .precision(4)
            .build()
    );

    // Parabolic SAR (overlay)
    catalog.insert(BarIndicatorId::Psar,
        RenderingMetadata::builder("PSAR")
            .overlay()
            .output(OutputSpec::line("psar", "PSAR", COLOR_PURPLE, 0.0, ValueExtractor::Main).with_style(LineStyle::Dotted))
            .precision(4)
            .build()
    );

    // ADX (already in momentum, but can also be here for trend)

    // DMI - Directional Movement Index

    // Trend Strength

    // Linear Regression Slope
    catalog.insert(BarIndicatorId::LrSlope,
        RenderingMetadata::builder("LR_SLOPE")
            .sub_pane()
            .line_output("slope", "LR Slope", COLOR_ORANGE)
            .zero_baseline()
            .precision(6)
            .build()
    );

    // Trend Intensity Index
    catalog.insert(BarIndicatorId::Tii,
        RenderingMetadata::builder("TII")
            .sub_pane()
            .line_output("tii", "TII", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(20.0, COLOR_OVERSOLD))
            .precision(2)
            .build()
    );

    // Choppiness Index
    catalog.insert(BarIndicatorId::Chop,
        RenderingMetadata::builder("CHOP")
            .sub_pane()
            .line_output("chop", "Choppiness", COLOR_TEAL)
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(61.8, COLOR_OVERBOUGHT).with_label("Choppy"))
            .reference_line(ReferenceLine::new(38.2, COLOR_OVERSOLD).with_label("Trending"))
            .precision(2)
            .build()
    );

    // Detrended Price Oscillator

    // QQE - Qualitative Quantitative Estimation
    catalog.insert(BarIndicatorId::Qqe,
        RenderingMetadata::builder("QQE")
            .sub_pane()
            .output(OutputSpec::line("qqe", "QQE", COLOR_BLUE, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("smoothed", "Smoothed RSI", COLOR_ORANGE, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // Schaff Trend Cycle
    catalog.insert(BarIndicatorId::Stc,
        RenderingMetadata::builder("STC")
            .sub_pane()
            .line_output("stc", "STC", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .overbought_oversold(75.0, 25.0)
            .precision(2)
            .build()
    );

    // Trend Magic (overlay)

    // Half Trend (overlay)

    // SSL Channel (overlay)
    catalog.insert(BarIndicatorId::Ssl,
        RenderingMetadata::builder("SSL")
            .overlay()
            .output(OutputSpec::line("ssl_up", "SSL Up", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("ssl_down", "SSL Down", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // === MISSING TREND INDICATORS ===

    // ADX Slope
    catalog.insert(BarIndicatorId::AdxSlope,
        RenderingMetadata::builder("ADX_SLOPE")
            .sub_pane()
            .line_output("slope", "ADX Slope", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // DIDI Index
    catalog.insert(BarIndicatorId::Didi,
        RenderingMetadata::builder("DIDI")
            .sub_pane()
            .output(OutputSpec::line("short", "Short", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("long", "Long", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Gann HiLo Activator
    catalog.insert(BarIndicatorId::GannHilo,
        RenderingMetadata::builder("GANN_HILO")
            .overlay()
            .line_output("hilo", "Gann HiLo", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Heikin Ashi Trend
    catalog.insert(BarIndicatorId::HaTrend,
        RenderingMetadata::builder("HA_TREND")
            .sub_pane()
            .output(OutputSpec::histogram("trend", "HA Trend", COLOR_BLUE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // KAMA Slope
    catalog.insert(BarIndicatorId::KamaSlope,
        RenderingMetadata::builder("KAMA_SLOPE")
            .sub_pane()
            .line_output("slope", "KAMA Slope", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // RAVI
    catalog.insert(BarIndicatorId::Ravi,
        RenderingMetadata::builder("RAVI")
            .sub_pane()
            .line_output("ravi", "RAVI", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Trend vs Efficiency Ratio
    catalog.insert(BarIndicatorId::TrEr,
        RenderingMetadata::builder("TR_ER")
            .sub_pane()
            .line_output("ratio", "Trend/ER", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Zero Lag LSMA
    catalog.insert(BarIndicatorId::Zlsma,
        RenderingMetadata::builder("ZLSMA")
            .overlay()
            .line_output("zlsma", "ZLSMA", COLOR_CYAN)
            .precision(4)
            .build()
    );
}

// ============================================================================
// LEVELS INDICATORS (Support/Resistance)
// ============================================================================

fn register_levels_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Pivot Points (overlay) - returns Triple(R1, S1, Pivot)
    catalog.insert(BarIndicatorId::Pivot,
        RenderingMetadata::builder("PIVOT")
            .overlay()
            .output(OutputSpec::line("pivot", "Pivot", COLOR_BLUE, 2.0, ValueExtractor::Triple(TriplePart::Third)))
            .output(OutputSpec::line("r1", "R1", COLOR_RED, 1.0, ValueExtractor::Triple(TriplePart::First)).with_style(LineStyle::Dashed))
            .output(OutputSpec::line("s1", "S1", COLOR_GREEN, 1.0, ValueExtractor::Triple(TriplePart::Second)).with_style(LineStyle::Dashed))
            .precision(4)
            .build()
    );

    // Fibonacci Levels (overlay)

    // Support/Resistance (overlay)

    // VWAP Levels (overlay)
    catalog.insert(BarIndicatorId::VwapLevels,
        RenderingMetadata::builder("VWAPLEVELS")
            .overlay()
            .line_output("vwap", "VWAP Levels", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Floor Trader Pivots (overlay)
    catalog.insert(BarIndicatorId::Floorpivot,
        RenderingMetadata::builder("FLOORPIVOT")
            .overlay()
            .line_output("pivot", "Floor Pivot", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Camarilla Pivots (overlay)
    catalog.insert(BarIndicatorId::Camarilla,
        RenderingMetadata::builder("CAMARILLA")
            .overlay()
            .line_output("pivot", "Camarilla", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Woodie Pivots (overlay)
    catalog.insert(BarIndicatorId::Woodie,
        RenderingMetadata::builder("WOODIE")
            .overlay()
            .line_output("pivot", "Woodie", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // === MISSING LEVELS INDICATORS ===

    // Anchored VWAP
    catalog.insert(BarIndicatorId::Avwap,
        RenderingMetadata::builder("AVWAP")
            .overlay()
            .line_output("avwap", "AVWAP", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Anchored VWAP Reversal
    catalog.insert(BarIndicatorId::Avwaprev,
        RenderingMetadata::builder("AVWAPREV")
            .overlay()
            .line_output("avwap", "AVWAP Rev", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // AVWAP Touch
    catalog.insert(BarIndicatorId::Avwaptouch,
        RenderingMetadata::builder("AVWAPTOUCH")
            .sub_pane()
            .output(OutputSpec::histogram("touch", "AVWAP Touch", COLOR_ORANGE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Fair Value Gap
    catalog.insert(BarIndicatorId::Fvg,
        RenderingMetadata::builder("FVG")
            .overlay()
            .output(OutputSpec::line("upper", "FVG Upper", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("lower", "FVG Lower", COLOR_GREEN, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // FVG Alternative
    catalog.insert(BarIndicatorId::Fvgalt,
        RenderingMetadata::builder("FVGALT")
            .overlay()
            .line_output("fvg", "FVG Alt", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // FVG Duration
    catalog.insert(BarIndicatorId::Fvgdur,
        RenderingMetadata::builder("FVGDUR")
            .sub_pane()
            .line_output("duration", "FVG Duration", COLOR_PURPLE)
            .precision(0)
            .build()
    );

    // FVG Reversal
    catalog.insert(BarIndicatorId::Fvgrev,
        RenderingMetadata::builder("FVGREV")
            .sub_pane()
            .output(OutputSpec::histogram("reversal", "FVG Rev", COLOR_TEAL, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Liquidity Gap
    catalog.insert(BarIndicatorId::Liqgap,
        RenderingMetadata::builder("LIQGAP")
            .overlay()
            .line_output("gap", "Liq Gap", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Pivot AVWAP
    catalog.insert(BarIndicatorId::Pivavwap,
        RenderingMetadata::builder("PIVAVWAP")
            .overlay()
            .line_output("avwap", "Pivot AVWAP", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Range Mid
    catalog.insert(BarIndicatorId::Rmid,
        RenderingMetadata::builder("RMID")
            .overlay()
            .line_output("mid", "Range Mid", COLOR_GRAY)
            .precision(4)
            .build()
    );

    // Range Quartiles
    catalog.insert(BarIndicatorId::Rquart,
        RenderingMetadata::builder("RQUART")
            .overlay()
            .output(OutputSpec::line("q1", "Q1", COLOR_GREEN, 1.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("q2", "Q2", COLOR_BLUE, 1.0, ValueExtractor::Triple(TriplePart::Second)))
            .output(OutputSpec::line("q3", "Q3", COLOR_RED, 1.0, ValueExtractor::Triple(TriplePart::Third)))
            .precision(4)
            .build()
    );

    // Swing Strength
    catalog.insert(BarIndicatorId::Swingstr,
        RenderingMetadata::builder("SWINGSTR")
            .sub_pane()
            .line_output("strength", "Swing Strength", COLOR_PURPLE)
            .precision(2)
            .build()
    );
}

// ============================================================================
// ENTROPY INDICATORS
// ============================================================================

fn register_entropy_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Shannon Entropy
    catalog.insert(BarIndicatorId::Shannon,
        RenderingMetadata::builder("SHANNON")
            .sub_pane()
            .line_output("entropy", "Shannon Entropy", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Sample Entropy
    catalog.insert(BarIndicatorId::Sampen,
        RenderingMetadata::builder("SAMPEN")
            .sub_pane()
            .line_output("sampen", "Sample Entropy", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Approximate Entropy
    catalog.insert(BarIndicatorId::Apen,
        RenderingMetadata::builder("APEN")
            .sub_pane()
            .line_output("apen", "Approx Entropy", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Permutation Entropy

    // Fuzzy Entropy

    // Multiscale Entropy

    // === MISSING ENTROPY INDICATORS ===

    // Conditional Entropy
    catalog.insert(BarIndicatorId::Conden,
        RenderingMetadata::builder("CONDEN")
            .sub_pane()
            .line_output("conden", "Conditional Entropy", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Information Gain
    catalog.insert(BarIndicatorId::Infog,
        RenderingMetadata::builder("INFOG")
            .sub_pane()
            .line_output("infog", "Info Gain", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Jensen-Shannon Divergence
    catalog.insert(BarIndicatorId::Jsd,
        RenderingMetadata::builder("JSD")
            .sub_pane()
            .line_output("jsd", "JS Divergence", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Kullback-Leibler Divergence
    catalog.insert(BarIndicatorId::Kld,
        RenderingMetadata::builder("KLD")
            .sub_pane()
            .line_output("kld", "KL Divergence", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Mutual Information
    catalog.insert(BarIndicatorId::Mi,
        RenderingMetadata::builder("MI")
            .sub_pane()
            .line_output("mi", "Mutual Info", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Permutation Entropy (alternate)
    catalog.insert(BarIndicatorId::Pe,
        RenderingMetadata::builder("PE")
            .sub_pane()
            .line_output("pe", "Perm Entropy", COLOR_INDIGO)
            .precision(4)
            .build()
    );

    // Transfer Entropy
    catalog.insert(BarIndicatorId::Te,
        RenderingMetadata::builder("TE")
            .sub_pane()
            .line_output("te", "Transfer Entropy", COLOR_PINK)
            .precision(4)
            .build()
    );

    // Cross Mutual Information
    catalog.insert(BarIndicatorId::Xmil,
        RenderingMetadata::builder("XMIL")
            .sub_pane()
            .line_output("xmil", "Cross MI", COLOR_PURPLE)
            .precision(4)
            .build()
    );
}

// ============================================================================
// KALMAN INDICATORS
// ============================================================================

fn register_kalman_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Kalman Filter (overlay)
    catalog.insert(BarIndicatorId::Kalman,
        RenderingMetadata::builder("KALMAN")
            .overlay()
            .line_output("kalman", "Kalman", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Kalman Trend (overlay)

    // Kalman with Volatility

    // Extended Kalman Filter (overlay)
    catalog.insert(BarIndicatorId::Ekf,
        RenderingMetadata::builder("EKF")
            .overlay()
            .line_output("ekf", "EKF", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Unscented Kalman Filter (overlay)
    catalog.insert(BarIndicatorId::Ukf,
        RenderingMetadata::builder("UKF")
            .overlay()
            .line_output("ukf", "UKF", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // === MISSING KALMAN INDICATORS ===

    // Alpha-Beta-Gamma Filter
    catalog.insert(BarIndicatorId::Abgfilter,
        RenderingMetadata::builder("ABGFILTER")
            .overlay()
            .line_output("abg", "ABG Filter", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Kalman Composite
    catalog.insert(BarIndicatorId::Kcomp,
        RenderingMetadata::builder("KCOMP")
            .overlay()
            .line_output("comp", "Kalman Composite", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Kalman Regime
    catalog.insert(BarIndicatorId::Kregime,
        RenderingMetadata::builder("KREGIME")
            .sub_pane()
            .output(OutputSpec::histogram("regime", "Kalman Regime", COLOR_BLUE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Kalman Score
    catalog.insert(BarIndicatorId::Kscr,
        RenderingMetadata::builder("KSCR")
            .sub_pane()
            .line_output("score", "Kalman Score", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Kalman Slope
    catalog.insert(BarIndicatorId::Kslope,
        RenderingMetadata::builder("KSLOPE")
            .sub_pane()
            .line_output("slope", "Kalman Slope", COLOR_TEAL)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Kalman Slope Z
    catalog.insert(BarIndicatorId::Kslopez,
        RenderingMetadata::builder("KSLOPEZ")
            .sub_pane()
            .line_output("slopez", "Kalman Slope Z", COLOR_PURPLE)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .reference_line(ReferenceLine::new(2.0, COLOR_OVERBOUGHT))
            .reference_line(ReferenceLine::new(-2.0, COLOR_OVERSOLD))
            .precision(2)
            .build()
    );

    // Particle Filter
    catalog.insert(BarIndicatorId::Particle,
        RenderingMetadata::builder("PARTICLE")
            .overlay()
            .line_output("particle", "Particle Filter", COLOR_PINK)
            .precision(4)
            .build()
    );

    // Rauch-Tung-Striebel Smoother
    catalog.insert(BarIndicatorId::Rts,
        RenderingMetadata::builder("RTS")
            .overlay()
            .line_output("rts", "RTS Smoother", COLOR_INDIGO)
            .precision(4)
            .build()
    );
}

// ============================================================================
// SIGNAL PROCESSING INDICATORS
// ============================================================================

fn register_signal_processing_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Hilbert Transform

    // Instantaneous Trendline (overlay)

    // Dominant Cycle Period

    // === MISSING SIGNAL PROCESSING INDICATORS ===

    // Butterworth Filter
    catalog.insert(BarIndicatorId::Butter,
        RenderingMetadata::builder("BUTTER")
            .overlay()
            .line_output("butter", "Butterworth", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Chebyshev Filter
    catalog.insert(BarIndicatorId::Cheby,
        RenderingMetadata::builder("CHEBY")
            .overlay()
            .line_output("cheby", "Chebyshev", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // CUSUM
    catalog.insert(BarIndicatorId::Cusum,
        RenderingMetadata::builder("CUSUM")
            .sub_pane()
            .line_output("cusum", "CUSUM", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Cyber Cycle
    catalog.insert(BarIndicatorId::Cyber,
        RenderingMetadata::builder("CYBER")
            .sub_pane()
            .line_output("cyber", "Cyber Cycle", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Decycler
    catalog.insert(BarIndicatorId::Decyc,
        RenderingMetadata::builder("DECYC")
            .overlay()
            .line_output("decyc", "Decycler", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Even Better Sinewave
    catalog.insert(BarIndicatorId::Esine,
        RenderingMetadata::builder("ESINE")
            .sub_pane()
            .line_output("esine", "Even Sinewave", COLOR_BLUE)
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // Ehlers Super Smoother
    catalog.insert(BarIndicatorId::Ess,
        RenderingMetadata::builder("ESS")
            .overlay()
            .line_output("ess", "Super Smoother", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // FFT
    catalog.insert(BarIndicatorId::Fft,
        RenderingMetadata::builder("FFT")
            .sub_pane()
            .line_output("fft", "FFT", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Hampel Filter
    catalog.insert(BarIndicatorId::Hampel,
        RenderingMetadata::builder("HAMPEL")
            .overlay()
            .line_output("hampel", "Hampel", COLOR_GREEN)
            .precision(4)
            .build()
    );

    // Homodyne Discriminator
    catalog.insert(BarIndicatorId::Hdc,
        RenderingMetadata::builder("HDC")
            .sub_pane()
            .line_output("hdc", "Homodyne DC", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Hilbert Transform
    catalog.insert(BarIndicatorId::Hilb,
        RenderingMetadata::builder("HILB")
            .sub_pane()
            .output(OutputSpec::line("amplitude", "Amplitude", COLOR_BLUE, 2.0, ValueExtractor::Hilbert(HilbertPart::Amplitude)))
            .output(OutputSpec::line("phase", "Phase", COLOR_ORANGE, 1.0, ValueExtractor::Hilbert(HilbertPart::Phase)))
            .output(OutputSpec::line("frequency", "Frequency", COLOR_GREEN, 1.0, ValueExtractor::Hilbert(HilbertPart::Frequency)))
            .precision(4)
            .build()
    );

    // Hilbert Momentum
    catalog.insert(BarIndicatorId::Hmom,
        RenderingMetadata::builder("HMOM")
            .sub_pane()
            .line_output("hmom", "Hilbert Mom", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Hysteresis
    catalog.insert(BarIndicatorId::Hyst,
        RenderingMetadata::builder("HYST")
            .sub_pane()
            .line_output("hyst", "Hysteresis", COLOR_ORANGE)
            .precision(2)
            .build()
    );

    // Logic gates
    for id in [BarIndicatorId::Logicand, BarIndicatorId::Logicor, BarIndicatorId::Logicxor, BarIndicatorId::Logicsign] {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("value", format!("{:?}", id), COLOR_BLUE)
                .bounds(-1.0, 1.0)
                .precision(0)
                .build()
        );
    }

    // Lempel-Ziv Complexity
    catalog.insert(BarIndicatorId::Lz,
        RenderingMetadata::builder("LZ")
            .sub_pane()
            .line_output("lz", "Lempel-Ziv", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Mean Reversion Filter
    catalog.insert(BarIndicatorId::Mrf,
        RenderingMetadata::builder("MRF")
            .sub_pane()
            .line_output("mrf", "Mean Rev Filter", COLOR_GREEN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Renko Charts
    catalog.insert(BarIndicatorId::Rc,
        RenderingMetadata::builder("RC")
            .overlay()
            .line_output("renko", "Renko", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Regime Composite V2 - returns Single (composite score)
    catalog.insert(BarIndicatorId::Rc2,
        RenderingMetadata::builder("RC2")
            .sub_pane()
            .line_output("rc2", "Regime Composite V2", COLOR_PURPLE)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Regime Composite V3 - returns Single (composite score)
    catalog.insert(BarIndicatorId::Rc3,
        RenderingMetadata::builder("RC3")
            .sub_pane()
            .line_output("rc3", "Regime Composite V3", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Regime Composite V4 - returns Single (composite score)
    catalog.insert(BarIndicatorId::Rc4,
        RenderingMetadata::builder("RC4")
            .sub_pane()
            .line_output("rc4", "Regime Composite V4", COLOR_ORANGE)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Roofing Filter
    catalog.insert(BarIndicatorId::Roof,
        RenderingMetadata::builder("ROOF")
            .sub_pane()
            .line_output("roof", "Roofing", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Signal Band Pass
    catalog.insert(BarIndicatorId::Sbp,
        RenderingMetadata::builder("SBP")
            .sub_pane()
            .line_output("sbp", "Band Pass", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Signal Band Pass RHL
    catalog.insert(BarIndicatorId::Sbprhl,
        RenderingMetadata::builder("SBPRHL")
            .sub_pane()
            .line_output("sbprhl", "Band Pass RHL", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Signal Band Whittaker-Henderson Filter
    catalog.insert(BarIndicatorId::Sbwf,
        RenderingMetadata::builder("SBWF")
            .overlay()
            .line_output("sbwf", "Whittaker Filter", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Spectral Centroid Filter
    catalog.insert(BarIndicatorId::Scf,
        RenderingMetadata::builder("SCF")
            .sub_pane()
            .line_output("scf", "Spectral Centroid", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Spectral Crest
    catalog.insert(BarIndicatorId::Screst,
        RenderingMetadata::builder("SCREST")
            .sub_pane()
            .line_output("screst", "Spectral Crest", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Spectral Crest Percent
    catalog.insert(BarIndicatorId::Screstp,
        RenderingMetadata::builder("SCRESTP")
            .sub_pane()
            .line_output("screstp", "Spectral Crest %", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Sentiment indicators
    catalog.insert(BarIndicatorId::Sent,
        RenderingMetadata::builder("SENT")
            .sub_pane()
            .line_output("sent", "Sentiment", COLOR_BLUE)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    catalog.insert(BarIndicatorId::Sentent,
        RenderingMetadata::builder("SENTENT")
            .sub_pane()
            .line_output("sentent", "Sentiment Entropy", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Sentr,
        RenderingMetadata::builder("SENTR")
            .sub_pane()
            .line_output("sentr", "Sentiment Rate", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Standard Error
    catalog.insert(BarIndicatorId::Ser,
        RenderingMetadata::builder("SER")
            .sub_pane()
            .line_output("ser", "Std Error", COLOR_RED)
            .precision(4)
            .build()
    );

    // Spectral Flatness
    catalog.insert(BarIndicatorId::Sflat,
        RenderingMetadata::builder("SFLAT")
            .sub_pane()
            .line_output("sflat", "Spectral Flatness", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Sflatp,
        RenderingMetadata::builder("SFLATP")
            .sub_pane()
            .line_output("sflatp", "Spectral Flatness %", COLOR_TEAL)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Spectral Flux
    catalog.insert(BarIndicatorId::Sflux,
        RenderingMetadata::builder("SFLUX")
            .sub_pane()
            .line_output("sflux", "Spectral Flux", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Savitzky-Golay Filter
    catalog.insert(BarIndicatorId::Sg,
        RenderingMetadata::builder("SG")
            .overlay()
            .line_output("sg", "Savitzky-Golay", COLOR_GREEN)
            .precision(4)
            .build()
    );

    // Spectral High/Low Mean Power Ratio
    catalog.insert(BarIndicatorId::Shmpr,
        RenderingMetadata::builder("SHMPR")
            .sub_pane()
            .line_output("shmpr", "High/Low MPR", COLOR_BLUE)
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Slmpr,
        RenderingMetadata::builder("SLMPR")
            .sub_pane()
            .line_output("slmpr", "Low Mean Power", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Spectral Rolloff
    catalog.insert(BarIndicatorId::Sroll,
        RenderingMetadata::builder("SROLL")
            .sub_pane()
            .line_output("sroll", "Spectral Rolloff", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Sroll95,
        RenderingMetadata::builder("SROLL95")
            .sub_pane()
            .line_output("sroll95", "Spectral Rolloff 95%", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Srollp,
        RenderingMetadata::builder("SROLLP")
            .sub_pane()
            .line_output("srollp", "Spectral Rolloff %", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    catalog.insert(BarIndicatorId::Srollrp,
        RenderingMetadata::builder("SROLLRP")
            .sub_pane()
            .line_output("srollrp", "Spectral Rolloff RP", COLOR_INDIGO)
            .precision(4)
            .build()
    );

    // Spectral Slope
    catalog.insert(BarIndicatorId::Sslope,
        RenderingMetadata::builder("SSLOPE")
            .sub_pane()
            .line_output("sslope", "Spectral Slope", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Sslopep,
        RenderingMetadata::builder("SSLOPEP")
            .sub_pane()
            .line_output("sslopep", "Spectral Slope %", COLOR_ORANGE)
            .bounds(-100.0, 100.0)
            .precision(2)
            .build()
    );

    catalog.insert(BarIndicatorId::Ssloperp,
        RenderingMetadata::builder("SSLOPERP")
            .sub_pane()
            .line_output("ssloperp", "Spectral Slope RP", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    catalog.insert(BarIndicatorId::Sslopez,
        RenderingMetadata::builder("SSLOPEZ")
            .sub_pane()
            .line_output("sslopez", "Spectral Slope Z", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Short-Time Fourier Transform
    catalog.insert(BarIndicatorId::Stft,
        RenderingMetadata::builder("STFT")
            .sub_pane()
            .line_output("stft", "STFT", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Trend Encoder
    catalog.insert(BarIndicatorId::Tenc,
        RenderingMetadata::builder("TENC")
            .sub_pane()
            .line_output("tenc", "Trend Encoder", COLOR_BLUE)
            .bounds(-1.0, 1.0)
            .precision(2)
            .build()
    );

    // Threshold
    catalog.insert(BarIndicatorId::Thresh,
        RenderingMetadata::builder("THRESH")
            .sub_pane()
            .line_output("thresh", "Threshold", COLOR_RED)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // Wave
    catalog.insert(BarIndicatorId::Wave,
        RenderingMetadata::builder("WAVE")
            .sub_pane()
            .line_output("wave", "Wave", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Wavelet Compression
    catalog.insert(BarIndicatorId::Wcomp,
        RenderingMetadata::builder("WCOMP")
            .sub_pane()
            .line_output("wcomp", "Wavelet Compress", COLOR_INDIGO)
            .precision(4)
            .build()
    );

    // Z-Score MAD
    catalog.insert(BarIndicatorId::Zmad,
        RenderingMetadata::builder("ZMAD")
            .sub_pane()
            .line_output("zmad", "Z-Score MAD", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // ========== LOGIC GATES ==========

    // AND Gate (returns Flag/bool)
    catalog.insert(BarIndicatorId::Logicand,
        RenderingMetadata::builder("LOGICAND")
            .sub_pane()
            .output(OutputSpec::line("and", "AND Gate", COLOR_GREEN, 2.0, ValueExtractor::Flag))
            .bounds(0.0, 1.0)
            .build()
    );

    // OR Gate (returns Flag/bool)
    catalog.insert(BarIndicatorId::Logicor,
        RenderingMetadata::builder("LOGICOR")
            .sub_pane()
            .output(OutputSpec::line("or", "OR Gate", COLOR_BLUE, 2.0, ValueExtractor::Flag))
            .bounds(0.0, 1.0)
            .build()
    );

    // XOR Gate (returns Flag/bool)
    catalog.insert(BarIndicatorId::Logicxor,
        RenderingMetadata::builder("LOGICXOR")
            .sub_pane()
            .output(OutputSpec::line("xor", "XOR Gate", COLOR_ORANGE, 2.0, ValueExtractor::Flag))
            .bounds(0.0, 1.0)
            .build()
    );

    // Sign Combiner (returns Signal i8: -1, 0, 1)
    catalog.insert(BarIndicatorId::Logicsign,
        RenderingMetadata::builder("LOGICSIGN")
            .sub_pane()
            .output(OutputSpec::line("sign", "Sign Combiner", COLOR_PURPLE, 2.0, ValueExtractor::Signal))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .build()
    );
}

// ============================================================================
// CHAOS INDICATORS
// ============================================================================

fn register_chaos_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Hurst Exponent
    catalog.insert(BarIndicatorId::Hurst,
        RenderingMetadata::builder("HURST")
            .sub_pane()
            .line_output("hurst", "Hurst", COLOR_PURPLE)
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.5, COLOR_GRAY).with_label("Random Walk"))
            .precision(4)
            .build()
    );

    // Lyapunov Exponent

    // Fractal Dimension

    // DFA - Detrended Fluctuation Analysis
    catalog.insert(BarIndicatorId::Dfa,
        RenderingMetadata::builder("DFA")
            .sub_pane()
            .line_output("dfa", "DFA", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Recurrence Quantification

    // === MISSING CHAOS INDICATORS ===


    // Alligator (Bill Williams)
    catalog.insert(BarIndicatorId::Alligator,
        RenderingMetadata::builder("ALLIGATOR")
            .overlay()
            .output(OutputSpec::line("jaw", "Jaw", COLOR_BLUE, 2.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("teeth", "Teeth", COLOR_RED, 1.5, ValueExtractor::Triple(TriplePart::Second)))
            .output(OutputSpec::line("lips", "Lips", COLOR_GREEN, 1.0, ValueExtractor::Triple(TriplePart::Third)))
            .precision(4)
            .build()
    );


    // Chaos Oscillator
    catalog.insert(BarIndicatorId::ChaosOsc,
        RenderingMetadata::builder("CHAOS_OSC")
            .sub_pane()
            .line_output("chaos", "Chaos Osc", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // DFA Percent
    catalog.insert(BarIndicatorId::DfaPct,
        RenderingMetadata::builder("DFA_PCT")
            .sub_pane()
            .line_output("dfa_pct", "DFA %", COLOR_TEAL)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Fractal Dimension (alternate name)
    catalog.insert(BarIndicatorId::FractalDim,
        RenderingMetadata::builder("FRACTAL_DIM")
            .sub_pane()
            .line_output("fracdim", "Fractal Dim", COLOR_INDIGO)
            .bounds(1.0, 2.0)
            .reference_line(ReferenceLine::new(1.5, COLOR_GRAY))
            .precision(4)
            .build()
    );

    // Fractals (Bill Williams)
    catalog.insert(BarIndicatorId::Fractals,
        RenderingMetadata::builder("FRACTALS")
            .overlay()
            .output(OutputSpec::line("up", "Fractal Up", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("down", "Fractal Down", COLOR_RED, 2.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Hurst Percent
    catalog.insert(BarIndicatorId::HurstPct,
        RenderingMetadata::builder("HURST_PCT")
            .sub_pane()
            .line_output("hurst_pct", "Hurst %", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, COLOR_GRAY).with_label("Random"))
            .precision(2)
            .build()
    );

    // Williams Market Facilitation Index
    catalog.insert(BarIndicatorId::WilliamsMfi,
        RenderingMetadata::builder("WILLIAMS_MFI")
            .sub_pane()
            .output(OutputSpec::histogram("wmfi", "Williams MFI", COLOR_CYAN, ValueExtractor::Main))
            .precision(4)
            .build()
    );
}

// ============================================================================
// REGRESSION INDICATORS
// ============================================================================

fn register_regression_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Linear Regression (overlay)

    // Polynomial Regression (overlay)
    catalog.insert(BarIndicatorId::PolyReg,
        RenderingMetadata::builder("POLYREG")
            .overlay()
            .line_output("polyreg", "Poly Reg", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // R-Squared
    catalog.insert(BarIndicatorId::RSquared,
        RenderingMetadata::builder("RSQUARED")
            .sub_pane()
            .line_output("rsq", "R-Squared", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Standard Error

    // Linear Regression Angle

    // Linear Regression Intercept

    // ARIMA
    catalog.insert(BarIndicatorId::Arima,
        RenderingMetadata::builder("ARIMA")
            .overlay()
            .line_output("arima", "ARIMA", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // VAR
}

// ============================================================================
// ADAPTIVE INDICATORS
// ============================================================================

fn register_adaptive_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // KAMA - Kaufman's Adaptive MA (overlay)
    catalog.insert(BarIndicatorId::Kama,
        RenderingMetadata::builder("KAMA")
            .overlay()
            .line_output("kama", "KAMA", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // FRAMA - Fractal Adaptive MA (overlay)
    catalog.insert(BarIndicatorId::Frama,
        RenderingMetadata::builder("FRAMA")
            .overlay()
            .line_output("frama", "FRAMA", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // VIDYA (overlay)
    catalog.insert(BarIndicatorId::Vidya,
        RenderingMetadata::builder("VIDYA")
            .overlay()
            .line_output("vidya", "VIDYA", COLOR_CYAN)
            .precision(4)
            .build()
    );

    // Adaptive Period

    // Efficiency Ratio
    catalog.insert(BarIndicatorId::Er,
        RenderingMetadata::builder("ER")
            .sub_pane()
            .line_output("er", "Efficiency Ratio", COLOR_GREEN)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Fractal Efficiency

    // === MISSING ADAPTIVE INDICATORS ===

    // Adaptive MA
    catalog.insert(BarIndicatorId::Adaptivema,
        RenderingMetadata::builder("ADAPTIVEMA")
            .overlay()
            .line_output("ama", "Adaptive MA", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // MAMA - Mesa Adaptive Moving Average (returns Single)
    catalog.insert(BarIndicatorId::Mama,
        RenderingMetadata::builder("MAMA")
            .overlay()
            .line_output("mama", "MESA Adaptive MA", COLOR_BLUE)
            .precision(4)
            .build()
    );
}

// ============================================================================
// ACCUMULATION INDICATORS
// ============================================================================

fn register_accumulation_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // A/D Line

    // Chaikin A/D Oscillator

    // Williams A/D
    catalog.insert(BarIndicatorId::Wad,
        RenderingMetadata::builder("WAD")
            .sub_pane()
            .line_output("wad", "Williams A/D", COLOR_PURPLE)
            .precision(0)
            .build()
    );

    // Price Volume Trend
    catalog.insert(BarIndicatorId::Pvt,
        RenderingMetadata::builder("PVT")
            .sub_pane()
            .line_output("pvt", "PVT", COLOR_TEAL)
            .precision(0)
            .build()
    );

    // Net Volume

    // === MISSING ACCUMULATION INDICATORS ===

    // Accumulation Swing Index
    catalog.insert(BarIndicatorId::Asi,
        RenderingMetadata::builder("ASI")
            .sub_pane()
            .line_output("asi", "ASI", COLOR_BLUE)
            .zero_baseline()
            .precision(2)
            .build()
    );

    // Chaikin Oscillator
    catalog.insert(BarIndicatorId::Cho,
        RenderingMetadata::builder("CHO")
            .sub_pane()
            .line_output("cho", "Chaikin Osc", COLOR_PURPLE)
            .zero_baseline()
            .precision(0)
            .build()
    );

    // Directional Indicator (+DI/-DI from ADX)
    catalog.insert(BarIndicatorId::DiPlusMinus,
        RenderingMetadata::builder("DI_PLUS_MINUS")
            .sub_pane()
            .output(OutputSpec::line("plus", "+DI", COLOR_GREEN, 2.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("minus", "-DI", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Ease of Movement
    catalog.insert(BarIndicatorId::Eom,
        RenderingMetadata::builder("EOM")
            .sub_pane()
            .line_output("eom", "EoM", COLOR_CYAN)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Intraday Intensity
    catalog.insert(BarIndicatorId::Ii,
        RenderingMetadata::builder("II")
            .sub_pane()
            .line_output("ii", "Intraday Intensity", COLOR_ORANGE)
            .precision(0)
            .build()
    );

    // II Percent
    catalog.insert(BarIndicatorId::Iip,
        RenderingMetadata::builder("IIP")
            .sub_pane()
            .line_output("iip", "II %", COLOR_PURPLE)
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(2)
            .build()
    );

    // II Ratio
    catalog.insert(BarIndicatorId::Iir,
        RenderingMetadata::builder("IIR")
            .sub_pane()
            .line_output("iir", "II Ratio", COLOR_TEAL)
            .reference_line(ReferenceLine::new(1.0, COLOR_GRAY))
            .precision(4)
            .build()
    );
}

// ============================================================================
// BOOK INDICATORS
// ============================================================================

fn register_book_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Order Book Imbalance

    // Bid/Ask Spread

    // Depth Ratio

    // Book Imbalance Ratio

    // === MISSING BOOK INDICATORS ===

    // Book Imbalance
    catalog.insert(BarIndicatorId::BookImb,
        RenderingMetadata::builder("BOOK_IMB")
            .sub_pane()
            .output(OutputSpec::histogram("imb", "Book Imbalance", COLOR_BLUE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );

    // Book Slope
    catalog.insert(BarIndicatorId::BookSlope,
        RenderingMetadata::builder("BOOK_SLOPE")
            .sub_pane()
            .line_output("slope", "Book Slope", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Order Flow Imbalance
    catalog.insert(BarIndicatorId::Ofi,
        RenderingMetadata::builder("OFI")
            .sub_pane()
            .output(OutputSpec::histogram("ofi", "OFI", COLOR_ORANGE, ValueExtractor::Main))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Queue Imbalance
    catalog.insert(BarIndicatorId::QueueImb,
        RenderingMetadata::builder("QUEUE_IMB")
            .sub_pane()
            .output(OutputSpec::histogram("imb", "Queue Imbalance", COLOR_TEAL, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    );
}

// ============================================================================
// CANDLE INDICATORS
// ============================================================================

fn register_candle_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Heikin Ashi (overlay as candles)

    // Candle Pattern Score

    // Doji Detector
    catalog.insert(BarIndicatorId::Doji,
        RenderingMetadata::builder("DOJI")
            .sub_pane()
            .output(OutputSpec::histogram("doji", "Doji", COLOR_PURPLE, ValueExtractor::Main))
            .bounds(0.0, 1.0)
            .precision(0)
            .build()
    );

    // Body Size Ratio

    // Wick Ratio

    // Candle Direction

    // === MISSING CANDLE INDICATORS ===

    // Candle Anatomy - returns CandleAnatomy variant
    catalog.insert(BarIndicatorId::Candleanatomy,
        RenderingMetadata::builder("CANDLEANATOMY")
            .sub_pane()
            .output(OutputSpec::line("body", "Body", COLOR_BLUE, 2.0, ValueExtractor::CandleAnatomy(CandleAnatomyPart::Body)))
            .output(OutputSpec::line("upper", "Upper Wick", COLOR_GREEN, 1.0, ValueExtractor::CandleAnatomy(CandleAnatomyPart::UpperWick)))
            .output(OutputSpec::line("lower", "Lower Wick", COLOR_RED, 1.0, ValueExtractor::CandleAnatomy(CandleAnatomyPart::LowerWick)))
            .precision(4)
            .build()
    );

    // Dark Cloud Cover
    catalog.insert(BarIndicatorId::Darkcloudcover,
        RenderingMetadata::builder("DARKCLOUDCOVER")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Dark Cloud", COLOR_RED, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Engulfing Pattern
    catalog.insert(BarIndicatorId::Engulfing,
        RenderingMetadata::builder("ENGULFING")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Engulfing", COLOR_PURPLE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Evening Star
    catalog.insert(BarIndicatorId::Eveningstar,
        RenderingMetadata::builder("EVENINGSTAR")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Evening Star", COLOR_RED, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Hammer
    catalog.insert(BarIndicatorId::Hammer,
        RenderingMetadata::builder("HAMMER")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Hammer", COLOR_GREEN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Harami
    catalog.insert(BarIndicatorId::Harami,
        RenderingMetadata::builder("HARAMI")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Harami", COLOR_ORANGE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Heikin Ashi (alternate name)
    catalog.insert(BarIndicatorId::Heikinashi,
        RenderingMetadata::builder("HEIKINASHI")
            .overlay()
            .line_output("close", "Heikin Ashi", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Marubozu
    catalog.insert(BarIndicatorId::Marubozu,
        RenderingMetadata::builder("MARUBOZU")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Marubozu", COLOR_TEAL, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Morning Star
    catalog.insert(BarIndicatorId::Morningstar,
        RenderingMetadata::builder("MORNINGSTAR")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Morning Star", COLOR_GREEN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Pattern Recognition
    catalog.insert(BarIndicatorId::Patternrec,
        RenderingMetadata::builder("PATTERNREC")
            .sub_pane()
            .output(OutputSpec::histogram("pattern", "Pattern Rec", COLOR_PURPLE, ValueExtractor::Main))
            .bounds(-100.0, 100.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Piercing Pattern
    catalog.insert(BarIndicatorId::Piercingpattern,
        RenderingMetadata::builder("PIERCINGPATTERN")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Piercing", COLOR_GREEN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Swing Failure Pattern
    catalog.insert(BarIndicatorId::Sfp,
        RenderingMetadata::builder("SFP")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "SFP", COLOR_CYAN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Shooting Star
    catalog.insert(BarIndicatorId::Shootingstar,
        RenderingMetadata::builder("SHOOTINGSTAR")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Shooting Star", COLOR_RED, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Three Black Crows
    catalog.insert(BarIndicatorId::Threeblackcrows,
        RenderingMetadata::builder("THREEBLACKCROWS")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Three Black Crows", COLOR_RED, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Three White Soldiers
    catalog.insert(BarIndicatorId::Threewhitesoldiers,
        RenderingMetadata::builder("THREEWHITESOLDIERS")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Three White Soldiers", COLOR_GREEN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    );

    // Tweezer
    catalog.insert(BarIndicatorId::Tweezer,
        RenderingMetadata::builder("TWEEZER")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Tweezer", COLOR_ORANGE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Wick Spike
    catalog.insert(BarIndicatorId::Wickspike,
        RenderingMetadata::builder("WICKSPIKE")
            .sub_pane()
            .output(OutputSpec::histogram("signal", "Wick Spike", COLOR_PINK, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );
}

// ============================================================================
// CLUSTER INDICATORS
// ============================================================================

fn register_cluster_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Volume Cluster

    // Price Cluster

    // Market Profile POC (overlay)
    catalog.insert(BarIndicatorId::Poc,
        RenderingMetadata::builder("POC")
            .overlay()
            .line_output("poc", "POC", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Value Area High/Low (overlay)


    // === MISSING CLUSTER INDICATORS ===

    // Cluster Queue Imbalance
    catalog.insert(BarIndicatorId::ClQueueImb,
        RenderingMetadata::builder("CL_QUEUE_IMB")
            .sub_pane()
            .output(OutputSpec::histogram("imb", "Queue Imbalance", COLOR_ORANGE, ValueExtractor::Main))
            .zero_baseline()
            .precision(2)
            .build()
    );

    // Market Microstructure
    catalog.insert(BarIndicatorId::MarketMicro,
        RenderingMetadata::builder("MARKET_MICRO")
            .sub_pane()
            .line_output("micro", "Market Micro", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Order Book Slope
    catalog.insert(BarIndicatorId::OrderBookSlope,
        RenderingMetadata::builder("ORDER_BOOK_SLOPE")
            .sub_pane()
            .line_output("slope", "OB Slope", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Order Flow Imbalance
    catalog.insert(BarIndicatorId::OrderFlowImb,
        RenderingMetadata::builder("ORDER_FLOW_IMB")
            .sub_pane()
            .output(OutputSpec::histogram("imb", "Order Flow Imb", COLOR_TEAL, ValueExtractor::Main))
            .zero_baseline()
            .precision(2)
            .build()
    );

    // Tick Volume
    catalog.insert(BarIndicatorId::TickVolume,
        RenderingMetadata::builder("TICK_VOLUME")
            .sub_pane()
            .output(OutputSpec::histogram("volume", "Tick Volume", COLOR_BLUE_GRAY, ValueExtractor::Main))
            .histogram_style(HistogramStyle::FromBottom)
            .precision(0)
            .build()
    );

}

// ============================================================================
// DIVERGENCE INDICATORS
// ============================================================================

fn register_divergence_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // RSI Divergence
    catalog.insert(BarIndicatorId::RsiDiv,
        RenderingMetadata::builder("RSIDIV")
            .sub_pane()
            .output(OutputSpec::histogram("divergence", "RSI Divergence", COLOR_PURPLE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // MACD Divergence
    catalog.insert(BarIndicatorId::MacdDiv,
        RenderingMetadata::builder("MACDDIV")
            .sub_pane()
            .output(OutputSpec::histogram("divergence", "MACD Divergence", COLOR_BLUE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // OBV Divergence
    catalog.insert(BarIndicatorId::ObvDiv,
        RenderingMetadata::builder("OBVDIV")
            .sub_pane()
            .output(OutputSpec::histogram("divergence", "OBV Divergence", COLOR_GREEN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Stochastic Divergence
    catalog.insert(BarIndicatorId::StochDiv,
        RenderingMetadata::builder("STOCHDIV")
            .sub_pane()
            .output(OutputSpec::histogram("divergence", "Stoch Divergence", COLOR_ORANGE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // General Divergence Scanner

    // === MISSING DIVERGENCE INDICATORS ===

    // CCI Divergence
    catalog.insert(BarIndicatorId::CciDiv,
        RenderingMetadata::builder("CCI_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "CCI Divergence", COLOR_PURPLE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Classic Divergence
    catalog.insert(BarIndicatorId::ClassicDiv,
        RenderingMetadata::builder("CLASSIC_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "Classic Div", COLOR_BLUE, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Divergence Strength
    catalog.insert(BarIndicatorId::DivStrength,
        RenderingMetadata::builder("DIV_STRENGTH")
            .sub_pane()
            .line_output("strength", "Div Strength", COLOR_ORANGE)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Hidden Divergence
    catalog.insert(BarIndicatorId::HiddenDiv,
        RenderingMetadata::builder("HIDDEN_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "Hidden Div", COLOR_TEAL, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // MACD Histogram Divergence
    catalog.insert(BarIndicatorId::MacdHistDiv,
        RenderingMetadata::builder("MACD_HIST_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "MACD Hist Div", COLOR_INDIGO, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Multi Divergence
    catalog.insert(BarIndicatorId::MultiDiv,
        RenderingMetadata::builder("MULTI_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "Multi Div", COLOR_CYAN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );



    // Volume Divergence
    catalog.insert(BarIndicatorId::VolDiv,
        RenderingMetadata::builder("VOL_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "Vol Div", COLOR_TEAL, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Williams Divergence
    catalog.insert(BarIndicatorId::WilliamsDiv,
        RenderingMetadata::builder("WILLIAMS_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "Williams Div", COLOR_CYAN, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );

    // Zigzag Divergence
    catalog.insert(BarIndicatorId::ZigzagDiv,
        RenderingMetadata::builder("ZIGZAG_DIV")
            .sub_pane()
            .output(OutputSpec::histogram("div", "Zigzag Div", COLOR_PINK, ValueExtractor::Main))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    );
}

// ============================================================================
// RATIO INDICATORS
// ============================================================================

fn register_ratio_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Price Ratio

    // Relative Strength (pair)

    // Correlation

    // Beta

    // Alpha (Jensen's)

    // Cointegration (returns t-statistic only)
    catalog.insert(BarIndicatorId::Coint,
        RenderingMetadata::builder("COINT")
            .sub_pane()
            .output(OutputSpec::line("t_stat", "Coint T-Statistic", COLOR_BLUE, 2.0, ValueExtractor::Main))
            .zero_baseline()
            .precision(4)
            .build()
    );

    // === MISSING RATIO INDICATORS ===

    // Efficiency Ratio Ring
    catalog.insert(BarIndicatorId::ErRing,
        RenderingMetadata::builder("ER_RING")
            .sub_pane()
            .line_output("er", "ER Ring", COLOR_PURPLE)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Range/ATR Ratio
    catalog.insert(BarIndicatorId::RangeAtr,
        RenderingMetadata::builder("RANGE_ATR")
            .sub_pane()
            .line_output("ratio", "Range/ATR", COLOR_ORANGE)
            .reference_line(ReferenceLine::new(1.0, COLOR_GRAY))
            .precision(4)
            .build()
    );

    // Spread Analyzer
    catalog.insert(BarIndicatorId::SpreadAnalyzer,
        RenderingMetadata::builder("SPREAD_ANALYZER")
            .sub_pane()
            .line_output("spread", "Spread", COLOR_TEAL)
            .precision(4)
            .build()
    );
}

// ============================================================================
// TREND STOP INDICATORS
// ============================================================================

fn register_trend_stop_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Chandelier Exit (overlay)

    // ATR Trailing Stop (overlay)

    // Donchian Breakout (overlay)

    // Volatility Stop (overlay)

    // SafeZone Stop (overlay)

    // Keltner Stop (overlay)

    // === MISSING TREND STOP INDICATORS ===

    // ATR Trailing Stop
    catalog.insert(BarIndicatorId::Atrts,
        RenderingMetadata::builder("ATRTS")
            .overlay()
            .line_output("stop", "ATR Trail Stop", COLOR_RED)
            .precision(4)
            .build()
    );

    // Chandelier (alternate name)
    catalog.insert(BarIndicatorId::Chand,
        RenderingMetadata::builder("CHAND")
            .overlay()
            .output(OutputSpec::line("long", "Long Exit", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("short", "Short Exit", COLOR_GREEN, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // CKS Trend Stop
    catalog.insert(BarIndicatorId::Cks,
        RenderingMetadata::builder("CKS")
            .overlay()
            .line_output("stop", "CKS Stop", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Donchian Breakout
    catalog.insert(BarIndicatorId::Donbo,
        RenderingMetadata::builder("DONBO")
            .overlay()
            .output(OutputSpec::line("upper", "Upper", COLOR_GREEN, 1.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("lower", "Lower", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Donchian Stop
    catalog.insert(BarIndicatorId::Dons,
        RenderingMetadata::builder("DONS")
            .overlay()
            .line_output("stop", "Donchian Stop", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Keltner Stop (alternate name)
    catalog.insert(BarIndicatorId::Kelts,
        RenderingMetadata::builder("KELTS")
            .overlay()
            .line_output("stop", "Keltner Stop", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Parabolic SAR Stop
    catalog.insert(BarIndicatorId::Psars,
        RenderingMetadata::builder("PSARS")
            .overlay()
            .line_output("sar", "SAR Stop", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Support Stop
    catalog.insert(BarIndicatorId::Supts,
        RenderingMetadata::builder("SUPTS")
            .overlay()
            .line_output("stop", "Support Stop", COLOR_GREEN)
            .precision(4)
            .build()
    );

    // Trend Stop Swings
    catalog.insert(BarIndicatorId::TsSwings,
        RenderingMetadata::builder("TS_SWINGS")
            .overlay()
            .output(OutputSpec::line("high", "Swing High", COLOR_RED, 1.0, ValueExtractor::Double(DoublePart::First)))
            .output(OutputSpec::line("low", "Swing Low", COLOR_GREEN, 1.0, ValueExtractor::Double(DoublePart::Second)))
            .precision(4)
            .build()
    );

    // Volatility Trailing Stop
    catalog.insert(BarIndicatorId::Volts,
        RenderingMetadata::builder("VOLTS")
            .overlay()
            .line_output("stop", "Vol Trail Stop", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Volatility Trailing Stop ATR
    catalog.insert(BarIndicatorId::VoltsAtr,
        RenderingMetadata::builder("VOLTS_ATR")
            .overlay()
            .line_output("stop", "Vol ATR Stop", COLOR_RED)
            .precision(4)
            .build()
    );
}

// ============================================================================
// POSITION INDICATORS
// ============================================================================

fn register_position_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Position Size

    // Risk/Reward Ratio

    // Kelly Criterion

    // Optimal F

    // === MISSING POSITION INDICATORS ===

    // AVWAP Distance
    catalog.insert(BarIndicatorId::AvwapDist,
        RenderingMetadata::builder("AVWAP_DIST")
            .sub_pane()
            .line_output("dist", "AVWAP Distance", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Central Pivot Range
    catalog.insert(BarIndicatorId::Cpr,
        RenderingMetadata::builder("CPR")
            .overlay()
            .output(OutputSpec::line("pivot", "Pivot", COLOR_BLUE, 2.0, ValueExtractor::Triple(TriplePart::First)))
            .output(OutputSpec::line("bc", "BC", COLOR_GREEN, 1.0, ValueExtractor::Triple(TriplePart::Second)))
            .output(OutputSpec::line("tc", "TC", COLOR_RED, 1.0, ValueExtractor::Triple(TriplePart::Third)))
            .precision(4)
            .build()
    );

    // Day/Week/Month
    catalog.insert(BarIndicatorId::DayWeekMonth,
        RenderingMetadata::builder("DAY_WEEK_MONTH")
            .sub_pane()
            .line_output("pos", "Day/Week/Month", COLOR_PURPLE)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Distance to Levels
    catalog.insert(BarIndicatorId::DistLevels,
        RenderingMetadata::builder("DIST_LEVELS")
            .sub_pane()
            .line_output("dist", "Dist to Levels", COLOR_ORANGE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // DOM Width of Quote
    catalog.insert(BarIndicatorId::DomWoq,
        RenderingMetadata::builder("DOM_WOQ")
            .sub_pane()
            .line_output("woq", "DOM WoQ", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Holiday Proximity
    catalog.insert(BarIndicatorId::HolidayProx,
        RenderingMetadata::builder("HOLIDAY_PROX")
            .sub_pane()
            .line_output("prox", "Holiday Prox", COLOR_PINK)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // Hour of Day
    catalog.insert(BarIndicatorId::HourDay,
        RenderingMetadata::builder("HOUR_DAY")
            .sub_pane()
            .line_output("hour", "Hour of Day", COLOR_BLUE)
            .bounds(0.0, 24.0)
            .precision(0)
            .build()
    );

    // Month of Quarter
    catalog.insert(BarIndicatorId::MonthQtr,
        RenderingMetadata::builder("MONTH_QTR")
            .sub_pane()
            .line_output("month", "Month/Qtr", COLOR_PURPLE)
            .bounds(1.0, 3.0)
            .precision(0)
            .build()
    );

    // Month Turn
    catalog.insert(BarIndicatorId::MonthTurn,
        RenderingMetadata::builder("MONTH_TURN")
            .sub_pane()
            .line_output("turn", "Month Turn", COLOR_ORANGE)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // Quarter Turn
    catalog.insert(BarIndicatorId::QtrTurn,
        RenderingMetadata::builder("QTR_TURN")
            .sub_pane()
            .line_output("turn", "Qtr Turn", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // Relative Trend Position
    catalog.insert(BarIndicatorId::RelTrendPos,
        RenderingMetadata::builder("REL_TREND_POS")
            .sub_pane()
            .line_output("pos", "Rel Trend Pos", COLOR_CYAN)
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    );

    // Session
    catalog.insert(BarIndicatorId::Session,
        RenderingMetadata::builder("SESSION")
            .sub_pane()
            .line_output("session", "Session", COLOR_BLUE)
            .precision(0)
            .build()
    );

    // Start/End of Month
    catalog.insert(BarIndicatorId::SomEom,
        RenderingMetadata::builder("SOM_EOM")
            .sub_pane()
            .line_output("pos", "SOM/EOM", COLOR_PURPLE)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // Start/End of Quarter
    catalog.insert(BarIndicatorId::SoqEoq,
        RenderingMetadata::builder("SOQ_EOQ")
            .sub_pane()
            .line_output("pos", "SOQ/EOQ", COLOR_ORANGE)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // Start/End of Week
    catalog.insert(BarIndicatorId::SowEow,
        RenderingMetadata::builder("SOW_EOW")
            .sub_pane()
            .line_output("pos", "SOW/EOW", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    );

    // VWAP Distance
    catalog.insert(BarIndicatorId::VwapDist,
        RenderingMetadata::builder("VWAP_DIST")
            .sub_pane()
            .line_output("dist", "VWAP Distance", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Week of Month
    catalog.insert(BarIndicatorId::WeekMonth,
        RenderingMetadata::builder("WEEK_MONTH")
            .sub_pane()
            .line_output("week", "Week/Month", COLOR_PINK)
            .bounds(1.0, 5.0)
            .precision(0)
            .build()
    );

    // Weekday
    catalog.insert(BarIndicatorId::Weekday,
        RenderingMetadata::builder("WEEKDAY")
            .sub_pane()
            .line_output("day", "Weekday", COLOR_CYAN)
            .bounds(1.0, 7.0)
            .precision(0)
            .build()
    );
}

// ============================================================================
// STATISTICS INDICATORS
// ============================================================================

fn register_statistics_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Z-Score

    // Skewness

    // Kurtosis

    // Percentile Rank

    // Mean Deviation

    // Median

    // Mode

    // Quartiles

    // Interquartile Range

    // ADF Test (proxy returns t-statistic only)
    catalog.insert(BarIndicatorId::Adf,
        RenderingMetadata::builder("ADF")
            .sub_pane()
            .output(OutputSpec::line("statistic", "ADF T-Statistic", COLOR_BLUE, 2.0, ValueExtractor::Main))
            .zero_baseline()
            .precision(4)
            .build()
    );

    // KPSS Test (proxy returns statistic only)
    catalog.insert(BarIndicatorId::Kpss,
        RenderingMetadata::builder("KPSS")
            .sub_pane()
            .output(OutputSpec::line("statistic", "KPSS Statistic", COLOR_PURPLE, 2.0, ValueExtractor::Main))
            .precision(4)
            .build()
    );

    // Autocorrelation
    catalog.insert(BarIndicatorId::Autocorr,
        RenderingMetadata::builder("AUTOCORR")
            .sub_pane()
            .line_output("autocorr", "Autocorrelation", COLOR_INDIGO)
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // Partial Autocorrelation
    catalog.insert(BarIndicatorId::Pacf,
        RenderingMetadata::builder("PACF")
            .sub_pane()
            .line_output("pacf", "PACF", COLOR_CYAN)
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // ========================================================================
    // MISSING STATISTICS INDICATORS (from checklist)
    // ========================================================================

    // ADF-KPSS combined (returns composite stationarity score 0-1)
    catalog.insert(BarIndicatorId::AdfKpss,
        RenderingMetadata::builder("ADF_KPSS")
            .sub_pane()
            .output(OutputSpec::line("score", "Stationarity Score", COLOR_BLUE, 2.0, ValueExtractor::Main))
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.5, COLOR_ZERO_LINE))
            .precision(4)
            .build()
    );

    // ARCH LM Test
    catalog.insert(BarIndicatorId::ArchLm,
        RenderingMetadata::builder("ARCH_LM")
            .sub_pane()
            .line_output("arch_lm", "ARCH LM", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // ARCH LM P-Value
    catalog.insert(BarIndicatorId::ArchLmPval,
        RenderingMetadata::builder("ARCH_LM_PVAL")
            .sub_pane()
            .line_output("pval", "ARCH LM P-Value", COLOR_ORANGE)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Bai-Perron CUSUM
    catalog.insert(BarIndicatorId::BpCusum,
        RenderingMetadata::builder("BP_CUSUM")
            .sub_pane()
            .line_output("cusum", "BP CUSUM", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Engle-Granger ADF
    catalog.insert(BarIndicatorId::EgAdf,
        RenderingMetadata::builder("EG_ADF")
            .sub_pane()
            .line_output("eg_adf", "EG ADF", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Engle-Granger Cointegration
    catalog.insert(BarIndicatorId::EgCoint,
        RenderingMetadata::builder("EG_COINT")
            .sub_pane()
            .line_output("eg_coint", "EG Coint", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Engle-Granger Trend
    catalog.insert(BarIndicatorId::EgTrend,
        RenderingMetadata::builder("EG_TREND")
            .sub_pane()
            .line_output("eg_trend", "EG Trend", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Half-Life Mean Reversion
    catalog.insert(BarIndicatorId::HalfLifeMr,
        RenderingMetadata::builder("HALF_LIFE_MR")
            .sub_pane()
            .line_output("half_life", "Half Life", COLOR_CYAN)
            .precision(2)
            .build()
    );

    // KPSS Trend
    catalog.insert(BarIndicatorId::KpssTrend,
        RenderingMetadata::builder("KPSS_TREND")
            .sub_pane()
            .line_output("kpss_trend", "KPSS Trend", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // KPSS Z-Score
    catalog.insert(BarIndicatorId::KpssZ,
        RenderingMetadata::builder("KPSS_Z")
            .sub_pane()
            .line_output("kpss_z", "KPSS Z", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Ljung-Box
    catalog.insert(BarIndicatorId::LjungBox,
        RenderingMetadata::builder("LJUNG_BOX")
            .sub_pane()
            .line_output("ljung_box", "Ljung-Box", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Phillips-Perron
    catalog.insert(BarIndicatorId::Pp,
        RenderingMetadata::builder("PP")
            .sub_pane()
            .line_output("pp", "Phillips-Perron", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Price Z-Score
    catalog.insert(BarIndicatorId::PriceZscore,
        RenderingMetadata::builder("PRICE_ZSCORE")
            .sub_pane()
            .line_output("zscore", "Price Z-Score", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Price-Volume Coherence
    catalog.insert(BarIndicatorId::PvCoherence,
        RenderingMetadata::builder("PV_COHERENCE")
            .sub_pane()
            .line_output("coherence", "PV Coherence", COLOR_TEAL)
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    );

    // Residual Statistics
    catalog.insert(BarIndicatorId::ResidStat,
        RenderingMetadata::builder("RESID_STAT")
            .sub_pane()
            .line_output("resid", "Residual Stat", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Structural CUSUM
    catalog.insert(BarIndicatorId::StCusum,
        RenderingMetadata::builder("ST_CUSUM")
            .sub_pane()
            .line_output("cusum", "Structural CUSUM", COLOR_BLUE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Variance Ratio
    catalog.insert(BarIndicatorId::Vr,
        RenderingMetadata::builder("VR")
            .sub_pane()
            .line_output("vr", "Variance Ratio", COLOR_TEAL)
            .reference_line(ReferenceLine::new(1.0, COLOR_GRAY))
            .precision(4)
            .build()
    );

    // Variance Ratio Aggregate
    catalog.insert(BarIndicatorId::VrAgg,
        RenderingMetadata::builder("VR_AGG")
            .sub_pane()
            .line_output("vr_agg", "VR Aggregate", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Variance Ratio Z-Score Aggregate
    catalog.insert(BarIndicatorId::VrZAgg,
        RenderingMetadata::builder("VR_Z_AGG")
            .sub_pane()
            .line_output("vr_z", "VR Z-Score", COLOR_TEAL)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // Zivot-Andrews
    catalog.insert(BarIndicatorId::Za,
        RenderingMetadata::builder("ZA")
            .sub_pane()
            .line_output("za", "Zivot-Andrews", COLOR_PURPLE)
            .precision(4)
            .build()
    );
}

// ============================================================================
// ZIGZAG INDICATORS
// ============================================================================

fn register_zigzag_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // Zigzag ATR
    catalog.insert(BarIndicatorId::ZigzagAtr,
        RenderingMetadata::builder("ZIGZAG_ATR")
            .overlay()
            .line_output("zigzag", "Zigzag ATR", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // Zigzag Candle
    catalog.insert(BarIndicatorId::ZigzagCandle,
        RenderingMetadata::builder("ZIGZAG_CANDLE")
            .overlay()
            .line_output("zigzag", "Zigzag Candle", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Zigzag Classic
    catalog.insert(BarIndicatorId::ZigzagClassic,
        RenderingMetadata::builder("ZIGZAG_CLASSIC")
            .overlay()
            .line_output("zigzag", "Zigzag Classic", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // Zigzag Lookahead
    catalog.insert(BarIndicatorId::ZigzagLookahead,
        RenderingMetadata::builder("ZIGZAG_LOOKAHEAD")
            .overlay()
            .line_output("zigzag", "Zigzag Lookahead", COLOR_TEAL)
            .precision(4)
            .build()
    );

    // Zigzag Time
    catalog.insert(BarIndicatorId::ZigzagTime,
        RenderingMetadata::builder("ZIGZAG_TIME")
            .overlay()
            .line_output("zigzag", "Zigzag Time", COLOR_CYAN)
            .precision(4)
            .build()
    );
}

// ============================================================================
// MISSING CATEGORIES (from checklist)
// ============================================================================

fn register_missing_indicators(catalog: &mut HashMap<BarIndicatorId, RenderingMetadata>) {
    // ========================================================================
    // LEVELS (missing)
    // ========================================================================

    // Anchored VWAP variants
    for id in [BarIndicatorId::Avwap, BarIndicatorId::Avwaprev, BarIndicatorId::Avwaptouch] {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .overlay()
                .line_output("avwap", "AVWAP", COLOR_PURPLE)
                .precision(4)
                .build()
        );
    }

    // Break of Structure
    catalog.insert(BarIndicatorId::Bos,
        RenderingMetadata::builder("BOS")
            .overlay()
            .line_output("bos", "Break of Structure", COLOR_ORANGE)
            .precision(4)
            .build()
    );

    // DeMark
    catalog.insert(BarIndicatorId::Demark,
        RenderingMetadata::builder("DEMARK")
            .overlay()
            .line_output("demark", "DeMark", COLOR_PURPLE)
            .precision(4)
            .build()
    );

    // Fair Value Gap variants
    for id in [BarIndicatorId::Fvg, BarIndicatorId::Fvgalt, BarIndicatorId::Fvgdur, BarIndicatorId::Fvgrev] {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .overlay()
                .line_output("fvg", "FVG", COLOR_CYAN)
                .precision(4)
                .build()
        );
    }

    // High-Low Volume Average
    catalog.insert(BarIndicatorId::Hlva,
        RenderingMetadata::builder("HLVA")
            .overlay()
            .line_output("hlva", "HL Vol Avg", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // ========================================================================
    // CANDLES (missing)
    // ========================================================================

    // Candle pattern indicators
    let candle_patterns = [
        BarIndicatorId::Darkcloudcover, BarIndicatorId::Engulfing, BarIndicatorId::Eveningstar,
        BarIndicatorId::Hammer, BarIndicatorId::Harami, BarIndicatorId::Marubozu,
        BarIndicatorId::Morningstar, BarIndicatorId::Patternrec, BarIndicatorId::Piercingpattern,
        BarIndicatorId::Shootingstar, BarIndicatorId::Threeblackcrows, BarIndicatorId::Threewhitesoldiers,
        BarIndicatorId::Tweezer
    ];
    for id in candle_patterns {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .output(OutputSpec::histogram("pattern", "Pattern", COLOR_GREEN, ValueExtractor::Main))
                .bounds(-1.0, 1.0)
                .histogram_style(HistogramStyle::Centered)
                .precision(0)
                .build()
        );
    }

    // ========================================================================
    // VOLUME (missing)
    // ========================================================================

    // ========================================================================
    // TREND (missing)
    // ========================================================================

    // Elder Impulse Trend
    catalog.insert(BarIndicatorId::Eit,
        RenderingMetadata::builder("EIT")
            .sub_pane()
            .line_output("eit", "Elder Impulse", COLOR_GREEN)
            .precision(0)
            .build()
    );

    // GMMA
    catalog.insert(BarIndicatorId::Gmma,
        RenderingMetadata::builder("GMMA")
            .overlay()
            .line_output("gmma", "GMMA", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // SDL
    catalog.insert(BarIndicatorId::Sdl,
        RenderingMetadata::builder("SDL")
            .overlay()
            .line_output("sdl", "SDL", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // ========================================================================
    // ACCUMULATION (missing)
    // ========================================================================

    // Demand Index
    catalog.insert(BarIndicatorId::Di,
        RenderingMetadata::builder("DI")
            .sub_pane()
            .line_output("di", "Demand Index", COLOR_PURPLE)
            .zero_baseline()
            .precision(4)
            .build()
    );

    // ========================================================================
    // ADAPTIVE (missing)
    // ========================================================================

    // ========================================================================
    // ENTROPY (missing)
    // ========================================================================

    let entropy_ids = [
        BarIndicatorId::Conden, BarIndicatorId::Infog, BarIndicatorId::Jsd, BarIndicatorId::Kld, BarIndicatorId::Mi, BarIndicatorId::Pe, BarIndicatorId::Te, BarIndicatorId::Xmil
    ];
    for id in entropy_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("entropy", "Entropy", COLOR_PURPLE)
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // KALMAN (missing)
    // ========================================================================

    let kalman_ids = [
        BarIndicatorId::Kcomp, BarIndicatorId::Kregime, BarIndicatorId::Kscr, BarIndicatorId::Kslope, BarIndicatorId::Kslopez, BarIndicatorId::Particle, BarIndicatorId::Rts
    ];
    for id in kalman_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("kalman", "Kalman", COLOR_BLUE)
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // TREND STOP (missing)
    // ========================================================================

    let trend_stop_ids = [
        BarIndicatorId::Atrts, BarIndicatorId::Chand, BarIndicatorId::Cks, BarIndicatorId::Donbo, BarIndicatorId::Dons, BarIndicatorId::Kelts, BarIndicatorId::Psars, BarIndicatorId::Supts, BarIndicatorId::TsSwings, BarIndicatorId::Volts, BarIndicatorId::VoltsAtr
    ];
    for id in trend_stop_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .overlay()
                .line_output("stop", "Stop", COLOR_RED)
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // CHAOS (missing)
    // ========================================================================

    let chaos_ids = [
        BarIndicatorId::DfaPct, BarIndicatorId::FractalDim, BarIndicatorId::Fractals, BarIndicatorId::HurstPct, BarIndicatorId::WilliamsMfi
    ];
    for id in chaos_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("chaos", "Chaos", COLOR_PURPLE)
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // DIVERGENCE (all missing)
    // ========================================================================

    let divergence_ids = [
        BarIndicatorId::CciDiv, BarIndicatorId::ClassicDiv, BarIndicatorId::DivStrength, BarIndicatorId::HiddenDiv, BarIndicatorId::MacdDiv, BarIndicatorId::MacdHistDiv, BarIndicatorId::MultiDiv, BarIndicatorId::ObvDiv, BarIndicatorId::RsiDiv, BarIndicatorId::StochDiv, BarIndicatorId::VolDiv, BarIndicatorId::WilliamsDiv, BarIndicatorId::ZigzagDiv
    ];
    for id in divergence_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("divergence", "Divergence", COLOR_PURPLE)
                .bounds(-1.0, 1.0)
                .zero_baseline()
                .precision(0)
                .build()
        );
    }

    // ========================================================================
    // REGRESSION (missing)
    // ========================================================================

    catalog.insert(BarIndicatorId::Arimax,
        RenderingMetadata::builder("ARIMAX")
            .overlay()
            .line_output("arimax", "ARIMAX", COLOR_BLUE)
            .precision(4)
            .build()
    );

    // ========================================================================
    // RATIO (missing)
    // ========================================================================

    // ========================================================================
    // CLUSTERS (missing)
    // ========================================================================

    let cluster_ids = [
        BarIndicatorId::ClQueueImb, BarIndicatorId::MarketMicro, BarIndicatorId::OrderBookSlope, BarIndicatorId::OrderFlowImb, BarIndicatorId::TickVolume, BarIndicatorId::VwapLevels
    ];
    for id in cluster_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("cluster", "Cluster", COLOR_BLUE)
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // POSITION (missing)
    // ========================================================================

    let position_ids = [
        BarIndicatorId::AvwapDist, BarIndicatorId::Cpr, BarIndicatorId::DayWeekMonth, BarIndicatorId::DistLevels, BarIndicatorId::DomWoq, BarIndicatorId::HolidayProx, BarIndicatorId::HourDay, BarIndicatorId::MonthQtr, BarIndicatorId::MonthTurn, BarIndicatorId::QtrTurn, BarIndicatorId::RelTrendPos, BarIndicatorId::Session, BarIndicatorId::SomEom, BarIndicatorId::SoqEoq, BarIndicatorId::SowEow, BarIndicatorId::VwapDist, BarIndicatorId::WeekMonth, BarIndicatorId::Weekday
    ];
    for id in position_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("position", "Position", COLOR_PURPLE)
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // BOOK (missing)
    // ========================================================================

    let book_ids = [
        BarIndicatorId::BookImb, BarIndicatorId::BookSlope, BarIndicatorId::Ofi, BarIndicatorId::QueueImb
    ];
    for id in book_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .sub_pane()
                .line_output("book", "Book", COLOR_TEAL)
                .zero_baseline()
                .precision(4)
                .build()
        );
    }

    // ========================================================================
    // ZIGZAG (missing)
    // ========================================================================

    let zigzag_ids = [
        BarIndicatorId::ZigzagAtr, BarIndicatorId::ZigzagCandle, BarIndicatorId::ZigzagClassic, BarIndicatorId::ZigzagLookahead, BarIndicatorId::ZigzagTime
    ];
    for id in zigzag_ids {
        catalog.insert(id,
            RenderingMetadata::builder(format!("{:?}", id))
                .overlay()
                .line_output("zigzag", "Zigzag", COLOR_PURPLE)
                .precision(4)
                .build()
        );
    }
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Get rendering metadata for an indicator by BarIndicatorId
pub fn get_rendering(id: BarIndicatorId) -> Option<&'static RenderingMetadata> {
    RENDERING_CATALOG.get(&id)
}

/// Get all indicator IDs in the rendering catalog
pub fn all_rendering_ids() -> Vec<&'static BarIndicatorId> {
    RENDERING_CATALOG.keys().collect()
}

/// Get count of indicators in rendering catalog
pub fn rendering_count() -> usize {
    RENDERING_CATALOG.len()
}

/// Check if an indicator has rendering metadata
pub fn has_rendering(id: BarIndicatorId) -> bool {
    RENDERING_CATALOG.contains_key(&id)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_not_empty() {
        assert!(rendering_count() > 100, "Should have 100+ indicators with rendering");
    }

    #[test]
    fn test_get_rsi_rendering() {
        let rsi = get_rendering(BarIndicatorId::Rsi).unwrap();
        assert!(!rsi.overlay);
        assert_eq!(rsi.bounds, Some((0.0, 100.0)));
        assert!(!rsi.outputs.is_empty());
    }

    #[test]
    fn test_get_macd_rendering() {
        let macd = get_rendering(BarIndicatorId::Macd).unwrap();
        assert!(!macd.overlay);
        assert!(macd.zero_baseline);
        assert_eq!(macd.outputs.len(), 3);
        assert_eq!(macd.histogram_style, HistogramStyle::Centered);
    }

    #[test]
    fn test_get_sma_rendering() {
        let sma = get_rendering(BarIndicatorId::Sma).unwrap();
        assert!(sma.overlay);
        assert_eq!(sma.outputs.len(), 1);
    }

    #[test]
    fn test_get_bollinger_rendering() {
        let bb = get_rendering(BarIndicatorId::Bb).unwrap();
        assert!(bb.overlay);
        assert!(bb.outputs.len() >= 3);
    }

    #[test]
    fn test_get_ichimoku_rendering() {
        let ich = get_rendering(BarIndicatorId::Ichimoku).unwrap();
        assert!(ich.overlay);
        assert!(ich.outputs.len() >= 5);
    }

    #[test]
    fn test_average_indicators_overlay() {
        let avg_ids = [
            BarIndicatorId::Sma, BarIndicatorId::Ema, BarIndicatorId::Wma,
            BarIndicatorId::Hma, BarIndicatorId::Dema, BarIndicatorId::Tema,
        ];
        for id in avg_ids {
            let meta = get_rendering(id).unwrap();
            assert!(meta.overlay, "{:?} should be overlay", id);
        }
    }

    #[test]
    fn test_oscillators_have_bounds() {
        let osc_ids = [
            BarIndicatorId::Rsi, BarIndicatorId::Stoch, BarIndicatorId::Mfi, BarIndicatorId::WilliamsR,
        ];
        for id in osc_ids {
            let meta = get_rendering(id).unwrap();
            assert!(meta.bounds.is_some(), "{:?} should have bounds", id);
        }
    }

    #[test]
    fn test_zero_baseline_indicators() {
        let zb_ids = [
            BarIndicatorId::Macd, BarIndicatorId::Cci, BarIndicatorId::Roc, BarIndicatorId::MomZscore,
        ];
        for id in zb_ids {
            let meta = get_rendering(id).unwrap();
            assert!(meta.zero_baseline, "{:?} should have zero baseline", id);
        }
    }

    #[test]
    fn test_rendering_coverage() {
        let count = rendering_count();
        println!("Rendering catalog has {} indicators", count);

        // Check coverage for common indicators
        let must_have = [
            BarIndicatorId::Sma, BarIndicatorId::Ema, BarIndicatorId::Rsi,
            BarIndicatorId::Macd, BarIndicatorId::Bb, BarIndicatorId::Atr,
            BarIndicatorId::Adx, BarIndicatorId::Cci, BarIndicatorId::Stoch,
            BarIndicatorId::Obv, BarIndicatorId::Mfi, BarIndicatorId::WilliamsR,
            BarIndicatorId::Roc, BarIndicatorId::Trix,
        ];

        let mut missing = Vec::new();
        for id in must_have {
            if get_rendering(id).is_none() {
                missing.push(id);
            }
        }

        assert!(missing.is_empty(), "Missing rendering for: {:?}", missing);
        assert!(count >= 200, "Should have at least 200 indicators with rendering, got {}", count);
    }

    #[test]
    fn test_output_names_consistency() {
        // Test that output names match what ValueAdapter expects
        let rsi = get_rendering(BarIndicatorId::Rsi).unwrap();
        assert_eq!(rsi.outputs[0].name, "rsi", "RSI output name should be 'rsi'");

        let macd = get_rendering(BarIndicatorId::Macd).unwrap();
        let macd_names: Vec<_> = macd.outputs.iter().map(|o| o.name.as_str()).collect();
        assert!(macd_names.contains(&"macd"), "MACD should have 'macd' output");
        assert!(macd_names.contains(&"signal"), "MACD should have 'signal' output");
        assert!(macd_names.contains(&"histogram"), "MACD should have 'histogram' output");

        let bb = get_rendering(BarIndicatorId::Bb).unwrap();
        let bb_names: Vec<_> = bb.outputs.iter().map(|o| o.name.as_str()).collect();
        assert!(bb_names.contains(&"upper") || bb_names.contains(&"bands"),
                "BB should have upper/bands output, got {:?}", bb_names);
    }

    #[test]
    fn test_catalog_coverage() {
        let count = rendering_count();
        println!("\n=== RENDERING CATALOG COVERAGE ===");
        println!("Total indicators in rendering catalog: {}", count);

        // We should have 460+ indicators (nearly all BarIndicatorId variants)
        assert!(count >= 460, "Should have at least 460 indicators with rendering, got {}", count);

        // Verify some key indicators exist
        let key_indicators = [
            BarIndicatorId::Sma, BarIndicatorId::Ema, BarIndicatorId::Rsi,
            BarIndicatorId::Macd, BarIndicatorId::Bb, BarIndicatorId::Atr,
            BarIndicatorId::Adx, BarIndicatorId::AdxSlope, BarIndicatorId::Adaptivebb,
        ];

        for id in key_indicators {
            assert!(get_rendering(id).is_some(), "Missing rendering for {:?}", id);
        }
    }
}
