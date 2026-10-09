//! `Param<T>` — the one dual-mode config-axis primitive every config field is built from.
//!
//! A field is either `Solo(value)` (one value) or `Many(values)` (a swept set). The SAME type
//! covers numbers, source fields, and slot members — there is no numeric-vs-categorical split.
//! A config of `Param` fields is therefore ONE type that is simultaneously a concrete instance
//! (every field `Solo`) and a cube template (some fields `Many`). The `#[derive(ConfigAxes)]`
//! reads these fields and generates `cube_size`/`iter` from them.
//!
//! Pure data. No serde.

use std::hash::{Hash, Hasher};

/// Hashable + equatable scalar for config dedup. The warmup builds one indicator column per
/// UNIQUE resolved [`IndicatorOrder`], so the whole config tree must be a lossless `HashMap`
/// key. `f64` is not `Hash`/`Eq`, so this trait routes it through `to_bits` while every other
/// scalar (periods, enums, slot orders) routes through its derived `Hash`. NO blanket impl
/// (it would collide with the explicit `f64` impl under coherence) — primitives are listed
/// here, enums/orders get `#[derive(ParamScalar)]` at their definition.
pub trait ParamScalar: Clone + PartialEq {
    fn scalar_hash<H: Hasher>(&self, state: &mut H);
}

macro_rules! param_scalar_via_hash {
    ($($t:ty),* $(,)?) => {
        $( impl ParamScalar for $t {
            fn scalar_hash<H: Hasher>(&self, state: &mut H) { Hash::hash(self, state); }
        } )*
    };
}
param_scalar_via_hash!(usize, u32, u16, i64, bool);

// Fixed-size arrays of `usize` implement `Hash`; expose them as `ParamScalar` so
// `Param<[usize; N]>` (e.g. KST's period arrays, DFA scales) can be config-hashed.
impl ParamScalar for [usize; 4] {
    fn scalar_hash<H: Hasher>(&self, state: &mut H) { Hash::hash(self, state); }
}
impl ParamScalar for [usize; 6] {
    fn scalar_hash<H: Hasher>(&self, state: &mut H) { Hash::hash(self, state); }
}

impl ParamScalar for f64 {
    fn scalar_hash<H: Hasher>(&self, state: &mut H) {
        // Bit pattern → total, deterministic ordering. Config params are never NaN.
        self.to_bits().hash(state);
    }
}

impl<T: ParamScalar> ParamScalar for Vec<T> {
    fn scalar_hash<H: Hasher>(&self, state: &mut H) {
        self.len().hash(state);
        for v in self {
            v.scalar_hash(state);
        }
    }
}

/// A config field: one value (`Solo`) or a swept set (`Many`). Uniform across numbers, fields,
/// and slot members.
#[derive(Debug, Clone, PartialEq)]
pub enum Param<T> {
    /// A single value.
    Solo(T),
    /// A swept set of values (a numeric range expanded, or an explicit list / slot members).
    Many(Vec<T>),
}

// Lossless `Hash`/`Eq` so a resolved order is a first-class warmup dedup key. `Eq` is a marker
// asserted over the existing `PartialEq` (config params are NaN-free); `Hash` routes through
// [`ParamScalar`] so `Param<f64>` hashes by bits.
impl<T: ParamScalar> Hash for Param<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Param::Solo(v) => {
                0u8.hash(state);
                v.scalar_hash(state);
            }
            Param::Many(vs) => {
                1u8.hash(state);
                vs.len().hash(state);
                for v in vs {
                    v.scalar_hash(state);
                }
            }
        }
    }
}

impl<T: PartialEq> Eq for Param<T> {}

// A `Param`-wrapped field is itself a `ParamScalar` (so nested `Param<Param<…>>` and slot
// orders compose), routing through the impl above.
impl<T: ParamScalar> ParamScalar for Param<T> {
    fn scalar_hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(self, state);
    }
}

impl<T: Clone> Param<T> {
    /// Single-value constructor.
    pub fn solo(v: T) -> Self {
        Param::Solo(v)
    }

    /// Multi-value (swept) constructor.
    pub fn many(vs: Vec<T>) -> Self {
        Param::Many(vs)
    }

    /// True when this is a single value.
    pub fn is_solo(&self) -> bool {
        matches!(self, Param::Solo(_))
    }

    /// How many values this axis yields: `Solo` → 1, `Many(v)` → `v.len()`.
    pub fn cardinality(&self) -> u128 {
        match self {
            Param::Solo(_) => 1,
            Param::Many(vs) => vs.len() as u128,
        }
    }

    /// The values this axis ranges over.
    pub fn values(&self) -> Vec<T> {
        match self {
            Param::Solo(v) => vec![v.clone()],
            Param::Many(vs) => vs.clone(),
        }
    }

    /// The single resolved value — the `Solo` value (`Many`'s first as a fallback). `create` is
    /// only ever handed an all-`Solo` config, so this is the concrete build value.
    pub fn resolved(&self) -> T {
        match self {
            Param::Solo(v) => v.clone(),
            Param::Many(vs) => vs[0].clone(),
        }
    }

    /// The `k`-th swept value of this axis: `Solo` ignores `k` and yields its single value;
    /// `Many` indexes directly. The O(1), allocation-free counterpart of [`values`](Self::values)
    /// — the cube-decode primitive (mixed-radix per-axis access WITHOUT materializing the set).
    /// `k` must be `< cardinality()` for a `Many` axis (the cube decode guarantees this).
    pub fn value_at(&self, k: usize) -> T {
        match self {
            Param::Solo(v) => v.clone(),
            Param::Many(vs) => vs[k].clone(),
        }
    }

    /// Reduce this axis to at most `k` values, ALWAYS materializing an explicit
    /// [`Param::Many`] (never a lazy view — the opt-in resolution allocator's only
    /// primitive; see `docs/mlq/plans/cost-layer1-closure-plan-2026-07-02.md` step C7b).
    ///
    /// - `Solo` is already a single point — returned unchanged (`k` is irrelevant).
    /// - `k >= cardinality()` — the axis already fits the budget; returned unchanged
    ///   (identity copy, no resampling).
    /// - `k <= 1` — degrades to a single representative value: the GEOMETRIC MIDPOINT
    ///   index (`(n-1)/2`, integer-rounded), materialized as a one-element `Many`. This
    ///   is the documented single-point rule (favors the axis's center over its first
    ///   or last value).
    /// - otherwise — `k` values picked at GEOMETRICALLY spaced INDICES into the
    ///   existing (already-ordered) value set: `idx_j = round((n-1) * (j/(k-1))^2)` for
    ///   `j = 0..k`, deduped while preserving order. Geometric index-spacing densifies
    ///   near the low end and thins near the high end — for the common case of a
    ///   `Param::range`-built ascending numeric axis (values = `lo + i*step`, an
    ///   arithmetic progression of the index) this reproduces genuine geometric spacing
    ///   in VALUE space too, without requiring numeric arithmetic on the generic `T`.
    ///   The two endpoints (first and last value) are always preserved. Dedup can leave
    ///   fewer than `k` values when the axis is small relative to `k` (never more).
    #[must_use]
    pub fn subsampled(&self, k: usize) -> Param<T> {
        let Param::Many(vs) = self else {
            // Solo: already a single point, k is irrelevant.
            return self.clone();
        };
        let n = vs.len();
        if k >= n {
            // Already fits the budget — identity copy, no resampling.
            return self.clone();
        }
        if k <= 1 {
            // Single representative point: the geometric midpoint index.
            let mid = (n - 1) / 2;
            return Param::Many(vec![vs[mid].clone()]);
        }

        // Geometric index spacing over `[0, n-1]`, `k` points, endpoints preserved,
        // dedup while preserving ascending order. `idx_j = round(n^t) - 1`, `t = j/(k-1)`
        // — the discrete index-space analog of the value-space geometric formula
        // `value_k = lo * (hi/lo)^(k/(points-1))` (exact when index maps linearly to
        // value, e.g. a `Param::range`-built ascending numeric axis).
        let n_f = n as f64;
        let mut indices: Vec<usize> = (0..k)
            .map(|j| {
                let t = j as f64 / (k - 1) as f64; // 0.0 ..= 1.0
                let idx = n_f.powf(t).round() as i64 - 1;
                idx.clamp(0, (n - 1) as i64) as usize
            })
            .collect();
        indices.dedup();

        Param::Many(indices.into_iter().map(|i| vs[i].clone()).collect())
    }
}

/// A LAZY cube enumerator with **O(1) `nth`** — the streaming replacement for materializing a
/// cartesian product into a `Vec`. `next` decodes the current index, `nth(n)` jumps `n` indices
/// then decodes once (the random-access the eager `Vec` had for free, but at O(1) memory instead
/// of O(cube)). Built over a decode closure so ONE type serves every config's axes_iter. A wide
/// machine-default cube (e.g. MACD `9998³`) streams point-by-point instead of OOMing on expansion.
pub struct CubeIter<T, F: Fn(u128) -> T> {
    pos: u128,
    total: u128,
    decode: F,
}

impl<T, F: Fn(u128) -> T> CubeIter<T, F> {
    /// `total` = cube cardinality; `decode(i)` yields the `i`-th point for `i < total`.
    pub fn new(total: u128, decode: F) -> Self {
        Self { pos: 0, total, decode }
    }
}

impl<T, F: Fn(u128) -> T> Iterator for CubeIter<T, F> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.pos < self.total {
            let v = (self.decode)(self.pos);
            self.pos += 1;
            Some(v)
        } else {
            None
        }
    }

    /// O(1): jump `n` indices, then decode the next — no per-element walk (unlike the default
    /// `Iterator::nth`). This is the property the warmup cube decoder relies on.
    fn nth(&mut self, n: usize) -> Option<T> {
        self.pos = self.pos.saturating_add(n as u128);
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.total.saturating_sub(self.pos);
        let r = rem.min(usize::MAX as u128) as usize;
        (r, Some(r))
    }
}

impl Param<usize> {
    /// A numeric range expanded into a swept set: `[min, min+step, … ≤ max]`. A degenerate
    /// range (`min == max`) collapses to `Solo`. (Defined only for `usize` — the common numeric
    /// sweep is a period; f64 sweeps use `Param::many` with an explicit set, keeping `range`
    /// unambiguous across `Param<usize>` / `Param<f64>`.)
    pub fn range(min: usize, max: usize, step: usize) -> Self {
        if min >= max {
            Param::Solo(min)
        } else {
            Param::Many((min..=max).step_by(step.max(1)).collect())
        }
    }
}

/// Expand an `f64` range `[min, min+step, … ≤ max]` into a swept `Vec<f64>` — the `f64` analog of
/// [`Param::range`] (which is `usize`-only, since the common numeric sweep is a period). Use as
/// `Param::many(sweep_f64(0.1, 10.0, 0.1))`. The step is SEMANTIC per axis (multiplier `0.1`, alpha
/// `0.01`, ratio `0.05` — see the sweep taxonomy) and is ALWAYS passed explicitly, never derived
/// from the type. A degenerate range (`min >= max`, or a non-positive / NaN step) yields the single
/// value `[min]`. Values are computed as `min + step*i` (not accumulated) to avoid float drift.
#[must_use]
pub fn sweep_f64(min: f64, max: f64, step: f64) -> Vec<f64> {
    if !(step > 0.0) || min >= max {
        return vec![min];
    }
    let n = ((max - min) / step).floor() as usize;
    (0..=n).map(|i| min + step * i as f64).collect()
}

// A `#[slot]` field is just a `Param<…SlotOrder>` — its admissible members are the choice set.
// This blanket impl lets `#[derive(Slots)]` (which expects a `SlotField`) keep working on a
// `Param`-wrapped slot: the candidate set is the inner order's, the resolved member is the pick.
impl<T: crate::contract::SlotField + Clone> crate::contract::SlotField for Param<T> {
    const CANDIDATES: &'static [crate::engine::indicator_id::IndicatorId] = T::CANDIDATES;
    fn member_id(&self) -> crate::engine::indicator_id::IndicatorId {
        self.resolved().member_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Solo` is unaffected by `subsampled` regardless of `k`.
    #[test]
    fn subsampled_solo_is_identity() {
        let p = Param::Solo(42usize);
        assert_eq!(p.subsampled(0), p);
        assert_eq!(p.subsampled(1), p);
        assert_eq!(p.subsampled(100), p);
    }

    /// `k >= cardinality()` is a no-op — identity copy, no resampling.
    #[test]
    fn subsampled_k_at_least_cardinality_is_identity() {
        let p = Param::range(1, 10, 1); // cardinality 10
        assert_eq!(p.cardinality(), 10);
        assert_eq!(p.subsampled(10), p);
        assert_eq!(p.subsampled(11), p);
        assert_eq!(p.subsampled(1000), p);
    }

    /// `k == 1` degrades to a single explicit value: the geometric midpoint index.
    #[test]
    fn subsampled_k_one_is_single_midpoint_value() {
        let p = Param::range(0, 9, 1); // values 0..=9, cardinality 10, mid index (10-1)/2 = 4
        let s = p.subsampled(1);
        assert_eq!(s.cardinality(), 1);
        assert_eq!(s.values(), vec![4]);
    }

    /// `k == 0` degrades identically to `k == 1` (never an empty `Many`).
    #[test]
    fn subsampled_k_zero_degrades_like_k_one() {
        let p = Param::range(0, 9, 1);
        assert_eq!(p.subsampled(0), p.subsampled(1));
        assert_eq!(p.subsampled(0).cardinality(), 1);
    }

    /// `subsampled(k)` on a `Many` axis of cardinality 100 with `k=10` yields exactly 10
    /// distinct, deduped values, endpoints preserved.
    #[test]
    fn subsampled_wide_axis_yields_k_distinct_values_with_endpoints() {
        let p = Param::range(1, 100, 1); // cardinality 100
        assert_eq!(p.cardinality(), 100);
        let s = p.subsampled(10);
        let vals = s.values();
        assert_eq!(vals.len(), 10, "must yield exactly k=10 distinct values (no collisions at this width)");
        assert_eq!(*vals.first().unwrap(), 1, "first value of the original range must be preserved");
        assert_eq!(*vals.last().unwrap(), 100, "last value of the original range must be preserved");
        // Strictly ascending — no duplicate collapse, no reordering.
        assert!(vals.windows(2).all(|w| w[0] < w[1]));
    }

    /// Geometric index-spacing skews density toward the LOW end of the range: for a wide
    /// `2..10000`-style axis reduced to 10 points, more points land below 100 than above 5000
    /// (linear spacing would be exactly the opposite — evenly spread, no skew).
    #[test]
    fn subsampled_geometric_skew_favors_low_end() {
        let p = Param::range(2, 10000, 1);
        let s = p.subsampled(10);
        let vals = s.values();
        let below_100 = vals.iter().filter(|&&v| v < 100).count();
        let above_5000 = vals.iter().filter(|&&v| v > 5000).count();
        assert!(
            below_100 > above_5000,
            "geometric spacing must skew low-end density higher: below_100={below_100} above_5000={above_5000} vals={vals:?}"
        );
        // endpoints preserved
        assert_eq!(*vals.first().unwrap(), 2);
        assert_eq!(*vals.last().unwrap(), 10000);
    }

    /// A small axis (cardinality 5) asked for k=3 dedups to <= 3 distinct values, never more,
    /// never empty, and never panics on out-of-range indices.
    #[test]
    fn subsampled_small_axis_never_overshoots() {
        let p = Param::range(1, 5, 1); // cardinality 5
        let s = p.subsampled(3);
        assert!(s.cardinality() <= 3 && s.cardinality() >= 1);
    }

    /// `subsampled` never panics on a boundary cardinality-2 axis (the smallest non-Solo Many).
    /// Mid index for n=2 is `(2-1)/2 = 0` (integer division) — the FIRST value.
    #[test]
    fn subsampled_cardinality_two_axis() {
        let p = Param::many(vec![10usize, 20]);
        let s = p.subsampled(1);
        assert_eq!(s.values(), vec![10]);
    }
}
