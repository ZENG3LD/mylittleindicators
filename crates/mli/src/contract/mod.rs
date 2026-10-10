//! Indicator self-declaration contract.
//!
//! An indicator owns its complete typed contract *next to its implementation*:
//! the typed config, its standard defaults, and how to construct itself. The
//! factory stops being a 9.6k-line store of per-indicator knowledge and becomes
//! dumb dispatch over self-declared indicators — i.e. the factory *is* the
//! catalog (take an id, get all the info).
//!
//! This is deliberately minimal. There is NO config-schema, NO `ConfigPoint`,
//! NO `Node`/`FamilyKind`, NO registry macro, NO parameter-domain apparatus.
//! Those were rejected. The only thing outside the trait + factory is a family
//! cluster-enum of machine ids (`MovingAverageType`, future `OscillatorType`,
//! …) with zero construction logic.
//!
//! The metadata an indicator self-declares grows by-the-fact from real fills:
//! the input-data descriptor appears when the first non-OHLCV indicator forces
//! it; the output descriptor appears when the first multi-output indicator
//! forces it. Neither is pre-designed from the trivial case.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

/// The natural cluster an indicator BELONGS to (≈ the existing catalog
/// categories: average / momentum / channels / book / open_interest /
/// liquidations / …). Each indicator self-declares its family; the per-family id
/// set is a derived view over the contracts.
///
/// The point of these tags is to make a family **pluggable** — so a consumer can
/// ask for "a channel" / "an oscillator" / "an MA" and plug any member by machine
/// id — replacing the stringly metadata catalogs. Membership is about WHAT the
/// indicator IS, not whether anything currently plugs it (that's the limitation
/// we are removing).
///
/// Grows by-the-fact as fills reach new categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Smoothing kernels (SMA, EMA, WMA, RMA, …). Eventually subsumes `ma_type`.
    MovingAverage,
    /// Momentum oscillators (RSI, MACD, Stochastic, CCI, Williams %R, …).
    Oscillator,
    /// Banded channels (Bollinger, Donchian, Keltner, … — ~a dozen).
    Channel,
    /// Volatility measures (ATR, realized vol, std-dev, NATR, …). Subsumes the
    /// `volatility_advanced` HV/VolIndex stream variants (same family, non-Bar INPUT).
    Volatility,
    /// Trend strength / direction (efficiency ratio, slope lines, ADX-slope, …).
    Trend,
    /// L2 order-book indicators (imbalance, slope, queue, …).
    OrderBook,
    /// Open-interest indicators (OI z-score, OI delta, …).
    OpenInterest,
    /// Liquidation-stream indicators (rate, intensity, …).
    Liquidations,
}

/// Per-bar update complexity as a function of the indicator's period/window.
/// Objective: does the per-bar update rescan the window or maintain running state?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateComplexity {
    /// O(1) — running state, no per-bar loop over the window (EMA, RMA, McGinley,
    /// and SMA: it keeps a buffer but updates via a running sum).
    Constant,
    /// O(log n) amortized (heap/tree-backed extremes).
    Logarithmic,
    /// O(period) — a per-bar rescan of the window (LR least-squares, FRAMA
    /// fractal-dimension scan, naive convolutions).
    Linear,
    /// O(period²) or heavier per bar.
    Quadratic,
}

/// The container backing a unit of runtime state. Distinguishes inline (stack)
/// storage from heap storage, which dominates real per-instance cost (allocation
/// + cache indirection across a parallel sweep). Declared by hand off the struct
/// — we see the internals when we fill the contract, so no macro is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    /// Plain scalar fields (`f64` / `usize`) — cheapest, inline, no container.
    Scalar,
    /// Fixed inline array `[T; N]` — stack, no heap, cache-friendly.
    StackArray,
    /// Bounded inline vector (`ArrayVec`) — stack + a length, no heap.
    ArrayVec,
    /// Heap vector (`Vec`) — allocation + pointer indirection.
    Vec,
    /// Heap ring (`VecDeque`) — like `Vec` plus both-ends bookkeeping.
    Deque,
    /// Order-maintaining heap vector (a `Vec` kept sorted) — `Vec` plus per-insert
    /// ordering work; the heaviest container.
    SortedVec,
}

impl StoreKind {
    /// Per-STORE cost factor by container kind — `Scalar` ≪ `StackArray` < `ArrayVec`
    /// < `Vec` < `Deque` < `SortedVec`. The factor is the WHOLE-container cost, NOT
    /// per-element: depth/period does not scale it (a deep buffer is a few cents of
    /// memory — negligible vs the kind's allocation/indirection class). Values live in
    /// [`WEIGHTS`] (the single calibration target), not inline here.
    fn factor(self) -> f64 {
        match self {
            StoreKind::Scalar => WEIGHTS.scalar,
            StoreKind::StackArray => WEIGHTS.stack_array,
            StoreKind::ArrayVec => WEIGHTS.array_vec,
            StoreKind::Vec => WEIGHTS.heap_vec,
            StoreKind::Deque => WEIGHTS.deque,
            StoreKind::SortedVec => WEIGHTS.sorted_vec,
        }
    }
}

/// How deep a store is — DESCRIPTIVE metadata only (a period-window buffer vs a
/// fixed-N buffer). Depth does NOT enter the weight: window/period depth is negligible
/// cost (a few bytes of memory); the [`StoreKind`] container CLASS is what is priced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// Depth scales with the indicator's period (a rolling window buffer).
    Window,
    /// A fixed element count, independent of period.
    Fixed(u16),
}

/// One unit of internal runtime storage: container kind × depth.
#[derive(Debug, Clone, Copy)]
pub struct Store {
    pub kind: StoreKind,
    pub depth: Depth,
}

impl Store {
    /// A rolling window of the indicator's period (the SMA/LR/FRAMA buffer).
    pub const fn window(kind: StoreKind) -> Self {
        Self { kind, depth: Depth::Window }
    }
    /// A fixed-depth store of `n` elements (independent of period).
    pub const fn fixed(kind: StoreKind, n: u16) -> Self {
        Self { kind, depth: Depth::Fixed(n) }
    }
}

/// Tunable coefficients of the weight assembly — the single **calibration
/// target**. The v1 values are a rubric: they give the correct *ordering* (period-FREE,
/// by structural class), but are NOT measured time. A future calibration wave refits
/// them to microbenchmarked per-update ns (and the input-side `stream_cost` in
/// dispatch) — that wave is where these constants stop being hand-picked and become
/// profiled. Swap this one const to re-weigh everything.
#[derive(Debug, Clone, Copy)]
pub struct WeightCoeffs {
    /// Per-store cost by container CLASS (inline ≪ heap). NOT per-element — store
    /// depth/period does not scale these (period is negligible).
    pub scalar: f64,
    pub stack_array: f64,
    pub array_vec: f64,
    pub heap_vec: f64,
    pub deque: f64,
    pub sorted_vec: f64,
    /// Per-bar compute-CLASS factors (period-FREE): a rescan class is a flat multiple of
    /// the O(1) base, never scaled by period. `Constant` is the unit base (1.0 in
    /// `local_cost`, 0.0 marginal) and so has no coefficient here.
    pub update_log: f64,
    pub update_linear: f64,
    pub update_quad: f64,
    /// Weight of the memory term relative to the compute term in `local_cost`.
    pub mem_weight: f64,
    /// Saturation constant: `weight = raw / (raw + norm_k)`.
    pub norm_k: f64,
}

/// The live weight coefficients (v1 rubric). The calibration wave replaces this.
pub const WEIGHTS: WeightCoeffs = WeightCoeffs {
    scalar: 0.2,
    stack_array: 0.5,
    array_vec: 0.6,
    heap_vec: 1.0,
    deque: 1.1,
    sorted_vec: 1.4,
    update_log: 2.0,
    update_linear: 4.0,
    update_quad: 8.0,
    mem_weight: 0.5,
    norm_k: 4.0,
};

/// Self-declared computational-cost facets of an indicator, **assembled** into a
/// weight — never a hand-tagged "cheap/heavy". The facets are objective structure
/// the contract-filler reads straight off the impl:
/// - `update`: does the per-bar update rescan the window (O(period)) or not (O(1))?
/// - `stores`: the runtime's containers (kind × depth) — this is what makes EMA
///   (no window store) cheaper than SMA (a period-deep `Vec`) though their config,
///   input and output are identical.
/// - `inner`: port-selective [`Port`]s to sub-indicators — recursive cost charges
///   each producer's `base + Σ marginal(consumed outputs)`, via dispatch.
///
/// Input cost is added by dispatch from [`Indicator::INPUT`]; the result is a
/// **machine-pruning signal**: a strategy search iterating a family slot ("pick
/// any MA") admits/skips members by weight, so heavy members are cut before they
/// blow up a combinatorial sweep. The weight is PERIOD-FREE — it discriminates by
/// STRUCTURE (store container class, update class, recursive inner ports/members),
/// e.g. SMA (a `Vec` window) over EMA (no window), independent of the chosen period
/// (which is negligible cost).
#[derive(Debug, Clone, Copy)]
pub struct Cost {
    pub update: UpdateComplexity,
    pub stores: &'static [Store],
    pub inner: &'static [Port],
}

impl Cost {
    /// Construct a leaf cost (no inner sub-indicators).
    pub const fn new(update: UpdateComplexity, stores: &'static [Store]) -> Self {
        Self { update, stores, inner: &[] }
    }

    /// Baseline default: O(1), no stores, no inner — the EMA-class cheapest leaf.
    /// Lets `const COST` be added to the trait non-breakingly; each indicator
    /// declares its real stores as it is filled.
    pub const DEFAULT: Cost = Cost::new(UpdateComplexity::Constant, &[]);

    /// This node's LOCAL (non-recursive) raw cost, BEFORE input cost and inner recursion
    /// (those are added by dispatch). PERIOD-FREE: memory = Σ store kind factors (the
    /// container CLASS, depth ignored); compute = the update-complexity class factor (a
    /// rescan is a flat multiple of the O(1) base, NOT scaled by period). Coefficients
    /// are a v1 rubric; the SHAPE (assembled from declared structure) is the point —
    /// calibrated to profiled ns later.
    pub fn local_cost(&self) -> f64 {
        let mem: f64 = self.stores.iter().map(|s| s.kind.factor()).sum();
        let compute = match self.update {
            UpdateComplexity::Constant => 1.0,
            UpdateComplexity::Logarithmic => WEIGHTS.update_log,
            UpdateComplexity::Linear => WEIGHTS.update_linear,
            UpdateComplexity::Quadratic => WEIGHTS.update_quad,
        };
        WEIGHTS.mem_weight * mem + compute
    }
}

/// The MARGINAL cost a port adds on top of the indicator's shared base
/// ([`Cost::stores`]/`update` — the core state + the primary `value` port).
///
/// This is the key to honest cost: a consumer that wires only the cheap `value`
/// port of a rich producer (e.g. KAMA) must NOT pay for its `efficiency_variance`
/// analytics. So weight is assembled per consumed port: `base + Σ marginal(taken
/// outputs)`. The `value`/primary port and byproduct outputs already computed for it
/// (KAMA's `efficiency_ratio`, `adaptive_period`) are [`OutputCost::FREE`]; outputs
/// that need their own buffer or per-bar pass declare it.
///
/// Independent outputs → `base == 0`, the marginals simply sum. Shared-core outputs →
/// `base > 0` paid once + additive marginals. One model, both regimes.
#[derive(Debug, Clone, Copy)]
pub struct OutputCost {
    /// Extra per-bar pass this port adds beyond the base (NOT the base's own
    /// complexity). `Constant` here means "no measurable extra pass" → 0.
    pub update: UpdateComplexity,
    /// Extra stores this port maintains beyond the base.
    pub stores: &'static [Store],
}

impl OutputCost {
    /// A free-rider port: no extra store, no extra pass (the primary `value` port
    /// and byproducts already computed for it).
    pub const FREE: OutputCost = OutputCost { update: UpdateComplexity::Constant, stores: &[] };

    /// Construct a marginal port cost.
    pub const fn new(update: UpdateComplexity, stores: &'static [Store]) -> Self {
        Self { update, stores }
    }

    /// This port's marginal cost (period-free). `Constant` adds 0 (a few ops, not a
    /// pass); a real extra scan/buffer adds its store-class mem + compute-class factor
    /// on top of the base.
    pub fn marginal_cost(&self) -> f64 {
        let mem: f64 = self.stores.iter().map(|s| s.kind.factor()).sum();
        let pass = match self.update {
            UpdateComplexity::Constant => 0.0,
            UpdateComplexity::Logarithmic => WEIGHTS.update_log,
            UpdateComplexity::Linear => WEIGHTS.update_linear,
            UpdateComplexity::Quadratic => WEIGHTS.update_quad,
        };
        WEIGHTS.mem_weight * mem + pass
    }
}

/// The COORDINATE SYSTEM a value lives in — a semantic property of the OUTPUT VALUE,
/// NOT of where it is drawn (render is downstream and DERIVES from this; the same value
/// could be rendered as an overlay marker without changing its domain). Two values are
/// comparable / crossable ONLY within the same domain; relating values across domains
/// (price-drawn ↔ oscillator) is a detector's job (divergence), never a raw `Cmp`.
///
/// Per-OUTPUT, not per-indicator: one indicator emits outputs in different domains
/// (KAMA: `value` = `Price`, `efficiency_ratio` = `Percent`, `current_direction` =
/// `Discrete`). The contract forces each output to declare its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueDomain {
    /// Absolute instrument price units — the chart axis. Levels crossable with each
    /// other and with price (MA, channel, VWAP, pivot, horizontal price-level).
    Price,
    /// "How wide / how far / how much spread" — a NON-NEGATIVE distance with a true
    /// zero, never a level and never signed (ATR, stdev, band-width, MAD, range,
    /// distance-to-level; also the dimensionless dispersions: realized/return vol,
    /// entropy in nats, CUSUM intensity). Reference 0. Crossable within itself
    /// (ATR × ATR-avg) and against positive thresholds. The UNIT (price vs
    /// dimensionless) is a secondary tag, not a domain split; price-unit members
    /// additionally compose with `Price` arithmetically (`price ± k·mag → a `Price`
    /// band`) — that composition is what ties Magnitude to the chart.
    Magnitude,
    /// "What fraction of a whole" — a BOUNDED fraction. The domain is the FRAME; a
    /// member may natively emit `[0,1]` (a coefficient), `[0,100]` (RSI/Stoch/MFI/%B),
    /// `[0,0.5]`, or even `[1,2]` (rescaled) — the comparison layer canonicalizes to
    /// `[0,100]`. Reference levels 0 / 50 / 100, OB/OS thresholds.
    Percent,
    /// "Which side, how far from neutral" — SIGNED, zero-cross reference, scale-free
    /// (MACD, CCI, ROC, momentum, TSI, z-scores, signed spreads, slopes, signed
    /// `[-1,1]` like correlation/imbalance). Reference 0, ± thresholds. A bound, if
    /// any, is a RENDER hint, not a domain change. The catch-all for the signed
    /// continuous oscillator space — neither `Discrete` nor `Ordinal`.
    Centered,
    /// "How many / how much" — a positive instantaneous tally or size in its OWN unit
    /// (volume, open interest, trade-count, currency notional, a period length in
    /// bars). Reference 0, crossable within itself (volume × volume-avg). The signed
    /// CUMULATIVE sibling is `Flow`.
    Count,
    /// "How many times" — a positive dimensionless RATIO of two like quantities whose
    /// multiplicative neutral is `1.0` (above 1 = numerator dominates). NOT `Magnitude`
    /// (ref 0, not 1), NOT `Percent` (unbounded, ref 1 not 50), NOT `Centered`
    /// (positive, ref 1 not 0). Crossable within itself and against the 1.0 line /
    /// multiplicative thresholds (Vortex, variance-ratio, Didi MA÷MA, auction
    /// current÷avg, spectral power ratios, mass-index).
    Ratio,
    /// A cumulative SIGNED accumulator whose zero carries NO meaning (it is wherever
    /// the running sum was initialized) — unbounded both ways, compared only to its
    /// OWN moving average / its slope / via divergence vs price, never to 0 and never
    /// to price (OBV, AD, PVT, VPT, CVD, KVO, ASI, NVI/PVI). A `Price`-like absolute
    /// level line in a non-price unit; distinct from `Count` (signed) and from
    /// `Centered` (no meaningful zero).
    Flow,
    /// Stepped signed level over a wide range, canonically `-10..=10` (sub-regime
    /// strength; practically within ±3, headroom to ±10). Ordered steps, not a
    /// continuous axis and not a small categorical set.
    Ordinal,
    /// Small categorical / sign — a fixed tiny set (-1/0/1 direction, regime tag,
    /// pattern-id). Compared by EQUALITY, never crossed.
    Discrete,
}

/// One OUTPUT an indicator emits — a computational scalar a consumer wires BY TYPED ID
/// through a [`Port`] (fixed) or [`Slot`] (config-chosen). PURE COMPUTE: whether this
/// value is DRAWN is the separate `Render` contract's concern. The `domain` declares the
/// value's COORDINATE SYSTEM ([`ValueDomain`]) — a compute-semantic property both
/// strategies (comparability) and render (pane/scale) consume.
/// Whether an [`Output`] is a single SCALAR (read via [`ContractFactory`](crate::engine::contract_engine::ContractFactory)`::read`
/// as an `f64`) or a whole TABLE/grid (read via `ContractFactory::grid` as a
/// [`MatrixGrid`](crate::engine::MatrixGrid)). Both share the ONE [`IndicatorOutputId`] space —
/// the shape is the only difference. A `#name` manifest brace entry is `Matrix`; a bare ident
/// is `Scalar`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputShape {
    /// One `f64` per bar.
    Scalar,
    /// A whole self-describing [`MatrixGrid`](crate::engine::MatrixGrid) (rows × cols, labeled
    /// axes) per bar.
    Matrix,
}

#[derive(Debug, Clone, Copy)]
pub struct Output {
    pub id: IndicatorOutputId,
    /// Marginal cost beyond the shared base — see [`OutputCost`]. The primary output
    /// and byproducts already computed for it are [`OutputCost::FREE`].
    pub cost: OutputCost,
    /// The coordinate system this value lives in — see [`ValueDomain`]. For a [`OutputShape::Matrix`]
    /// output this is the CELL domain.
    pub domain: ValueDomain,
    /// Scalar (`f64`) vs Matrix (whole grid). Routes the read surface — see [`OutputShape`].
    pub shape: OutputShape,
}

impl Output {
    /// Full constructor: explicit cost + domain (scalar).
    pub const fn typed(id: IndicatorOutputId, cost: OutputCost, domain: ValueDomain) -> Self {
        Self { id, cost, domain, shape: OutputShape::Scalar }
    }

    // ── domain-tagged, cost-free constructors (the primary output + byproducts) ──
    /// A `Price`-domain free-rider output.
    pub const fn price(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Price, shape: OutputShape::Scalar }
    }
    /// A `Magnitude`-domain free-rider output.
    pub const fn magnitude(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Magnitude, shape: OutputShape::Scalar }
    }
    /// A `Percent`-domain (`[0,100]`) free-rider output.
    pub const fn percent(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Percent, shape: OutputShape::Scalar }
    }
    /// A `Centered`-domain (zero-line) free-rider output.
    pub const fn centered(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Centered, shape: OutputShape::Scalar }
    }
    /// A `Count`-domain free-rider output.
    pub const fn count(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Count, shape: OutputShape::Scalar }
    }
    /// An `Ordinal`-domain free-rider output.
    pub const fn ordinal(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Ordinal, shape: OutputShape::Scalar }
    }
    /// A `Discrete`-domain free-rider output.
    pub const fn discrete(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Discrete, shape: OutputShape::Scalar }
    }
    /// A `Ratio`-domain (positive, neutral 1.0) free-rider output.
    pub const fn ratio(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Ratio, shape: OutputShape::Scalar }
    }
    /// A `Flow`-domain (cumulative signed accumulator) free-rider output.
    pub const fn flow(id: IndicatorOutputId) -> Self {
        Self { id, cost: OutputCost::FREE, domain: ValueDomain::Flow, shape: OutputShape::Scalar }
    }
    /// A TABLE output — a whole [`MatrixGrid`](crate::engine::MatrixGrid) read via
    /// `ContractFactory::grid`. `domain` is the CELL domain. Free-rider on the indicator's
    /// compute. Declare the matching `#name` in the manifest brace so it shares the id space.
    pub const fn matrix(id: IndicatorOutputId, domain: ValueDomain) -> Self {
        Self { id, cost: OutputCost::FREE, domain, shape: OutputShape::Matrix }
    }
}

/// A PORT: a FIXED allocation of a `producer` indicator inside this one, wiring
/// specific typed `outputs` of it (declared at compile time — the consumer knows what
/// it embeds). The unit that makes consumption explicit — a composite reads typed
/// outputs THROUGH the contract instead of reaching into the producer's struct
/// methods. Recursive cost charges `base + Σ marginal(outputs)` of the producer, so
/// a node that takes only a cheap output is weighed cheaply. The config-chosen
/// sibling is a [`Slot`] (any family member instead of one fixed producer).
#[derive(Debug, Clone, Copy)]
pub struct Port {
    pub producer: IndicatorId,
    pub outputs: &'static [IndicatorOutputId],
}

impl Port {
    /// An edge consuming `outputs` of `producer`.
    pub const fn new(producer: IndicatorId, outputs: &'static [IndicatorOutputId]) -> Self {
        Self { producer, outputs }
    }
}

/// A SLOT: a CONFIG-CHOSEN [`Port`] — this indicator embeds any member of `family`
/// (picked by config, fed an internal derived series). Unlike a [`Port`] (one
/// producer named at compile time), the concrete member is chosen at order time. Cost
/// is still fully recursive: the slot charges the base of whichever member is chosen
/// plus the inputs and edges — the primary `value` port is FREE, so the slot adds no
/// marginal on top of the chosen member's base. The admissible SET is the typed
/// field's [`FamilyId::MEMBERS`] (carried as `candidates`) — e.g. a smoother slot
/// typed to `SmootherId` admits only the smoother-safe MAs, by the COMPILER, no
/// weight cap. This is the typed replacement for the legacy `ma_type` /
/// `MovingAverageProvider` knob.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    /// The admissible member set — the typed `#[slot]` field's [`FamilyId::MEMBERS`]
    /// (e.g. `SmootherId::MEMBERS` = the smoother-safe MAs). The COMPILER already
    /// confines the field to these; this carries them for the cost model's weight
    /// range. The "which members" boundary is the TYPE — no weight cap, no hand list.
    pub candidates: &'static [IndicatorId],
}

impl Slot {
    /// A slot whose admissible members are `candidates` (the typed field's
    /// `FamilyId::MEMBERS`). The resolved choice is read from the TYPED config via
    /// [`Indicator::slot_members`] (no `config_key`, no `ma_types` bag); the
    /// STATIC-weight filler is the indicator's own [`Indicator::defaults`].
    /// Generated from the `#[slot]` config field by `#[derive(Slots)]`.
    pub const fn new(candidates: &'static [IndicatorId]) -> Self {
        Self { candidates }
    }
}

/// A typed id-subset enum — a generated id-only enum (`SmootherId` = the flagged
/// smoother-safe MA subset), the config-field type for a [`Slot`]. The COMPILER admits only its members into such
/// a field, so the slot can never hold a foreign id; `MEMBERS` carries that admissible
/// set (widened) for `#[derive(Slots)]` and the cost model's weight range — the typed
/// boundary, no weight cap, no hand list.
pub trait FamilyId: Copy {
    /// Every member of this id-enum, widened to the global [`IndicatorId`] — the
    /// candidate set a [`Slot`] typed to this enum admits.
    const MEMBERS: &'static [IndicatorId];
    /// Widen one id to the global [`IndicatorId`].
    fn to_bar_id(self) -> IndicatorId;
}

/// One input LANE an indicator draws a scalar from — which kline field, and whether the
/// order may vary it. The atomic unit of [`SourceAxis`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLane {
    /// A FIXED kline field the indicator cannot vary — the contract delivers exactly it
    /// (ATR/CCI High/Low/Close, OBV's volume lane).
    Fixed(OhlcvField),
    /// An order-CONFIGURABLE field: the order picks any [`OhlcvField`], the optimizer sweeps
    /// it; `default` resolves when the order is silent. The chosen value lives in the typed
    /// config; the factory reads it via [`Indicator::source_fields`].
    Config { default: OhlcvField },
}

/// The INPUT-side axis: the ORDERED list of scalar LANES an indicator draws from its
/// [`Indicator::INPUT`] stream. The input-side mirror of the [`Slot`]/[`Port`] dependency
/// side — `SLOTS`/`Port`s pick sub-indicators, `SOURCE` picks the input fields. It expresses
/// ANY arity/mix: one lane (every single-source MA/oscillator), N fixed lanes (ATR/CCI =
/// H/L/C), N configurable lanes (MACD = fast/slow field), or a mix (OBV = configurable price
/// + fixed volume). The factory resolves the declared fields from the sample and feeds the
/// pure core the scalar(s); the core knows NO transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAxis {
    /// CONFIGURABLE single field — sugar for `Lanes(&[SourceLane::Config { default }])`.
    /// Any single-series indicator (every MA, single-field oscillator) over Close/HL2/….
    Field { default: OhlcvField },
    /// N FIXED kline fields the indicator cannot vary — sugar for an all-`Fixed` `Lanes`
    /// (range/multi-field indicators: WilliamsR/Stochastic/ATR/CCI draw on h/l/c at once).
    KlineSlice(&'static [OhlcvField]),
    /// The GENERAL ordered lane list — N lanes, each `Fixed` or `Config` (or any mix):
    /// MACD = `[Config{Close}, Config{Close}]`, OBV = `[Config{Close}, Fixed(Volume)]`.
    Lanes(&'static [SourceLane]),
}

impl SourceAxis {
    /// The ordered lanes this axis declares, normalizing the `Field`/`KlineSlice` sugar
    /// into the general `SourceLane` list. Up to 4 lanes inline.
    pub fn lanes(&self) -> arrayvec::ArrayVec<SourceLane, 8> {
        let mut out = arrayvec::ArrayVec::new();
        match self {
            SourceAxis::Field { default } => {
                let _ = out.try_push(SourceLane::Config { default: *default });
            }
            SourceAxis::KlineSlice(fs) => {
                for f in fs.iter() {
                    let _ = out.try_push(SourceLane::Fixed(*f));
                }
            }
            SourceAxis::Lanes(ls) => {
                for l in ls.iter() {
                    let _ = out.try_push(*l);
                }
            }
        }
        out
    }
}

/// A self-declaring indicator: it owns its typed config, its standard defaults,
/// its family tags and its construction. Implemented directly on the indicator
/// struct.
pub trait Indicator {
    /// The machine id this indicator answers to.
    const ID: IndicatorId;

    /// Families this indicator belongs to (self-declared). `&[]` if it is not a
    /// pluggable family member. The MA/Oscillator/Channel id sets derive from
    /// this across all contracts — no separate `ma_type`/`oscillator_type` truth.
    const FAMILY: &'static [Family];

    /// The dig3 stream(s) this indicator consumes (routing). WHICH slice/field of the
    /// stream it draws from is the separate [`Self::SOURCE`] axis (configurable or
    /// bound) — not folded in here. OrderBook snapshot vs delta are distinct
    /// `StreamKind`s, not a slice.
    const INPUT: &'static [StreamKind];

    /// The full typed output surface — the unit of contract wiring (ind→ind,
    /// ind→event). A consumer wires specific typed outputs through a [`Port`]
    /// or [`Slot`]; it may take only PART of a producer's surface. Every indicator
    /// must declare its outputs explicitly with a domain-tagged constructor
    /// (`Output::price`/`percent`/`centered`/`magnitude`/`count`/`ratio`/`flow`/
    /// `ordinal`/`discrete`, or `Output::typed` for a cost-bearing output),
    /// `IndicatorOutputId::…` in the same order as the manifest — the contract
    /// FORCES every output to declare its [`ValueDomain`]. Rich producers (e.g. KAMA) add extra
    /// typed outputs so composites wire to them through the contract instead of
    /// reaching into the struct's methods.
    /// Every output this indicator emits — scalars AND tables — in the ONE list, each with its
    /// [`ValueDomain`] and [`OutputShape`]. A table output ([`Output::matrix`]) shares the same
    /// [`IndicatorOutputId`](crate::engine::contract_engine::IndicatorOutputId) space as the
    /// scalars (declared `#name` in the manifest brace) and is read whole via
    /// `ContractFactory::grid` instead of `read`.
    const OUTPUTS: &'static [Output];

    /// Self-declared computational-cost facets, folded into a `[0, 1]` weight by
    /// [`Cost::local_weight`] (+ recursive inner cost via dispatch). The machine-
    /// pruning signal: a family-slot search admits/skips members by weight, so the
    /// heavy twin of a duplicated algorithm (e.g. the vector-heavy KAMA vs the O(1)
    /// AMA) is cut automatically in cost-bounded sweeps. Defaults to the cheapest
    /// leaf; refined per indicator as it is characterized.
    const COST: Cost = Cost::DEFAULT;

    /// Config-chosen family slots this indicator embeds (the typed replacement for
    /// `ma_type`). Empty for most indicators. A slot reduces to the same recursive
    /// consumed-port cost as an [`Port`], filled by its `default` for static weight.
    const SLOTS: &'static [Slot] = &[];

    /// The KLINE source axis — which field(s) of the [`Self::INPUT`] kline this
    /// indicator draws from. `Some` ONLY for kline (Bar) consumers; `None` for non-bar
    /// indicators (order book / OI / liquidations have no kline slices — the axis does
    /// not apply). Defaults to a CONFIGURABLE single field over the close — any
    /// single-series indicator can take any `OhlcvField` and the optimizer can sweep it
    /// (the input-side mirror of `SLOTS`). Range/multi-field indicators override with
    /// `Some(KlineSlice(..))`; non-bar indicators with `None`.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });

    /// `true` if this indicator REQUIRES the volume stream to compute (volume-weighted
    /// MAs: VWMA, VWAP). A price-smoother [`Slot`] feeds its member a single derived
    /// SCALAR series with NO volume, so a volume-dependent member degenerates there —
    /// slot admission ([`crate::indicators::contract_dispatch::weight_range`])
    /// excludes them. Default `false` (price-only); orthogonal to family membership
    /// (VWMA/VWAP are still `MovingAverage` for standalone use where volume IS present).
    const NEEDS_VOLUME: bool = false;

    /// The indicator's typed configuration — real types/enums, never a bag.
    type Config;

    /// The runtime struct the factory wraps (usually `Self`).
    type Runtime;

    // NO `defaults()` and NO `config_for_periods()`. The COMPUTE path FORBIDS defaults:
    // a fallback hides the native config complexity, so the order path must demand every
    // param be filled and ERROR on anything missing (that error is the precise spec of the
    // required config). The indicator's standard/cold-start config lives on the RENDER half
    // ([`Render::defaults`]) — render needs an instance at cold start; compute does not.

    /// Construct the runtime from the FULLY-specified typed config. No `Default`, no
    /// `Option` fallback — every field is the caller's responsibility (the demand-fill
    /// order path supplies a complete config or errors before reaching here).
    fn create(cfg: Self::Config) -> Self::Runtime;

    /// The resolved slot members for a concrete config, in the SAME order as `SLOTS`
    /// — the TYPED replacement for the old bag's `ma_types[config_key]` lookup in
    /// the cost model. ONE entry per `SLOTS` instance: an indicator that builds the
    /// same smoother twice (RSI's gain/loss, MACD's three lines) returns one id per
    /// built MA, in `SLOTS` order. Widens the typed family-id config field(s) (e.g.
    /// [`crate::engine::contract_engine::SmootherId`]) to `IndicatorId`
    /// for the generic, family-agnostic cost model. Default empty (slot-free).
    fn slot_members(_cfg: &Self::Config) -> Vec<IndicatorId> {
        Vec::new()
    }

    /// The full ordered list of kline fields this node feeds on — the RUNTIME resolution of
    /// [`Self::SOURCE`]. The factory extracts each field from the sample and feeds the pure
    /// core the scalar(s); the core never sees the bar (the factory handles bars + other
    /// data sources). The DEFAULT reads `const SOURCE`: all-`Fixed` lanes (a `KlineSlice`)
    /// resolve straight from the const, so fixed-field cores need NOT override — no
    /// re-declaration of what `SOURCE` already states. Cores with CONFIGURABLE lanes (a
    /// single `Field` MA, MACD's two fields) override to splice in the config's CHOSEN fields
    /// (the const only carries the default). Up to 4 lanes inline (no heap).
    fn source_fields(_cfg: &Self::Config) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let mut out = arrayvec::ArrayVec::new();
        if let Some(axis) = Self::SOURCE {
            for lane in axis.lanes() {
                let f = match lane {
                    SourceLane::Fixed(f) => f,
                    SourceLane::Config { default } => default,
                };
                let _ = out.try_push(f);
            }
        }
        out
    }
}


pub mod market_sample;
pub use market_sample::MarketSample;

pub mod bar;
pub use bar::ResearchBar;

pub mod timeframe;
pub use timeframe::ResearchTimeframe;

pub mod host_overlay;
pub use host_overlay::{
    DomHeatmapOverlay, HostOverlayConfig, HostOverlayId, LiquidationHeatmapOverlay,
    LiquidationProjectionOverlay, PaintSpace, ProfileSide, ProfileWindow, SignedStripOverlay,
    TpoProfileOverlay, VenueFold, VolumeProfileOverlay,
};

pub mod render;
pub use render::{
    Color, Config, HistogramStyle, LineStyle, OutputType, ReferenceLine, Render,
    RenderOutput, RenderSpec, RenderSpecBuilder,
};

pub mod gpu;
pub use gpu::{shader_of, CubeFormula, CubeParams, CubeSmoother, GpuCube, GpuMode, GpuShader, ShaderSpec};

pub mod gpu_sample;
pub use gpu_sample::{GpuColumns, GpuLevel, GpuSample, GpuTimes};

pub mod event_frame;
pub use event_frame::{GpuEventFrame, EVENT_COLS};

#[cfg(feature = "gpu")]
pub mod kernels;

/// Composite formulas (codes 500..=699): an inner series from an existing formula, then a
/// post stage (rolling rank / z-score / EMA-detrend / …). UNTESTED on GPU.
#[cfg(feature = "gpu")]
pub mod kernels_post;

/// Calendar formulas, codes 700..=709. UNTESTED on GPU.
#[cfg(feature = "gpu")]
pub mod kernels_cal;

/// wgpu dispatcher for hand-written WGSL rows. UNTESTED on GPU.
#[cfg(feature = "gpu-shader")]
pub mod shader_run;

/// Smoother-chain composites, codes 1000..=1099. UNTESTED on GPU.
#[cfg(feature = "gpu")]
pub mod kernels_comp;

/// Spectral family (FFT averaging port). UNTESTED on GPU.
#[cfg(feature = "gpu")]
pub mod kernels_spec;

/// Event-frame formulas, codes 900..=999. UNTESTED on GPU.
#[cfg(feature = "gpu")]
pub mod kernels_ev;

pub mod axis;
pub use axis::{sweep_f64, CubeIter, Param, ParamScalar};

/// The slot-config of a smoother-safe MA: its REAL params (period + shape) WITHOUT a
/// source — a slot is fed a pre-extracted scalar. The source-less dual of the
/// standalone `Indicator::Config`.
pub trait Smoother: Sized {
    type Params: Clone + Copy + core::fmt::Debug + PartialEq;
    fn from_params(p: Self::Params) -> Self;
    fn params_period(p: &Self::Params) -> usize;
}

/// The slot-config of a scalar-source oscillator (RSI / CMO / …): its REAL params
/// WITHOUT a source — fed a pre-extracted scalar (default close). The oscillator twin of
/// [`Smoother`]: the `+oscillator`-flagged members implement it so the macro can generate
/// an `OscillatorSlotOrder` (same machinery as `SmootherSlotOrder`) for composites that
/// embed a config-chosen oscillator inner. Only single-period close-source oscillators
/// (a scalar `feed(f64) -> f64`) are flagged — multi-field oscillators (Stochastic, CCI)
/// cannot be a scalar slot, exactly as heavy MAs cannot be a smoother slot.
pub trait Oscillator: Sized {
    type Params: Clone + Copy + core::fmt::Debug + PartialEq;
    fn from_params(p: Self::Params) -> Self;
    fn params_period(p: &Self::Params) -> usize;
}

/// The machine-generator sweep over a slot member's OWN source-less params — period + any
/// shape knobs (ALMA `offset`/`sigma`). A slot-order's `machine_sweep()` concatenates this
/// over EVERY family member, so each member contributes ITS axes (EMA → period; ALMA →
/// period × offset × sigma) — a disjoint union, never a product across members. Source is
/// absent by construction (a slot is fed a pre-extracted scalar), so no dead `source` axis.
pub trait SlotParamsSweep: Sized {
    /// This member's slot-relevant param cube, source-less. A SLOT period is a sub-component
    /// lookback (a curated moderate set), NOT the host's full sweep range — kept small so a
    /// multi-slot host stays tractable; the host's own period axis is separate.
    fn machine_params() -> Vec<Self>;
}

/// Uniform slot interface for both a bare id (transitional) and a typed order
/// (`SmootherSlotOrder`). Keeps `#[derive(Slots)]` generating consistent `Slot::new` calls
/// regardless of which concrete type the `#[slot]` field holds.
pub trait SlotField {
    const CANDIDATES: &'static [crate::engine::indicator_id::IndicatorId];
    fn member_id(&self) -> crate::engine::indicator_id::IndicatorId;
}

/// WHERE a slot member's period comes from — the two configurable modes:
/// - `Follow`: inherit the HOST indicator's period (the default; standard usage — RSI's
///   gain/loss smoother at RSI's period, Bollinger's centre MA at the std-dev window). The
///   slot contributes NO independent period axis; it rides the host's `period` sweep.
/// - `Own(p)`: the member runs at its OWN period `p`, independent of the host ("sometimes
///   interesting"). Swept via the OUTER `Param<…Choice>::Many` listing several `Own(p)` orders.
///
/// One resolved point is EITHER `Follow` OR `Own` — never both (it is an enum). A sweep MAY list
/// both as alternatives (`Many([k(Follow), k(Own(20))])`) to explore them as separate cube points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotPeriod {
    /// Inherit the host indicator's period (default).
    Follow,
    /// Run the member at its own period, independent of the host.
    Own(usize),
}

impl SlotPeriod {
    /// Resolve to a concrete period given the HOST's period: `Follow` → host, `Own(p)` → `p`.
    pub fn resolve(self, host_period: usize) -> usize {
        match self {
            SlotPeriod::Follow => host_period,
            SlotPeriod::Own(p) => p,
        }
    }
}
