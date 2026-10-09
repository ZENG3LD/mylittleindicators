//! Price × N-level crossover with touch mode classification.
//!
//! Levels = all Price-domain outputs of a producer operand (or a single constant level).
//! Touch behaviour is selected via `TouchMode`.
//!
//! Outputs:
//! - scalars `line_value`, `close`, `signal` — the PRIMARY (level-0) signal.
//! - matrix `IndicatorOutputId::PriceLineCrossCrossVectorGrid` — the Nx1 per-level touch signal grid.
//!
//! Signal:
//! - `+1.0` Bullish (close above / wick reject up / hammer above support)
//! - `-1.0` Bearish
//! - `0.0`  No event

use crate::indicators::candles::candle_pattern::CandlePatternDetector;
use crate::indicators::signal_logic::line_cross::{LineProducer, LineProducerOrder};
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// How price interacts with the line to trigger a signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchMode {
    /// Close transitions from below to above the line → +1 bullish.
    CloseAbove,
    /// Close transitions from above to below the line → -1 bearish.
    CloseBelow,
    /// Any wick (high or low) crosses through the line.
    /// +1 if high crosses above, -1 if low crosses below.
    WickThrough,
    /// Wick crosses line but close returns to the same side as the previous bar.
    /// Bullish reject (+1): low < line && close > line (wick swept below, closed back above).
    /// Bearish reject (-1): high > line && close < line (wick swept above, closed back below).
    WickReject,
    /// High or low comes within `tolerance` of the line (absolute price distance).
    Touch {
        /// Maximum distance between bar extreme and line to fire.
        tolerance: f64,
    },
    /// Close-above/below crossover AND the bar also matches a candle pattern.
    WithCandle(crate::indicators::candles::candle_pattern::CandlePatternKind),
}

impl ::core::hash::Hash for TouchMode {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        ::core::mem::discriminant(self).hash(state);
        match self {
            Self::Touch { tolerance } => tolerance.to_bits().hash(state),
            Self::WithCandle(k) => k.hash(state),
            _ => {}
        }
    }
}

impl crate::contract::ParamScalar for TouchMode {
    fn scalar_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        ::core::hash::Hash::hash(self, state);
    }
}

/// Compute the touch signal for a SINGLE level — verbatim logic from the original scalar
/// detector, with `level` replacing the old `line_val`. The candle pattern match is
/// PRECOMPUTED (the detector is stateful; calling it per-level would corrupt state).
fn touch_signal(
    mode: TouchMode,
    prev_above: Option<bool>,
    _open: f64,
    high: f64,
    low: f64,
    close: f64,
    level: f64,
    pattern_match: bool,
) -> i8 {
    match mode {
        TouchMode::CloseAbove => {
            match prev_above {
                Some(false) if close > level => 1,
                _ => 0,
            }
        }
        TouchMode::CloseBelow => {
            match prev_above {
                Some(true) if close < level => -1,
                _ => 0,
            }
        }
        TouchMode::WickThrough => {
            // Both wicks can cross simultaneously — prefer bullish if both.
            if low <= level && high >= level {
                // Bar straddles line — pick direction by close.
                if close >= level { 1 } else { -1 }
            } else if high > level && matches!(prev_above, Some(false)) {
                1
            } else if low < level && matches!(prev_above, Some(true)) {
                -1
            } else {
                0
            }
        }
        TouchMode::WickReject => {
            // Bullish SFP: low swept below line, close back above (prev was above).
            let bullish = low < level
                && close > level
                && matches!(prev_above, Some(true) | None);
            // Bearish SFP: high swept above line, close back below (prev was below).
            let bearish = high > level
                && close < level
                && matches!(prev_above, Some(false) | None);
            if bullish { 1 } else if bearish { -1 } else { 0 }
        }
        TouchMode::Touch { tolerance } => {
            let near_high = (high - level).abs() <= tolerance;
            let near_low = (low - level).abs() <= tolerance;
            if near_high || near_low { 1 } else { 0 }
        }
        TouchMode::WithCandle(_) => {
            // Cross direction: CloseAbove semantics.
            let crossed = match prev_above {
                Some(false) if close > level => true,
                Some(true) if close < level => true,
                _ => false,
            };
            if crossed && pattern_match {
                if close > level { 1 } else { -1 }
            } else {
                0
            }
        }
    }
}

/// Price × N-level crossover detector with configurable touch semantics.
///
/// Each of the N levels is tested against OHLC independently; the full result is the
/// Nx1 `PriceLineCrosses` matrix grid. The PRIMARY (level-0) signal is also exposed as
/// the scalar `signal` output for backward-compatible single-level use.
#[derive(Debug, Clone)]
pub struct PriceLineCross {
    levels: LineProducer,
    mode: TouchMode,
    /// Whether close was above each level on the previous bar (one per level).
    prev_above: Vec<Option<bool>>,
    /// Reused per-bar level buffer (length = N).
    cur: Vec<f64>,
    /// Last level values (for the `line` scalar accessor = level-0).
    last_levels: Vec<f64>,
    last_close: f64,
    /// The PRIMARY (level-0) signal.
    last_signal: i8,
    /// The Nx1 per-level touch signal grid.
    grid: MatrixGrid,
    /// Stateful candle pattern detector — computed ONCE per bar (not per level).
    candle_detector: Option<CandlePatternDetector>,
}

impl PriceLineCross {
    /// Construct with a producer of N price levels and a touch mode.
    pub fn new(levels: LineProducer, mode: TouchMode) -> Self {
        let n = levels.len();
        let candle_detector = if let TouchMode::WithCandle(kind) = mode {
            Some(CandlePatternDetector::new(kind))
        } else {
            None
        };
        let grid = MatrixGrid::new(n as u16, 1, false).with_labels(
            AxisLabels::Outputs(levels.output_ids().to_vec()),
            AxisLabels::Bars,
        );
        Self {
            levels,
            mode,
            prev_above: vec![None; n],
            cur: Vec::with_capacity(n),
            last_levels: vec![0.0; n],
            last_close: 0.0,
            last_signal: 0,
            grid,
            candle_detector,
        }
    }

    /// Feed one resolved bar (ts + `[open, high, low, close, volume]`).
    /// Drives the level producer ONCE, computes the candle pattern ONCE, then
    /// writes the Nx1 touch signal grid.
    pub fn feed(&mut self, ts: i64, lanes: &[f64]) {
        let (open, high, low, close, volume) = (lanes[0], lanes[1], lanes[2], lanes[3], lanes[4]);

        // Drive the producer — returns &[f64] of length N.
        self.cur.clear();
        self.cur
            .extend_from_slice(self.levels.feed_bar(open, high, low, close, volume, ts));

        // Compute the candle pattern ONCE — the detector is stateful; calling it per-level
        // would advance it N times and corrupt state.
        let pattern_match = if let TouchMode::WithCandle(_) = self.mode {
            self.candle_detector
                .as_mut()
                .map_or(false, |d| d.detect_from_values(open, high, low, close).is_some())
        } else {
            false
        };

        self.grid.reset();

        let n = self.cur.len();
        for i in 0..n {
            let level = self.cur[i];
            let sig = if self.levels.is_ready() {
                touch_signal(self.mode, self.prev_above[i], open, high, low, close, level, pattern_match)
            } else {
                0
            };
            self.grid.set_direction(MatrixCell::new(i as u16, 0), sig as f64);
            if i == 0 {
                self.last_signal = sig;
            }
            self.prev_above[i] = Some(close > level);
        }

        self.last_levels.clear();
        self.last_levels.extend_from_slice(&self.cur);
        self.last_close = close;
    }

    /// Named getter for the `line` brace output — the PRIMARY (level-0) level value.
    pub fn line(&self) -> f64 {
        self.last_levels.first().copied().unwrap_or(0.0)
    }

    /// Named getter for the `close` brace output (the last close fed).
    pub fn close(&self) -> f64 {
        self.last_close
    }

    /// Named getter for the `signal` brace output — the PRIMARY (level-0) touch sign.
    pub fn signal(&self) -> f64 {
        self.last_signal as f64
    }

    /// The full Nx1 per-level touch signal grid.
    pub fn cross_vector_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    /// True once the level producer has warmed up and at least one level has been seen.
    pub fn is_ready(&self) -> bool {
        self.levels.is_ready() && self.prev_above.iter().any(|p| p.is_some())
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.levels.reset();
        for p in &mut self.prev_above {
            *p = None;
        }
        self.grid.reset();
        self.last_signal = 0;
        self.last_close = 0.0;
        for v in &mut self.last_levels {
            *v = 0.0;
        }
        if let Some(ref mut det) = self.candle_detector {
            det.reset();
        }
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField as ContractOhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, Slot, SourceAxis,
    UpdateComplexity, ValueDomain,
};
use crate::engine::stream_kind::StreamKind;
use crate::indicators::signal_logic::line_cross::LINE_PRODUCERS;

/// Typed config for [`PriceLineCross`] — the level producer and the touch mode.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PriceLineCrossConfig {
    /// Producer whose Price-domain outputs form the N level lines.
    pub levels: Param<LineProducerOrder>,
    pub mode: Param<TouchMode>,
}

impl Indicator for PriceLineCross {
    const ID: IndicatorId = IndicatorId::PriceLineCross;
    /// No family — a price/level touch DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full O/H/L/C/V — wick/close geometry plus the level producer.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        ContractOhlcvField::Open,
        ContractOhlcvField::High,
        ContractOhlcvField::Low,
        ContractOhlcvField::Close,
        ContractOhlcvField::Volume,
    ]));
    /// O(N) outer — one comparison per level; the producer slot cost is config-chosen.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::PriceLineCrossLine),
        Output::price(IndicatorOutputId::PriceLineCrossClose),
        Output::ordinal(IndicatorOutputId::PriceLineCrossSignal),
        Output::matrix(IndicatorOutputId::PriceLineCrossCrossVectorGrid, ValueDomain::Discrete),
    ];
    /// One producer slot — declared so the barometer's nested walk and the cost model
    /// SEE the embedded producer (NOT `Cost::DEFAULT`).
    const SLOTS: &'static [Slot] = &[Slot::new(LINE_PRODUCERS)];

    type Config = PriceLineCrossConfig;
    type Runtime = PriceLineCross;

    fn create(cfg: PriceLineCrossConfig) -> PriceLineCross {
        PriceLineCross::new(
            cfg.levels
                .resolved()
                .into_producer()
                .expect("levels operand must be a Price-domain producer"),
            cfg.mode.resolved(),
        )
    }

    /// The resolved producer id of the levels operand (constants contribute none).
    fn slot_members(cfg: &PriceLineCrossConfig) -> Vec<IndicatorId> {
        let mut v = Vec::with_capacity(1);
        if let Some(id) = cfg.levels.resolved().producer() {
            v.push(id);
        }
        v
    }
}

impl crate::contract::Config for PriceLineCrossConfig {
    fn defaults() -> Self {
        PriceLineCrossConfig {
            // SMA(20) line — 1 Price output, updates every bar (ts-independent; a period-based
            // pivot producer would stall under ts=0). N>1 levels come from a multi-output producer
            // operand (Bollinger/Donchian/Keltner bands, or a pivot ladder fed real timestamps).
            levels: Param::Solo(LineProducerOrder::reference(
                IndicatorOrder::from_defaults(IndicatorId::Sma)
                    .expect("Sma is contract-backed")
                    .with_period(Param::Solo(20)),
            )),
            mode: Param::Solo(TouchMode::CloseAbove),
        }
    }
    fn machine_defaults() -> Self {
        // levels: Param<LineProducerOrder> — structural producer wiring (deferred wave).
        // Left Solo (machine_defaults_auto default). [FLAG: producer slot order, deferred wave]
        //
        // mode: Param<TouchMode> — two variants embed payload (Touch{tolerance: f64} and
        // WithCandle(CandlePatternKind)), so the full enum cannot be swept as plain scalars.
        // Leave Solo pending a dedicated parametric-variant sweep wave. [FLAG: complex enum, deferred]
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Render for PriceLineCross {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::PriceLineCrossLine, "Line", Color::hex(0xCDDC39))
            .line_output(IndicatorOutputId::PriceLineCrossSignal, "Touch", Color::hex(0xFF9800))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::candles::candle_pattern::CandlePatternKind;

    /// Build a constant-level (N=1) detector for touch-mode tests.
    fn constant_plc(level: f64, mode: TouchMode) -> PriceLineCross {
        PriceLineCross::new(
            LineProducerOrder::Constant(level)
                .into_producer()
                .unwrap(),
            mode,
        )
    }

    /// Build an SMA(period)-level detector for the SMA-line tests.
    fn sma_plc(period: usize, mode: TouchMode) -> PriceLineCross {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::Param;
        let order = IndicatorOrder::from_defaults(IndicatorId::Sma)
            .unwrap()
            .with_period(Param::Solo(period));
        PriceLineCross::new(
            LineProducerOrder::reference(order)
                .into_producer()
                .unwrap(),
            mode,
        )
    }

    fn drive(plc: &mut PriceLineCross, prices: &[f64]) {
        for &p in prices {
            plc.feed(0, &[p, p, p, p, 0.0]);
        }
    }

    #[test]
    fn factory_feeds_resolved_price_line_cross() {
        use crate::engine::contract_engine::IndicatorOutputId;
        use crate::contract::MarketSample;
        // Default: close vs SMA(20), CloseAbove. Dip below then surge above -> +1.
        let mut f = IndicatorOrder::PriceLineCross(
            <<PriceLineCross as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let bar = |c: f64| MarketSample::Bar {
            open: c,
            high: c + 0.5,
            low: c - 0.5,
            close: c,
            volume: 1.0,
        };
        // Warm the SMA(20) line at 100.
        for _ in 0..25 {
            f.feed(0, bar(100.0));
        }
        f.feed(0, bar(95.0)); // close below the SMA
        let mut saw_up = false;
        for _ in 0..10 {
            f.feed(0, bar(130.0));
            if f.read(IndicatorOutputId::PriceLineCrossSignal) > 0.0 {
                saw_up = true;
            }
        }
        assert!(saw_up, "close crossing above the SMA line must fire +1");
    }

    // ---- CloseAbove ----

    #[test]
    fn close_above_fires_on_transition() {
        let mut plc = constant_plc(100.0, TouchMode::CloseAbove);
        drive(&mut plc, &[90.0, 90.0, 90.0]);
        // First bar that closes above 100.
        plc.feed(0, &[100.0, 105.0, 99.0, 105.0, 0.0]);
        assert_eq!(plc.signal(), 1.0, "CloseAbove must fire +1 on up-transition");
        // Stay above — no further signal.
        plc.feed(0, &[105.0, 106.0, 104.0, 105.0, 0.0]);
        assert_eq!(plc.signal(), 0.0, "CloseAbove must not repeat while above");
    }

    #[test]
    fn close_above_no_signal_when_already_above() {
        let mut plc = constant_plc(100.0, TouchMode::CloseAbove);
        drive(&mut plc, &[110.0; 5]);
        plc.feed(0, &[110.0, 111.0, 109.0, 110.0, 0.0]);
        assert_eq!(plc.signal(), 0.0, "CloseAbove: no signal when always above");
    }

    // ---- CloseBelow ----

    #[test]
    fn close_below_fires_on_transition() {
        let mut plc = constant_plc(100.0, TouchMode::CloseBelow);
        drive(&mut plc, &[110.0, 110.0, 110.0]);
        plc.feed(0, &[100.0, 101.0, 95.0, 95.0, 0.0]);
        assert_eq!(plc.signal(), -1.0, "CloseBelow must fire -1 on down-transition");
        plc.feed(0, &[95.0, 96.0, 94.0, 95.0, 0.0]);
        assert_eq!(plc.signal(), 0.0, "CloseBelow must not repeat while below");
    }

    // ---- WickThrough ----

    #[test]
    fn wick_through_high_fires() {
        let mut plc = constant_plc(100.0, TouchMode::WickThrough);
        drive(&mut plc, &[90.0; 3]);
        // High pierces 100 but close stays below — wick through.
        plc.feed(0, &[90.0, 110.0, 89.0, 91.0, 0.0]);
        assert_eq!(
            plc.signal(),
            -1.0,
            "bar below with high spike: straddle resolves bearish (close < line)"
        );
    }

    #[test]
    fn wick_through_low_fires() {
        let mut plc = constant_plc(100.0, TouchMode::WickThrough);
        drive(&mut plc, &[110.0; 3]);
        // Low dips below 100 but close stays above — wick through.
        plc.feed(0, &[110.0, 111.0, 95.0, 109.0, 0.0]);
        assert_eq!(
            plc.signal(),
            1.0,
            "bar above with low spike: straddle resolves bullish (close > line)"
        );
    }

    // ---- WickReject ----

    #[test]
    fn wick_reject_bullish() {
        // Bullish SFP: prev close above, low sweeps below line, close back above.
        let mut plc = constant_plc(100.0, TouchMode::WickReject);
        drive(&mut plc, &[110.0; 3]); // prev_above = Some(true)
        plc.feed(0, &[108.0, 112.0, 95.0, 105.0, 0.0]);
        assert_eq!(plc.signal(), 1.0, "bullish wick reject: low below line, close above");
    }

    #[test]
    fn wick_reject_bearish() {
        // Bearish SFP: prev close below, high sweeps above line, close back below.
        let mut plc = constant_plc(100.0, TouchMode::WickReject);
        drive(&mut plc, &[90.0; 3]); // prev_above = Some(false)
        plc.feed(0, &[92.0, 115.0, 91.0, 95.0, 0.0]);
        assert_eq!(plc.signal(), -1.0, "bearish wick reject: high above line, close below");
    }

    // ---- Touch ----

    #[test]
    fn touch_fires_within_tolerance() {
        let mut plc = constant_plc(100.0, TouchMode::Touch { tolerance: 2.0 });
        // High is 101.5 — within 2.0 of 100.0.
        plc.feed(0, &[98.0, 101.5, 97.0, 98.5, 0.0]);
        assert_eq!(plc.signal(), 1.0, "Touch: bar within tolerance must fire");
    }

    #[test]
    fn touch_no_fire_outside_tolerance() {
        let mut plc = constant_plc(100.0, TouchMode::Touch { tolerance: 1.0 });
        // High is 97, low is 93 — both further than 1.0 from 100.
        plc.feed(0, &[95.0, 97.0, 93.0, 94.0, 0.0]);
        assert_eq!(plc.signal(), 0.0, "Touch: bar outside tolerance must not fire");
    }

    // ---- WithCandle ----

    #[test]
    fn with_candle_hammer_requires_cross() {
        let mut plc = constant_plc(100.0, TouchMode::WithCandle(CandlePatternKind::Hammer));
        // Set up: price was below 100.
        drive(&mut plc, &[90.0; 3]);
        // Hammer-shaped bar that crosses above 100:
        // open=99, close=100.5 (above), low=90 (big lower wick), high=100.6.
        plc.feed(0, &[99.0, 100.6, 90.0, 100.5, 0.0]);
        // Hammer: lower_wick=9.5, body=1.5, upper_wick=0.1 → qualifies.
        // Crossed: prev below, now above.
        assert_eq!(plc.signal(), 1.0, "WithCandle(Hammer): must fire +1 when hammer + cross");
    }

    #[test]
    fn with_candle_no_pattern_no_fire() {
        let mut plc = constant_plc(100.0, TouchMode::WithCandle(CandlePatternKind::Hammer));
        // Price crosses above but bar is not a hammer (large body, no lower wick).
        drive(&mut plc, &[90.0; 3]);
        // open=90, close=110 — full-body candle, not a hammer.
        plc.feed(0, &[90.0, 110.0, 89.0, 110.0, 0.0]);
        assert_eq!(plc.signal(), 0.0, "WithCandle: must not fire without pattern match");
    }

    // ---- with SMA line ----

    #[test]
    fn with_indicator_line_close_above() {
        let mut plc = sma_plc(5, TouchMode::CloseAbove);
        // Warm up SMA below 100.
        for _ in 0..10 {
            plc.feed(0, &[95.0, 95.0, 95.0, 95.0, 0.0]);
        }
        // Jump well above — must produce CloseAbove signal at some point.
        let mut fired = false;
        for _ in 0..10 {
            plc.feed(0, &[120.0, 120.0, 120.0, 120.0, 0.0]);
            if plc.signal() > 0.0 {
                fired = true;
            }
        }
        assert!(fired, "CloseAbove with SMA line: must fire when price surges above SMA");
    }

    // ---- Reset ----

    #[test]
    fn reset_clears_state() {
        let mut plc = constant_plc(100.0, TouchMode::CloseAbove);
        drive(&mut plc, &[110.0; 5]);
        plc.reset();
        assert!(!plc.is_ready());
        assert_eq!(plc.signal(), 0.0);
    }

    // ---- Vector output ----

    #[test]
    fn vector_dims_match_level_count() {
        // Pivot has 3 Price outputs (R1, S1, PP) → 3×1 grid.
        let pivot_order =
            IndicatorOrder::from_defaults(IndicatorId::Pivot).expect("Pivot is contract-backed");
        let plc = PriceLineCross::new(
            LineProducerOrder::reference(pivot_order)
                .into_producer()
                .unwrap(),
            TouchMode::CloseAbove,
        );
        assert_eq!(plc.cross_vector_grid().rows(), 3, "Pivot contributes 3 rows");
        assert_eq!(plc.cross_vector_grid().cols(), 1, "touch vector has 1 col");
    }

    #[test]
    fn axis_labels_reflect_bb_outputs() {
        use crate::engine::matrix_grid::Label;
        use crate::engine::contract_engine::IndicatorOutputId;
        // Bollinger (3 Price outputs: BbUpper, BbMiddle, BbLower) → row 0 = BbUpper.
        let bb_order =
            IndicatorOrder::from_defaults(IndicatorId::Bb).expect("Bb is contract-backed");
        let plc = PriceLineCross::new(
            LineProducerOrder::reference(bb_order)
                .into_producer()
                .unwrap(),
            TouchMode::CloseAbove,
        );
        assert_eq!(
            plc.cross_vector_grid().row_label(0),
            Label::Output(IndicatorOutputId::BbUpper),
            "row 0 must be labeled with BbUpper"
        );
    }

    #[test]
    fn factory_exposes_price_line_crosses_vector() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::PriceLineCross(
            <<PriceLineCross as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let bar = |c: f64| MarketSample::Bar {
            open: c,
            high: c + 1.0,
            low: c - 1.0,
            close: c,
            volume: 1.0,
        };
        for _ in 0..10 {
            f.feed(0, bar(100.0));
        }
        let g = f
            .grid(IndicatorOutputId::PriceLineCrossCrossVectorGrid)
            .expect("PriceLineCross emits PriceLineCrossCrossVectorGrid matrix");
        // Default = SMA(20), a single Price line → 1×1 grid.
        assert_eq!(g.cols(), 1, "touch vector must have 1 col");
        assert_eq!(g.rows(), 1, "the SMA line contributes 1 row");

        // A scalar-only producer returns None for this matrix id.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma)
            .unwrap()
            .build_solo()
            .unwrap();
        assert!(
            sma.grid(IndicatorOutputId::PriceLineCrossCrossVectorGrid).is_none(),
            "Sma has no PriceLineCrossCrossVectorGrid matrix"
        );
    }
}
