// Swing Age: bars since last local high/low (naive HH/LL detectors)

#[derive(Clone, Debug)]
pub struct SwingAge {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    pub age_since_high: usize,
    pub age_since_low: usize,
}

impl SwingAge {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(2),
            highs: vec![0.0; lookback.max(2)],
            lows: vec![0.0; lookback.max(2)],
            idx: 0,
            filled: false,
            age_since_high: 0,
            age_since_low: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.age_since_high = 0;
        self.age_since_low = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }


    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Returns (age_since_high, age_since_low). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (usize, usize) {
        let high = lanes[0];
        let low = lanes[1];
        // update rings
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        self.idx = (self.idx + 1) % self.lookback;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return (self.age_since_high, self.age_since_low);
        }

        let len = self.lookback;
        let curr_i = (self.idx + len - 1) % len;
        let h = self.highs[curr_i];
        let l = self.lows[curr_i];
        // if new HH/LL versus previous len-1
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            let i = (self.idx + len - 1 - k) % len;
            prev_max = prev_max.max(self.highs[i]);
            prev_min = prev_min.min(self.lows[i]);
        }
        if h >= prev_max {
            self.age_since_high = 0;
        } else {
            self.age_since_high = self.age_since_high.saturating_add(1);
        }
        if l <= prev_min {
            self.age_since_low = 0;
        } else {
            self.age_since_low = self.age_since_low.saturating_add(1);
        }
        (self.age_since_high, self.age_since_low)
    }

    pub fn lookback(&self) -> usize {
        self.lookback
    }

    /// Brace-named getter for the `since_high` output.
    #[inline]
    pub fn since_high(&self) -> f64 {
        self.age_since_high as f64
    }

    /// Brace-named getter for the `since_low` output.
    #[inline]
    pub fn since_low(&self) -> f64 {
        self.age_since_low as f64
    }
}

impl Default for SwingAge {
    /// Factory default: lookback = 20.
    fn default() -> Self {
        Self::new(20)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, RenderOutput, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`SwingAge`] — lookback period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SwingAgeConfig {
    pub period: Param<usize>,
}

impl Indicator for SwingAge {
    const ID: IndicatorId = IndicatorId::SwingAge;
    /// No family — a swing-structure DETECTOR (bars since the rolling HH / LL), an atomic
    /// producer consumed by name, not a pluggable oscillator / MA member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to high / low — it ages the rolling highest-high and lowest-low of the window.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(lookback): each bar rescans the window for the prior extreme. Two period-deep Vec
    /// windows (highs, lows) — its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::count(IndicatorOutputId::SwingAgeSinceHigh),
        Output::count(IndicatorOutputId::SwingAgeSinceLow),
    ];
    type Config = SwingAgeConfig;
    type Runtime = SwingAge;

    fn create(cfg: SwingAgeConfig) -> SwingAge {
        SwingAge::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for SwingAgeConfig {
    fn defaults() -> Self {
        SwingAgeConfig { period: Param::Solo(20) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1) — single axis, auto suffices
    }
}


impl Render for SwingAge {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::SwingAgeSinceHigh,
                "Age Since High",
                Color::hex(0x4CAF50),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::SwingAgeSinceLow,
                "Age Since Low",
                Color::hex(0xF44336),
                2.0,
            ))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swing_age_creation() {
        let sa = SwingAge::new(14);
        assert!(!sa.is_ready());
        assert_eq!(sa.age_since_high, 0);
        assert_eq!(sa.age_since_low, 0);
        assert_eq!(sa.lookback(), 14);
    }

    #[test]
    fn test_swing_age_basic() {
        let mut sa = SwingAge::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            sa.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(sa.is_ready());
    }

    #[test]
    fn test_swing_age_new_high_resets() {
        let mut sa = SwingAge::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 2.0;
            let (age_h, _) = sa.feed(&[price + 1.0, price - 1.0]);
            if sa.is_ready() {
                assert_eq!(age_h, 0, "In uptrend, new highs should keep resetting age");
            }
        }
    }

    #[test]
    fn test_swing_age_reset() {
        let mut sa = SwingAge::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            sa.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(sa.is_ready());
        sa.reset();
        assert!(!sa.is_ready());
        assert_eq!(sa.age_since_high, 0);
        assert_eq!(sa.age_since_low, 0);
    }

    /// The factory resolves the fixed High/Low lanes from `const SOURCE` (not the wild close)
    /// and feeds the pair; a steady uptrend makes a new high every bar -> age-since-high
    /// (`main`) stays 0 end-to-end.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::SwingAge(<<SwingAge as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=40 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: -1.0, high: base + 1.0, low: base - 1.0, close: 9999.0, volume: -1.0,
            });
        }
        assert!(f.is_ready());
        assert_eq!(f.primary(), 0.0, "uptrend -> age since high 0, got {}", f.primary());
    }
}
