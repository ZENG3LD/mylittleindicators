//! Rolling Volume Profile — POC, VAH, VAL over a sliding window.
//!
//! Builds a price-bucket histogram over the last `rolling_window` bars and
//! computes:
//!
//! - **POC** (Point of Control): price bucket with the highest cumulative volume.
//! - **VAH** (Value Area High): upper boundary of the Value Area.
//! - **VAL** (Value Area Low): lower boundary of the Value Area.
//!
//! The Value Area contains approximately 70 % of total volume (configurable
//! via `value_area_pct`). The algorithm expands from POC outward, adding
//! the bucket with the largest volume on each side alternately until the
//! target percentage is reached.
//!
//! **Bucket size** is `price_bucket_size` (absolute price units). Use a
//! value matched to the instrument's tick size (e.g. `0.01` for crypto at
//! 2-decimal precision, `1.0` for index futures, etc.).
//!
//! Output: `Triple(poc, vah, val)`.

use std::collections::{HashMap, VecDeque};

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Rolling Volume Profile indicator.
///
/// Returns `Triple(poc, vah, val)` once `rolling_window` bars have been fed.
/// Returns `Single(close)` during warm-up.
#[derive(Debug, Clone)]
pub struct RollingVolumeProfile {
    rolling_window: usize,
    price_bucket_size: f64,
    value_area_pct: f64,
    bars: VecDeque<(f64, f64)>, // (typical_price, volume)
    last_poc: f64,
    last_vah: f64,
    last_val: f64,
    ready: bool,
}

impl RollingVolumeProfile {
    /// Create a new `RollingVolumeProfile`.
    ///
    /// - `rolling_window`   — number of bars in the sliding window (≥ 2).
    /// - `price_bucket_size`— absolute bucket width in price units (> 0).
    /// - `value_area_pct`   — fraction of total volume defining the value area (clamped 0.01–0.99).
    pub fn new(rolling_window: usize, price_bucket_size: f64, value_area_pct: f64) -> Self {
        let w = rolling_window.max(2);
        Self {
            rolling_window: w,
            price_bucket_size: price_bucket_size.max(1e-9),
            value_area_pct: value_area_pct.clamp(0.01, 0.99),
            bars: VecDeque::with_capacity(w + 1),
            last_poc: 0.0,
            last_vah: 0.0,
            last_val: 0.0,
            ready: false,
        }
    }

    /// Feed one OHLCV bar and return `Triple(poc, vah, val)` once ready,
    /// otherwise `Single(close)` during warm-up.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> IndicatorValue {
        let typical = (high + low + close) / 3.0;
        self.bars.push_back((typical, volume));
        if self.bars.len() > self.rolling_window {
            self.bars.pop_front();
        }

        if self.bars.len() < self.rolling_window {
            return IndicatorValue::Single(close);
        }

        self.compute_profile();
        self.ready = true;
        IndicatorValue::Triple(self.last_poc, self.last_vah, self.last_val)
    }

    /// Recompute POC/VAH/VAL from current `bars`.
    fn compute_profile(&mut self) {
        let bs = self.price_bucket_size;

        // Build bucket map.
        let mut buckets: HashMap<i64, f64> = HashMap::new();
        for &(price, vol) in &self.bars {
            if vol > 0.0 {
                let key = (price / bs).floor() as i64;
                *buckets.entry(key).or_insert(0.0) += vol;
            }
        }

        if buckets.is_empty() {
            return;
        }

        // POC = bucket with max volume.
        let poc_key = buckets
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&k, _)| k)
            .unwrap_or(0);

        let poc_price = (poc_key as f64 + 0.5) * bs;
        self.last_poc = poc_price;

        // Value Area: expand from POC outward until value_area_pct of total volume included.
        let total_vol: f64 = buckets.values().sum();
        let target = total_vol * self.value_area_pct;

        let mut sorted_keys: Vec<i64> = buckets.keys().copied().collect();
        sorted_keys.sort_unstable();

        let poc_pos = sorted_keys.partition_point(|&k| k < poc_key);

        let mut included_vol = buckets[&poc_key];
        let mut lo_idx = poc_pos; // exclusive lower pointer (moves left)
        let mut hi_idx = poc_pos; // exclusive upper pointer (moves right)

        // Expand greedily: at each step pick the side with higher next volume.
        loop {
            if included_vol >= target {
                break;
            }

            let lo_vol = if lo_idx > 0 {
                buckets[&sorted_keys[lo_idx - 1]]
            } else {
                f64::NEG_INFINITY
            };
            let hi_vol = if hi_idx + 1 < sorted_keys.len() {
                buckets[&sorted_keys[hi_idx + 1]]
            } else {
                f64::NEG_INFINITY
            };

            match (lo_vol > f64::NEG_INFINITY, hi_vol > f64::NEG_INFINITY) {
                (false, false) => break,
                (true, false) => {
                    included_vol += lo_vol;
                    lo_idx -= 1;
                }
                (false, true) => {
                    included_vol += hi_vol;
                    hi_idx += 1;
                }
                (true, true) => {
                    if lo_vol >= hi_vol {
                        included_vol += lo_vol;
                        lo_idx -= 1;
                    } else {
                        included_vol += hi_vol;
                        hi_idx += 1;
                    }
                }
            }
        }

        let val_key = sorted_keys[lo_idx];
        let vah_key = sorted_keys[hi_idx];
        self.last_val = val_key as f64 * bs;
        self.last_vah = (vah_key as f64 + 1.0) * bs;
    }

    /// Returns the last computed `Triple(poc, vah, val)` without advancing state.
    pub fn value(&self) -> IndicatorValue {
        if self.ready {
            IndicatorValue::Triple(self.last_poc, self.last_vah, self.last_val)
        } else {
            IndicatorValue::Single(0.0)
        }
    }

    /// Returns `true` once `rolling_window` bars have been fed.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Clears all state.
    pub fn reset(&mut self) {
        self.bars.clear();
        self.last_poc = 0.0;
        self.last_vah = 0.0;
        self.last_val = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(ind: &mut RollingVolumeProfile, price: f64, vol: f64) -> IndicatorValue {
        ind.update_bar(price, price + 0.5, price - 0.5, price, vol)
    }

    #[test]
    fn warmup_returns_single() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        for i in 0..4u32 {
            let r = feed(&mut rvp, 100.0 + i as f64, 100.0);
            assert!(
                matches!(r, IndicatorValue::Single(_)),
                "expected Single during warmup; got {:?}",
                r
            );
        }
    }

    #[test]
    fn after_warmup_returns_triple() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        let mut last = IndicatorValue::Single(0.0);
        for i in 0..5u32 {
            last = feed(&mut rvp, 100.0 + i as f64, 100.0);
        }
        assert!(
            matches!(last, IndicatorValue::Triple(_, _, _)),
            "expected Triple after warmup; got {:?}",
            last
        );
    }

    #[test]
    fn poc_is_highest_volume_bucket() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        // Feed 5 bars: price 100 with vol 1000 (dominant), others with vol 100.
        feed(&mut rvp, 100.0, 1000.0);
        feed(&mut rvp, 101.0, 100.0);
        feed(&mut rvp, 102.0, 100.0);
        feed(&mut rvp, 103.0, 100.0);
        let r = feed(&mut rvp, 104.0, 100.0);
        match r {
            IndicatorValue::Triple(poc, _vah, _val) => {
                // POC should be near 100 (the dominant bucket centre).
                assert!(
                    (poc - 100.5).abs() < 1.0,
                    "expected POC near 100.5, got {poc}"
                );
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn vah_above_val() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        for i in 0..5u32 {
            feed(&mut rvp, 100.0 + i as f64, 100.0 + i as f64 * 10.0);
        }
        match rvp.value() {
            IndicatorValue::Triple(poc, vah, val) => {
                assert!(vah >= poc, "VAH {vah} must be >= POC {poc}");
                assert!(poc >= val, "POC {poc} must be >= VAL {val}");
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut rvp = RollingVolumeProfile::new(3, 1.0, 0.7);
        for i in 0..3u32 {
            feed(&mut rvp, 100.0 + i as f64, 100.0);
        }
        assert!(rvp.is_ready());
        rvp.reset();
        assert!(!rvp.is_ready());
        let r = feed(&mut rvp, 100.0, 100.0);
        assert!(
            matches!(r, IndicatorValue::Single(_)),
            "expected Single after reset; got {:?}",
            r
        );
    }
}
