//! Accumulative Swing Index (ASI) - Welles Wilder
//!
//! The ASI is a cumulative total of the Swing Index (SI) values.
//! It attempts to show the "real" market direction by comparing
//! the relationship between consecutive bars.
//!
//! Swing Index Formula:
//! SI = 50 * (Cy - C + 0.5*(Cy - Oy) + 0.25*(C - O)) / R * K / T
//! Where:
//!   Cy = Yesterday's close
//!   C  = Today's close
//!   Oy = Yesterday's open
//!   O  = Today's open
//!   K  = Max(|H - Cy|, |L - Cy|)
//!   R  = Largest of: |H-Cy|, |L-Cy|, |H-L|
//!   T  = Limit move value (typically the maximum daily price change allowed)

#[derive(Debug, Clone)]
pub struct AccumulativeSwingIndex {
    limit_move: f64,

    // Previous bar data
    prev_open: f64,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,

    // Values
    swing_index: f64,
    asi: f64,

    // State
    bars_count: usize,
    is_ready: bool,
}

impl Default for AccumulativeSwingIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl AccumulativeSwingIndex {
    pub fn new() -> Self {
        Self::with_limit_move(0.0) // 0 means auto-calculate based on price
    }

    pub fn with_limit_move(limit_move: f64) -> Self {
        Self {
            limit_move: limit_move.max(0.0),
            prev_open: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            prev_close: 0.0,
            swing_index: 0.0,
            asi: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prev_open = 0.0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.prev_close = 0.0;
        self.swing_index = 0.0;
        self.asi = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.asi
    }

    /// Feed resolved input lanes `[open, high, low, close]`.
    /// Returns current ASI value.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let open = lanes[0];
        let high = lanes[1];
        let low = lanes[2];
        let close = lanes[3];

        self.bars_count += 1;

        // Need at least 2 bars to calculate SI
        if self.bars_count < 2 {
            self.prev_open = open;
            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;
            return self.asi;
        }

        // Calculate Swing Index components
        let cy = self.prev_close; // Yesterday's close
        let oy = self.prev_open;  // Yesterday's open
        let c = close;            // Today's close
        let o = open;             // Today's open
        let h = high;             // Today's high
        let l = low;              // Today's low

        // K = Max(|H - Cy|, |L - Cy|)
        let k = (h - cy).abs().max((l - cy).abs());

        // Calculate R (largest of the three values)
        let hl = h - l;
        let h_cy = (h - cy).abs();
        let l_cy = (l - cy).abs();

        let r = if h_cy >= l_cy && h_cy >= hl {
            h_cy + 0.5 * l_cy + 0.25 * (cy - oy).abs()
        } else if l_cy >= h_cy && l_cy >= hl {
            l_cy + 0.5 * h_cy + 0.25 * (cy - oy).abs()
        } else {
            hl + 0.25 * (cy - oy).abs()
        };

        // T (limit move) - if 0, use a percentage of current price
        let t = if self.limit_move > 0.0 {
            self.limit_move
        } else {
            close * 0.03
        }.max(1e-10);

        // SI = 50 * (Cy - C + 0.5*(Cy - Oy) + 0.25*(C - O)) / R * K / T
        if r > 1e-10 {
            let numerator = (cy - c) + 0.5 * (cy - oy) + 0.25 * (c - o);
            self.swing_index = 50.0 * numerator / r * k / t;
            self.swing_index = self.swing_index.clamp(-100.0, 100.0);
        } else {
            self.swing_index = 0.0;
        }

        self.asi += self.swing_index;

        self.prev_open = open;
        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;

        if self.bars_count >= 2 {
            self.is_ready = true;
        }

        self.asi
    }

    /// Get current Swing Index (non-cumulative)
    pub fn swing_index(&self) -> f64 {
        self.swing_index
    }

    /// Get limit move value
    pub fn limit_move(&self) -> f64 {
        self.limit_move
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Unit config — ASI has no configurable parameters (limit_move auto-derived from price).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AsiConfig;

impl Indicator for AccumulativeSwingIndex {
    const ID: IndicatorId = IndicatorId::Asi;
    /// No family — a cumulative A/D-style PRODUCER, not a pluggable oscillator.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: open, high, low, close — the full Wilder SI formula.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1) cumulative update — running state only, no window rescan.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 6)]);
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Asi)];
    type Config = AsiConfig;
    type Runtime = AccumulativeSwingIndex;

    fn create(_cfg: AsiConfig) -> AccumulativeSwingIndex {
        AccumulativeSwingIndex::new()
    }
}

impl crate::contract::Config for AsiConfig {
    fn defaults() -> Self {
        AsiConfig
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AccumulativeSwingIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Asi, "ASI", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asi_creation() {
        let asi = AccumulativeSwingIndex::new();
        assert!(!asi.is_ready());
        assert_eq!(asi.value(), 0.0);
    }

    #[test]
    fn test_asi_with_limit_move() {
        let asi = AccumulativeSwingIndex::with_limit_move(5.0);
        assert_eq!(asi.limit_move(), 5.0);
    }

    #[test]
    fn test_asi_warmup() {
        let mut asi = AccumulativeSwingIndex::new();
        asi.feed(&[100.0, 102.0, 99.0, 101.0]);
        assert!(!asi.is_ready());
        asi.feed(&[101.0, 103.0, 100.0, 102.0]);
        assert!(asi.is_ready());
    }

    #[test]
    fn test_asi_uptrend() {
        let mut asi = AccumulativeSwingIndex::new();
        for i in 0..20 {
            let base = 100.0 + i as f64 * 2.0;
            asi.feed(&[base, base + 2.5, base - 0.5, base + 2.0]);
        }
        assert!(asi.is_ready());
    }

    #[test]
    fn test_asi_downtrend() {
        let mut asi = AccumulativeSwingIndex::new();
        for i in 0..20 {
            let base = 200.0 - i as f64 * 2.0;
            asi.feed(&[base, base + 0.5, base - 2.5, base - 2.0]);
        }
        assert!(asi.is_ready());
    }

    #[test]
    fn test_asi_values_finite() {
        let mut asi = AccumulativeSwingIndex::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = asi.feed(&[price, price + 1.0, price - 1.0, price + 0.5]);
            assert!(value.is_finite(), "ASI value should be finite");
        }
    }

    #[test]
    fn test_asi_swing_index() {
        let mut asi = AccumulativeSwingIndex::new();
        asi.feed(&[100.0, 102.0, 99.0, 101.0]);
        assert_eq!(asi.swing_index(), 0.0);
        asi.feed(&[101.0, 104.0, 100.0, 103.0]);
        assert!(asi.swing_index().is_finite());
    }

    #[test]
    fn test_asi_reset() {
        let mut asi = AccumulativeSwingIndex::new();
        for i in 0..10 {
            asi.feed(&[100.0 + i as f64, 105.0, 95.0, 101.0]);
        }
        assert!(asi.is_ready());
        asi.reset();
        assert!(!asi.is_ready());
        assert_eq!(asi.value(), 0.0);
        assert_eq!(asi.swing_index(), 0.0);
    }

    /// Factory resolves the fixed Open/High/Low/Close lanes (volume is ignored).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Asi(<<AccumulativeSwingIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // First bar — no SI yet
        f.feed(0, MarketSample::Bar {
            open: 100.0, high: 102.0, low: 99.0, close: 101.0, volume: 9999.0,
        });
        assert!(!f.is_ready());
        // Second bar — ready, SI computed
        f.feed(0, MarketSample::Bar {
            open: 101.0, high: 104.0, low: 100.0, close: 103.0, volume: 9999.0,
        });
        assert!(f.is_ready());
        assert!(f.read(IndicatorOutputId::Asi).is_finite());
    }
}
