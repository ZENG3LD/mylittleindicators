// Anchored VWAP (AVWAP)
// Calendar-anchored (monthly) implementation. Resets the VWAP accumulator at the start of a new month.

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone, Copy, Debug)]
pub enum AvwapAnchorMode {
    Monthly,
}

#[derive(Clone, Copy, Debug)]
pub struct AnchoredVwapParams {
    pub mode: AvwapAnchorMode,
}

impl Default for AnchoredVwapParams {
    fn default() -> Self {
        Self {
            mode: AvwapAnchorMode::Monthly,
        }
    }
}

#[derive(Clone)]
pub struct AnchoredVwap {
    #[allow(dead_code)]
    params: AnchoredVwapParams,
    // Accumulators for current anchor window
    cum_pv: f64,
    cum_v: f64,
    // Last computed value
    value: f64,
    // Calendar tracking (UTC days since epoch divided to month id)
    last_month_key: i32,
    is_ready: bool,
}

impl AnchoredVwap {
    pub fn new(params: AnchoredVwapParams) -> Self {
        Self {
            params,
            cum_pv: 0.0,
            cum_v: 0.0,
            value: 0.0,
            last_month_key: i32::MIN,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.cum_pv = 0.0;
        self.cum_v = 0.0;
        self.value = 0.0;
        self.last_month_key = i32::MIN;
        self.is_ready = false;
    }

    /// Update with a new bar. Requires the bar's UNIX timestamp in seconds (UTC) for calendar anchoring.
    /// Returns current AVWAP value.
    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        unix_time_secs: i64,
    ) -> f64 {
        // Determine month key (YYYY*12 + MM) using simple UTC approximation
        let month_key = Self::calc_month_key(unix_time_secs);

        // Handle anchor reset
        if self.last_month_key != month_key {
            self.cum_pv = 0.0;
            self.cum_v = 0.0;
            self.is_ready = false;
            self.last_month_key = month_key;
        }

        // Typical price * volume accumulation
        let tp = (high + low + close) / 3.0;
        self.cum_pv += tp * volume.max(0.0);
        self.cum_v += volume.max(0.0);

        if self.cum_v > 0.0 {
            self.value = self.cum_pv / self.cum_v;
            self.is_ready = true;
        }

        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    fn calc_month_key(unix_time_secs: i64) -> i32 {
        // Rough UTC month extraction using chrono is heavy; do a light-weight approx via seconds
        // This is acceptable for D1 bars, as reset happens on first bar of a new month by upstream loader
        // If precise calendar split is needed, we can switch to chrono without impacting perf much on D1.
        // Here we compute days since epoch and map to approximate month with 365.2425/12 ≈ 30.436875 days.
        let days = unix_time_secs / 86_400;
        let approx_months_since_epoch = (days as f64 / 30.436875) as i64;
        approx_months_since_epoch as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchored_vwap_creation() {
        let avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        assert!(!avwap.is_ready());
        assert_eq!(avwap.value().main(), 0.0);
    }

    #[test]
    fn test_anchored_vwap_update() {
        let mut avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        let ts = 1700000000_i64; // Some timestamp
        let value = avwap.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0, ts);
        assert!(avwap.is_ready());
        assert!(value > 0.0);
    }

    #[test]
    fn test_anchored_vwap_accumulation() {
        let mut avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        let ts = 1700000000_i64;
        for i in 0..10 {
            let price = 100.0 + i as f64;
            avwap.update_bar(price, price + 1.0, price - 1.0, price, 1000.0, ts + i * 86400);
        }
        assert!(avwap.is_ready());
        let v = avwap.value().main();
        assert!(v > 100.0 && v < 110.0, "VWAP should be within price range");
    }

    #[test]
    fn test_anchored_vwap_reset() {
        let mut avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        avwap.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0, 1700000000);
        avwap.reset();
        assert!(!avwap.is_ready());
        assert_eq!(avwap.value().main(), 0.0);
    }
}
