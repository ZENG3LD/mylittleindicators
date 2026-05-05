//! candles_catalog.rs: Catalog of all Candle pattern indicators
//!
//! Auto-generated catalog based on actual indicator implementations.
//! Contains IndicatorSignature definitions for candle pattern indicators.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Candles;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Heikin-Ashi - Smoothed candlestick representation
pub fn signature_heikin_ashi() -> IndicatorSignature {
    IndicatorSignature::builder("HEIKINASHI", CATEGORY)
        .name("Heikin-Ashi")
        .description("Smoothed candlestick representation reducing noise")
        .metadata("author", "Classic TA")
        .metadata("outputs", "open, high, low, close")
        .metadata("complexity", "O(1)")
        .machine_id(BarIndicatorId::Heikinashi) // TODO: Add to enum
        // Note: "HEIKINASHI" is already the main ID, no need for alias
        .alias("Heikinashi")
        .alias("heikinashi")
        .alias("HeikinAshi")
        .alias("heikin_ashi")
        .alias("HEIKIN_ASHI")
        .alias("Heikin_Ashi")
        .build()
}

/// Candle Anatomy - Body and wick ratio analysis
pub fn signature_candle_anatomy() -> IndicatorSignature {
    IndicatorSignature::builder("CANDLEANATOMY", CATEGORY)
        .name("Candle Anatomy")
        .description("Analyzes body, upper wick, and lower wick ratios")
        .add_constraint(
            ParamConstraint::new("long_wick_ratio_threshold", ParamType::F64)
                .with_min(ParamValue::F64(0.1))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.5))
                .required()
        )
        .metadata("outputs", "body_ratio, upper_wick_ratio, lower_wick_ratio, long_upper_flag, long_lower_flag")
        .metadata("complexity", "O(1)")
        .machine_id(BarIndicatorId::Candleanatomy) // TODO: Add to enum
        // Note: "CANDLEANATOMY" is already the main ID, no need for alias
        .alias("Candleanatomy")
        .alias("candleanatomy")
        .alias("CandleAnatomy")
        .alias("candle_anatomy")
        .alias("CANDLE_ANATOMY")
        .alias("Candle_Anatomy")
        .build()
}

/// Pattern Recognition - Advanced candlestick pattern detection
pub fn signature_pattern_recognition() -> IndicatorSignature {
    IndicatorSignature::builder("PATTERNREC", CATEGORY)
        .name("Advanced Pattern Recognition")
        .description("Detects 40+ candlestick patterns with confidence scoring")
        .add_constraint(
            ParamConstraint::new("min_candle_size_pct", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(0.5))
        )
        .add_constraint(
            ParamConstraint::new("doji_body_ratio", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.3))
                .with_default(ParamValue::F64(0.1))
        )
        .add_constraint(
            ParamConstraint::new("hammer_shadow_ratio", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(2.0))
        )
        .add_constraint(
            ParamConstraint::new("marubozu_body_ratio", ParamType::F64)
                .with_min(ParamValue::F64(0.8))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.95))
        )
        .metadata("patterns", "Hammer, Doji, Engulfing, Harami, Stars, Soldiers, etc.")
        .metadata("outputs", "pattern_type, confidence, strength, bullish_prob, bearish_prob")
        .metadata("complexity", "O(1) per pattern check")
        .machine_id(BarIndicatorId::Patternrec) // TODO: Add to enum
        // Note: "PATTERNREC" is already the main ID, no need for alias
        .alias("Patternrec")
        .alias("patternrec")
        .alias("ADVANCEDPATTERNRECOGNITION")
        .alias("AdvancedPatternRecognition")
        .alias("advancedpatternrecognition")
        .alias("advanced_pattern_recognition")
        .alias("ADVANCED_PATTERN_RECOGNITION")
        .alias("Advanced_Pattern_Recognition")
        .build()
}

/// SFP Detector - Swing Failure Pattern detection
pub fn signature_sfp_detector() -> IndicatorSignature {
    IndicatorSignature::builder("SFP", CATEGORY)
        .name("Swing Failure Pattern")
        .description("Detects liquidity sweeps followed by rejection")
        .add_constraint(ParamConstraint::period(5, 100, 20))
        .metadata("aka", "Stop Hunt, Liquidity Grab")
        .metadata("outputs", "bull_sfp, bear_sfp")
        .metadata("complexity", "O(1) with ring buffer")
        .machine_id(BarIndicatorId::Sfp) // TODO: Add to enum
        // Note: "SFP" is already the main ID, no need for alias
        .alias("Sfp")
        .alias("sfp")
        .alias("SWINGFAILUREPATTERN")
        .alias("SwingFailurePattern")
        .alias("swingfailurepattern")
        .alias("swing_failure_pattern")
        .alias("SWING_FAILURE_PATTERN")
        .alias("Swing_Failure_Pattern")
        .build()
}

/// Wick Spike - Detects unusually long wicks
pub fn signature_wick_spike() -> IndicatorSignature {
    IndicatorSignature::builder("WICKSPIKE", CATEGORY)
        .name("Wick Spike Detector")
        .description("Flags unusually long wicks vs rolling percentile")
        .add_constraint(ParamConstraint::period(10, 200, 50))
        .metadata("outputs", "is_upper_spike, is_lower_spike, upper_percentile, lower_percentile")
        .metadata("threshold", "95th percentile")
        .metadata("complexity", "O(n) per update")
        .machine_id(BarIndicatorId::Wickspike) // TODO: Add to enum
        // Note: "WICKSPIKE" is already the main ID, no need for alias
        .alias("Wickspike")
        .alias("wickspike")
        .alias("WICKSPIKEDETECTOR")
        .alias("WickSpikeDetector")
        .alias("wickspikedetector")
        .alias("wick_spike_detector")
        .alias("WICK_SPIKE_DETECTOR")
        .alias("Wick_Spike_Detector")
        .build()
}

// ============================================================================
// Pattern-specific signatures
// ============================================================================

/// Doji Pattern - Indecision candle
pub fn signature_doji() -> IndicatorSignature {
    IndicatorSignature::builder("DOJI", CATEGORY)
        .name("Doji Pattern")
        .description("Single candle with very small body indicating indecision")
        .add_constraint(
            ParamConstraint::new("body_ratio_max", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(0.2))
                .with_default(ParamValue::F64(0.1))
        )
        .metadata("variants", "Gravestone, Dragonfly, Long-Legged")
        .metadata("signal", "Reversal or continuation")
        .machine_id(BarIndicatorId::Doji) // TODO: Add to enum
        // Note: "DOJI" is already the main ID, no need for alias
        .alias("Doji")
        .alias("doji")
        .alias("DOJIPATTERN")
        .alias("DojiPattern")
        .alias("dojipattern")
        .alias("doji_pattern")
        .alias("DOJI_PATTERN")
        .alias("Doji_Pattern")
        .build()
}

/// Hammer/Hanging Man Pattern
pub fn signature_hammer() -> IndicatorSignature {
    IndicatorSignature::builder("HAMMER", CATEGORY)
        .name("Hammer/Hanging Man")
        .description("Long lower shadow with small body at top of range")
        .add_constraint(
            ParamConstraint::new("shadow_ratio", ParamType::F64)
                .with_min(ParamValue::F64(1.5))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(2.0))
        )
        .metadata("bullish", "Hammer at support")
        .metadata("bearish", "Hanging Man at resistance")
        .machine_id(BarIndicatorId::Hammer) // TODO: Add to enum
        // Note: "HAMMER" is already the main ID, no need for alias
        .alias("Hammer")
        .alias("hammer")
        .alias("HAMMERHANGINGMAN")
        .alias("HammerHangingMan")
        .alias("hammerhangingman")
        .alias("hammer_hanging_man")
        .alias("HAMMER_HANGING_MAN")
        .alias("Hammer_Hanging_Man")
        .build()
}

/// Shooting Star/Inverted Hammer Pattern
pub fn signature_shooting_star() -> IndicatorSignature {
    IndicatorSignature::builder("SHOOTINGSTAR", CATEGORY)
        .name("Shooting Star/Inverted Hammer")
        .description("Long upper shadow with small body at bottom of range")
        .add_constraint(
            ParamConstraint::new("shadow_ratio", ParamType::F64)
                .with_min(ParamValue::F64(1.5))
                .with_max(ParamValue::F64(5.0))
                .with_default(ParamValue::F64(2.0))
        )
        .metadata("bearish", "Shooting Star at resistance")
        .metadata("bullish", "Inverted Hammer at support")
        .machine_id(BarIndicatorId::Shootingstar) // TODO: Add to enum
        // Note: "SHOOTINGSTAR" is already the main ID, no need for alias
        .alias("Shootingstar")
        .alias("shootingstar")
        .alias("SHOOTINGSTARINVERTEDHAMMER")
        .alias("ShootingStarInvertedHammer")
        .alias("shootingstarinvertedhammer")
        .alias("shooting_star_inverted_hammer")
        .alias("SHOOTING_STAR_INVERTED_HAMMER")
        .alias("Shooting_Star_Inverted_Hammer")
        .build()
}

/// Engulfing Pattern - Two candle reversal
pub fn signature_engulfing() -> IndicatorSignature {
    IndicatorSignature::builder("ENGULFING", CATEGORY)
        .name("Engulfing Pattern")
        .description("Second candle completely engulfs first candle body")
        .add_constraint(
            ParamConstraint::new("min_size_ratio", ParamType::F64)
                .with_min(ParamValue::F64(1.0))
                .with_max(ParamValue::F64(3.0))
                .with_default(ParamValue::F64(1.2))
        )
        .metadata("candle_count", "2")
        .metadata("signal", "Strong reversal")
        .metadata("variants", "Bullish, Bearish")
        .machine_id(BarIndicatorId::Engulfing) // TODO: Add to enum
        // Note: "ENGULFING" is already the main ID, no need for alias
        .alias("Engulfing")
        .alias("engulfing")
        .alias("ENGULFINGPATTERN")
        .alias("EngulfingPattern")
        .alias("engulfingpattern")
        .alias("engulfing_pattern")
        .alias("ENGULFING_PATTERN")
        .alias("Engulfing_Pattern")
        .build()
}

/// Harami Pattern - Two candle indecision
pub fn signature_harami() -> IndicatorSignature {
    IndicatorSignature::builder("HARAMI", CATEGORY)
        .name("Harami Pattern")
        .description("Second candle body contained within first candle body")
        .metadata("candle_count", "2")
        .metadata("signal", "Reversal or consolidation")
        .metadata("variants", "Bullish, Bearish")
        .machine_id(BarIndicatorId::Harami) // TODO: Add to enum
        // Note: "HARAMI" is already the main ID, no need for alias
        .alias("Harami")
        .alias("harami")
        .alias("HARAMIPATTERN")
        .alias("HaramiPattern")
        .alias("haramipattern")
        .alias("harami_pattern")
        .alias("HARAMI_PATTERN")
        .alias("Harami_Pattern")
        .build()
}

/// Morning Star - Three candle bullish reversal
pub fn signature_morning_star() -> IndicatorSignature {
    IndicatorSignature::builder("MORNINGSTAR", CATEGORY)
        .name("Morning Star")
        .description("Three candle bullish reversal pattern")
        .add_constraint(
            ParamConstraint::new("star_gap_ratio", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(0.5))
                .with_default(ParamValue::F64(0.1))
        )
        .metadata("candle_count", "3")
        .metadata("signal", "Strong bullish reversal")
        .metadata("variants", "Morning Star, Morning Doji Star")
        .machine_id(BarIndicatorId::Morningstar) // TODO: Add to enum
        // Note: "MORNINGSTAR" is already the main ID, no need for alias
        .alias("Morningstar")
        .alias("morningstar")
        .alias("MorningStar")
        .alias("morning_star")
        .alias("MORNING_STAR")
        .alias("Morning_Star")
        .build()
}

/// Evening Star - Three candle bearish reversal
pub fn signature_evening_star() -> IndicatorSignature {
    IndicatorSignature::builder("EVENINGSTAR", CATEGORY)
        .name("Evening Star")
        .description("Three candle bearish reversal pattern")
        .add_constraint(
            ParamConstraint::new("star_gap_ratio", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(0.5))
                .with_default(ParamValue::F64(0.1))
        )
        .metadata("candle_count", "3")
        .metadata("signal", "Strong bearish reversal")
        .metadata("variants", "Evening Star, Evening Doji Star")
        .machine_id(BarIndicatorId::Eveningstar) // TODO: Add to enum
        // Note: "EVENINGSTAR" is already the main ID, no need for alias
        .alias("Eveningstar")
        .alias("eveningstar")
        .alias("EveningStar")
        .alias("evening_star")
        .alias("EVENING_STAR")
        .alias("Evening_Star")
        .build()
}

/// Three White Soldiers - Bullish continuation
pub fn signature_three_white_soldiers() -> IndicatorSignature {
    IndicatorSignature::builder("THREEWHITESOLDIERS", CATEGORY)
        .name("Three White Soldiers")
        .description("Three consecutive bullish candles with progressive closes")
        .add_constraint(
            ParamConstraint::new("min_body_ratio", ParamType::F64)
                .with_min(ParamValue::F64(0.4))
                .with_max(ParamValue::F64(0.9))
                .with_default(ParamValue::F64(0.6))
        )
        .metadata("candle_count", "3")
        .metadata("signal", "Strong bullish continuation")
        .machine_id(BarIndicatorId::Threewhitesoldiers) // TODO: Add to enum
        // Note: "THREEWHITESOLDIERS" is already the main ID, no need for alias
        .alias("Threewhitesoldiers")
        .alias("threewhitesoldiers")
        .alias("ThreeWhiteSoldiers")
        .alias("three_white_soldiers")
        .alias("THREE_WHITE_SOLDIERS")
        .alias("Three_White_Soldiers")
        .build()
}

/// Three Black Crows - Bearish continuation
pub fn signature_three_black_crows() -> IndicatorSignature {
    IndicatorSignature::builder("THREEBLACKCROWS", CATEGORY)
        .name("Three Black Crows")
        .description("Three consecutive bearish candles with progressive closes")
        .add_constraint(
            ParamConstraint::new("min_body_ratio", ParamType::F64)
                .with_min(ParamValue::F64(0.4))
                .with_max(ParamValue::F64(0.9))
                .with_default(ParamValue::F64(0.6))
        )
        .metadata("candle_count", "3")
        .metadata("signal", "Strong bearish continuation")
        .machine_id(BarIndicatorId::Threeblackcrows) // TODO: Add to enum
        // Note: "THREEBLACKCROWS" is already the main ID, no need for alias
        .alias("Threeblackcrows")
        .alias("threeblackcrows")
        .alias("ThreeBlackCrows")
        .alias("three_black_crows")
        .alias("THREE_BLACK_CROWS")
        .alias("Three_Black_Crows")
        .build()
}

/// Marubozu - Strong directional candle
pub fn signature_marubozu() -> IndicatorSignature {
    IndicatorSignature::builder("MARUBOZU", CATEGORY)
        .name("Marubozu")
        .description("Candle with little to no wicks, strong directional move")
        .add_constraint(
            ParamConstraint::new("body_ratio_min", ParamType::F64)
                .with_min(ParamValue::F64(0.8))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.95))
        )
        .metadata("signal", "Strong continuation")
        .metadata("variants", "White (bullish), Black (bearish)")
        .machine_id(BarIndicatorId::Marubozu) // TODO: Add to enum
        // Note: "MARUBOZU" is already the main ID, no need for alias
        .alias("Marubozu")
        .alias("marubozu")
        .build()
}

/// Piercing Pattern - Bullish reversal
pub fn signature_piercing_pattern() -> IndicatorSignature {
    IndicatorSignature::builder("PIERCINGPATTERN", CATEGORY)
        .name("Piercing Pattern")
        .description("Bullish reversal with second candle closing above midpoint of first")
        .metadata("candle_count", "2")
        .metadata("signal", "Bullish reversal")
        .machine_id(BarIndicatorId::Piercingpattern) // TODO: Add to enum
        // Note: "PIERCINGPATTERN" is already the main ID, no need for alias
        .alias("Piercingpattern")
        .alias("piercingpattern")
        .alias("PiercingPattern")
        .alias("piercing_pattern")
        .alias("PIERCING_PATTERN")
        .alias("Piercing_Pattern")
        .build()
}

/// Dark Cloud Cover - Bearish reversal
pub fn signature_dark_cloud_cover() -> IndicatorSignature {
    IndicatorSignature::builder("DARKCLOUDCOVER", CATEGORY)
        .name("Dark Cloud Cover")
        .description("Bearish reversal with second candle closing below midpoint of first")
        .metadata("candle_count", "2")
        .metadata("signal", "Bearish reversal")
        .machine_id(BarIndicatorId::Darkcloudcover) // TODO: Add to enum
        // Note: "DARKCLOUDCOVER" is already the main ID, no need for alias
        .alias("Darkcloudcover")
        .alias("darkcloudcover")
        .alias("DarkCloudCover")
        .alias("dark_cloud_cover")
        .alias("DARK_CLOUD_COVER")
        .alias("Dark_Cloud_Cover")
        .build()
}

/// Tweezer Top/Bottom - Two candle reversal
pub fn signature_tweezer() -> IndicatorSignature {
    IndicatorSignature::builder("TWEEZER", CATEGORY)
        .name("Tweezer Top/Bottom")
        .description("Two candles with matching highs or lows")
        .add_constraint(
            ParamConstraint::new("tolerance_pct", ParamType::F64)
                .with_min(ParamValue::F64(0.0))
                .with_max(ParamValue::F64(0.1))
                .with_default(ParamValue::F64(0.02))
        )
        .metadata("candle_count", "2")
        .metadata("signal", "Reversal")
        .metadata("variants", "Tweezer Top (bearish), Tweezer Bottom (bullish)")
        .machine_id(BarIndicatorId::Tweezer) // TODO: Add to enum
        // Note: "TWEEZER" is already the main ID, no need for alias
        .alias("Tweezer")
        .alias("tweezer")
        .alias("TWEEZERTOPBOTTOM")
        .alias("TweezerTopBottom")
        .alias("tweezertopbottom")
        .alias("tweezer_top_bottom")
        .alias("TWEEZER_TOP_BOTTOM")
        .alias("Tweezer_Top_Bottom")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Candle pattern indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("HEIKINASHI", signature_heikin_ashi as fn() -> IndicatorSignature),
    ("CANDLEANATOMY", signature_candle_anatomy as fn() -> IndicatorSignature),
    ("PATTERNREC", signature_pattern_recognition as fn() -> IndicatorSignature),
    ("SFP", signature_sfp_detector as fn() -> IndicatorSignature),
    ("WICKSPIKE", signature_wick_spike as fn() -> IndicatorSignature),
    ("DOJI", signature_doji as fn() -> IndicatorSignature),
    ("HAMMER", signature_hammer as fn() -> IndicatorSignature),
    ("SHOOTINGSTAR", signature_shooting_star as fn() -> IndicatorSignature),
    ("ENGULFING", signature_engulfing as fn() -> IndicatorSignature),
    ("HARAMI", signature_harami as fn() -> IndicatorSignature),
    ("MORNINGSTAR", signature_morning_star as fn() -> IndicatorSignature),
    ("EVENINGSTAR", signature_evening_star as fn() -> IndicatorSignature),
    ("THREEWHITESOLDIERS", signature_three_white_soldiers as fn() -> IndicatorSignature),
    ("THREEBLACKCROWS", signature_three_black_crows as fn() -> IndicatorSignature),
    ("MARUBOZU", signature_marubozu as fn() -> IndicatorSignature),
    ("PIERCINGPATTERN", signature_piercing_pattern as fn() -> IndicatorSignature),
    ("DARKCLOUDCOVER", signature_dark_cloud_cover as fn() -> IndicatorSignature),
    ("TWEEZER", signature_tweezer as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static CANDLES_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
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
///
/// ## Example
/// ```rust
/// use zengeld_chart_indicators::bar_indicators::candles::candles_catalog;
///
/// let sig = candles_catalog::get_signature("HEIKINASHI").unwrap();
/// assert_eq!(sig.id, "HEIKINASHI");
/// ```
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    CANDLES_CATALOG.get(id).map(|f| f())
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
    fn test_get_heikin_ashi_signature() {
        let sig = get_signature("HEIKINASHI").unwrap();
        assert_eq!(sig.id, "HEIKINASHI");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_get_pattern_recognition_signature() {
        let sig = get_signature("PATTERNREC").unwrap();
        assert_eq!(sig.id, "PATTERNREC");
        assert_eq!(sig.name, "Advanced Pattern Recognition");
    }

    #[test]
    fn test_get_sfp_signature() {
        let sig = get_signature("SFP").unwrap();
        assert_eq!(sig.id, "SFP");
        assert!(sig.required_params().len() >= 1);
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
        assert_eq!(count(), 18); // 18 candle indicators
    }

    #[test]
    fn test_candle_anatomy_validation() {
        let sig = get_signature("CANDLEANATOMY").unwrap();

        // Valid params
        let params = vec![("long_wick_ratio_threshold", ParamValue::F64(0.5))];
        assert!(sig.validate_params(&params).is_ok());

        // Invalid: out of range
        let params = vec![("long_wick_ratio_threshold", ParamValue::F64(2.0))];
        assert!(sig.validate_params(&params).is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let sig = get_signature("SFP").unwrap();
        let params = vec![("period", ParamValue::USize(20))];
        let key = sig.cache_key(&params);
        assert_eq!(key, "SFP_20");
    }

    #[test]
    fn test_pattern_recognition_params() {
        let sig = get_signature("PATTERNREC").unwrap();
        let params = vec![
            ("min_candle_size_pct", ParamValue::F64(0.5)),
            ("doji_body_ratio", ParamValue::F64(0.1)),
            ("hammer_shadow_ratio", ParamValue::F64(2.0)),
            ("marubozu_body_ratio", ParamValue::F64(0.95)),
        ];
        assert!(sig.validate_params(&params).is_ok());
    }

    #[test]
    fn test_morning_star_metadata() {
        let sig = get_signature("MORNINGSTAR").unwrap();
        assert!(sig.metadata.contains_key("candle_count"));
        assert_eq!(sig.metadata.get("candle_count"), Some(&"3".to_string()));
    }
}
