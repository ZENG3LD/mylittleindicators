//! Aroon indicator.

/// Aroon indicator - measures time since highest high and lowest low.
///
/// Aroon Up = 100 × (period - bars since highest high) / period
/// Aroon Down = 100 × (period - bars since lowest low) / period
/// Aroon Oscillator = Aroon Up - Aroon Down
///
/// Both Aroon Up and Down oscillate between 0 and 100:
/// - Aroon Up > 70: Strong uptrend
/// - Aroon Down > 70: Strong downtrend
/// - Aroon Oscillator > 0: Bullish
/// - Aroon Oscillator < 0: Bearish
///
/// # Parameters
/// - `period`: Lookback period (typically 25)
///
/// # Implementation
///
/// Uses ring buffer to track recent highs and lows. O(period) per update.
/// Maximum period is 512.
#[derive(Debug, Clone)]
pub struct Aroon {
    period: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    filled: bool,
    aroon_up: f64,
    aroon_down: f64,
    aroon_osc: f64,
}

impl Aroon {
    /// Creates a new Aroon indicator with the specified period.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: Vec::with_capacity(period),
            lows: Vec::with_capacity(period),
            filled: false,
            aroon_up: 0.0,
            aroon_down: 0.0,
            aroon_osc: 0.0,
        }
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Returns (Aroon Up, Aroon Down, Oscillator). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        if self.highs.len() == self.period {
            self.highs.pop();
        }
        if self.lows.len() == self.period {
            self.lows.pop();
        }
        self.highs.insert(0, high);
        self.lows.insert(0, low);

        if self.highs.len() == self.period && self.lows.len() == self.period {
            self.filled = true;
        }

        let len = self.highs.len().min(self.lows.len());
        if len < self.period {
            self.aroon_up = 0.0;
            self.aroon_down = 0.0;
            self.aroon_osc = 0.0;
            return (self.aroon_up, self.aroon_down, self.aroon_osc);
        }

        // Find index of highest high and lowest low (0 = most recent)
        let (mut max_idx, mut max_val) = (0, self.highs[0]);
        let (mut min_idx, mut min_val) = (0, self.lows[0]);

        for (i, &v) in self.highs.iter().enumerate() {
            if v > max_val {
                max_val = v;
                max_idx = i;
            }
        }
        for (i, &v) in self.lows.iter().enumerate() {
            if v < min_val {
                min_val = v;
                min_idx = i;
            }
        }

        self.aroon_up = 100.0 * (self.period - max_idx) as f64 / self.period as f64;
        self.aroon_down = 100.0 * (self.period - min_idx) as f64 / self.period as f64;
        self.aroon_osc = self.aroon_up - self.aroon_down;

        (self.aroon_up, self.aroon_down, self.aroon_osc)
    }


    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.filled = false;
        self.aroon_up = 0.0;
        self.aroon_down = 0.0;
        self.aroon_osc = 0.0;
    }

    /// Returns the period of this Aroon.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns Aroon Up value.
    #[inline]
    pub fn aroon_up(&self) -> f64 {
        self.aroon_up
    }

    /// Returns Aroon Down value.
    #[inline]
    pub fn aroon_down(&self) -> f64 {
        self.aroon_down
    }

    /// Returns Aroon Oscillator value.
    #[inline]
    pub fn oscillator(&self) -> f64 {
        self.aroon_osc
    }

    /// Brace-named getter: `up` output (Aroon Up).
    #[inline]
    pub fn up(&self) -> f64 {
        self.aroon_up
    }

    /// Brace-named getter: `down` output (Aroon Down).
    #[inline]
    pub fn down(&self) -> f64 {
        self.aroon_down
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`Aroon`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AroonConfig {
    pub period: Param<usize>,
}

impl Indicator for Aroon {
    const ID: IndicatorId = IndicatorId::Aroon;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to high/low — it tracks bars-since the window's extreme high and low.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// The full surface: up, down, and the up-down oscillator. `AroonUp`/`AroonDown`/
    /// `AroonOsc` are these outputs, not separate indicators (they were absorbed here).
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::AroonUp),
        Output::percent(IndicatorOutputId::AroonDown),
        Output::centered(IndicatorOutputId::AroonOscillator),
    ];
    /// O(period): each bar rescans two period-deep windows (highs, lows) for the
    /// argmax/argmin. Its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );

    type Config = AroonConfig;
    type Runtime = Aroon;

    fn create(cfg: AroonConfig) -> Aroon {
        Aroon::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for AroonConfig {
    fn defaults() -> Self {
        AroonConfig { period: Param::Solo(25) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for Aroon {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::AroonUp, "Aroon Up", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::AroonDown, "Aroon Down", Color::hex(0xF44336), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::AroonOscillator, "Aroon Osc", Color::hex(0x2196F3), 1.0))
            .bounds(-100.0, 100.0)
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_aroon_basic_calculation() {
        let mut aroon = Aroon::new(25);

        // Feed uptrend data - new highs being made
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            aroon.feed(&[base + 2.0, base - 2.0]);
        }

        assert!(aroon.is_ready());
        // In uptrend with new highs, Aroon Up should be high
        assert!(aroon.aroon_up() > 80.0, "Aroon Up in uptrend should be > 80, got {}", aroon.aroon_up());
    }

    #[test]
    fn test_aroon_downtrend() {
        let mut aroon = Aroon::new(25);

        // Feed downtrend data - new lows being made
        for i in 1..=30 {
            let base = 200.0 - i as f64;
            aroon.feed(&[base + 2.0, base - 2.0]);
        }

        assert!(aroon.is_ready());
        // In downtrend with new lows, Aroon Down should be high
        assert!(aroon.aroon_down() > 80.0, "Aroon Down in downtrend should be > 80, got {}", aroon.aroon_down());
    }

    #[test]
    fn test_aroon_range() {
        let mut aroon = Aroon::new(25);

        for i in 1..=30 {
            let base = 100.0 + (i % 10) as f64;
            aroon.feed(&[base + 3.0, base - 3.0]);
        }

        assert!(aroon.is_ready());
        let (up, down, osc) = (aroon.up(), aroon.down(), aroon.oscillator());
        assert!(up >= 0.0 && up <= 100.0, "Aroon Up should be in [0, 100]");
        assert!(down >= 0.0 && down <= 100.0, "Aroon Down should be in [0, 100]");
        assert!(osc >= -100.0 && osc <= 100.0, "Oscillator should be in [-100, 100]");
    }

    #[test]
    fn test_aroon_oscillator() {
        let mut aroon = Aroon::new(25);

        // Strong uptrend
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            aroon.feed(&[base + 2.0, base - 2.0]);
        }

        assert!(aroon.is_ready());
        // Oscillator should be positive in uptrend
        assert!(aroon.oscillator() > 0.0, "Oscillator in uptrend should be positive");
    }

    #[test]
    fn test_aroon_reset() {
        let mut aroon = Aroon::new(25);

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            aroon.feed(&[base + 2.0, base - 2.0]);
        }
        assert!(aroon.is_ready());

        aroon.reset();
        assert!(!aroon.is_ready());
        assert!((aroon.aroon_up()).abs() < 1e-10);
        assert!((aroon.aroon_down()).abs() < 1e-10);
    }

    #[test]
    fn test_aroon_period_getter() {
        let aroon = Aroon::new(25);
        assert_eq!(aroon.period(), 25);
    }

    #[test]
    fn test_aroon_at_high() {
        let mut aroon = Aroon::new(10);

        // Fill with same prices, then make new high at end
        for _ in 0..9 {
            aroon.feed(&[100.0, 90.0]);
        }
        // New high on last bar
        aroon.feed(&[120.0, 90.0]);

        assert!(aroon.is_ready());
        // Aroon Up should be 100 (high just made)
        assert!((aroon.aroon_up() - 100.0).abs() < 1.0, "Aroon Up at new high should be 100, got {}", aroon.aroon_up());
    }

    #[test]
    fn test_aroon_at_low() {
        let mut aroon = Aroon::new(10);

        // Fill with same prices, then make new low at end
        for _ in 0..9 {
            aroon.feed(&[100.0, 90.0]);
        }
        // New low on last bar
        aroon.feed(&[100.0, 70.0]);

        assert!(aroon.is_ready());
        // Aroon Down should be 100 (low just made)
        assert!((aroon.aroon_down() - 100.0).abs() < 1.0, "Aroon Down at new low should be 100, got {}", aroon.aroon_down());
    }

    /// The factory resolves the fixed High/Low lanes from `const SOURCE` and feeds the pair;
    /// Aroon tracks the window extremes end-to-end (not from a raw bar).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Aroon(<<Aroon as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 0.0, high: base + 2.0, low: base - 2.0, close: base, volume: 0.0,
            });
        }
        // Uptrend with new highs -> Aroon Up high -> oscillator (main) positive.
        assert!(f.primary() > 0.0, "factory Aroon in uptrend should be > 0, got {}", f.primary());
    }
}
