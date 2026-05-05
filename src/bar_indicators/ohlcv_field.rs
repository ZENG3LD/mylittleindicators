//! OHLCV field selector for extracting specific values from bar data.
//!
//! This module provides the `OhlcvField` enum for selecting which field(s) to extract
//! from OHLCV bar data, including composite sources like HL2, HLC3, OHLC4, etc.
//!
//! Used by indicators to make the data source configurable instead of hardcoding Close.

/// OHLCV field selector for applying moving averages to specific price components.
///
/// Supports both single fields (Open, High, Low, Close, Volume) and composite sources
/// that combine multiple fields (HL2, HLC3, OHLC4).
///
/// # Examples
///
/// ```rust
/// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
///
/// let field = OhlcvField::Close;
/// let value = field.extract(100.0, 110.0, 90.0, 105.0, 1000.0);
/// assert_eq!(value, 105.0);
///
/// let hl2 = OhlcvField::HL2;
/// let value = hl2.extract(100.0, 110.0, 90.0, 105.0, 1000.0);
/// assert_eq!(value, 100.0);  // (110 + 90) / 2
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub enum OhlcvField {
    /// Opening price
    Open,
    /// Highest price
    High,
    /// Lowest price
    Low,
    /// Closing price (default for most indicators)
    #[default]
    Close,
    /// Trading volume
    Volume,
    /// (High + Low) / 2 - Median price
    HL2,
    /// (High + Low + Close) / 3 - Typical price
    HLC3,
    /// (Open + High + Low + Close) / 4 - Average price
    OHLC4,
}


impl OhlcvField {
    /// Extract value from OHLCV bar data based on the selected field.
    ///
    /// # Arguments
    ///
    /// * `open` - Opening price
    /// * `high` - Highest price
    /// * `low` - Lowest price
    /// * `close` - Closing price
    /// * `volume` - Trading volume
    ///
    /// # Returns
    ///
    /// The extracted value based on the field type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
    ///
    /// let bar = (100.0, 110.0, 90.0, 105.0, 1000.0);
    ///
    /// assert_eq!(OhlcvField::Close.extract(bar.0, bar.1, bar.2, bar.3, bar.4), 105.0);
    /// assert_eq!(OhlcvField::HL2.extract(bar.0, bar.1, bar.2, bar.3, bar.4), 100.0);
    /// assert_eq!(OhlcvField::HLC3.extract(bar.0, bar.1, bar.2, bar.3, bar.4), 101.66666666666667);
    /// ```
    #[inline]
    pub fn extract(&self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        match self {
            Self::Open => open,
            Self::High => high,
            Self::Low => low,
            Self::Close => close,
            Self::Volume => volume,
            Self::HL2 => (high + low) / 2.0,
            Self::HLC3 => (high + low + close) / 3.0,
            Self::OHLC4 => (open + high + low + close) / 4.0,
        }
    }

    /// Parse from string (for UI/config).
    ///
    /// # Arguments
    ///
    /// * `s` - String representation (case-insensitive)
    ///
    /// # Returns
    ///
    /// `Some(OhlcvField)` if the string is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
    ///
    /// assert_eq!(OhlcvField::from_str("close"), Some(OhlcvField::Close));
    /// assert_eq!(OhlcvField::from_str("HL2"), Some(OhlcvField::HL2));
    /// assert_eq!(OhlcvField::from_str("invalid"), None);
    /// ```
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "open" => Some(Self::Open),
            "high" => Some(Self::High),
            "low" => Some(Self::Low),
            "close" => Some(Self::Close),
            "volume" => Some(Self::Volume),
            "hl2" => Some(Self::HL2),
            "hlc3" => Some(Self::HLC3),
            "ohlc4" => Some(Self::OHLC4),
            _ => None,
        }
    }

    /// Convert to string (for UI/config).
    ///
    /// # Returns
    ///
    /// Lowercase string representation of the field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
    ///
    /// assert_eq!(OhlcvField::Close.as_str(), "close");
    /// assert_eq!(OhlcvField::HL2.as_str(), "hl2");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
            Self::HL2 => "hl2",
            Self::HLC3 => "hlc3",
            Self::OHLC4 => "ohlc4",
        }
    }

    /// All available price-based options for UI dropdown.
    ///
    /// Returns options ordered by most common usage:
    /// 1. Close (default)
    /// 2. Composite sources (HL2, HLC3, OHLC4)
    /// 3. Individual price fields (Open, High, Low)
    ///
    /// Note: Volume is intentionally excluded as it's on a different scale
    /// and not suitable for price-based indicators (SMA, EMA, RSI, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zengeld_chart_indicators::bar_indicators::ohlcv_field::OhlcvField;
    ///
    /// let options = OhlcvField::all_options();
    /// assert_eq!(options[0], "close");
    /// assert!(options.contains(&"hl2"));
    /// ```
    pub fn all_options() -> &'static [&'static str] {
        &[
            "close", "hl2", "hlc3", "ohlc4", "open", "high", "low",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        assert_eq!(OhlcvField::default(), OhlcvField::Close);
    }

    #[test]
    fn test_extract_single_fields() {
        let (open, high, low, close, volume) = (100.0, 110.0, 90.0, 105.0, 1000.0);

        assert_eq!(OhlcvField::Open.extract(open, high, low, close, volume), 100.0);
        assert_eq!(OhlcvField::High.extract(open, high, low, close, volume), 110.0);
        assert_eq!(OhlcvField::Low.extract(open, high, low, close, volume), 90.0);
        assert_eq!(OhlcvField::Close.extract(open, high, low, close, volume), 105.0);
        assert_eq!(OhlcvField::Volume.extract(open, high, low, close, volume), 1000.0);
    }

    #[test]
    fn test_extract_composite_fields() {
        let (open, high, low, close, volume) = (100.0, 110.0, 90.0, 105.0, 1000.0);

        // HL2: (110 + 90) / 2 = 100
        assert_eq!(OhlcvField::HL2.extract(open, high, low, close, volume), 100.0);

        // HLC3: (110 + 90 + 105) / 3 = 101.666...
        let hlc3 = OhlcvField::HLC3.extract(open, high, low, close, volume);
        assert!((hlc3 - 101.66666666666667).abs() < 1e-10);

        // OHLC4: (100 + 110 + 90 + 105) / 4 = 101.25
        assert_eq!(OhlcvField::OHLC4.extract(open, high, low, close, volume), 101.25);
    }

    #[test]
    fn test_from_str_case_insensitive() {
        assert_eq!(OhlcvField::from_str("close"), Some(OhlcvField::Close));
        assert_eq!(OhlcvField::from_str("Close"), Some(OhlcvField::Close));
        assert_eq!(OhlcvField::from_str("CLOSE"), Some(OhlcvField::Close));

        assert_eq!(OhlcvField::from_str("hl2"), Some(OhlcvField::HL2));
        assert_eq!(OhlcvField::from_str("HL2"), Some(OhlcvField::HL2));
        assert_eq!(OhlcvField::from_str("Hl2"), Some(OhlcvField::HL2));
    }

    #[test]
    fn test_from_str_all_variants() {
        assert_eq!(OhlcvField::from_str("open"), Some(OhlcvField::Open));
        assert_eq!(OhlcvField::from_str("high"), Some(OhlcvField::High));
        assert_eq!(OhlcvField::from_str("low"), Some(OhlcvField::Low));
        assert_eq!(OhlcvField::from_str("close"), Some(OhlcvField::Close));
        assert_eq!(OhlcvField::from_str("volume"), Some(OhlcvField::Volume));
        assert_eq!(OhlcvField::from_str("hl2"), Some(OhlcvField::HL2));
        assert_eq!(OhlcvField::from_str("hlc3"), Some(OhlcvField::HLC3));
        assert_eq!(OhlcvField::from_str("ohlc4"), Some(OhlcvField::OHLC4));
    }

    #[test]
    fn test_from_str_invalid() {
        assert_eq!(OhlcvField::from_str("invalid"), None);
        assert_eq!(OhlcvField::from_str(""), None);
        assert_eq!(OhlcvField::from_str("hl3"), None);
        assert_eq!(OhlcvField::from_str("ohlc5"), None);
    }

    #[test]
    fn test_as_str_all_variants() {
        assert_eq!(OhlcvField::Open.as_str(), "open");
        assert_eq!(OhlcvField::High.as_str(), "high");
        assert_eq!(OhlcvField::Low.as_str(), "low");
        assert_eq!(OhlcvField::Close.as_str(), "close");
        assert_eq!(OhlcvField::Volume.as_str(), "volume");
        assert_eq!(OhlcvField::HL2.as_str(), "hl2");
        assert_eq!(OhlcvField::HLC3.as_str(), "hlc3");
        assert_eq!(OhlcvField::OHLC4.as_str(), "ohlc4");
    }

    #[test]
    fn test_round_trip_string_conversion() {
        let fields = [
            OhlcvField::Open,
            OhlcvField::High,
            OhlcvField::Low,
            OhlcvField::Close,
            OhlcvField::Volume,
            OhlcvField::HL2,
            OhlcvField::HLC3,
            OhlcvField::OHLC4,
        ];

        for field in &fields {
            let s = field.as_str();
            let parsed = OhlcvField::from_str(s);
            assert_eq!(parsed, Some(*field), "Round trip failed for {:?}", field);
        }
    }

    #[test]
    fn test_all_options() {
        let options = OhlcvField::all_options();

        // Check that all price-based options are present
        assert!(options.contains(&"close"));
        assert!(options.contains(&"open"));
        assert!(options.contains(&"high"));
        assert!(options.contains(&"low"));
        assert!(options.contains(&"hl2"));
        assert!(options.contains(&"hlc3"));
        assert!(options.contains(&"ohlc4"));

        // Volume intentionally excluded (different scale, not for price indicators)
        assert!(!options.contains(&"volume"));

        // Check count (7 price-based options)
        assert_eq!(options.len(), 7);

        // Verify close is first (most common default)
        assert_eq!(options[0], "close");
    }

    #[test]
    fn test_all_options_are_parseable() {
        for option in OhlcvField::all_options() {
            let parsed = OhlcvField::from_str(option);
            assert!(parsed.is_some(), "Option '{}' should be parseable", option);
        }
    }

    #[test]
    fn test_clone_copy_traits() {
        let field = OhlcvField::HLC3;
        let cloned = field.clone();
        let copied = field;

        assert_eq!(field, cloned);
        assert_eq!(field, copied);
    }

    #[test]
    fn test_hash_trait() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert(OhlcvField::Close, "close_value");
        map.insert(OhlcvField::HL2, "hl2_value");

        assert_eq!(map.get(&OhlcvField::Close), Some(&"close_value"));
        assert_eq!(map.get(&OhlcvField::HL2), Some(&"hl2_value"));
    }

    #[test]
    fn test_eq_trait() {
        assert_eq!(OhlcvField::Close, OhlcvField::Close);
        assert_ne!(OhlcvField::Close, OhlcvField::Open);
        assert_ne!(OhlcvField::HL2, OhlcvField::HLC3);
    }
}
