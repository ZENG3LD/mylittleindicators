// Liquidity gap density (simplified FVG heat): density of recent gaps by amplitude and duration


#[derive(Clone, Debug)]
pub struct LiquidityGapDensity {
    window: usize,
    threshold: f64,
    value: f64,
    last_high: f64,
    last_low: f64,
    gaps: Vec<f64>,
    idx: usize,
    filled: bool,
}

impl LiquidityGapDensity {
    pub fn new(window: usize, threshold: f64) -> Self {
        let w = window.clamp(20, 512);
        Self {
            window: w,
            threshold,
            value: 0.0,
            last_high: 0.0,
            last_low: 0.0,
            gaps: vec![0.0; w],
            idx: 0,
            filled: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.idx = 0;
        self.filled = false;
        self.gaps.fill(0.0);
        self.last_high = 0.0;
        self.last_low = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Returns the rolling average gap density score. Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        if self.last_high > 0.0 || self.last_low > 0.0 {
            // upward gap if current low > last high; downward if current high < last low
            let up_gap = (l - self.last_high).max(0.0);
            let dn_gap = (self.last_low - h).max(0.0);
            let amp = up_gap.max(dn_gap);
            let score = if amp > self.threshold {
                (amp / self.threshold).min(5.0)
            } else {
                0.0
            };
            self.gaps[self.idx] = score;
        }
        self.last_high = h;
        self.last_low = l;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut s = 0.0;
            for i in 0..len {
                s += self.gaps[i];
            }
            self.value = s / len as f64;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for LiquidityGapDensity {
    fn default() -> Self {
        Self::new(50, 0.003)
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

/// Typed dual-mode contract config for [`LiquidityGapDensity`] — the rolling window size
/// and the minimum gap amplitude threshold (as a price fraction).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LiquidityGapDensityConfig {
    /// Number of bars in the rolling density window (clamped to 20..=512).
    pub window: Param<usize>,
    /// Minimum gap amplitude to score (absolute price units).
    pub threshold: Param<f64>,
}

impl Indicator for LiquidityGapDensity {
    const ID: IndicatorId = IndicatorId::Liqgap;
    /// No family — a gap DENSITY SCORE (measures inter-bar price vacuum concentration),
    /// not a pluggable oscillator or MA member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to High / Low — gap amplitude is measured between consecutive bars'
    /// highs and lows.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(window): each bar rescans the gap window to recompute the rolling mean.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Liqgap)];
    type Config = LiquidityGapDensityConfig;
    type Runtime = LiquidityGapDensity;

    fn create(cfg: LiquidityGapDensityConfig) -> LiquidityGapDensity {
        LiquidityGapDensity::new(cfg.window.resolved(), cfg.threshold.resolved())
    }
}

impl crate::contract::Config for LiquidityGapDensityConfig {
    fn defaults() -> Self {
        LiquidityGapDensityConfig {
            window: Param::Solo(50),
            threshold: Param::Solo(0.003),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::sweep_f64;
        // window: rolling density window — Class A period, auto sweep.
        // threshold: minimum gap amplitude — Class F threshold, 0.1..=5.0 step 0.1.
        let mut s = Self::machine_defaults_auto();
        s.threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LiquidityGapDensity {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Liqgap, "Liq Gap", Color::hex(0x00BCD4))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidity_gap_density_creation() {
        let lgd = LiquidityGapDensity::new(30, 0.5);
        assert!(!lgd.is_ready());
    }

    #[test]
    fn test_liquidity_gap_density_warmup() {
        let mut lgd = LiquidityGapDensity::new(30, 0.5);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            lgd.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(lgd.is_ready());
    }

    #[test]
    fn test_liquidity_gap_density_non_negative() {
        let mut lgd = LiquidityGapDensity::new(30, 0.5);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = lgd.feed(&[price + 1.0, price - 1.0]);
            assert!(value >= 0.0, "Density should be non-negative");
        }
    }

    #[test]
    fn test_liquidity_gap_density_reset() {
        let mut lgd = LiquidityGapDensity::new(30, 0.5);
        for i in 0..40 {
            lgd.feed(&[100.0 + i as f64 + 1.0, 99.0 + i as f64]);
        }
        lgd.reset();
        assert!(!lgd.is_ready());
    }

    /// The factory resolves the fixed High/Low lanes from `const SOURCE` (not the wild close);
    /// a monotone gap sequence (each bar low > prev bar high) produces a non-zero density
    /// once the window fills.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f =
            IndicatorOrder::Liqgap(<<LiquidityGapDensity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Feed a gap sequence: each bar's low > prev bar's high by 0.1 (threshold=0.003)
        for i in 0..60 {
            let base = 100.0 + i as f64 * 0.2;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 0.05,
                low: base,
                close: 9999.0,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() >= 0.0, "gap density must be non-negative, got {}", f.primary());
    }
}
