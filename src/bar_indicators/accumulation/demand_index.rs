//! Demand Index (James Sibbet)
//!
//! Combines price movement and volume to identify buying/selling pressure.
//! Formula: DI = (Close - Close[1]) / Range * Volume_Factor
//! Where Volume_Factor incorporates volume relative to its moving average.
//!
//! Positive values indicate buying pressure, negative values indicate selling pressure.

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DemandIndex {
    period: usize,

    // Previous bar data
    prev_close: f64,
    prev_high: f64,
    prev_low: f64,

    // Volume moving average
    volume_sum: f64,
    volumes: Vec<f64>,
    vol_idx: usize,
    vol_filled: bool,

    // Demand Index value
    demand_index: f64,
    cumulative_di: f64,

    // State
    bars_count: usize,
    is_ready: bool,
}

impl Default for DemandIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl DemandIndex {
    pub fn new() -> Self {
        Self::with_period(14)
    }

    pub fn with_period(period: usize) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            prev_close: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            volume_sum: 0.0,
            volumes: vec![0.0; p],
            vol_idx: 0,
            vol_filled: false,
            demand_index: 0.0,
            cumulative_di: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prev_close = 0.0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.volume_sum = 0.0;
        self.volumes.fill(0.0);
        self.vol_idx = 0;
        self.vol_filled = false;
        self.demand_index = 0.0;
        self.cumulative_di = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.demand_index)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;

        // Update volume moving average (ring buffer)
        if self.vol_filled {
            self.volume_sum -= self.volumes[self.vol_idx];
        }
        self.volumes[self.vol_idx] = volume;
        self.volume_sum += volume;
        self.vol_idx = (self.vol_idx + 1) % self.period;
        if self.vol_idx == 0 {
            self.vol_filled = true;
        }

        // Need at least 2 bars
        if self.bars_count < 2 {
            self.prev_close = close;
            self.prev_high = high;
            self.prev_low = low;
            return self.demand_index;
        }

        // Calculate price change
        let price_change = close - self.prev_close;

        // Calculate true range for normalization
        let true_high = high.max(self.prev_close);
        let true_low = low.min(self.prev_close);
        let true_range = (true_high - true_low).max(1e-10);

        // Buying Pressure (BP) and Selling Pressure (SP)
        // BP = High - Open + Close - Low
        // SP = High - Close + Open - Low
        let buying_pressure = (high - open) + (close - low);
        let selling_pressure = (high - close) + (open - low);

        // Volume factor: current volume relative to average
        let avg_volume = if self.vol_filled {
            self.volume_sum / self.period as f64
        } else {
            self.volume_sum / self.bars_count as f64
        };

        let volume_factor = if avg_volume > 1e-10 {
            volume / avg_volume
        } else {
            1.0
        };

        // Calculate Demand Index
        // Combines price movement direction, range position, and volume
        let range = high - low;
        if range > 1e-10 {
            // Position of close within the bar's range (0 to 1)
            let close_position = (close - low) / range;

            // Pressure ratio
            let total_pressure = buying_pressure + selling_pressure;
            let pressure_ratio = if total_pressure > 1e-10 {
                (buying_pressure - selling_pressure) / total_pressure
            } else {
                0.0
            };

            // Demand Index = price_change_normalized * volume_factor * close_position_factor
            let price_factor = price_change / true_range;
            let position_factor = (close_position - 0.5) * 2.0; // -1 to 1

            // Combine factors
            self.demand_index = price_factor * volume_factor * (1.0 + position_factor) + pressure_ratio * volume_factor * 0.5;

            // Scale to reasonable range
            self.demand_index *= 100.0;
        } else {
            self.demand_index = 0.0;
        }

        // Accumulate
        self.cumulative_di += self.demand_index;

        // Update previous values
        self.prev_close = close;
        self.prev_high = high;
        self.prev_low = low;

        // Ready after period bars
        if self.bars_count >= self.period {
            self.is_ready = true;
        }

        self.demand_index
    }

    /// Get cumulative Demand Index
    pub fn cumulative(&self) -> f64 {
        self.cumulative_di
    }

    /// Get period
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demand_index_creation() {
        let di = DemandIndex::new();
        assert!(!di.is_ready());
        assert_eq!(di.value().main(), 0.0);
    }

    #[test]
    fn test_demand_index_with_period() {
        let di = DemandIndex::with_period(20);
        assert_eq!(di.period(), 20);
    }

    #[test]
    fn test_demand_index_warmup() {
        let mut di = DemandIndex::new();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            di.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0 + i as f64 * 10.0);
        }
        assert!(di.is_ready());
    }

    #[test]
    fn test_demand_index_uptrend() {
        let mut di = DemandIndex::new();
        // Strong uptrend with increasing volume
        for i in 0..20 {
            let base = 100.0 + i as f64 * 2.0;
            let open = base;
            let high = base + 2.0;
            let low = base - 0.5;
            let close = base + 1.5; // Close near high
            let volume = 1000.0 + i as f64 * 100.0; // Increasing volume
            di.update_bar(open, high, low, close, volume);
        }
        assert!(di.is_ready());
        // In uptrend with increasing volume, DI should be positive
        assert!(di.value().main() > 0.0, "DI should be positive in uptrend: {}", di.value().main());
    }

    #[test]
    fn test_demand_index_downtrend() {
        let mut di = DemandIndex::new();
        // Strong downtrend with increasing volume
        for i in 0..20 {
            let base = 200.0 - i as f64 * 2.0;
            let open = base;
            let high = base + 0.5;
            let low = base - 2.0;
            let close = base - 1.5; // Close near low
            let volume = 1000.0 + i as f64 * 100.0;
            di.update_bar(open, high, low, close, volume);
        }
        assert!(di.is_ready());
        // In downtrend, DI should be negative
        assert!(di.value().main() < 0.0, "DI should be negative in downtrend: {}", di.value().main());
    }

    #[test]
    fn test_demand_index_values_finite() {
        let mut di = DemandIndex::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = di.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Value should be finite");
        }
    }

    #[test]
    fn test_demand_index_reset() {
        let mut di = DemandIndex::new();
        for i in 0..20 {
            di.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        assert!(di.is_ready());
        di.reset();
        assert!(!di.is_ready());
        assert_eq!(di.value().main(), 0.0);
    }
}
