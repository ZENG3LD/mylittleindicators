//! Line × Line crossover detector.
//!
//! Crosses every Price-domain line of producer A against every Price-domain line of producer B.
//! Producer A emits M lines, producer B emits N → the detector reports the M×N table of cross
//! events (cell `(i, j)` = A's line `i` against B's line `j`: direction `+1`/`-1`/`0` plus
//! timing = bars since the last cross), the owner's "each output × each output" model. The
//! common single-line case is the degenerate 1×1.
//!
//! Outputs:
//! - scalars `left`, `right`, `signal` — the PRIMARY (line-0 × line-0) cross, for the simple
//!   two-line use; backward-compatible with the old detector.
//! - matrix `IndicatorOutputId::LineCrossCrossGrid` — the full M×N cross grid (read whole via
//!   `ContractFactory::grid`).
//!
//! Both operands are FIXED (two producer slots), so M and N are static once the producers are
//! chosen (each producer's Price-output count) — a fixed-shape detector, barometer-visible via
//! the two `SLOTS` (NOT a variable-N consensus, which is a strategy primitive).

use crate::contract::{MarketSample, ValueDomain};
use crate::engine::contract_engine::{ContractFactory, FillError, IndicatorOrder, IndicatorOutputId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// The Price-domain SCALAR output ids of `id`, in declared order — the axis lines a producer
/// contributes (a single-line MA → 1, Bollinger → 3 bands). A cross axis crosses scalar price
/// LINES, so table outputs are excluded even though a table's cell domain may be `Price` (since
/// the unification, a `#`-declared grid is an `Output` too — filter it out by `shape`).
fn price_outputs_of(id: IndicatorId) -> Vec<IndicatorOutputId> {
    use crate::engine::contract_catalog::outputs_of;
    use crate::contract::OutputShape;
    outputs_of(id)
        .map(|os| {
            os.iter()
                .filter(|o| o.domain == ValueDomain::Price && o.shape == OutputShape::Scalar)
                .map(|o| o.id)
                .collect()
        })
        .unwrap_or_default()
}

/// Why a producer operand is rejected.
#[derive(Debug, Clone)]
pub enum LineProducerError {
    /// The producer declares no Price-domain output — it cannot be a cross axis.
    NoPriceOutputs { producer: IndicatorId },
    /// The producer order failed to build.
    Fill(FillError),
}

/// Config-time operand for one cross axis — a producer (all its Price lines) or a constant.
/// The `IndicatorOrder` is BOXED: `LineCross` is itself an `IndicatorOrder` member config, so an
/// inline order would make the box-free enum recursively infinite.
#[derive(Debug, Clone)]
pub enum LineProducerOrder {
    /// A Price-domain producer — all its Price outputs form the axis.
    Ref(Box<IndicatorOrder>),
    /// A fixed horizontal level — a single-line axis.
    Constant(f64),
}

impl ::core::hash::Hash for LineProducerOrder {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        ::core::mem::discriminant(self).hash(state);
        match self {
            Self::Ref(order) => order.config_hash().hash(state),
            Self::Constant(v) => v.to_bits().hash(state),
        }
    }
}

impl PartialEq for LineProducerOrder {
    fn eq(&self, other: &Self) -> bool {
        // Proxy equality through the config hash (NaN-free, deterministic).
        use ::core::hash::{Hash as _, Hasher as _};
        let hash_of = |x: &Self| {
            let mut h = ::std::collections::hash_map::DefaultHasher::new();
            x.hash(&mut h);
            h.finish()
        };
        hash_of(self) == hash_of(other)
    }
}

impl crate::contract::ParamScalar for LineProducerOrder {
    fn scalar_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        ::core::hash::Hash::hash(self, state);
    }
}

impl LineProducerOrder {
    /// A producer axis (all of `order`'s Price lines).
    pub fn reference(order: IndicatorOrder) -> Self {
        LineProducerOrder::Ref(Box::new(order))
    }

    /// The producing indicator's id, or `None` for a constant — the barometer-visible nested
    /// producer this axis embeds.
    pub fn producer(&self) -> Option<IndicatorId> {
        match self {
            LineProducerOrder::Ref(o) => Some(o.id()),
            LineProducerOrder::Constant(_) => None,
        }
    }

    /// How many lines this axis contributes (M or N) — the producer's Price-output count, or 1
    /// for a constant. Fixed once the producer is chosen.
    pub fn line_count(&self) -> usize {
        match self {
            LineProducerOrder::Ref(o) => price_outputs_of(o.id()).len(),
            LineProducerOrder::Constant(_) => 1,
        }
    }

    /// Build the runtime axis operand.
    pub fn into_producer(self) -> Result<LineProducer, LineProducerError> {
        match self {
            LineProducerOrder::Ref(order) => {
                let order = *order;
                let outputs = price_outputs_of(order.id());
                if outputs.is_empty() {
                    return Err(LineProducerError::NoPriceOutputs { producer: order.id() });
                }
                let n = outputs.len();
                let factory = Box::new(order.build_solo().map_err(LineProducerError::Fill)?);
                Ok(LineProducer {
                    factory: Some(factory),
                    outputs,
                    buf: vec![0.0; n],
                })
            }
            LineProducerOrder::Constant(k) => Ok(LineProducer {
                factory: None,
                outputs: Vec::new(),
                buf: vec![k],
            }),
        }
    }
}

/// Runtime cross axis: a built producer read on ALL its Price outputs (or a single constant).
/// [`Self::feed_bar`] returns the current line vector (length = the axis dimension).
#[derive(Debug, Clone)]
pub struct LineProducer {
    /// `None` for a constant axis. Boxed — see [`LineProducerOrder::Ref`].
    factory: Option<Box<ContractFactory>>,
    /// The producer's Price-domain output ids (empty for a constant).
    outputs: Vec<IndicatorOutputId>,
    /// Reused per-bar value buffer (length = axis dimension).
    buf: Vec<f64>,
}

impl LineProducer {
    /// The Price-domain output ids this axis contributes (empty for a constant axis).
    pub fn output_ids(&self) -> &[IndicatorOutputId] {
        &self.outputs
    }

    /// The number of lines on this axis (M or N).
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the axis is empty (never for a built axis).
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Feed one bar and return the current line vector. A constant axis ignores the bar.
    pub fn feed_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64, ts: i64) -> &[f64] {
        if let Some(f) = &mut self.factory {
            f.feed(
                ts,
                MarketSample::Bar { open, high, low, close, volume },
            );
            for (slot, &out) in self.buf.iter_mut().zip(self.outputs.iter()) {
                *slot = f.read(out);
            }
        }
        &self.buf
    }

    /// `true` once the producer has warmed up (a constant is always ready).
    pub fn is_ready(&self) -> bool {
        self.factory.as_ref().map_or(true, |f| f.is_ready())
    }

    /// Reset to cold state (a constant is stateless).
    pub fn reset(&mut self) {
        if let Some(f) = &mut self.factory {
            f.reset();
        }
    }
}

/// Signal stickiness mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum CrossMode {
    /// Non-zero only on the crossover bar itself.
    Momentary,
    /// Holds the last crossover sign (+1 or -1) until the next crossover.
    Sticky,
}

/// Line × Line crossover detector — see the module docs.
#[derive(Debug, Clone)]
pub struct LineCross {
    left: LineProducer,
    right: LineProducer,
    mode: CrossMode,
    cur_left: Vec<f64>,
    cur_right: Vec<f64>,
    prev_left: Vec<f64>,
    prev_right: Vec<f64>,
    /// Held sticky sign per cell (length M·N), used only in [`CrossMode::Sticky`].
    sticky: Vec<i8>,
    /// The M×N direction + timing grid.
    grid: MatrixGrid,
    has_prev: bool,
    /// The PRIMARY (line-0 × line-0) cross signal.
    last_signal: i8,
}

impl LineCross {
    /// Construct from two built axis producers and a stickiness mode.
    pub fn new(left: LineProducer, right: LineProducer, mode: CrossMode) -> Self {
        let m = left.len();
        let n = right.len();
        let grid = MatrixGrid::new(m as u16, n as u16, true).with_labels(
            AxisLabels::Outputs(left.output_ids().to_vec()),
            AxisLabels::Outputs(right.output_ids().to_vec()),
        );
        Self {
            left,
            right,
            mode,
            cur_left: Vec::with_capacity(m),
            cur_right: Vec::with_capacity(n),
            prev_left: vec![0.0; m],
            prev_right: vec![0.0; n],
            sticky: vec![0i8; m * n],
            grid,
            has_prev: false,
            last_signal: 0,
        }
    }

    /// Feed one resolved bar (`+time` flavor → `ts` then lanes `[O,H,L,C,V]`). Drives both
    /// producers, then writes the M×N cross grid (and the primary scalar).
    pub fn feed(&mut self, ts: i64, lanes: &[f64]) {
        let (open, high, low, close, volume) = (lanes[0], lanes[1], lanes[2], lanes[3], lanes[4]);

        self.grid.tick_timing();

        self.cur_left.clear();
        self.cur_left
            .extend_from_slice(self.left.feed_bar(open, high, low, close, volume, ts));
        self.cur_right.clear();
        self.cur_right
            .extend_from_slice(self.right.feed_bar(open, high, low, close, volume, ts));

        let m = self.cur_left.len();
        let n = self.cur_right.len();

        if self.has_prev {
            for i in 0..m {
                let (a_prev, a_now) = (self.prev_left[i], self.cur_left[i]);
                for j in 0..n {
                    let (b_prev, b_now) = (self.prev_right[j], self.cur_right[j]);
                    let raw: i8 = if a_prev <= b_prev && a_now > b_now {
                        1
                    } else if a_prev >= b_prev && a_now < b_now {
                        -1
                    } else {
                        0
                    };
                    let idx = i * n + j;
                    let out = match self.mode {
                        CrossMode::Momentary => raw,
                        CrossMode::Sticky => {
                            if raw != 0 {
                                self.sticky[idx] = raw;
                            }
                            self.sticky[idx]
                        }
                    };
                    let cell = MatrixCell::new(i as u16, j as u16);
                    if raw != 0 {
                        self.grid.set(cell, out as f64, 0.0);
                    } else {
                        self.grid.set_direction(cell, out as f64);
                    }
                    if i == 0 && j == 0 {
                        self.last_signal = out;
                    }
                }
            }
        }

        core::mem::swap(&mut self.prev_left, &mut self.cur_left);
        core::mem::swap(&mut self.prev_right, &mut self.cur_right);
        self.has_prev = true;
    }

    /// Named getter for the `left` brace output — the PRIMARY (line-0) left value.
    pub fn left(&self) -> f64 {
        self.prev_left.first().copied().unwrap_or(0.0)
    }

    /// Named getter for the `right` brace output — the PRIMARY (line-0) right value.
    pub fn right(&self) -> f64 {
        self.prev_right.first().copied().unwrap_or(0.0)
    }

    /// Named getter for the `signal` brace output — the PRIMARY (line-0 × line-0) cross sign.
    pub fn signal(&self) -> f64 {
        self.last_signal as f64
    }

    /// The full M×N cross grid (direction + timing) — the matrix output behind
    /// `IndicatorOutputId::LineCrossCrossGrid`.
    pub fn cross_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    /// True once both operands have warmed up and a prior bar exists.
    pub fn is_ready(&self) -> bool {
        self.has_prev && self.left.is_ready() && self.right.is_ready()
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.cur_left.clear();
        self.cur_right.clear();
        for v in self.prev_left.iter_mut() {
            *v = 0.0;
        }
        for v in self.prev_right.iter_mut() {
            *v = 0.0;
        }
        for s in self.sticky.iter_mut() {
            *s = 0;
        }
        self.grid.reset();
        self.has_prev = false;
        self.last_signal = 0;
    }
}

// ---- Indicator contract ----

use crate::engine::ohlcv_field::OhlcvField as ContractOhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render,
    RenderSpec, Slot, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// The admissible producer set for a cross axis — Price-domain producers. The slot's typed
/// candidate set for the cost model / barometer weight range; additive. Includes the defaults.
pub const LINE_PRODUCERS: &[IndicatorId] = &[
    IndicatorId::Ema,
    IndicatorId::Sma,
    IndicatorId::Wma,
    IndicatorId::Rma,
    IndicatorId::Dema,
    IndicatorId::Tema,
    IndicatorId::Tma,
    IndicatorId::Hma,
    IndicatorId::Trima,
    IndicatorId::Alma,
    IndicatorId::Vwap,
    IndicatorId::Vwma,
    IndicatorId::Kama,
    IndicatorId::Bb,
    IndicatorId::Dc,
    IndicatorId::Kc,
    IndicatorId::Pivot,
    IndicatorId::Camarilla,
    IndicatorId::Woodie,
    IndicatorId::Floorpivot,
];

/// An EMA(`period`) producer order.
fn ema_order(period: usize) -> IndicatorOrder {
    IndicatorOrder::from_defaults(IndicatorId::Ema)
        .expect("Ema is contract-backed")
        .with_period(Param::Solo(period))
}

/// An SMA(`period`) producer order.
fn sma_order(period: usize) -> IndicatorOrder {
    IndicatorOrder::from_defaults(IndicatorId::Sma)
        .expect("Sma is contract-backed")
        .with_period(Param::Solo(period))
}

/// Typed config for [`LineCross`] — two producer axes and the stickiness mode.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LineCrossConfig {
    /// Left axis producer (M Price lines).
    pub left: Param<LineProducerOrder>,
    /// Right axis producer (N Price lines).
    pub right: Param<LineProducerOrder>,
    /// Signal stickiness.
    pub mode: Param<CrossMode>,
}

impl Indicator for LineCross {
    const ID: IndicatorId = IndicatorId::LineCross;
    /// No family — a crossover DETECTOR over two operands, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full O/H/L/C/V — the producers draw whatever fields they declare.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        ContractOhlcvField::Open,
        ContractOhlcvField::High,
        ContractOhlcvField::Low,
        ContractOhlcvField::Close,
        ContractOhlcvField::Volume,
    ]));
    /// O(M·N) outer compare; the dominant cost is the two producer slots (recursive).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Vec, 1)]);
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::LineCrossLeft),
        Output::magnitude(IndicatorOutputId::LineCrossRight),
        Output::ordinal(IndicatorOutputId::LineCrossSignal),
        Output::matrix(IndicatorOutputId::LineCrossCrossGrid, ValueDomain::Discrete),
    ];
    /// Two producer slots (left, right) — declared so the barometer's nested walk and the cost
    /// model SEE the embedded producers (NOT `Cost::DEFAULT`).
    const SLOTS: &'static [Slot] = &[Slot::new(LINE_PRODUCERS), Slot::new(LINE_PRODUCERS)];

    type Config = LineCrossConfig;
    type Runtime = LineCross;

    fn create(cfg: LineCrossConfig) -> LineCross {
        let left = cfg
            .left
            .resolved()
            .into_producer()
            .expect("left axis must be a Price-domain producer");
        let right = cfg
            .right
            .resolved()
            .into_producer()
            .expect("right axis must be a Price-domain producer");
        LineCross::new(left, right, cfg.mode.resolved())
    }

    /// The resolved producer ids of the two operand slots (constants contribute none) — the
    /// barometer-visible nested members, in `SLOTS` order.
    fn slot_members(cfg: &LineCrossConfig) -> Vec<IndicatorId> {
        let mut v = Vec::with_capacity(2);
        if let Some(id) = cfg.left.resolved().producer() {
            v.push(id);
        }
        if let Some(id) = cfg.right.resolved().producer() {
            v.push(id);
        }
        v
    }
}

impl crate::contract::Config for LineCrossConfig {
    fn defaults() -> Self {
        LineCrossConfig {
            left: Param::Solo(LineProducerOrder::reference(ema_order(9))),
            right: Param::Solo(LineProducerOrder::reference(sma_order(21))),
            mode: Param::Solo(CrossMode::Momentary),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::Param;
        let mut s = Self::machine_defaults_auto();
        // mode: Class Q — all CrossMode variants.
        s.mode = Param::many(vec![CrossMode::Momentary, CrossMode::Sticky]);
        // left / right: Param<LineProducerOrder> — structural producer wiring (deferred wave).
        // Left Solo (machine_defaults_auto default). [FLAG: producer slot order, deferred wave]
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Render for LineCross {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LineCrossLeft, "Left", Color::hex(0x8BC34A))
            .line_output(IndicatorOutputId::LineCrossRight, "Right", Color::hex(0x607D8B))
            .line_output(IndicatorOutputId::LineCrossSignal, "Cross", Color::hex(0xFF9800))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A producer-vs-constant cross with the given operands.
    fn cross(left: LineProducerOrder, right: LineProducerOrder, mode: CrossMode) -> LineCross {
        LineCross::new(
            left.into_producer().unwrap(),
            right.into_producer().unwrap(),
            mode,
        )
    }

    fn drive(lc: &mut LineCross, prices: &[f64]) {
        for &p in prices {
            lc.feed(0, &[p, p, p, p, 1.0]);
        }
    }

    #[test]
    fn primary_ema_vs_constant_crosses_up() {
        // EMA(3) vs constant 100: warm below, then surge above → primary signal +1.
        let mut lc = cross(
            LineProducerOrder::reference(ema_order(3)),
            LineProducerOrder::Constant(100.0),
            CrossMode::Momentary,
        );
        drive(&mut lc, &[90.0, 92.0, 94.0]);
        let mut saw_up = false;
        for _ in 0..15 {
            lc.feed(0, &[130.0, 130.0, 130.0, 130.0, 1.0]);
            if lc.signal() > 0.0 {
                saw_up = true;
            }
        }
        assert!(saw_up, "EMA must cross above 100 when price surges to 130");
    }

    #[test]
    fn matrix_dimensions_are_producer_output_counts() {
        // Bollinger (3 bands) × constant (1) → a 3×1 cross grid.
        let lc = cross(
            LineProducerOrder::reference(IndicatorOrder::from_defaults(IndicatorId::Bb).unwrap()),
            LineProducerOrder::Constant(100.0),
            CrossMode::Momentary,
        );
        assert_eq!(lc.cross_grid().rows(), 3, "Bollinger contributes 3 rows");
        assert_eq!(lc.cross_grid().cols(), 1, "constant contributes 1 col");
    }

    #[test]
    fn axis_labels_reflect_producer_output_ids() {
        use crate::engine::matrix_grid::Label;
        // Bollinger (3 Price outputs) × constant → row 0 = BbUpper, col 0 = Index(0).
        let lc = cross(
            LineProducerOrder::reference(IndicatorOrder::from_defaults(IndicatorId::Bb).unwrap()),
            LineProducerOrder::Constant(100.0),
            CrossMode::Momentary,
        );
        assert_eq!(
            lc.cross_grid().row_label(0),
            Label::Output(IndicatorOutputId::BbUpper),
            "row 0 must be labeled with BbUpper"
        );
        assert_eq!(
            lc.cross_grid().col_label(0),
            Label::Index(0),
            "constant axis col 0 falls back to Label::Index"
        );
    }

    #[test]
    fn sticky_holds_primary_sign() {
        let mut lc = cross(
            LineProducerOrder::reference(ema_order(3)),
            LineProducerOrder::Constant(100.0),
            CrossMode::Sticky,
        );
        drive(&mut lc, &[90.0, 90.0, 90.0]);
        drive(&mut lc, &[130.0, 130.0, 130.0, 130.0]);
        assert_eq!(lc.signal(), 1.0, "sticky holds +1 after crossing up");
    }

    #[test]
    fn reset_clears() {
        let mut lc = cross(
            LineProducerOrder::reference(ema_order(3)),
            LineProducerOrder::Constant(100.0),
            CrossMode::Sticky,
        );
        drive(&mut lc, &[90.0, 130.0, 130.0, 130.0]);
        lc.reset();
        assert!(!lc.is_ready());
        assert_eq!(lc.signal(), 0.0);
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_line_cross() {
        // Default EMA(9) × SMA(21), momentary. A sustained uptrend makes the fast EMA cross the
        // slower SMA → a +1 primary signal at some point.
        let mut f = IndicatorOrder::LineCross(
            <<LineCross as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let mut saw_up = false;
        let mut price = 100.0;
        for _ in 0..80 {
            price += 1.0;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5, high: price + 0.5, low: price - 1.0, close: price, volume: 1.0,
            });
            if f.read(IndicatorOutputId::LineCrossSignal) > 0.0 {
                saw_up = true;
            }
        }
        assert!(saw_up, "fast EMA must cross above the slower SMA during a sustained uptrend");
    }

    #[test]
    fn factory_exposes_line_cross_matrix() {
        let f = IndicatorOrder::LineCross(
            <<LineCross as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::LineCrossCrossGrid)
            .expect("LineCross emits the LineCrossCrossGrid matrix");
        // Default EMA(9) × SMA(21) → single-line producers → 1×1 grid.
        assert_eq!(g.rows(), 1);
        assert_eq!(g.cols(), 1);

        // A scalar-only producer returns None.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::LineCrossCrossGrid).is_none());
    }
}
