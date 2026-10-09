//! Runtime storage for a MATRIX/TABLE output — the M×N sibling of the scalar value
//! store keyed by [`IndicatorOutputId`](crate::engine::contract_engine::IndicatorOutputId).
//!
//! A scalar output is ONE `f64` read by id. Some indicators instead emit a whole TABLE
//! whose arity is config-derived (a cross detector crossing M left lines × N right lines
//! emits M×N cells; a level ladder emits a column of price levels; a correlation indicator
//! emits a K×K grid). The cells are NOT folded into the flat
//! [`IndicatorOutputId`](crate::engine::contract_engine::IndicatorOutputId) enum (that would
//! bloat it by the M×N product — 20×20 = 400 variants for one detector). Instead a table is ONE
//! `IndicatorOutputId` of [`OutputShape::Matrix`](crate::contract::OutputShape) (declared with
//! `#name` in the manifest brace + [`Output::matrix`](crate::contract::Output::matrix)), stored
//! at runtime in this `MatrixGrid` — owned by the indicator's runtime struct, sized at `create`
//! from the config, read whole via the generated
//! [`ContractFactory::grid`](crate::engine::contract_engine::ContractFactory::grid).
//!
//! The grid is SELF-DESCRIBING: alongside the numbers it carries per-axis [`AxisLabels`] so a
//! consumer can ask [`MatrixGrid::row_label`] / [`MatrixGrid::col_label`] "what is row i / col j"
//! and get a typed [`Label`] (a name, a producer output, a valued coordinate, an ordinal index,
//! or a bar) — no external per-indicator interpreter. The producer attaches these labels at
//! `create` (and updates moving axes per bar via [`MatrixGrid::set_row_values`]).

use crate::engine::contract_engine::IndicatorOutputId;

/// The resolved meaning of ONE axis index — the answer to "what is row i / col j". Typed so a
/// consumer never hardcodes "column 8 means …": it reads the label off the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Label {
    /// A fixed name — a pivot rung (`"S1"`), a channel band (`"upper"`), a side (`"buy"`).
    Name(&'static str),
    /// A producer's named output line (a cross axis element, e.g. the `EMA9` Price output).
    Output(IndicatorOutputId),
    /// A runtime numeric coordinate — a price-bucket's price, an FFT bin's frequency, a scale.
    Value(f64),
    /// A pure positional index (a correlation component `k`, an anonymous rung) — meaning is the
    /// axis's role tag in its [`AxisLabels::Ordinal`].
    Index(u16),
    /// The bar/time accumulation axis — column `j` is bar `j` (the axis a consumer stacks over).
    Bar(u16),
}

/// The resolved per-index labels for ONE axis of a [`MatrixGrid`] — what each row/col MEANS.
/// Set at `create` from the producer's config; [`AxisLabels::Values`] axes are refreshed per
/// bar for moving coordinates (a profile whose row prices shift with the POC).
#[derive(Debug, Clone)]
pub enum AxisLabels {
    /// Statically-known names, one per index (borrowed from the declaration's `&'static` slice).
    Named(&'static [&'static str]),
    /// Producer output ids, one per index — resolved once at `create` (cross axes).
    Outputs(Vec<IndicatorOutputId>),
    /// Runtime numeric coordinates, one per index — refreshed per bar for moving axes.
    Values(Vec<f64>),
    /// A pure ordinal axis with a static role tag (e.g. `"component"`, `"rung"`).
    Ordinal(&'static str),
    /// The bar/time accumulation axis.
    Bars,
}

impl AxisLabels {
    /// The [`Label`] at index `i`, falling back to [`Label::Index`] when the index is past the
    /// resolved labels (a not-yet-populated dynamic axis).
    pub fn label(&self, i: u16) -> Label {
        match self {
            AxisLabels::Named(names) => names
                .get(i as usize)
                .map_or(Label::Index(i), |s| Label::Name(s)),
            AxisLabels::Outputs(ids) => ids
                .get(i as usize)
                .map_or(Label::Index(i), |id| Label::Output(*id)),
            AxisLabels::Values(vals) => vals
                .get(i as usize)
                .map_or(Label::Index(i), |v| Label::Value(*v)),
            AxisLabels::Ordinal(_) => Label::Index(i),
            AxisLabels::Bars => Label::Bar(i),
        }
    }
}

/// A compact `(row, col)` cell address. `u16` each (max 65535 rows/cols) keeps the address
/// inline and cheap to pass by value — a matrix that needs more than 65k of either axis is
/// not a per-bar indicator output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatrixCell {
    pub row: u16,
    pub col: u16,
}

impl MatrixCell {
    /// A cell at `(row, col)`.
    pub const fn new(row: u16, col: u16) -> Self {
        Self { row, col }
    }

    /// The flat index of this cell into a row-major buffer of width `cols`.
    pub const fn flat(self, cols: u16) -> usize {
        self.row as usize * cols as usize + self.col as usize
    }
}

/// Runtime storage for ONE matrix output: a heap-allocated row-major grid of `direction`
/// scalars, optionally paired with a `timing` grid (bars since the last event in each cell).
///
/// Owned by the producing indicator's runtime struct; allocated once at `create` (dimensions
/// come from the config), never per bar. A cross detector writes `direction` ∈ {+1, -1, 0}
/// (row crossed above / below / no-cross) and `timing` (0 on the bar of the cross, ageing by
/// one each subsequent bar). A 1-D vector output is the degenerate `cols == 1` grid.
///
/// `direction` and `timing` are kept as two parallel `Box<[f64]>` (struct-of-arrays) so a
/// consumer that wants only one channel reads it contiguously without decoding a packed cell.
#[derive(Debug, Clone)]
pub struct MatrixGrid {
    rows: u16,
    cols: u16,
    /// Row-major direction scalars (`+1.0` / `-1.0` / `0.0`). Length `rows * cols`.
    direction: Box<[f64]>,
    /// Optional row-major timing scalars (bars since the last event). Same length as
    /// `direction`; `None` when the producer built the grid with `has_timing == false`.
    timing: Option<Box<[f64]>>,
    /// What each ROW index means (the producer resolves it at `create` — see [`AxisLabels`]).
    row_labels: AxisLabels,
    /// What each COL index means.
    col_labels: AxisLabels,
}

impl MatrixGrid {
    /// Allocate a zeroed `rows × cols` grid; `has_timing` adds the parallel timing layer.
    ///
    /// Labels default to an ordinal row axis and a bar/time column axis — the common Nx1
    /// per-bar vector shape. A producer that knows its axes attaches real labels via
    /// [`Self::with_labels`] (in `new`) and refreshes moving axes via [`Self::set_row_values`].
    pub fn new(rows: u16, cols: u16, has_timing: bool) -> Self {
        let n = rows as usize * cols as usize;
        Self {
            rows,
            cols,
            direction: vec![0.0_f64; n].into_boxed_slice(),
            timing: if has_timing {
                Some(vec![0.0_f64; n].into_boxed_slice())
            } else {
                None
            },
            row_labels: AxisLabels::Ordinal("row"),
            col_labels: AxisLabels::Bars,
        }
    }

    /// Builder: attach the resolved row/col axis labels. Call in the producer's constructor
    /// once the axes are known (e.g. `MatrixGrid::new(9, 1, false).with_labels(AxisLabels::Named(CAMARILLA_RUNGS), AxisLabels::Bars)`).
    pub fn with_labels(mut self, row_labels: AxisLabels, col_labels: AxisLabels) -> Self {
        self.row_labels = row_labels;
        self.col_labels = col_labels;
        self
    }

    /// Replace the row labels in place (e.g. once a producer's operand outputs are resolved).
    pub fn set_row_labels(&mut self, labels: AxisLabels) {
        self.row_labels = labels;
    }

    /// Replace the col labels in place.
    pub fn set_col_labels(&mut self, labels: AxisLabels) {
        self.col_labels = labels;
    }

    /// Refresh the per-index numeric coordinates of a [`AxisLabels::Values`] ROW axis — for a
    /// moving axis (a volume profile whose row prices shift with the POC each bar). Reuses the
    /// existing buffer; promotes the axis to `Values` if it was something else.
    pub fn set_row_values(&mut self, values: &[f64]) {
        match &mut self.row_labels {
            AxisLabels::Values(v) => {
                v.clear();
                v.extend_from_slice(values);
            }
            _ => self.row_labels = AxisLabels::Values(values.to_vec()),
        }
    }

    /// The label for row `i` — "what is this row?".
    pub fn row_label(&self, i: u16) -> Label {
        self.row_labels.label(i)
    }

    /// The label for col `j` — "what is this column?".
    pub fn col_label(&self, j: u16) -> Label {
        self.col_labels.label(j)
    }

    /// The whole row-axis label set.
    pub fn row_labels(&self) -> &AxisLabels {
        &self.row_labels
    }

    /// The whole col-axis label set.
    pub fn col_labels(&self) -> &AxisLabels {
        &self.col_labels
    }

    /// The row count (the producing indicator's M axis).
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// The column count (the producing indicator's N axis).
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// `true` if this grid carries a timing layer.
    pub fn has_timing(&self) -> bool {
        self.timing.is_some()
    }

    /// The direction scalar at `cell`, or `NaN` if out of bounds (a foreign read, mirroring
    /// the scalar `read` convention for foreign ids — out-of-bounds is a contract violation,
    /// not a panic).
    pub fn read_direction(&self, cell: MatrixCell) -> f64 {
        if cell.row >= self.rows || cell.col >= self.cols {
            return f64::NAN;
        }
        self.direction[cell.flat(self.cols)]
    }

    /// The timing scalar at `cell`, or `NaN` if there is no timing layer or `cell` is out of
    /// bounds.
    pub fn read_timing(&self, cell: MatrixCell) -> f64 {
        match &self.timing {
            Some(t) if cell.row < self.rows && cell.col < self.cols => t[cell.flat(self.cols)],
            _ => f64::NAN,
        }
    }

    /// The full row-major direction layer — the chart reads this to draw the whole grid.
    pub fn direction_slice(&self) -> &[f64] {
        &self.direction
    }

    /// The full row-major timing layer, or `None` when there is no timing.
    pub fn timing_slice(&self) -> Option<&[f64]> {
        self.timing.as_deref()
    }

    /// Write `direction` (and `timing`, when present) at `cell`. Out-of-bounds is ignored.
    pub fn set(&mut self, cell: MatrixCell, direction: f64, timing: f64) {
        if cell.row >= self.rows || cell.col >= self.cols {
            return;
        }
        let idx = cell.flat(self.cols);
        self.direction[idx] = direction;
        if let Some(t) = &mut self.timing {
            t[idx] = timing;
        }
    }

    /// Write only the `direction` at `cell`, leaving the timing layer as-is (the caller has
    /// already aged it via [`Self::tick_timing`]). For a no-event cell that still needs its
    /// direction refreshed (a momentary `0`, or a held sticky sign) without resetting its age.
    pub fn set_direction(&mut self, cell: MatrixCell, direction: f64) {
        if cell.row >= self.rows || cell.col >= self.cols {
            return;
        }
        self.direction[cell.flat(self.cols)] = direction;
    }

    /// Age every timing cell by one bar. Call at the START of each bar, before writing the
    /// bar's new events — a cell that gets a fresh event is then re-set to `0.0`. No-op when
    /// there is no timing layer.
    pub fn tick_timing(&mut self) {
        if let Some(t) = &mut self.timing {
            for v in t.iter_mut() {
                *v += 1.0;
            }
        }
    }

    /// Zero both layers — the cold-start reset the indicator's `reset` calls.
    pub fn reset(&mut self) {
        for v in self.direction.iter_mut() {
            *v = 0.0;
        }
        if let Some(t) = &mut self.timing {
            for v in t.iter_mut() {
                *v = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_index_is_row_major() {
        assert_eq!(MatrixCell::new(0, 0).flat(5), 0);
        assert_eq!(MatrixCell::new(1, 2).flat(5), 7);
        assert_eq!(MatrixCell::new(2, 4).flat(5), 14);
    }

    #[test]
    fn new_grid_is_zeroed() {
        let g = MatrixGrid::new(3, 3, true);
        for r in 0..3 {
            for c in 0..3 {
                let cell = MatrixCell::new(r, c);
                assert_eq!(g.read_direction(cell), 0.0);
                assert_eq!(g.read_timing(cell), 0.0);
            }
        }
    }

    #[test]
    fn set_and_read_roundtrip() {
        let mut g = MatrixGrid::new(2, 3, true);
        g.set(MatrixCell::new(1, 2), -1.0, 0.0);
        assert_eq!(g.read_direction(MatrixCell::new(1, 2)), -1.0);
        assert_eq!(g.read_direction(MatrixCell::new(0, 0)), 0.0);
    }

    #[test]
    fn tick_ages_all_timing_cells() {
        let mut g = MatrixGrid::new(2, 2, true);
        g.tick_timing();
        g.tick_timing();
        assert_eq!(g.read_timing(MatrixCell::new(0, 0)), 2.0);
        // A fresh event resets that cell's age to zero.
        g.set(MatrixCell::new(0, 0), 1.0, 0.0);
        assert_eq!(g.read_timing(MatrixCell::new(0, 0)), 0.0);
        assert_eq!(g.read_timing(MatrixCell::new(1, 1)), 2.0);
    }

    #[test]
    fn out_of_bounds_reads_nan() {
        let g = MatrixGrid::new(3, 3, true);
        assert!(g.read_direction(MatrixCell::new(99, 0)).is_nan());
        assert!(g.read_timing(MatrixCell::new(0, 99)).is_nan());
    }

    #[test]
    fn no_timing_layer_reads_nan() {
        let g = MatrixGrid::new(3, 3, false);
        assert!(!g.has_timing());
        assert!(g.read_timing(MatrixCell::new(0, 0)).is_nan());
        assert!(g.timing_slice().is_none());
    }

    #[test]
    fn reset_zeroes_both_layers() {
        let mut g = MatrixGrid::new(2, 2, true);
        g.tick_timing();
        g.set(MatrixCell::new(0, 1), 1.0, 0.0);
        g.reset();
        assert_eq!(g.read_direction(MatrixCell::new(0, 1)), 0.0);
        assert_eq!(g.read_timing(MatrixCell::new(0, 0)), 0.0);
    }

    #[test]
    fn vector_is_degenerate_single_column_grid() {
        let mut g = MatrixGrid::new(4, 1, false);
        g.set(MatrixCell::new(2, 0), 42.0, 0.0);
        assert_eq!(g.read_direction(MatrixCell::new(2, 0)), 42.0);
        assert_eq!(g.cols(), 1);
        assert_eq!(g.direction_slice().len(), 4);
    }
}
