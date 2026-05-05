//! momentum_catalog.rs: Complete catalog of all Momentum indicators
//!
//! This catalog contains 92+ momentum indicators extracted from actual implementations.
//! Organized alphabetically for easy navigation.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
    SourceType,
};
use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Momentum;

// ============================================================================
// Individual indicator signatures (alphabetically sorted)
// ============================================================================

/// Adaptive Stochastic - стохастик с адаптивными параметрами
pub fn signature_adaptive_stochastic() -> IndicatorSignature {
    IndicatorSignature::builder("ADAPTIVE_STOCH", CATEGORY)
        .name("Adaptive Stochastic")
        .description("Stochastic oscillator with adaptive parameters")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AdaptiveStoch) // TODO: Add to enum
        // Note: "ADAPTIVE_STOCH" is already the main ID, no need for alias
        .alias("AdaptiveStoch")
        .alias("adaptive_stoch")
        .alias("ADAPTIVESTOCHASTIC")
        .alias("AdaptiveStochastic")
        .alias("adaptivestochastic")
        .alias("adaptive_stochastic")
        .alias("ADAPTIVE_STOCHASTIC")
        .alias("Adaptive_Stochastic")
        .build()
}

/// Average Directional Index - сила тренда
pub fn signature_adx() -> IndicatorSignature {
    IndicatorSignature::builder("ADX", CATEGORY)
        .name("Average Directional Index")
        .description("Measures trend strength (0-100)")
        .add_constraint(ParamConstraint::period(2, 512, 14))
        .metadata("author", "J. Welles Wilder")
        .metadata("range", "0-100")
        .machine_id(BarIndicatorId::Adx)
        // Note: "ADX" is already the main ID, no need for alias
        .alias("Adx")
        .alias("adx")
        .alias("AVERAGEDIRECTIONALINDEX")
        .alias("AverageDirectionalIndex")
        .alias("averagedirectionalindex")
        .alias("average_directional_index")
        .alias("AVERAGE_DIRECTIONAL_INDEX")
        .alias("Average_Directional_Index")
        .build()
}

/// Архангельский Moving Average Timing - тайминг индикатор
pub fn signature_amat() -> IndicatorSignature {
    IndicatorSignature::builder("AMAT", CATEGORY)
        .name("Arkhangelsk Moving Average Timing")
        .description("MA timing indicator")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type_named("fast_ma", MovingAverageType::SMA))
        .add_constraint(ParamConstraint::ma_type_named("slow_ma", MovingAverageType::SMA))
        .metadata("fast_ma_desc", "Fast MA type")
        .metadata("slow_ma_desc", "Slow MA type")
        .machine_id(BarIndicatorId::Amat)
        // Note: "AMAT" is already the main ID, no need for alias
        .alias("Amat")
        .alias("amat")
        .alias("ARKHANGELSKMOVINGAVERAGETIMING")
        .alias("ArkhangelskMovingAverageTiming")
        .alias("arkhangelskmovingaveragetiming")
        .alias("arkhangelsk_moving_average_timing")
        .alias("ARKHANGELSK_MOVING_AVERAGE_TIMING")
        .alias("Arkhangelsk_Moving_Average_Timing")
        .build()
}

/// Absolute Price Oscillator
pub fn signature_apo() -> IndicatorSignature {
    IndicatorSignature::builder("APO", CATEGORY)
        .name("Absolute Price Oscillator")
        .description("Difference between two MAs")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(12))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(26))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Apo)
        // Note: "APO" is already the main ID, no need for alias
        .alias("Apo")
        .alias("apo")
        .alias("ABSOLUTEPRICEOSCILLATOR")
        .alias("AbsolutePriceOscillator")
        .alias("absolutepriceoscillator")
        .alias("absolute_price_oscillator")
        .alias("ABSOLUTE_PRICE_OSCILLATOR")
        .alias("Absolute_Price_Oscillator")
        .build()
}

/// Aroon - определяет силу тренда и точки разворота
pub fn signature_aroon() -> IndicatorSignature {
    IndicatorSignature::builder("AROON", CATEGORY)
        .name("Aroon")
        .description("Identifies trend strength and reversal points")
        .add_constraint(ParamConstraint::period(2, 512, 25))
        .metadata("outputs", "aroon_up, aroon_down, aroon_oscillator")
        .machine_id(BarIndicatorId::Aroon)
        // Note: "AROON" is already the main ID, no need for alias
        .alias("Aroon")
        .alias("aroon")
        .build()
}

/// Aroon Down
pub fn signature_aroon_down() -> IndicatorSignature {
    IndicatorSignature::builder("AROON_DOWN", CATEGORY)
        .name("Aroon Down")
        .description("Aroon down component")
        .add_constraint(ParamConstraint::period(2, 512, 25))
        .machine_id(BarIndicatorId::AroonDown)
        // Note: "AROON_DOWN" is already the main ID, no need for alias
        .alias("AroonDown")
        .alias("aroon_down")
        .alias("AROONDOWN")
        .alias("aroondown")
        .alias("Aroon_Down")
        .build()
}

/// Aroon Oscillator
pub fn signature_aroon_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("AROON_OSC", CATEGORY)
        .name("Aroon Oscillator")
        .description("Difference between Aroon Up and Aroon Down")
        .add_constraint(ParamConstraint::period(2, 512, 25))
        .machine_id(BarIndicatorId::AroonOsc) // TODO: Add to enum
        // Note: "AROON_OSC" is already the main ID, no need for alias
        .alias("AroonOsc")
        .alias("aroon_osc")
        .alias("AROONOSCILLATOR")
        .alias("AroonOscillator")
        .alias("aroonoscillator")
        .alias("aroon_oscillator")
        .alias("AROON_OSCILLATOR")
        .alias("Aroon_Oscillator")
        .build()
}

/// Aroon Up
pub fn signature_aroon_up() -> IndicatorSignature {
    IndicatorSignature::builder("AROON_UP", CATEGORY)
        .name("Aroon Up")
        .description("Aroon up component")
        .add_constraint(ParamConstraint::period(2, 512, 25))
        .machine_id(BarIndicatorId::AroonUp)
        // Note: "AROON_UP" is already the main ID, no need for alias
        .alias("AroonUp")
        .alias("aroon_up")
        .alias("AROONUP")
        .alias("aroonup")
        .alias("Aroon_Up")
        .build()
}

/// ATR-based RSI - RSI с использованием ATR
pub fn signature_atr_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("ATR_RSI", CATEGORY)
        .name("ATR RSI")
        .description("RSI calculated using ATR")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::AtrRsi)
        // Note: "ATR_RSI" is already the main ID, no need for alias
        .alias("AtrRsi")
        .alias("atr_rsi")
        .alias("ATRRSI")
        .alias("atrrsi")
        .alias("Atr_Rsi")
        .build()
}

/// Automatic Fibonacci - автоматические уровни Фибоначчи
pub fn signature_auto_fibo() -> IndicatorSignature {
    IndicatorSignature::builder("AUTO_FIBO", CATEGORY)
        .name("Automatic Fibonacci")
        .description("Automatic Fibonacci retracement levels")
        .add_constraint(ParamConstraint::period(2, 200, 50))
        .machine_id(BarIndicatorId::AutoFibo)
        // Note: "AUTO_FIBO" is already the main ID, no need for alias
        .alias("AutoFibo")
        .alias("auto_fibo")
        .alias("AUTOMATICFIBONACCI")
        .alias("AutomaticFibonacci")
        .alias("automaticfibonacci")
        .alias("automatic_fibonacci")
        .alias("AUTOMATIC_FIBONACCI")
        .alias("Automatic_Fibonacci")
        .build()
}

/// Bollinger Band Period - динамический период на основе BB
pub fn signature_bb_period() -> IndicatorSignature {
    IndicatorSignature::builder("BB_PERIOD", CATEGORY)
        .name("Bollinger Band Period")
        .description("Dynamic period based on Bollinger Bands")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .machine_id(BarIndicatorId::BbPeriod)
        // Note: "BB_PERIOD" is already the main ID, no need for alias
        .alias("BbPeriod")
        .alias("bb_period")
        .alias("BOLLINGERBANDPERIOD")
        .alias("BollingerBandPeriod")
        .alias("bollingerbandperiod")
        .alias("bollinger_band_period")
        .alias("BOLLINGER_BAND_PERIOD")
        .alias("Bollinger_Band_Period")
        .build()
}

/// Bias - отклонение цены от MA
pub fn signature_bias() -> IndicatorSignature {
    IndicatorSignature::builder("BIAS", CATEGORY)
        .name("Bias")
        .description("Price deviation from moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .machine_id(BarIndicatorId::Bias)
        // Note: "BIAS" is already the main ID, no need for alias
        .alias("Bias")
        .alias("bias")
        .build()
}

/// Balance of Power - баланс между быками и медведями
pub fn signature_bop() -> IndicatorSignature {
    IndicatorSignature::builder("BOP", CATEGORY)
        .name("Balance of Power")
        .description("Balance between bulls and bears")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Bop) // TODO: Add to enum
        // Note: "BOP" is already the main ID, no need for alias
        .alias("Bop")
        .alias("bop")
        .alias("BALANCEOFPOWER")
        .alias("BalanceofPower")
        .alias("balanceofpower")
        .alias("balance_of_power")
        .alias("BALANCE_OF_POWER")
        .alias("Balance_Of_Power")
        .build()
}

/// Candle Patterns - паттерны свечей
pub fn signature_candle_patterns() -> IndicatorSignature {
    IndicatorSignature::builder("CANDLE_PATTERNS", CATEGORY)
        .name("Candle Patterns")
        .description("Candlestick pattern recognition")
        .add_constraint(ParamConstraint::period(2, 50, 3))
        .machine_id(BarIndicatorId::CandlePatterns)
        // Note: "CANDLE_PATTERNS" is already the main ID, no need for alias
        .alias("CandlePatterns")
        .alias("candle_patterns")
        .alias("CANDLEPATTERNS")
        .alias("candlepatterns")
        .alias("Candle_Patterns")
        .build()
}

/// Commodity Channel Index
pub fn signature_cci() -> IndicatorSignature {
    IndicatorSignature::builder("CCI", CATEGORY)
        .name("Commodity Channel Index")
        .description("Measures deviation from average price")
        .add_constraint(ParamConstraint::period(2, 512, 20))
        .add_constraint(
            ParamConstraint::new("scalar", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.015))
        )
        .metadata("author", "Donald Lambert")
        .machine_id(BarIndicatorId::Cci)
        // Note: "CCI" is already the main ID, no need for alias
        .alias("Cci")
        .alias("cci")
        .alias("COMMODITYCHANNELINDEX")
        .alias("CommodityChannelIndex")
        .alias("commoditychannelindex")
        .alias("commodity_channel_index")
        .alias("COMMODITY_CHANNEL_INDEX")
        .alias("Commodity_Channel_Index")
        .build()
}

/// Center of Gravity - центр гравитации
pub fn signature_center_of_gravity() -> IndicatorSignature {
    IndicatorSignature::builder("COG", CATEGORY)
        .name("Center of Gravity")
        .description("Center of gravity oscillator")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::Cog) // TODO: Add to enum
        // Note: "COG" is already the main ID, no need for alias
        .alias("Cog")
        .alias("cog")
        .alias("CENTEROFGRAVITY")
        .alias("CenterofGravity")
        .alias("centerofgravity")
        .alias("center_of_gravity")
        .alias("CENTER_OF_GRAVITY")
        .alias("Center_Of_Gravity")
        .build()
}

/// Chande Forecast Oscillator
pub fn signature_cfo() -> IndicatorSignature {
    IndicatorSignature::builder("CFO", CATEGORY)
        .name("Chande Forecast Oscillator")
        .description("Forecast oscillator by Tushar Chande")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("author", "Tushar Chande")
        .machine_id(BarIndicatorId::Cfo) // TODO: Add to enum
        // Note: "CFO" is already the main ID, no need for alias
        .alias("Cfo")
        .alias("cfo")
        .alias("CHANDEFORECASTOSCILLATOR")
        .alias("ChandeForecastOscillator")
        .alias("chandeforecastoscillator")
        .alias("chande_forecast_oscillator")
        .alias("CHANDE_FORECAST_OSCILLATOR")
        .alias("Chande_Forecast_Oscillator")
        .build()
}

/// Chande Momentum Oscillator
pub fn signature_cmo() -> IndicatorSignature {
    IndicatorSignature::builder("CMO", CATEGORY)
        .name("Chande Momentum Oscillator")
        .description("Momentum oscillator (-100 to +100)")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "Tushar Chande")
        .metadata("range", "-100 to +100")
        .machine_id(BarIndicatorId::Cmo)
        // Note: "CMO" is already the main ID, no need for alias
        .alias("Cmo")
        .alias("cmo")
        .alias("CHANDEMOMENTUMOSCILLATOR")
        .alias("ChandeMomentumOscillator")
        .alias("chandemomentumoscillator")
        .alias("chande_momentum_oscillator")
        .alias("CHANDE_MOMENTUM_OSCILLATOR")
        .alias("Chande_Momentum_Oscillator")
        .build()
}

/// Connors RSI - составной RSI индикатор
pub fn signature_connors_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("CONNORS_RSI", CATEGORY)
        .name("Connors RSI")
        .description("Composite RSI indicator")
        .add_constraint(ParamConstraint::period(2, 200, 3))
        .metadata("author", "Larry Connors")
        .machine_id(BarIndicatorId::ConnorsRsi)
        // Note: "CONNORS_RSI" is already the main ID, no need for alias
        .alias("ConnorsRsi")
        .alias("connors_rsi")
        .alias("CONNORSRSI")
        .alias("ConnorsRSI")
        .alias("connorsrsi")
        .alias("Connors_Rsi")
        .build()
}

/// Coppock Curve - долгосрочный momentum индикатор
pub fn signature_coppock() -> IndicatorSignature {
    IndicatorSignature::builder("COPPOCK", CATEGORY)
        .name("Coppock Curve")
        .description("Long-term momentum indicator")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::WMA))
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("author", "Edwin Coppock")
        .machine_id(BarIndicatorId::Coppock) // TODO: Add to enum
        // Note: "COPPOCK" is already the main ID, no need for alias
        .alias("Coppock")
        .alias("coppock")
        .alias("COPPOCKCURVE")
        .alias("CoppockCurve")
        .alias("coppockcurve")
        .alias("coppock_curve")
        .alias("COPPOCK_CURVE")
        .alias("Coppock_Curve")
        .build()
}

/// DeMarker - осциллятор Демарка
pub fn signature_demarker() -> IndicatorSignature {
    IndicatorSignature::builder("DEMARKER", CATEGORY)
        .name("DeMarker")
        .description("DeMarker oscillator (0-1)")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "Tom DeMark")
        .metadata("range", "0-1")
        .machine_id(BarIndicatorId::Demarker)
        // Note: "DEMARKER" is already the main ID, no need for alias
        .alias("Demarker")
        .alias("demarker")
        .alias("DeMarker")
        .build()
}

/// Detrended Synthetic Price
pub fn signature_detrended_synthetic_price() -> IndicatorSignature {
    IndicatorSignature::builder("DSP", CATEGORY)
        .name("Detrended Synthetic Price")
        .description("Detrended synthetic price oscillator")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Dsp) // TODO: Add to enum
        // Note: "DSP" is already the main ID, no need for alias
        .alias("Dsp")
        .alias("dsp")
        .alias("DETRENDEDSYNTHETICPRICE")
        .alias("DetrendedSyntheticPrice")
        .alias("detrendedsyntheticprice")
        .alias("detrended_synthetic_price")
        .alias("DETRENDED_SYNTHETIC_PRICE")
        .alias("Detrended_Synthetic_Price")
        .build()
}

/// Directional Movement - компоненты ADX
pub fn signature_dm() -> IndicatorSignature {
    IndicatorSignature::builder("DM", CATEGORY)
        .name("Directional Movement")
        .description("Directional movement components (+DM, -DM)")
        .add_constraint(ParamConstraint::period(2, 512, 14))
        .metadata("outputs", "+DI, -DI")
        .machine_id(BarIndicatorId::Dm)
        // Note: "DM" is already the main ID, no need for alias
        .alias("Dm")
        .alias("dm")
        .alias("DIRECTIONALMOVEMENT")
        .alias("DirectionalMovement")
        .alias("directionalmovement")
        .alias("directional_movement")
        .alias("DIRECTIONAL_MOVEMENT")
        .alias("Directional_Movement")
        .build()
}

/// Detrended Price Oscillator
pub fn signature_dpo() -> IndicatorSignature {
    IndicatorSignature::builder("DPO", CATEGORY)
        .name("Detrended Price Oscillator")
        .description("Removes trend to identify cycles")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Dpo) // TODO: Add to enum
        // Note: "DPO" is already the main ID, no need for alias
        .alias("Dpo")
        .alias("dpo")
        .alias("DETRENDEDPRICEOSCILLATOR")
        .alias("DetrendedPriceOscillator")
        .alias("detrendedpriceoscillator")
        .alias("detrended_price_oscillator")
        .alias("DETRENDED_PRICE_OSCILLATOR")
        .alias("Detrended_Price_Oscillator")
        .build()
}

/// DPO Percent - процентный DPO
pub fn signature_dpo_percent() -> IndicatorSignature {
    IndicatorSignature::builder("DPO_PCT", CATEGORY)
        .name("DPO Percent")
        .description("Percentage-based DPO")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .machine_id(BarIndicatorId::DpoPct) // TODO: Add to enum
        // Note: "DPO_PCT" is already the main ID, no need for alias
        .alias("DpoPct")
        .alias("dpo_pct")
        .alias("DPOPERCENT")
        .alias("DPOPercent")
        .alias("dpopercent")
        .alias("dpo_percent")
        .alias("DPO_PERCENT")
        .alias("Dpo_Percent")
        .build()
}

/// Double Smoothed Stochastics (Bressert)
pub fn signature_dss_bressert() -> IndicatorSignature {
    IndicatorSignature::builder("DSS", CATEGORY)
        .name("Double Smoothed Stochastics")
        .description("Double smoothed stochastic by Walter Bressert")
        .add_constraint(ParamConstraint::period(2, 200, 13))
        .metadata("author", "Walter Bressert")
        .machine_id(BarIndicatorId::Dss) // TODO: Add to enum
        // Note: "DSS" is already the main ID, no need for alias
        .alias("Dss")
        .alias("dss")
        .alias("DOUBLESMOOTHEDSTOCHASTICS")
        .alias("DoubleSmoothedStochastics")
        .alias("doublesmoothedstochastics")
        .alias("double_smoothed_stochastics")
        .alias("DOUBLE_SMOOTHED_STOCHASTICS")
        .alias("Double_Smoothed_Stochastics")
        .build()
}

/// Ehlers Cyber Cycle
pub fn signature_ehlers_cyber_cycle() -> IndicatorSignature {
    IndicatorSignature::builder("EHLERS_CC", CATEGORY)
        .name("Ehlers Cyber Cycle")
        .description("Cyber Cycle indicator by John Ehlers")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::EhlersCc) // TODO: Add to enum
        // Note: "EHLERS_CC" is already the main ID, no need for alias
        .alias("EhlersCc")
        .alias("ehlers_cc")
        .alias("EHLERSCYBERCYCLE")
        .alias("EhlersCyberCycle")
        .alias("ehlerscybercycle")
        .alias("ehlers_cyber_cycle")
        .alias("EHLERS_CYBER_CYCLE")
        .alias("Ehlers_Cyber_Cycle")
        .build()
}

/// Ehlers Rocket RSI
pub fn signature_ehlers_rocket_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("EHLERS_ROCKET", CATEGORY)
        .name("Ehlers Rocket RSI")
        .description("Fast RSI by John Ehlers")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::EhlersRocket) // TODO: Add to enum
        // Note: "EHLERS_ROCKET" is already the main ID, no need for alias
        .alias("EhlersRocket")
        .alias("ehlers_rocket")
        .alias("EHLERSROCKETRSI")
        .alias("EhlersRocketRSI")
        .alias("ehlersrocketrsi")
        .alias("ehlers_rocket_rsi")
        .alias("EHLERS_ROCKET_RSI")
        .alias("Ehlers_Rocket_Rsi")
        .build()
}

/// Elder Impulse System
pub fn signature_elder_impulse() -> IndicatorSignature {
    IndicatorSignature::builder("ELDER_IMPULSE", CATEGORY)
        .name("Elder Impulse System")
        .description("Combines MACD and EMA for impulse signals")
        .add_constraint(ParamConstraint::period(2, 200, 13))
        .add_constraint(
            ParamConstraint::new("ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("author", "Alexander Elder")
        .machine_id(BarIndicatorId::ElderImpulse) // TODO: Add to enum
        // Note: "ELDER_IMPULSE" is already the main ID, no need for alias
        .alias("ElderImpulse")
        .alias("elder_impulse")
        .alias("ELDERIMPULSESYSTEM")
        .alias("ElderImpulseSystem")
        .alias("elderimpulsesystem")
        .alias("elder_impulse_system")
        .alias("ELDER_IMPULSE_SYSTEM")
        .alias("Elder_Impulse_System")
        .build()
}

/// Elder Ray (Bull Power / Bear Power)
pub fn signature_elder_ray() -> IndicatorSignature {
    IndicatorSignature::builder("ELDER_RAY", CATEGORY)
        .name("Elder Ray")
        .description("Bull Power and Bear Power indicators")
        .add_constraint(ParamConstraint::period(2, 200, 13))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("author", "Alexander Elder")
        .metadata("outputs", "bull_power, bear_power")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::ElderRay)
        // Note: "ELDER_RAY" is already the main ID, no need for alias
        .alias("ElderRay")
        .alias("elder_ray")
        .alias("ELDERRAY")
        .alias("elderray")
        .alias("Elder_Ray")
        .build()
}

/// EMA Slope - наклон EMA
pub fn signature_ema_slope() -> IndicatorSignature {
    IndicatorSignature::builder("EMA_SLOPE", CATEGORY)
        .name("EMA Slope")
        .description("Slope of exponential moving average")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::EmaSlope)
        // Note: "EMA_SLOPE" is already the main ID, no need for alias
        .alias("EmaSlope")
        .alias("ema_slope")
        .alias("EMASLOPE")
        .alias("EMASlope")
        .alias("emaslope")
        .alias("Ema_Slope")
        .build()
}

/// EWMAC - Exponentially Weighted Moving Average Crossover
pub fn signature_ewmac() -> IndicatorSignature {
    IndicatorSignature::builder("EWMAC", CATEGORY)
        .name("EWMAC")
        .description("Exponentially weighted MA crossover")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(16))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(64))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Ewmac)
        // Note: "EWMAC" is already the main ID, no need for alias
        .alias("Ewmac")
        .alias("ewmac")
        .build()
}

/// EWMAC Robust - робастная версия EWMAC
pub fn signature_ewmac_robust() -> IndicatorSignature {
    IndicatorSignature::builder("EWMAC_ROBUST", CATEGORY)
        .name("EWMAC Robust")
        .description("Robust version of EWMAC")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(16))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(64))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::EwmacRobust) // TODO: Add to enum
        // Note: "EWMAC_ROBUST" is already the main ID, no need for alias
        .alias("EwmacRobust")
        .alias("ewmac_robust")
        .alias("EWMACROBUST")
        .alias("EWMACRobust")
        .alias("ewmacrobust")
        .alias("Ewmac_Robust")
        .build()
}

/// Fisher Transform
pub fn signature_fisher_transform() -> IndicatorSignature {
    IndicatorSignature::builder("MO_FISHER", CATEGORY)
        .name("Fisher Transform")
        .description("Transforms prices to Gaussian distribution")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::MoFisher)
        // Note: "MO_FISHER" is already the main ID, no need for alias
        .alias("MoFisher")
        .alias("mo_fisher")
        .alias("FISHERTRANSFORM")
        .alias("FisherTransform")
        .alias("fishertransform")
        .alias("fisher_transform")
        .alias("FISHER_TRANSFORM")
        .alias("Fisher_Transform")
        .build()
}

/// Gopalakrishnan Range Index (GAPO)
pub fn signature_gapo() -> IndicatorSignature {
    IndicatorSignature::builder("GAPO", CATEGORY)
        .name("Gopalakrishnan Range Index")
        .description("Volatility-based range indicator")
        .add_constraint(ParamConstraint::period(2, 200, 21))
        .machine_id(BarIndicatorId::Gapo) // TODO: Add to enum
        // Note: "GAPO" is already the main ID, no need for alias
        .alias("Gapo")
        .alias("gapo")
        .alias("GOPALAKRISHNANRANGEINDEX")
        .alias("GopalakrishnanRangeIndex")
        .alias("gopalakrishnanrangeindex")
        .alias("gopalakrishnan_range_index")
        .alias("GOPALAKRISHNAN_RANGE_INDEX")
        .alias("Gopalakrishnan_Range_Index")
        .build()
}

/// Gator Oscillator
pub fn signature_gator_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("GATOR", CATEGORY)
        .name("Gator Oscillator")
        .description("Extension of Alligator indicator")
        .add_constraint(ParamConstraint::period(2, 200, 13))
        .add_constraint(
            ParamConstraint::new("ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("author", "Bill Williams")
        .machine_id(BarIndicatorId::Gator) // TODO: Add to enum
        // Note: "GATOR" is already the main ID, no need for alias
        .alias("Gator")
        .alias("gator")
        .alias("GATOROSCILLATOR")
        .alias("GatorOscillator")
        .alias("gatoroscillator")
        .alias("gator_oscillator")
        .alias("GATOR_OSCILLATOR")
        .alias("Gator_Oscillator")
        .build()
}

/// Highest High over period
pub fn signature_highest() -> IndicatorSignature {
    IndicatorSignature::builder("HIGHEST", CATEGORY)
        .name("Highest")
        .description("Highest high over N periods")
        .add_constraint(ParamConstraint::period(2, 512, 20))
        .machine_id(BarIndicatorId::Highest)
        // Note: "HIGHEST" is already the main ID, no need for alias
        .alias("Highest")
        .alias("highest")
        .build()
}

/// Inverse Fisher Transform RSI
pub fn signature_ift_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("IFT_RSI", CATEGORY)
        .name("Inverse Fisher Transform RSI")
        .description("RSI with inverse Fisher transform")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::IftRsi)
        // Note: "IFT_RSI" is already the main ID, no need for alias
        .alias("IftRsi")
        .alias("ift_rsi")
        .alias("INVERSEFISHERTRANSFORMRSI")
        .alias("InverseFisherTransformRSI")
        .alias("inversefishertransformrsi")
        .alias("inverse_fisher_transform_rsi")
        .alias("INVERSE_FISHER_TRANSFORM_RSI")
        .alias("Inverse_Fisher_Transform_Rsi")
        .build()
}

/// Intraday Momentum Index
pub fn signature_intraday_momentum_index() -> IndicatorSignature {
    IndicatorSignature::builder("IMI", CATEGORY)
        .name("Intraday Momentum Index")
        .description("Intraday momentum based on candle bodies")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Imi) // TODO: Add to enum
        // Note: "IMI" is already the main ID, no need for alias
        .alias("Imi")
        .alias("imi")
        .alias("INTRADAYMOMENTUMINDEX")
        .alias("IntradayMomentumIndex")
        .alias("intradaymomentumindex")
        .alias("intraday_momentum_index")
        .alias("INTRADAY_MOMENTUM_INDEX")
        .alias("Intraday_Momentum_Index")
        .build()
}

/// KDJ Indicator (Stochastic + J line)
pub fn signature_kdj() -> IndicatorSignature {
    IndicatorSignature::builder("KDJ", CATEGORY)
        .name("KDJ Indicator")
        .description("Stochastic with K, D, and J lines")
        .add_constraint(ParamConstraint::period(2, 200, 9))
        .metadata("outputs", "K, D, J")
        .machine_id(BarIndicatorId::Kdj)
        // Note: "KDJ" is already the main ID, no need for alias
        .alias("Kdj")
        .alias("kdj")
        .alias("KDJINDICATOR")
        .alias("KDJIndicator")
        .alias("kdjindicator")
        .alias("kdj_indicator")
        .alias("KDJ_INDICATOR")
        .alias("Kdj_Indicator")
        .build()
}

/// Know Sure Thing (KST)
pub fn signature_kst() -> IndicatorSignature {
    IndicatorSignature::builder("KST", CATEGORY)
        .name("Know Sure Thing")
        .description("Momentum oscillator by Martin Pring")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(9))
        )
        .add_constraint(
            ParamConstraint::new("roc_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::SMA))
        )
        .add_constraint(
            ParamConstraint::new("signal_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::SMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("author", "Martin Pring")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: SMA")
        .machine_id(BarIndicatorId::Kst) // TODO: Add to enum
        // Note: "KST" is already the main ID, no need for alias
        .alias("Kst")
        .alias("kst")
        .alias("KNOWSURETHING")
        .alias("KnowSureThing")
        .alias("knowsurething")
        .alias("know_sure_thing")
        .alias("KNOW_SURE_THING")
        .alias("Know_Sure_Thing")
        .build()
}

/// Klinger Volume Oscillator
pub fn signature_kvo() -> IndicatorSignature {
    IndicatorSignature::builder("KVO", CATEGORY)
        .name("Klinger Volume Oscillator")
        .description("Volume-based oscillator")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(34))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(55))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("uses_volume", "true")
        .machine_id(BarIndicatorId::Kvo)
        // Note: "KVO" is already the main ID, no need for alias
        .alias("Kvo")
        .alias("kvo")
        .alias("KLINGERVOLUMEOSCILLATOR")
        .alias("KlingerVolumeOscillator")
        .alias("klingervolumeoscillator")
        .alias("klinger_volume_oscillator")
        .alias("KLINGER_VOLUME_OSCILLATOR")
        .alias("Klinger_Volume_Oscillator")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Laguerre RSI
pub fn signature_laguerre_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("LAGUERRE_RSI", CATEGORY)
        .name("Laguerre RSI")
        .description("RSI using Laguerre transform")
        .add_constraint(
            ParamConstraint::new("gamma", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.5))
        )
        .metadata("author", "John Ehlers")
        .machine_id(BarIndicatorId::LaguerreRsi)
        // Note: "LAGUERRE_RSI" is already the main ID, no need for alias
        .alias("LaguerreRsi")
        .alias("laguerre_rsi")
        .alias("LAGUERRERSI")
        .alias("LaguerreRSI")
        .alias("laguerrersi")
        .alias("Laguerre_Rsi")
        .build()
}

/// Lowest Low over period
pub fn signature_lowest() -> IndicatorSignature {
    IndicatorSignature::builder("LOWEST", CATEGORY)
        .name("Lowest")
        .description("Lowest low over N periods")
        .add_constraint(ParamConstraint::period(2, 512, 20))
        .machine_id(BarIndicatorId::Lowest)
        // Note: "LOWEST" is already the main ID, no need for alias
        .alias("Lowest")
        .alias("lowest")
        .build()
}

/// Moving Average Cross
pub fn signature_ma_cross() -> IndicatorSignature {
    IndicatorSignature::builder("MA_CROSS", CATEGORY)
        .name("MA Cross")
        .description("Moving average crossover signals")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(12))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(26))
                .required()
        )
        .add_constraint(ParamConstraint::ma_type_named("fast_ma", MovingAverageType::EMA))
        .add_constraint(ParamConstraint::ma_type_named("slow_ma", MovingAverageType::EMA))
        .metadata("fast_ma_desc", "Fast MA type")
        .metadata("slow_ma_desc", "Slow MA type")
        .machine_id(BarIndicatorId::MaCross)
        // Note: "MA_CROSS" is already the main ID, no need for alias
        .alias("MaCross")
        .alias("ma_cross")
        .alias("MACROSS")
        .alias("MACross")
        .alias("macross")
        .alias("Ma_Cross")
        .build()
}

/// MACD - Moving Average Convergence Divergence
pub fn signature_macd() -> IndicatorSignature {
    IndicatorSignature::builder("MACD", CATEGORY)
        .name("MACD")
        .description("Moving Average Convergence Divergence")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(12))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(26))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(9))
        )
        .add_constraint(
            ParamConstraint::new("fast_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("slow_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("signal_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::source("fast_source", OhlcvField::Close)
        )
        .add_constraint(
            ParamConstraint::source("slow_source", OhlcvField::Close)
        )
        .metadata("author", "Gerald Appel")
        .metadata("outputs", "macd_line, signal_line, histogram")
        .machine_id(BarIndicatorId::Macd)
        // Note: "MACD" is already the main ID, no need for alias
        .alias("Macd")
        .alias("macd")
        .build()
}

/// MACD Histogram Z-Score
pub fn signature_macd_hist_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("MACD_HIST_Z", CATEGORY)
        .name("MACD Histogram Z-Score")
        .description("Z-score of MACD histogram")
        .add_constraint(ParamConstraint::period(2, 200, 12))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .machine_id(BarIndicatorId::MacdHistZ) // TODO: Add to enum
        // Note: "MACD_HIST_Z" is already the main ID, no need for alias
        .alias("MacdHistZ")
        .alias("macd_hist_z")
        .alias("MACDHISTOGRAMZSCORE")
        .alias("MACDHistogramZScore")
        .alias("macdhistogramzscore")
        .alias("macd_histogram_z_score")
        .alias("MACD_HISTOGRAM_Z_SCORE")
        .alias("Macd_Histogram_Z_Score")
        .build()
}

/// MACD Histogram
pub fn signature_macd_histogram() -> IndicatorSignature {
    IndicatorSignature::builder("MACD_HIST", CATEGORY)
        .name("MACD Histogram")
        .description("MACD histogram (MACD - Signal)")
        .add_constraint(ParamConstraint::period(2, 200, 12))
        .machine_id(BarIndicatorId::MacdHist) // TODO: Add to enum
        // Note: "MACD_HIST" is already the main ID, no need for alias
        .alias("MacdHist")
        .alias("macd_hist")
        .alias("MACDHISTOGRAM")
        .alias("MACDHistogram")
        .alias("macdhistogram")
        .alias("macd_histogram")
        .alias("MACD_HISTOGRAM")
        .alias("Macd_Histogram")
        .build()
}

/// MACD Signal Line
pub fn signature_macd_signal() -> IndicatorSignature {
    IndicatorSignature::builder("MACD_SIGNAL", CATEGORY)
        .name("MACD Signal Line")
        .description("MACD signal line (EMA of MACD)")
        .add_constraint(ParamConstraint::period(2, 200, 9))
        .machine_id(BarIndicatorId::MacdSignal) // TODO: Add to enum
        // Note: "MACD_SIGNAL" is already the main ID, no need for alias
        .alias("MacdSignal")
        .alias("macd_signal")
        .alias("MACDSIGNALLINE")
        .alias("MACDSignalLine")
        .alias("macdsignalline")
        .alias("macd_signal_line")
        .alias("MACD_SIGNAL_LINE")
        .alias("Macd_Signal_Line")
        .build()
}

/// Market Cipher - комплексный индикатор рынка
pub fn signature_market_cipher() -> IndicatorSignature {
    IndicatorSignature::builder("MARKET_CIPHER", CATEGORY)
        .name("Market Cipher")
        .description("Comprehensive market analysis indicator")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MarketCipher)
        // Note: "MARKET_CIPHER" is already the main ID, no need for alias
        .alias("MarketCipher")
        .alias("market_cipher")
        .alias("MARKETCIPHER")
        .alias("marketcipher")
        .alias("Market_Cipher")
        .build()
}

/// Momentum Z-Score
pub fn signature_momentum_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("MOM_ZSCORE", CATEGORY)
        .name("Momentum Z-Score")
        .description("Standardized momentum indicator")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MomZscore) // TODO: Add to enum
        // Note: "MOM_ZSCORE" is already the main ID, no need for alias
        .alias("MomZscore")
        .alias("mom_zscore")
        .alias("MOMENTUMZSCORE")
        .alias("MomentumZScore")
        .alias("momentumzscore")
        .alias("momentum_z_score")
        .alias("MOMENTUM_Z_SCORE")
        .alias("Momentum_Z_Score")
        .build()
}

/// Multi-Timeframe Momentum Divergence
pub fn signature_multi_timeframe_momentum_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("MTF_MOM_DIV", CATEGORY)
        .name("Multi-Timeframe Momentum Divergence")
        .description("Momentum divergence across timeframes")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::MtfMomDiv) // TODO: Add to enum
        // Note: "MTF_MOM_DIV" is already the main ID, no need for alias
        .alias("MtfMomDiv")
        .alias("mtf_mom_div")
        .alias("MULTITIMEFRAMEMOMENTUMDIVERGENCE")
        .alias("MultiTimeframeMomentumDivergence")
        .alias("multitimeframemomentumdivergence")
        .alias("multi_timeframe_momentum_divergence")
        .alias("MULTI_TIMEFRAME_MOMENTUM_DIVERGENCE")
        .alias("Multi_Timeframe_Momentum_Divergence")
        .build()
}

/// Neural Momentum Network
pub fn signature_neural_momentum_network() -> IndicatorSignature {
    IndicatorSignature::builder("NEURAL_MOM", CATEGORY)
        .name("Neural Momentum Network")
        .description("Neural network-based momentum")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::NeuralMom) // TODO: Add to enum
        // Note: "NEURAL_MOM" is already the main ID, no need for alias
        .alias("NeuralMom")
        .alias("neural_mom")
        .alias("NEURALMOMENTUMNETWORK")
        .alias("NeuralMomentumNetwork")
        .alias("neuralmomentumnetwork")
        .alias("neural_momentum_network")
        .alias("NEURAL_MOMENTUM_NETWORK")
        .alias("Neural_Momentum_Network")
        .build()
}

/// On-Balance Volume
pub fn signature_obv() -> IndicatorSignature {
    IndicatorSignature::builder("MO_OBV", CATEGORY)
        .name("On-Balance Volume")
        .description("Cumulative volume indicator")
        .metadata("author", "Joseph Granville")
        .metadata("uses_volume", "true")
        .machine_id(BarIndicatorId::MoObv)
        // Note: "MO_OBV" is already the main ID, no need for alias
        .alias("MoObv")
        .alias("mo_obv")
        .alias("ONBALANCEVOLUME")
        .alias("OnBalanceVolume")
        .alias("onbalancevolume")
        .alias("on_balance_volume")
        .alias("ON_BALANCE_VOLUME")
        .alias("On_Balance_Volume")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Parabolic SAR (Stop and Reverse)
pub fn signature_parabolic_sar() -> IndicatorSignature {
    IndicatorSignature::builder("PSAR", CATEGORY)
        .name("Parabolic SAR")
        .description("Stop and Reverse indicator")
        .add_constraint(
            ParamConstraint::new("af_start", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.1))
                .with_default(ParamValue::F64(0.02))
        )
        .add_constraint(
            ParamConstraint::new("af_increment", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.1))
                .with_default(ParamValue::F64(0.02))
        )
        .add_constraint(
            ParamConstraint::new("af_max", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(0.5))
                .with_default(ParamValue::F64(0.2))
        )
        .metadata("author", "J. Welles Wilder")
        .machine_id(BarIndicatorId::Psar) // TODO: Add to enum
        // Note: "PSAR" is already the main ID, no need for alias
        .alias("Psar")
        .alias("psar")
        .alias("PARABOLICSAR")
        .alias("ParabolicSAR")
        .alias("parabolicsar")
        .alias("parabolic_sar")
        .alias("PARABOLIC_SAR")
        .alias("Parabolic_Sar")
        .build()
}

/// Polarized Fractal Efficiency
pub fn signature_pfe() -> IndicatorSignature {
    IndicatorSignature::builder("PFE", CATEGORY)
        .name("Polarized Fractal Efficiency")
        .description("Measures trend efficiency")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .machine_id(BarIndicatorId::Pfe) // TODO: Add to enum
        // Note: "PFE" is already the main ID, no need for alias
        .alias("Pfe")
        .alias("pfe")
        .alias("POLARIZEDFRACTALEFFICIENCY")
        .alias("PolarizedFractalEfficiency")
        .alias("polarizedfractalefficiency")
        .alias("polarized_fractal_efficiency")
        .alias("POLARIZED_FRACTAL_EFFICIENCY")
        .alias("Polarized_Fractal_Efficiency")
        .build()
}

/// Price Momentum Oscillator
pub fn signature_pmo() -> IndicatorSignature {
    IndicatorSignature::builder("PMO", CATEGORY)
        .name("Price Momentum Oscillator")
        .description("Double-smoothed ROC oscillator")
        .add_constraint(ParamConstraint::period(2, 200, 35))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::EMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Pmo)
        // Note: "PMO" is already the main ID, no need for alias
        .alias("Pmo")
        .alias("pmo")
        .alias("PRICEMOMENTUMOSCILLATOR")
        .alias("PriceMomentumOscillator")
        .alias("pricemomentumoscillator")
        .alias("price_momentum_oscillator")
        .alias("PRICE_MOMENTUM_OSCILLATOR")
        .alias("Price_Momentum_Oscillator")
        .build()
}

/// Percentage Price Oscillator
pub fn signature_ppo() -> IndicatorSignature {
    IndicatorSignature::builder("PPO", CATEGORY)
        .name("Percentage Price Oscillator")
        .description("Percentage version of MACD")
        .add_constraint(
            ParamConstraint::new("fast_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(12))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("slow_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(26))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(9))
        )
        .add_constraint(
            ParamConstraint::new("fast_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("slow_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("signal_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .machine_id(BarIndicatorId::Ppo)
        // Note: "PPO" is already the main ID, no need for alias
        .alias("Ppo")
        .alias("ppo")
        .alias("PERCENTAGEPRICEOSCILLATOR")
        .alias("PercentagePriceOscillator")
        .alias("percentagepriceoscillator")
        .alias("percentage_price_oscillator")
        .alias("PERCENTAGE_PRICE_OSCILLATOR")
        .alias("Percentage_Price_Oscillator")
        .build()
}

/// PPO Signal Line
pub fn signature_ppo_signal() -> IndicatorSignature {
    IndicatorSignature::builder("PPO_SIGNAL", CATEGORY)
        .name("PPO Signal Line")
        .description("Signal line for PPO")
        .add_constraint(ParamConstraint::period(2, 200, 9))
        .machine_id(BarIndicatorId::PpoSignal) // TODO: Add to enum
        // Note: "PPO_SIGNAL" is already the main ID, no need for alias
        .alias("PpoSignal")
        .alias("ppo_signal")
        .alias("PPOSIGNALLINE")
        .alias("PPOSignalLine")
        .alias("pposignalline")
        .alias("ppo_signal_line")
        .alias("PPO_SIGNAL_LINE")
        .alias("Ppo_Signal_Line")
        .build()
}

/// Pressure - давление рынка
pub fn signature_pressure() -> IndicatorSignature {
    IndicatorSignature::builder("PRESSURE", CATEGORY)
        .name("Pressure")
        .description("Market pressure indicator")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Pressure)
        // Note: "PRESSURE" is already the main ID, no need for alias
        .alias("Pressure")
        .alias("pressure")
        .build()
}

/// Psychological Line
pub fn signature_psl() -> IndicatorSignature {
    IndicatorSignature::builder("PSL", CATEGORY)
        .name("Psychological Line")
        .description("Percentage of up days over period")
        .add_constraint(ParamConstraint::period(2, 200, 12))
        .machine_id(BarIndicatorId::Psl)
        // Note: "PSL" is already the main ID, no need for alias
        .alias("Psl")
        .alias("psl")
        .alias("PSYCHOLOGICALLINE")
        .alias("PsychologicalLine")
        .alias("psychologicalline")
        .alias("psychological_line")
        .alias("PSYCHOLOGICAL_LINE")
        .alias("Psychological_Line")
        .build()
}

/// Quantitative Qualitative Estimation
pub fn signature_qqe() -> IndicatorSignature {
    IndicatorSignature::builder("QQE", CATEGORY)
        .name("QQE")
        .description("Quantitative Qualitative Estimation")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Qqe)
        // Note: "QQE" is already the main ID, no need for alias
        .alias("Qqe")
        .alias("qqe")
        .build()
}

/// QStick
pub fn signature_qstick() -> IndicatorSignature {
    IndicatorSignature::builder("QSTICK", CATEGORY)
        .name("QStick")
        .description("Candlestick momentum indicator")
        .add_constraint(ParamConstraint::period(2, 200, 8))
        .machine_id(BarIndicatorId::Qstick)
        // Note: "QSTICK" is already the main ID, no need for alias
        .alias("Qstick")
        .alias("qstick")
        .alias("QStick")
        .build()
}

/// Relative Momentum Index
pub fn signature_rmi() -> IndicatorSignature {
    IndicatorSignature::builder("RMI", CATEGORY)
        .name("Relative Momentum Index")
        .description("RSI with momentum lookback")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: RMA")
        .machine_id(BarIndicatorId::Rmi) // TODO: Add to enum
        // Note: "RMI" is already the main ID, no need for alias
        .alias("Rmi")
        .alias("rmi")
        .alias("RELATIVEMOMENTUMINDEX")
        .alias("RelativeMomentumIndex")
        .alias("relativemomentumindex")
        .alias("relative_momentum_index")
        .alias("RELATIVE_MOMENTUM_INDEX")
        .alias("Relative_Momentum_Index")
        .build()
}

/// Rate of Change
pub fn signature_roc() -> IndicatorSignature {
    IndicatorSignature::builder("ROC", CATEGORY)
        .name("Rate of Change")
        .description("Percentage change over N periods")
        .add_constraint(ParamConstraint::period(2, 512, 12))
        .add_constraint(
            ParamConstraint::new("use_log", ParamType::Bool)
                .with_default(ParamValue::Bool(false))
        )
        .machine_id(BarIndicatorId::Roc)
        // Note: "ROC" is already the main ID, no need for alias
        .alias("Roc")
        .alias("roc")
        .alias("RATEOFCHANGE")
        .alias("RateofChange")
        .alias("rateofchange")
        .alias("rate_of_change")
        .alias("RATE_OF_CHANGE")
        .alias("Rate_Of_Change")
        .build()
}

/// ROC Percentile
pub fn signature_roc_percentile() -> IndicatorSignature {
    IndicatorSignature::builder("ROC_PCT", CATEGORY)
        .name("ROC Percentile")
        .description("Percentile rank of ROC")
        .add_constraint(ParamConstraint::period(2, 200, 12))
        .machine_id(BarIndicatorId::RocPct) // TODO: Add to enum
        // Note: "ROC_PCT" is already the main ID, no need for alias
        .alias("RocPct")
        .alias("roc_pct")
        .alias("ROCPERCENTILE")
        .alias("ROCPercentile")
        .alias("rocpercentile")
        .alias("roc_percentile")
        .alias("ROC_PERCENTILE")
        .alias("Roc_Percentile")
        .build()
}

/// Relative Strength Index
pub fn signature_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("RSI", CATEGORY)
        .name("Relative Strength Index")
        .description("Momentum oscillator (0-100)")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "J. Welles Wilder")
        .metadata("range", "0-100")
        .metadata("overbought", "70")
        .metadata("oversold", "30")
        .machine_id(BarIndicatorId::Rsi)
        // Note: "RSI" is already the main ID, no need for alias
        .alias("Rsi")
        .alias("rsi")
        .alias("RELATIVESTRENGTHINDEX")
        .alias("RelativeStrengthIndex")
        .alias("relativestrengthindex")
        .alias("relative_strength_index")
        .alias("RELATIVE_STRENGTH_INDEX")
        .alias("Relative_Strength_Index")
        .build()
}

/// RSI Percentile Bands
pub fn signature_rsi_percentile_bands() -> IndicatorSignature {
    IndicatorSignature::builder("RSI_PCT_BANDS", CATEGORY)
        .name("RSI Percentile Bands")
        .description("Dynamic RSI bands based on percentiles")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RsiPctBands) // TODO: Add to enum
        // Note: "RSI_PCT_BANDS" is already the main ID, no need for alias
        .alias("RsiPctBands")
        .alias("rsi_pct_bands")
        .alias("RSIPERCENTILEBANDS")
        .alias("RSIPercentileBands")
        .alias("rsipercentilebands")
        .alias("rsi_percentile_bands")
        .alias("RSI_PERCENTILE_BANDS")
        .alias("Rsi_Percentile_Bands")
        .build()
}

/// RSI Percentile Rank
pub fn signature_rsi_percentile_rank() -> IndicatorSignature {
    IndicatorSignature::builder("RSI_PCT_RANK", CATEGORY)
        .name("RSI Percentile Rank")
        .description("Percentile rank of RSI")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RsiPctRank) // TODO: Add to enum
        // Note: "RSI_PCT_RANK" is already the main ID, no need for alias
        .alias("RsiPctRank")
        .alias("rsi_pct_rank")
        .alias("RSIPERCENTILERANK")
        .alias("RSIPercentileRank")
        .alias("rsipercentilerank")
        .alias("rsi_percentile_rank")
        .alias("RSI_PERCENTILE_RANK")
        .alias("Rsi_Percentile_Rank")
        .build()
}

/// RSI Z-Score
pub fn signature_rsi_zscore() -> IndicatorSignature {
    IndicatorSignature::builder("RSI_ZSCORE", CATEGORY)
        .name("RSI Z-Score")
        .description("Standardized RSI")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::RsiZscore)
        // Note: "RSI_ZSCORE" is already the main ID, no need for alias
        .alias("RsiZscore")
        .alias("rsi_zscore")
        .alias("RSIZSCORE")
        .alias("RSIZScore")
        .alias("rsizscore")
        .alias("rsi_z_score")
        .alias("RSI_Z_SCORE")
        .alias("Rsi_Z_Score")
        .build()
}

/// RSI with Overlayed MA (RSIOMA)
pub fn signature_rsioma() -> IndicatorSignature {
    IndicatorSignature::builder("RSIOMA", CATEGORY)
        .name("RSI OMA")
        .description("RSI with overlayed moving average")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::Rsioma) // TODO: Add to enum
        // Note: "RSIOMA" is already the main ID, no need for alias
        .alias("Rsioma")
        .alias("rsioma")
        .alias("rsi_oma")
        .alias("RSI_OMA")
        .alias("Rsi_Oma")
        .build()
}

/// RSX - Jurik's RSI
pub fn signature_rsx() -> IndicatorSignature {
    IndicatorSignature::builder("RSX", CATEGORY)
        .name("RSX")
        .description("Jurik's smooth RSI")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("author", "Mark Jurik")
        .machine_id(BarIndicatorId::Rsx)
        // Note: "RSX" is already the main ID, no need for alias
        .alias("Rsx")
        .alias("rsx")
        .build()
}

/// Relative Vigor Index
pub fn signature_rvgi() -> IndicatorSignature {
    IndicatorSignature::builder("RVGI", CATEGORY)
        .name("Relative Vigor Index")
        .description("Measures strength of trend")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .machine_id(BarIndicatorId::Rvgi)
        // Note: "RVGI" is already the main ID, no need for alias
        .alias("Rvgi")
        .alias("rvgi")
        .alias("RELATIVEVIGORINDEX")
        .alias("RelativeVigorIndex")
        .alias("relativevigorindex")
        .alias("relative_vigor_index")
        .alias("RELATIVE_VIGOR_INDEX")
        .alias("Relative_Vigor_Index")
        .build()
}

/// Random Walk Index
pub fn signature_rwi() -> IndicatorSignature {
    IndicatorSignature::builder("RWI", CATEGORY)
        .name("Random Walk Index")
        .description("Determines if price is trending or random")
        .add_constraint(ParamConstraint::period(2, 200, 9))
        .machine_id(BarIndicatorId::Rwi)
        // Note: "RWI" is already the main ID, no need for alias
        .alias("Rwi")
        .alias("rwi")
        .alias("RANDOMWALKINDEX")
        .alias("RandomWalkIndex")
        .alias("randomwalkindex")
        .alias("random_walk_index")
        .alias("RANDOM_WALK_INDEX")
        .alias("Random_Walk_Index")
        .build()
}

/// Stochastic Momentum Index
pub fn signature_smi() -> IndicatorSignature {
    IndicatorSignature::builder("SMI", CATEGORY)
        .name("Stochastic Momentum Index")
        .description("Refined stochastic indicator")
        .add_constraint(ParamConstraint::period(2, 200, 13))
        .machine_id(BarIndicatorId::Smi)
        // Note: "SMI" is already the main ID, no need for alias
        .alias("Smi")
        .alias("smi")
        .alias("STOCHASTICMOMENTUMINDEX")
        .alias("StochasticMomentumIndex")
        .alias("stochasticmomentumindex")
        .alias("stochastic_momentum_index")
        .alias("STOCHASTIC_MOMENTUM_INDEX")
        .alias("Stochastic_Momentum_Index")
        .build()
}

/// Schaff Trend Cycle
pub fn signature_stc() -> IndicatorSignature {
    IndicatorSignature::builder("STC", CATEGORY)
        .name("Schaff Trend Cycle")
        .description("Combines MACD and stochastic")
        .add_constraint(ParamConstraint::period(2, 200, 10))
        .metadata("author", "Doug Schaff")
        .machine_id(BarIndicatorId::Stc)
        // Note: "STC" is already the main ID, no need for alias
        .alias("Stc")
        .alias("stc")
        .alias("SCHAFFTRENDCYCLE")
        .alias("SchaffTrendCycle")
        .alias("schafftrendcycle")
        .alias("schaff_trend_cycle")
        .alias("SCHAFF_TREND_CYCLE")
        .alias("Schaff_Trend_Cycle")
        .build()
}

/// Stochastic RSI
pub fn signature_stochastic_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("STOCH_RSI", CATEGORY)
        .name("Stochastic RSI")
        .description("Stochastic applied to RSI")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .add_constraint(
            ParamConstraint::new("k_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::SMA))
        )
        .add_constraint(
            ParamConstraint::new("d_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::SMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .machine_id(BarIndicatorId::StochRsi) // TODO: Add to enum
        // Note: "STOCH_RSI" is already the main ID, no need for alias
        .alias("StochRsi")
        .alias("stoch_rsi")
        .alias("STOCHASTICRSI")
        .alias("StochasticRSI")
        .alias("stochasticrsi")
        .alias("stochastic_rsi")
        .alias("STOCHASTIC_RSI")
        .alias("Stochastic_Rsi")
        .build()
}

/// Stochastics (%K, %D)
pub fn signature_stochastics() -> IndicatorSignature {
    IndicatorSignature::builder("STOCH", CATEGORY)
        .name("Stochastics")
        .description("Stochastic oscillator (%K, %D)")
        .add_constraint(
            ParamConstraint::new("period_k", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(512))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("period_d", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(512))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .metadata("author", "George Lane")
        .metadata("outputs", "%K, %D")
        .machine_id(BarIndicatorId::Stoch) // TODO: Add to enum
        // Note: "STOCH" is already the main ID, no need for alias
        .alias("Stoch")
        .alias("stoch")
        .alias("STOCHASTICS")
        .alias("Stochastics")
        .alias("stochastics")
        .build()
}

/// Stochastic KD (alternative implementation)
pub fn signature_stochastikd() -> IndicatorSignature {
    IndicatorSignature::builder("STOCHKD", CATEGORY)
        .name("Stochastic KD")
        .description("Alternative stochastic implementation")
        .add_constraint(ParamConstraint::period(2, 512, 14))
        .machine_id(BarIndicatorId::Stochkd) // TODO: Add to enum
        // Note: "STOCHKD" is already the main ID, no need for alias
        .alias("Stochkd")
        .alias("stochkd")
        .alias("STOCHASTICKD")
        .alias("StochasticKD")
        .alias("stochastickd")
        .alias("stochastic_kd")
        .alias("STOCHASTIC_KD")
        .alias("Stochastic_Kd")
        .build()
}

/// Sweep Reversion - sweep and reversion indicator
pub fn signature_sweep_reversion() -> IndicatorSignature {
    IndicatorSignature::builder("SWEEP_REV", CATEGORY)
        .name("Sweep Reversion")
        .description("Detects sweeps and reversions")
        .add_constraint(ParamConstraint::period(2, 200, 20))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::RMA))
        .metadata("note", "Uses ATR with configurable MA type (default Wilder)")
        .machine_id(BarIndicatorId::SweepRev) // TODO: Add to enum
        // Note: "SWEEP_REV" is already the main ID, no need for alias
        .alias("SweepRev")
        .alias("sweep_rev")
        .alias("SWEEPREVERSION")
        .alias("SweepReversion")
        .alias("sweepreversion")
        .alias("sweep_reversion")
        .alias("SWEEP_REVERSION")
        .alias("Sweep_Reversion")
        .build()
}

/// Swing Age - возраст текущего свинга
pub fn signature_swing_age() -> IndicatorSignature {
    IndicatorSignature::builder("SWING_AGE", CATEGORY)
        .name("Swing Age")
        .description("Age of current swing in bars")
        .add_constraint(ParamConstraint::period(2, 200, 5))
        .machine_id(BarIndicatorId::SwingAge)
        // Note: "SWING_AGE" is already the main ID, no need for alias
        .alias("SwingAge")
        .alias("swing_age")
        .alias("SWINGAGE")
        .alias("swingage")
        .alias("Swing_Age")
        .build()
}

/// Swings - определение swing high/low
pub fn signature_swings() -> IndicatorSignature {
    IndicatorSignature::builder("SWINGS", CATEGORY)
        .name("Swings")
        .description("Identifies swing highs and lows")
        .add_constraint(ParamConstraint::period(2, 200, 5))
        .machine_id(BarIndicatorId::Swings)
        // Note: "SWINGS" is already the main ID, no need for alias
        .alias("Swings")
        .alias("swings")
        .build()
}

/// Swings Soft - мягкое определение свингов
pub fn signature_swings_soft() -> IndicatorSignature {
    IndicatorSignature::builder("SWINGS_SOFT", CATEGORY)
        .name("Swings Soft")
        .description("Soft swing detection")
        .add_constraint(ParamConstraint::period(2, 200, 5))
        .machine_id(BarIndicatorId::SwingsSoft)
        // Note: "SWINGS_SOFT" is already the main ID, no need for alias
        .alias("SwingsSoft")
        .alias("swings_soft")
        .alias("SWINGSSOFT")
        .alias("swingssoft")
        .alias("Swings_Soft")
        .build()
}

/// Traders Dynamic Index
pub fn signature_tdi() -> IndicatorSignature {
    IndicatorSignature::builder("TDI", CATEGORY)
        .name("Traders Dynamic Index")
        .description("Combination of RSI, Bollinger Bands, and MAs")
        .add_constraint(ParamConstraint::period(2, 200, 13))
        .machine_id(BarIndicatorId::Tdi) // TODO: Add to enum
        // Note: "TDI" is already the main ID, no need for alias
        .alias("Tdi")
        .alias("tdi")
        .alias("TRADERSDYNAMICINDEX")
        .alias("TradersDynamicIndex")
        .alias("tradersdynamicindex")
        .alias("traders_dynamic_index")
        .alias("TRADERS_DYNAMIC_INDEX")
        .alias("Traders_Dynamic_Index")
        .build()
}

/// TRIX - Triple Exponential Average
pub fn signature_trix() -> IndicatorSignature {
    IndicatorSignature::builder("TRIX", CATEGORY)
        .name("TRIX")
        .description("Triple exponential average ROC")
        .add_constraint(ParamConstraint::period(2, 200, 15))
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(9))
        )
        .add_constraint(
            ParamConstraint::new("smoothing_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("signal_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Trix)
        // Note: "TRIX" is already the main ID, no need for alias
        .alias("Trix")
        .alias("trix")
        .build()
}

/// True Strength Index
pub fn signature_tsi() -> IndicatorSignature {
    IndicatorSignature::builder("TSI", CATEGORY)
        .name("True Strength Index")
        .description("Double smoothed momentum oscillator")
        .add_constraint(
            ParamConstraint::new("first_smoothing", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(25))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("second_smoothing", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(13))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("signal_period", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(13))
        )
        .add_constraint(
            ParamConstraint::new("smoothing_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(
            ParamConstraint::new("signal_ma_type", ParamType::MaType)
                .with_default(ParamValue::MaType(MovingAverageType::EMA))
        )
        .add_constraint(ParamConstraint::source("source", OhlcvField::Close))
        .metadata("author", "William Blau")
        .metadata("ma_support", "Supports all 11 MA types for smoothing")
        .metadata("ma_note", "Original default: EMA")
        .machine_id(BarIndicatorId::Tsi) // TODO: Add to enum
        // Note: "TSI" is already the main ID, no need for alias
        .alias("Tsi")
        .alias("tsi")
        .alias("TRUESTRENGTHINDEX")
        .alias("TrueStrengthIndex")
        .alias("truestrengthindex")
        .alias("true_strength_index")
        .alias("TRUE_STRENGTH_INDEX")
        .alias("True_Strength_Index")
        .build()
}

/// Ultimate Oscillator
pub fn signature_ultimate_oscillator() -> IndicatorSignature {
    IndicatorSignature::builder("UO", CATEGORY)
        .name("Ultimate Oscillator")
        .description("Multi-timeframe momentum oscillator")
        .add_constraint(
            ParamConstraint::new("period1", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(7))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("period2", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(14))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("period3", ParamType::USize)
                .with_min(ParamValue::USize(2))
                .with_max(ParamValue::USize(200))
                .with_default(ParamValue::USize(28))
                .required()
        )
        .metadata("author", "Larry Williams")
        .machine_id(BarIndicatorId::Uo) // TODO: Add to enum
        // Note: "UO" is already the main ID, no need for alias
        .alias("Uo")
        .alias("uo")
        .alias("ULTIMATEOSCILLATOR")
        .alias("UltimateOscillator")
        .alias("ultimateoscillator")
        .alias("ultimate_oscillator")
        .alias("ULTIMATE_OSCILLATOR")
        .alias("Ultimate_Oscillator")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Ultimate Oscillator Smooth
pub fn signature_ultimate_oscillator_smooth() -> IndicatorSignature {
    IndicatorSignature::builder("UO_SMOOTH", CATEGORY)
        .name("Ultimate Oscillator Smooth")
        .description("Smoothed version of Ultimate Oscillator")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .machine_id(BarIndicatorId::UoSmooth) // TODO: Add to enum
        // Note: "UO_SMOOTH" is already the main ID, no need for alias
        .alias("UoSmooth")
        .alias("uo_smooth")
        .alias("ULTIMATEOSCILLATORSMOOTH")
        .alias("UltimateOscillatorSmooth")
        .alias("ultimateoscillatorsmooth")
        .alias("ultimate_oscillator_smooth")
        .alias("ULTIMATE_OSCILLATOR_SMOOTH")
        .alias("Ultimate_Oscillator_Smooth")
        .build()
}

/// Vertical Horizontal Filter
pub fn signature_vhf() -> IndicatorSignature {
    IndicatorSignature::builder("VHF", CATEGORY)
        .name("Vertical Horizontal Filter")
        .description("Detects trending vs ranging market")
        .add_constraint(ParamConstraint::period(2, 200, 28))
        .metadata("author", "Adam White")
        .machine_id(BarIndicatorId::Vhf)
        // Note: "VHF" is already the main ID, no need for alias
        .alias("Vhf")
        .alias("vhf")
        .alias("VERTICALHORIZONTALFILTER")
        .alias("VerticalHorizontalFilter")
        .alias("verticalhorizontalfilter")
        .alias("vertical_horizontal_filter")
        .alias("VERTICAL_HORIZONTAL_FILTER")
        .alias("Vertical_Horizontal_Filter")
        .build()
}

/// VHF with MA - VHF со скользящей средней
pub fn signature_vhf_ma() -> IndicatorSignature {
    IndicatorSignature::builder("VHF_MA", CATEGORY)
        .name("VHF MA")
        .description("VHF with moving average overlay")
        .add_constraint(ParamConstraint::period(2, 200, 28))
        .add_constraint(ParamConstraint::ma_type(MovingAverageType::SMA))
        .machine_id(BarIndicatorId::VhfMa)
        // Note: "VHF_MA" is already the main ID, no need for alias
        .alias("VhfMa")
        .alias("vhf_ma")
        .alias("VHFMA")
        .alias("vhfma")
        .alias("Vhf_Ma")
        .build()
}

/// Volume Weighted RSI
pub fn signature_volume_weighted_rsi() -> IndicatorSignature {
    IndicatorSignature::builder("VWRSI", CATEGORY)
        .name("Volume Weighted RSI")
        .description("RSI weighted by volume")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("uses_volume", "true")
        .machine_id(BarIndicatorId::Vwrsi) // TODO: Add to enum
        // Note: "VWRSI" is already the main ID, no need for alias
        .alias("Vwrsi")
        .alias("vwrsi")
        .alias("VOLUMEWEIGHTEDRSI")
        .alias("VolumeWeightedRSI")
        .alias("volumeweightedrsi")
        .alias("volume_weighted_rsi")
        .alias("VOLUME_WEIGHTED_RSI")
        .alias("Volume_Weighted_Rsi")
        .source_type(SourceType::PriceAndVolume)
        .build()
}

/// Vortex Indicator
pub fn signature_vortex_indicator() -> IndicatorSignature {
    IndicatorSignature::builder("VORTEX", CATEGORY)
        .name("Vortex Indicator")
        .description("Identifies trend direction and strength")
        .add_constraint(ParamConstraint::period(2, 200, 14))
        .metadata("outputs", "VI+, VI-")
        .machine_id(BarIndicatorId::Vortex) // TODO: Add to enum
        // Note: "VORTEX" is already the main ID, no need for alias
        .alias("Vortex")
        .alias("vortex")
        .alias("VORTEXINDICATOR")
        .alias("VortexIndicator")
        .alias("vortexindicator")
        .alias("vortex_indicator")
        .alias("VORTEX_INDICATOR")
        .alias("Vortex_Indicator")
        .build()
}

/// Williams %R
pub fn signature_williams_r() -> IndicatorSignature {
    IndicatorSignature::builder("WILLIAMS_R", CATEGORY)
        .name("Williams %R")
        .description("Momentum indicator (-100 to 0)")
        .add_constraint(ParamConstraint::period(2, 512, 14))
        .metadata("author", "Larry Williams")
        .metadata("range", "-100 to 0")
        .metadata("overbought", "-20")
        .metadata("oversold", "-80")
        .machine_id(BarIndicatorId::WilliamsR)
        // Note: "WILLIAMS_R" is already the main ID, no need for alias
        .alias("WilliamsR")
        .alias("williams_r")
        .alias("WILLIAMS%R")
        .alias("Williams%R")
        .alias("williams%r")
        .alias("williams_%r")
        .alias("WILLIAMS_%R")
        .alias("Williams_%r")
        .build()
}

/// ZigZag - identifies significant price swings
pub fn signature_zigzag() -> IndicatorSignature {
    IndicatorSignature::builder("ZIGZAG", CATEGORY)
        .name("ZigZag")
        .description("Identifies significant price swings")
        .add_constraint(
            ParamConstraint::new("deviation", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.05))
        )
        .machine_id(BarIndicatorId::Zigzag) // TODO: Add to enum
        // Note: "ZIGZAG" is already the main ID, no need for alias
        .alias("Zigzag")
        .alias("zigzag")
        .alias("ZigZag")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Momentum indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("ADAPTIVE_STOCH", signature_adaptive_stochastic as fn() -> IndicatorSignature),
    ("ADX", signature_adx as fn() -> IndicatorSignature),
    ("AMAT", signature_amat as fn() -> IndicatorSignature),
    ("APO", signature_apo as fn() -> IndicatorSignature),
    ("AROON", signature_aroon as fn() -> IndicatorSignature),
    ("AROON_DOWN", signature_aroon_down as fn() -> IndicatorSignature),
    ("AROON_OSC", signature_aroon_oscillator as fn() -> IndicatorSignature),
    ("AROON_UP", signature_aroon_up as fn() -> IndicatorSignature),
    ("ATR_RSI", signature_atr_rsi as fn() -> IndicatorSignature),
    ("AUTO_FIBO", signature_auto_fibo as fn() -> IndicatorSignature),
    ("BB_PERIOD", signature_bb_period as fn() -> IndicatorSignature),
    ("BIAS", signature_bias as fn() -> IndicatorSignature),
    ("BOP", signature_bop as fn() -> IndicatorSignature),
    ("CANDLE_PATTERNS", signature_candle_patterns as fn() -> IndicatorSignature),
    ("CCI", signature_cci as fn() -> IndicatorSignature),
    ("COG", signature_center_of_gravity as fn() -> IndicatorSignature),
    ("CFO", signature_cfo as fn() -> IndicatorSignature),
    ("CMO", signature_cmo as fn() -> IndicatorSignature),
    ("CONNORS_RSI", signature_connors_rsi as fn() -> IndicatorSignature),
    ("COPPOCK", signature_coppock as fn() -> IndicatorSignature),
    ("DEMARKER", signature_demarker as fn() -> IndicatorSignature),
    ("DSP", signature_detrended_synthetic_price as fn() -> IndicatorSignature),
    ("DM", signature_dm as fn() -> IndicatorSignature),
    ("DPO", signature_dpo as fn() -> IndicatorSignature),
    ("DPO_PCT", signature_dpo_percent as fn() -> IndicatorSignature),
    ("DSS", signature_dss_bressert as fn() -> IndicatorSignature),
    ("EHLERS_CC", signature_ehlers_cyber_cycle as fn() -> IndicatorSignature),
    ("EHLERS_ROCKET", signature_ehlers_rocket_rsi as fn() -> IndicatorSignature),
    ("ELDER_IMPULSE", signature_elder_impulse as fn() -> IndicatorSignature),
    ("ELDER_RAY", signature_elder_ray as fn() -> IndicatorSignature),
    ("EMA_SLOPE", signature_ema_slope as fn() -> IndicatorSignature),
    ("EWMAC", signature_ewmac as fn() -> IndicatorSignature),
    ("EWMAC_ROBUST", signature_ewmac_robust as fn() -> IndicatorSignature),
    ("MO_FISHER", signature_fisher_transform as fn() -> IndicatorSignature),
    ("GAPO", signature_gapo as fn() -> IndicatorSignature),
    ("GATOR", signature_gator_oscillator as fn() -> IndicatorSignature),
    ("HIGHEST", signature_highest as fn() -> IndicatorSignature),
    ("IFT_RSI", signature_ift_rsi as fn() -> IndicatorSignature),
    ("IMI", signature_intraday_momentum_index as fn() -> IndicatorSignature),
    ("KDJ", signature_kdj as fn() -> IndicatorSignature),
    ("KST", signature_kst as fn() -> IndicatorSignature),
    ("KVO", signature_kvo as fn() -> IndicatorSignature),
    ("LAGUERRE_RSI", signature_laguerre_rsi as fn() -> IndicatorSignature),
    ("LOWEST", signature_lowest as fn() -> IndicatorSignature),
    ("MA_CROSS", signature_ma_cross as fn() -> IndicatorSignature),
    ("MACD", signature_macd as fn() -> IndicatorSignature),
    ("MACD_HIST_Z", signature_macd_hist_zscore as fn() -> IndicatorSignature),
    ("MACD_HIST", signature_macd_histogram as fn() -> IndicatorSignature),
    ("MACD_SIGNAL", signature_macd_signal as fn() -> IndicatorSignature),
    ("MARKET_CIPHER", signature_market_cipher as fn() -> IndicatorSignature),
    ("MOM_ZSCORE", signature_momentum_zscore as fn() -> IndicatorSignature),
    ("MTF_MOM_DIV", signature_multi_timeframe_momentum_divergence as fn() -> IndicatorSignature),
    ("NEURAL_MOM", signature_neural_momentum_network as fn() -> IndicatorSignature),
    ("MO_OBV", signature_obv as fn() -> IndicatorSignature),
    ("PSAR", signature_parabolic_sar as fn() -> IndicatorSignature),
    ("PFE", signature_pfe as fn() -> IndicatorSignature),
    ("PMO", signature_pmo as fn() -> IndicatorSignature),
    ("PPO", signature_ppo as fn() -> IndicatorSignature),
    ("PPO_SIGNAL", signature_ppo_signal as fn() -> IndicatorSignature),
    ("PRESSURE", signature_pressure as fn() -> IndicatorSignature),
    ("PSL", signature_psl as fn() -> IndicatorSignature),
    ("QQE", signature_qqe as fn() -> IndicatorSignature),
    ("QSTICK", signature_qstick as fn() -> IndicatorSignature),
    ("RMI", signature_rmi as fn() -> IndicatorSignature),
    ("ROC", signature_roc as fn() -> IndicatorSignature),
    ("ROC_PCT", signature_roc_percentile as fn() -> IndicatorSignature),
    ("RSI", signature_rsi as fn() -> IndicatorSignature),
    ("RSI_PCT_BANDS", signature_rsi_percentile_bands as fn() -> IndicatorSignature),
    ("RSI_PCT_RANK", signature_rsi_percentile_rank as fn() -> IndicatorSignature),
    ("RSI_ZSCORE", signature_rsi_zscore as fn() -> IndicatorSignature),
    ("RSIOMA", signature_rsioma as fn() -> IndicatorSignature),
    ("RSX", signature_rsx as fn() -> IndicatorSignature),
    ("RVGI", signature_rvgi as fn() -> IndicatorSignature),
    ("RWI", signature_rwi as fn() -> IndicatorSignature),
    ("SMI", signature_smi as fn() -> IndicatorSignature),
    ("STC", signature_stc as fn() -> IndicatorSignature),
    ("STOCH_RSI", signature_stochastic_rsi as fn() -> IndicatorSignature),
    ("STOCH", signature_stochastics as fn() -> IndicatorSignature),
    ("STOCHKD", signature_stochastikd as fn() -> IndicatorSignature),
    ("SWEEP_REV", signature_sweep_reversion as fn() -> IndicatorSignature),
    ("SWING_AGE", signature_swing_age as fn() -> IndicatorSignature),
    ("SWINGS", signature_swings as fn() -> IndicatorSignature),
    ("SWINGS_SOFT", signature_swings_soft as fn() -> IndicatorSignature),
    ("TDI", signature_tdi as fn() -> IndicatorSignature),
    ("TRIX", signature_trix as fn() -> IndicatorSignature),
    ("TSI", signature_tsi as fn() -> IndicatorSignature),
    ("UO", signature_ultimate_oscillator as fn() -> IndicatorSignature),
    ("UO_SMOOTH", signature_ultimate_oscillator_smooth as fn() -> IndicatorSignature),
    ("VHF", signature_vhf as fn() -> IndicatorSignature),
    ("VHF_MA", signature_vhf_ma as fn() -> IndicatorSignature),
    ("VWRSI", signature_volume_weighted_rsi as fn() -> IndicatorSignature),
    ("VORTEX", signature_vortex_indicator as fn() -> IndicatorSignature),
    ("WILLIAMS_R", signature_williams_r as fn() -> IndicatorSignature),
    ("ZIGZAG", signature_zigzag as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static MOMENTUM_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
    MOMENTUM_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rsi_signature() {
        let sig = get_signature("RSI").unwrap();
        assert_eq!(sig.id, "RSI");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_macd_signature() {
        let sig = get_signature("MACD").unwrap();
        assert_eq!(sig.id, "MACD");
        assert_eq!(sig.required_params().len(), 2); // fast + slow (signal is optional)
    }

    #[test]
    fn test_get_stoch_signature() {
        let sig = get_signature("STOCH").unwrap();
        assert_eq!(sig.id, "STOCH");
        assert_eq!(sig.required_params().len(), 2); // period_k + period_d
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
        assert_eq!(count(), 94); // 95 momentum indicators
    }

    #[test]
    fn test_rsi_validation() {
        let sig = get_signature("RSI").unwrap();

        // Valid params
        let params = vec![("period", ParamValue::USize(14))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("period", ParamValue::USize(1))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("RSI").unwrap();
        let params = vec![("period", ParamValue::USize(14))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "RSI_14");
    }

    #[test]
    fn test_macd_cache_key() {
        let sig = get_signature("MACD").unwrap();
        let params = vec![
            ("fast_period", ParamValue::USize(12)),
            ("slow_period", ParamValue::USize(26)),
        ];
        let key = sig.cache_key(&params);
        assert!(key.contains("MACD"));
        assert!(key.contains("12"));
        assert!(key.contains("26"));
    }
}
