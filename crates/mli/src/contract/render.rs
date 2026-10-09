//! The VISUALIZATION half of the indicator contract — sibling of [`super::Indicator`]
//! (the COMPUTE half). One indicator fills BOTH traits; they coexist on the same
//! struct and do not interfere:
//!
//! - [`super::Indicator`] — the strategy / quant engine consumes the indicator as a
//!   COMPUTATIONAL machine: typed scalar outputs ([`super::Output`], identified by
//!   [`IndicatorOutputId`]) wired into further computation, priced by cost.
//! - [`Render`] — the chart / UI consumes the indicator as vectors / buffers to DRAW:
//!   which typed output channel becomes a line / band / histogram, overlay vs sub-pane,
//!   styles, Y-bounds, reference lines, precision.
//!
//! The two axes are ORTHOGONAL: being renderable says nothing about being
//! computationally consumed, and vice-versa — so they are declared separately, not
//! folded into one `Output`. This trait is the typed, self-declared replacement for
//! the hand-maintained `rendering_catalog` (`get_rendering`).
//!
//! **Fully typed, zero strings inside the system.** Which output a [`RenderOutput`]
//! draws is the typed [`IndicatorOutputId`] — it subsumes BOTH the old stringly output
//! `name` AND the separate `ValueExtractor` (the id alone identifies the value: the
//! runtime value store is keyed by [`IndicatorOutputId`]). Colour is a typed [`Color`]
//! value, not a hex string — a consumer that wants a hex string converts at ITS
//! boundary via [`Color::to_hex`]. The only surviving string is the human display
//! `label`, which IS boundary text (the literal shown to a person).

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;

/// A typed RGB colour. Strings live only at the UI boundary: inside the system a
/// colour is a value type. A consumer that needs a hex string (CSS, a theme file)
/// calls [`Color::to_hex`] at its own edge; a consumer that needs raw channels reads
/// `r`/`g`/`b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// From explicit channels.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// From a packed `0xRRGGBB` literal — the ergonomic declaration form
    /// (`Color::hex(0x2196F3)`).
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: (rgb & 0xFF) as u8,
        }
    }

    /// Boundary conversion: a `#RRGGBB` string for consumers that speak hex.
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// The visual form of an output channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// Single line plot.
    Line,
    /// Histogram bars (volume-style or MACD-style).
    Histogram,
    /// Filled area between two values.
    Band,
    /// Filled area from line to baseline.
    Area,
    /// Dot markers.
    Dots,
    /// Background colour zones.
    Background,
    /// Cloud fill (Ichimoku).
    Cloud,
}

/// Line style for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

/// Histogram rendering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistogramStyle {
    /// Bars grow from bottom (volume).
    #[default]
    FromBottom,
    /// Bars grow from center/zero line (MACD).
    Centered,
    /// Bars grow from top.
    FromTop,
}

/// One drawn output. WHICH output is the typed [`IndicatorOutputId`] — it alone
/// identifies the value (the runtime value store is keyed by it), subsuming the legacy
/// string `name` and the separate `ValueExtractor`. The rest is pure visual style.
#[derive(Debug, Clone)]
pub struct RenderOutput {
    /// The typed output this draws — matches an entry of [`super::Indicator::OUTPUTS`].
    pub id: IndicatorOutputId,
    /// Human display text (legend / tooltip). The one boundary string — it is the
    /// literal shown to a person, not an identity.
    pub label: &'static str,
    /// Visual form.
    pub kind: OutputType,
    /// Default colour (typed).
    pub color: Color,
    /// Default line width.
    pub line_width: f32,
    /// Default line style.
    pub line_style: LineStyle,
    /// Whether this output is drawn by default.
    pub visible: bool,
}

impl RenderOutput {
    const fn base(id: IndicatorOutputId, label: &'static str, kind: OutputType, color: Color, line_width: f32) -> Self {
        Self {
            id,
            label,
            kind,
            color,
            line_width,
            line_style: LineStyle::Solid,
            visible: true,
        }
    }

    /// A line output.
    pub const fn line(id: IndicatorOutputId, label: &'static str, color: Color, line_width: f32) -> Self {
        Self::base(id, label, OutputType::Line, color, line_width)
    }

    /// A histogram output.
    pub const fn histogram(id: IndicatorOutputId, label: &'static str, color: Color) -> Self {
        Self::base(id, label, OutputType::Histogram, color, 1.0)
    }

    /// A band output (channel fill).
    pub const fn band(id: IndicatorOutputId, label: &'static str, color: Color) -> Self {
        Self::base(id, label, OutputType::Band, color, 1.0)
    }

    /// An area output (line-to-baseline fill).
    pub const fn area(id: IndicatorOutputId, label: &'static str, color: Color) -> Self {
        Self::base(id, label, OutputType::Area, color, 1.0)
    }

    /// A dot-marker output.
    pub const fn dots(id: IndicatorOutputId, label: &'static str, color: Color) -> Self {
        Self::base(id, label, OutputType::Dots, color, 1.0)
    }

    /// A cloud output (Ichimoku fill).
    pub const fn cloud(id: IndicatorOutputId, label: &'static str, color: Color) -> Self {
        Self::base(id, label, OutputType::Cloud, color, 1.0)
    }

    /// Override the line style.
    pub const fn with_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Hide by default.
    pub const fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }
}

/// A horizontal reference line (overbought/oversold, zero, …).
#[derive(Debug, Clone)]
pub struct ReferenceLine {
    /// Y-axis value.
    pub value: f64,
    /// Line colour (typed).
    pub color: Color,
    /// Line style.
    pub style: LineStyle,
    /// Optional human label.
    pub label: Option<&'static str>,
}

impl ReferenceLine {
    /// A dashed reference line at `value`.
    pub const fn new(value: f64, color: Color) -> Self {
        Self {
            value,
            color,
            style: LineStyle::Dashed,
            label: None,
        }
    }

    /// Attach a label.
    pub const fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Override the line style.
    pub const fn with_style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }
}

/// The complete typed render spec for an indicator — the return of [`Render::rendering`]
/// and the typed replacement for a `rendering_catalog` entry.
#[derive(Debug, Clone)]
pub struct RenderSpec {
    /// The indicator this spec answers to (typed — matches [`super::Indicator::ID`]).
    pub indicator: IndicatorId,
    /// Draw on the main price chart (`true`) vs a dedicated sub-pane (`false`).
    pub overlay: bool,
    /// The drawn outputs.
    pub outputs: Vec<RenderOutput>,
    /// Fixed Y-axis bounds (e.g. `Some((0.0, 100.0))` for RSI).
    pub bounds: Option<(f64, f64)>,
    /// Extend the Y range to include zero (MACD-style).
    pub zero_baseline: bool,
    /// Histogram rendering style.
    pub histogram_style: HistogramStyle,
    /// Horizontal reference lines.
    pub reference_lines: Vec<ReferenceLine>,
    /// Default sub-pane height ratio (relative to the main pane).
    pub height_ratio: f32,
    /// Value-display precision (decimal places).
    pub precision: u32,
}

impl RenderSpec {
    /// Start building a spec for `indicator` (defaults to a sub-pane).
    pub fn builder(indicator: IndicatorId) -> RenderSpecBuilder {
        RenderSpecBuilder::new(indicator)
    }
}

/// Builder for [`RenderSpec`].
pub struct RenderSpecBuilder {
    spec: RenderSpec,
}

impl RenderSpecBuilder {
    fn new(indicator: IndicatorId) -> Self {
        Self {
            spec: RenderSpec {
                indicator,
                overlay: false,
                outputs: Vec::new(),
                bounds: None,
                zero_baseline: false,
                histogram_style: HistogramStyle::FromBottom,
                reference_lines: Vec::new(),
                height_ratio: 0.15,
                precision: 4,
            },
        }
    }

    /// Draw on the main price chart.
    pub fn overlay(mut self) -> Self {
        self.spec.overlay = true;
        self
    }

    /// Draw in a dedicated sub-pane (the default).
    pub fn sub_pane(mut self) -> Self {
        self.spec.overlay = false;
        self
    }

    /// Add an explicit output.
    pub fn output(mut self, output: RenderOutput) -> Self {
        self.spec.outputs.push(output);
        self
    }

    /// Add a default-width line output (the common single-channel shorthand).
    pub fn line_output(self, id: IndicatorOutputId, label: &'static str, color: Color) -> Self {
        self.output(RenderOutput::line(id, label, color, 2.0))
    }

    /// Fix the Y-axis bounds.
    pub fn bounds(mut self, lo: f64, hi: f64) -> Self {
        self.spec.bounds = Some((lo, hi));
        self
    }

    /// Extend the Y range to include zero.
    pub fn zero_baseline(mut self) -> Self {
        self.spec.zero_baseline = true;
        self
    }

    /// Add a horizontal reference line.
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.spec.reference_lines.push(line);
        self
    }

    /// Set the histogram style.
    pub fn histogram_style(mut self, style: HistogramStyle) -> Self {
        self.spec.histogram_style = style;
        self
    }

    /// Set the value-display precision.
    pub fn precision(mut self, precision: u32) -> Self {
        self.spec.precision = precision;
        self
    }

    /// Set the sub-pane height ratio.
    pub fn height_ratio(mut self, ratio: f32) -> Self {
        self.spec.height_ratio = ratio;
        self
    }

    /// Finish the spec.
    pub fn build(self) -> RenderSpec {
        self.spec
    }
}

/// The CONFIG half of an indicator's contract — the dual-mode SWEEP surface of a typed
/// `Config` struct.
///
/// Impl'd ON THE CONFIG STRUCT (e.g. `RsiConfig`), not the runtime. Every field is a `Param<T>`
/// (`Solo` = one value, `Many` = a swept set), so ONE config type is BOTH a concrete instance
/// (all `Solo`) AND a cube template (some fields `Many`). No concrete-vs-swept twin, no stringly
/// bag.
///
/// `cube_size` / `iter` MEASURE and EXPAND the ranges — they never CAP them (no combinatorics
/// limits, owner law). The generator AST holds the typed config and CALLS these; it never
/// reflects fields (the reflection lives in the `#[derive(ConfigAxes)]` on the config).
///
/// `defaults()` is the cold-start single-value instance (every field `Solo`) — the owner's law,
/// "the default carries the single value." It lives HERE because each indicator owns its own
/// config struct (the self-declaring contract): the per-indicator default IS the config's job,
/// not a separate trait. Config types are NOT shared across indicators with different standard
/// values — a smoother's resolved slot-params payload (concrete, `Copy`) is a different concept
/// and a different type, never reused as an indicator config.
pub trait Config: Sized + Clone {
    /// The cold-start standard instance — every axis `Solo`. The single home for an indicator's
    /// default config; the render path builds an instance from it, the cost projection prices it,
    /// a harness drives every id with it, the consumer overrides later.
    fn defaults() -> Self;

    /// How many resolved configs this (possibly swept) config expands to: the product of every
    /// field's [`Param`] cardinality (`Solo` → 1, `Many(v)` → `v.len()`). An all-`Solo` config → 1.
    fn cube_size(&self) -> u128;

    /// Expand every range into the cartesian product of resolved (all-`Solo`) configs — each a
    /// concrete cube point ready for `Indicator::create`.
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_>;

    /// Override the PRIMARY period axis (the universal lookback) with `period` — the generator's
    /// typed sweep injection: build [`Config::defaults`] (Solo, render), then overlay a swept
    /// `Param::Many`/`range`. The period stays a `Param` FIELD in the config; this only sets it.
    /// Default is a NO-OP (periodless / field-less configs). `#[derive(ConfigAxes)]` emits an
    /// INHERENT `set_primary_period` that sets the first `Param<usize>` field and, by Rust's
    /// inherent-over-trait method resolution, takes precedence over this default.
    fn set_primary_period(&mut self, period: super::Param<usize>) {
        let _ = period;
    }

    /// The resolved PRIMARY period (the first `Param<usize>` field's value) — the
    /// memory-relevant lookback depth a buffer is sized to. `None` for a periodless config.
    /// Default `None`; `#[derive(ConfigAxes)]` emits an inherent override returning the real
    /// field (inherent-over-trait resolution shadows this default for period-bearing configs).
    fn primary_period(&self) -> Option<usize> {
        None
    }

    /// The RAW primary-period axis (the first `Param<usize>` field, `Solo` or a swept `Many`) —
    /// unlike [`Self::primary_period`] (which collapses to one resolved scalar, losing a sweep),
    /// this is the round-trip primitive a re-emitter needs to reconstruct
    /// `with_period(axis)` byte-for-byte, including a swept `Param::range(..)`. `None` for a
    /// periodless config. Default `None`; `#[derive(ConfigAxes)]` emits an inherent override
    /// returning the real field (inherent-over-trait resolution shadows this default).
    fn primary_period_axis(&self) -> Option<super::Param<usize>> {
        None
    }

    /// The MACHINE-GENERATOR sweep instance — the swept twin of [`defaults`](Self::defaults).
    /// Where `defaults()` is all-`Solo` (the render / cold-start single value), `machine_defaults()`
    /// sets EVERY sweepable axis to a `Param::Many` over its MACHINE-DEFAULT RANGE with its
    /// MACHINE-DEFAULT STEP — period `1..=10000` step 1, multiplier `0.1..=10.0` step 0.1, alpha
    /// `0.01..=0.99` step 0.01, ratio `0.0..=1.0` step 0.05, threshold `0.1..=5.0` step 0.1, a
    /// structural count (`num_bins`, `levels`, …) over its OWN bounded range, an enum / `bool` over
    /// all variants — per the sweep taxonomy. Do-NOT-sweep axes (`dt`, `sampling_rate`,
    /// `scale_factor`, `annualize_factor`, instrument-relative `tick_size`/`price_bucket`) stay at
    /// their `Solo` default. The strategy generator drives the sweep cube from THIS; the render path
    /// from `defaults()`. A consumer overrides any axis afterwards — no clamp, no conflict.
    ///
    /// The default body falls back to `defaults()` (no sweep) so an un-filled indicator is never
    /// broken, only un-swept. Each indicator FILLS this. `#[derive(ConfigAxes)]` emits an inherent
    /// `machine_defaults_auto()` helper (type-driven wide `Many` for the `usize`/`bool`/`OhlcvField`
    /// axes) to build the `Many`-twin on, e.g.
    /// `{ let mut s = Self::machine_defaults_auto(); s.std_dev_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); s }`.
    fn machine_defaults() -> Self {
        Self::defaults()
    }

    /// Validate this RESOLVED config's params — the UNIVERSAL per-indicator viability check.
    /// Applied to ONE concrete (all-`Solo`) instance, identically whether it came from
    /// `build_solo` (a directly-ordered solo) or one point of a `build_many` sweep — invalidity
    /// is a property of the PARAMS, not of the sweep. `Ok(())` = viable combo; `Err(reason)` =
    /// reject, where `reason` is a human one-liner (e.g. `"fast(20) >= slow(10)"`).
    ///
    /// Default `Ok(())`: an unconstrained indicator is always valid (mirrors `machine_defaults` —
    /// an un-filled indicator is never broken). A CONSTRAINED indicator FILLS its real rule
    /// (intra-config field relationships only — fast<slow, p1<p2<p3, min<max, α+β<1, num_bins
    /// range, …). Cross-role rules live in `mli-strategies`, not here.
    fn valid_params(&self) -> Result<(), String> {
        Ok(())
    }
}

/// The VISUALIZATION half of an indicator's contract: how the UI draws it. `Render: Indicator`
/// gives a renderer the typed [`super::Indicator::Config`] associated type; the cold-start
/// instance comes from [`Config::defaults`] (the single default home) — orthogonal to render.
pub trait Render: super::Indicator {
    /// The self-declared visualization spec: the typed output channels (which
    /// [`IndicatorOutputId`] draws as a line / band / histogram), overlay vs sub-pane,
    /// Y-bounds, reference lines, precision. The typed replacement for a
    /// `rendering_catalog` entry.
    fn rendering() -> RenderSpec;
}
