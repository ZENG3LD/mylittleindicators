// Swing/Fractal strength score: normalized strength of recent swing


#[derive(Clone, Debug)]
pub struct SwingStrengthScore {
    left: usize,
    right: usize,
    value: f64,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
}

impl SwingStrengthScore {
    pub fn new(left: usize, right: usize) -> Self {
        let l = left.clamp(1, 10);
        let r = right.clamp(1, 10);
        let cap = (l + r + 2).max(16);
        Self {
            left: l,
            right: r,
            value: 0.0,
            highs: vec![0.0; cap],
            lows: vec![0.0; cap],
            idx: 0,
            filled: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.idx = 0;
        self.filled = false;
        self.highs.fill(0.0);
        self.lows.fill(0.0);
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Returns the tanh-normalized swing strength. Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let n = self.highs.len();
        self.highs[self.idx] = h;
        self.lows[self.idx] = l;
        self.idx = (self.idx + 1) % n;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // check pivot at position left back from current
            let pivot_idx = (self.idx + n - self.right - 1) % n; // recent pivot
            let pivot_high = self.highs[pivot_idx];
            let pivot_low = self.lows[pivot_idx];
            // left/right ranges
            let mut left_max = f64::NEG_INFINITY;
            let mut left_min = f64::INFINITY;
            for i in 1..=self.left {
                let ii = (pivot_idx + n - i) % n;
                left_max = left_max.max(self.highs[ii]);
                left_min = left_min.min(self.lows[ii]);
            }
            let mut right_max = f64::NEG_INFINITY;
            let mut right_min = f64::INFINITY;
            for i in 1..=self.right {
                let ii = (pivot_idx + i) % n;
                right_max = right_max.max(self.highs[ii]);
                right_min = right_min.min(self.lows[ii]);
            }
            let up_strength = (pivot_high - left_max).max(0.0) + (pivot_high - right_max).max(0.0);
            let down_strength = (left_min - pivot_low).max(0.0) + (right_min - pivot_low).max(0.0);
            let raw = up_strength - down_strength;
            // normalize by recent ATR proxy (range average)
            let mut rng_sum = 0.0;
            for i in 0..(self.left + self.right + 1) {
                let ii = (pivot_idx + n - self.left + i) % n;
                rng_sum += self.highs[ii] - self.lows[ii];
            }
            let denom = (rng_sum / (self.left + self.right + 1) as f64).max(1e-6);
            self.value = (raw / denom).tanh();
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for SwingStrengthScore {
    /// Factory default: left=3, right=3.
    fn default() -> Self {
        Self::new(3, 3)
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
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`SwingStrengthScore`] — the left / right pivot half-widths
/// (each clamped to 1..=10 by `new`). No source field: it draws the fixed High/Low slice.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SwingStrConfig {
    pub left: Param<usize>,
    pub right: Param<usize>,
}

impl Indicator for SwingStrengthScore {
    const ID: IndicatorId = IndicatorId::Swingstr;
    /// No family — a swing-structure SCORE (pivot prominence), an atomic producer consumed
    /// by name, not a pluggable oscillator member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to high / low — pivot prominence is measured off the window's highs and lows.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(left+right): each bar rescans both pivot wings + the range window. Two fixed-cap Vec
    /// windows (highs, lows) — its OWN state, no sub-deps.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Swingstr)];
    type Config = SwingStrConfig;
    type Runtime = SwingStrengthScore;

    fn create(cfg: SwingStrConfig) -> SwingStrengthScore {
        SwingStrengthScore::new(cfg.left.resolved(), cfg.right.resolved())
    }
}

impl crate::contract::Config for SwingStrConfig {
    fn defaults() -> Self {
        SwingStrConfig {
            left: Param::Solo(3),
            right: Param::Solo(3),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // left/right→range(2,4048,1) — both Class A lookbacks, auto suffices
    }
}


impl Render for SwingStrengthScore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Swingstr, "Swing Strength", Color::hex(0x9C27B0))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swing_strength_score_creation() {
        let sss = SwingStrengthScore::new(3, 3);
        assert!(!sss.is_ready());
    }

    #[test]
    fn test_swing_strength_score_warmup() {
        let mut sss = SwingStrengthScore::new(3, 3);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sss.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(sss.is_ready());
    }

    #[test]
    fn test_swing_strength_score_range() {
        let mut sss = SwingStrengthScore::new(3, 3);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sss.feed(&[price + 1.0, price - 1.0]);
            // tanh output is in [-1, 1]
            assert!(value >= -1.0 && value <= 1.0, "Value should be in [-1, 1]");
        }
    }

    #[test]
    fn test_swing_strength_score_reset() {
        let mut sss = SwingStrengthScore::new(3, 3);
        for _ in 0..20 {
            sss.feed(&[101.0, 99.0]);
        }
        sss.reset();
        assert!(!sss.is_ready());
    }

    /// The factory resolves the fixed High/Low lanes from `const SOURCE` (not the wild close)
    /// and feeds the pair; the tanh score stays bounded in [-1, 1] end-to-end.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f =
            IndicatorOrder::Swingstr(<<SwingStrengthScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: -1.0, high: price + 1.0, low: price - 1.0, close: 9999.0, volume: -1.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!((-1.0..=1.0).contains(&v), "swing strength must stay in [-1, 1], got {v}");
    }
}
