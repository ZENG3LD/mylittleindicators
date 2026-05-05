//! Know Sure Thing (KST) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Know Sure Thing (KST) - composite momentum indicator by Martin Pring.
///
/// KST = Weighted average of four smoothed ROC values:
/// KST = (ROC1×SMA1×1 + ROC2×SMA2×2 + ROC3×SMA3×3 + ROC4×SMA4×4) / 10
///
/// Combines multiple Rate of Change indicators at different timeframes
/// to create a smoother, more reliable momentum oscillator.
///
/// Interpretation:
/// - KST > 0 and rising: Bullish momentum
/// - KST < 0 and falling: Bearish momentum
/// - KST crossing signal line: Trading signals
/// - Zero line crossings: Trend changes
///
/// # Parameters
/// - `roc_periods`: ROC lookback periods [10, 15, 20, 30]
/// - `sma_periods`: SMA smoothing periods [10, 10, 10, 15]
/// - `signal_period`: Signal line period (typically 9)
/// - `roc_ma_type`: Type of moving average for ROC smoothing
/// - `signal_ma_type`: Type of moving average for signal line
/// - `source`: OHLCV field to use for ROC calculation
///
/// # Implementation
///
/// Calculates four ROC values, smooths each with MA, applies weights, sums them.
#[derive(Clone)]
pub struct KnowSureThing {
    roc_periods: [usize; 4],
    sma_periods: [usize; 4],
    signal_period: usize,
    roc_ma_type: MovingAverageType,
    signal_ma_type: MovingAverageType,
    source: OhlcvField,

    source_prices: ArrayVec<f64, 512>,
    roc_smas: [MovingAverageProvider; 4],
    signal_sma: MovingAverageProvider,
    kst_values: ArrayVec<f64, 512>,

    kst_value: f64,
    signal_value: f64,

    bars_count: usize,
    is_ready: bool,
}

impl KnowSureThing {
    /// Creates a new KST with default parameters.
    pub fn new() -> Self {
        Self::with_params([10, 15, 20, 30], [10, 10, 10, 15], 9, MovingAverageType::SMA)
    }

    /// Creates a new KST with custom parameters using SMA (backward compatibility).
    pub fn with_params_default(roc_periods: [usize; 4], sma_periods: [usize; 4], signal_period: usize) -> Self {
        Self::with_params(roc_periods, sma_periods, signal_period, MovingAverageType::SMA)
    }

    /// Creates a new KST with custom parameters.
    ///
    /// # Arguments
    /// * `roc_periods` - ROC lookback periods (typically [10, 15, 20, 30])
    /// * `sma_periods` - SMA smoothing periods (typically [10, 10, 10, 15])
    /// * `signal_period` - Signal line period (typically 9)
    /// * `ma_type` - Type of moving average for both ROC smoothing and signal line
    pub fn with_params(roc_periods: [usize; 4], sma_periods: [usize; 4], signal_period: usize, ma_type: MovingAverageType) -> Self {
        assert!(roc_periods.iter().all(|&p| p > 0), "All ROC periods must be greater than 0");
        assert!(sma_periods.iter().all(|&p| p > 0), "All SMA periods must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            roc_periods,
            sma_periods,
            signal_period,
            roc_ma_type: ma_type,
            signal_ma_type: ma_type,
            source: OhlcvField::Close,
            source_prices: ArrayVec::new(),
            roc_smas: [
                MovingAverageProvider::new(ma_type, sma_periods[0]),
                MovingAverageProvider::new(ma_type, sma_periods[1]),
                MovingAverageProvider::new(ma_type, sma_periods[2]),
                MovingAverageProvider::new(ma_type, sma_periods[3]),
            ],
            signal_sma: MovingAverageProvider::new(ma_type, signal_period),
            kst_values: ArrayVec::new(),
            kst_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new KST with custom source field.
    ///
    /// # Arguments
    /// * `roc_periods` - ROC lookback periods (typically [10, 15, 20, 30])
    /// * `sma_periods` - SMA smoothing periods (typically [10, 10, 10, 15])
    /// * `signal_period` - Signal line period (typically 9)
    /// * `source` - OHLCV field to use for ROC calculation
    pub fn with_source(roc_periods: [usize; 4], sma_periods: [usize; 4], signal_period: usize, source: OhlcvField) -> Self {
        let mut kst = Self::with_params(roc_periods, sma_periods, signal_period, MovingAverageType::SMA);
        kst.source = source;
        kst
    }

    /// Creates a new KST with custom MA types.
    ///
    /// # Arguments
    /// * `roc_periods` - ROC lookback periods (typically [10, 15, 20, 30])
    /// * `sma_periods` - SMA smoothing periods (typically [10, 10, 10, 15])
    /// * `signal_period` - Signal line period (typically 9)
    /// * `roc_ma_type` - MA type for ROC smoothing
    /// * `signal_ma_type` - MA type for signal line
    pub fn with_ma_types(
        roc_periods: [usize; 4],
        sma_periods: [usize; 4],
        signal_period: usize,
        roc_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType
    ) -> Self {
        assert!(roc_periods.iter().all(|&p| p > 0), "All ROC periods must be greater than 0");
        assert!(sma_periods.iter().all(|&p| p > 0), "All SMA periods must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            roc_periods,
            sma_periods,
            signal_period,
            roc_ma_type,
            signal_ma_type,
            source: OhlcvField::Close,
            source_prices: ArrayVec::new(),
            roc_smas: [
                MovingAverageProvider::new(roc_ma_type, sma_periods[0]),
                MovingAverageProvider::new(roc_ma_type, sma_periods[1]),
                MovingAverageProvider::new(roc_ma_type, sma_periods[2]),
                MovingAverageProvider::new(roc_ma_type, sma_periods[3]),
            ],
            signal_sma: MovingAverageProvider::new(signal_ma_type, signal_period),
            kst_values: ArrayVec::new(),
            kst_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new KST with full configuration (MA types + source).
    ///
    /// # Arguments
    /// * `roc_periods` - ROC lookback periods (typically [10, 15, 20, 30])
    /// * `sma_periods` - SMA smoothing periods (typically [10, 10, 10, 15])
    /// * `signal_period` - Signal line period (typically 9)
    /// * `roc_ma_type` - MA type for ROC smoothing
    /// * `signal_ma_type` - MA type for signal line
    /// * `source` - OHLCV field to use for ROC calculation
    pub fn with_full_config(
        roc_periods: [usize; 4],
        sma_periods: [usize; 4],
        signal_period: usize,
        roc_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
        source: OhlcvField
    ) -> Self {
        let mut kst = Self::with_ma_types(roc_periods, sma_periods, signal_period, roc_ma_type, signal_ma_type);
        kst.source = source;
        kst
    }
    
    /// Updates the KST with a new bar and returns the KST value.
    ///
    /// Uses the configured source field to extract the price value.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;

        // Extract price based on source field
        let source_value = self.source.extract(open, high, low, close, volume);

        if self.source_prices.len() >= 512 {
            self.source_prices.remove(0);
        }
        self.source_prices.push(source_value);

        // Проверяем, можем ли рассчитать все ROC
        let max_roc_period = *self.roc_periods.iter().max().unwrap();
        if self.source_prices.len() < max_roc_period + 1 {
            return self.kst_value;
        }

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        // Рассчитываем каждый компонент KST
        for i in 0..4 {
            let roc_period = self.roc_periods[i];

            if self.source_prices.len() > roc_period {
                // Рассчитываем ROC
                let current_price = source_value;
                let past_price = self.source_prices[self.source_prices.len() - roc_period - 1];

                let roc = if past_price.abs() < 1e-12 {
                    0.0
                } else {
                    ((current_price - past_price) / past_price) * 100.0
                };

                // Сглаживаем ROC с помощью MA
                let smoothed_roc = self.roc_smas[i].update_bar(roc, roc, roc, roc, 1.0);

                // Добавляем к взвешенной сумме
                let weight = match i {
                    0 => 1.0,  // ROC 10 - вес 1
                    1 => 2.0,  // ROC 15 - вес 2
                    2 => 3.0,  // ROC 20 - вес 3
                    3 => 4.0,  // ROC 30 - вес 4
                    _ => 1.0,
                };

                weighted_sum += smoothed_roc * weight;
                total_weight += weight;
            }
        }

        // Рассчитываем KST
        self.kst_value = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        // Добавляем в буфер
        if self.kst_values.len() >= 512 {
            self.kst_values.remove(0);
        }
        self.kst_values.push(self.kst_value);

        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_sma.update_bar(self.kst_value, self.kst_value, self.kst_value, self.kst_value, 1.0);

        // Проверяем готовность
        let min_bars = max_roc_period + self.sma_periods.iter().max().unwrap() + self.signal_period;
        if self.bars_count >= min_bars {
            self.is_ready = true;
        }

        self.kst_value
    }
    
    /// Returns the current KST and signal values.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.kst_value, self.signal_value)
    }

    /// Returns the signal line value.
    #[inline]
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }

    /// Returns the histogram (KST - Signal).
    #[inline]
    pub fn histogram(&self) -> f64 {
        self.kst_value - self.signal_value
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the indicator parameters (roc_periods, sma_periods, signal_period).
    #[inline]
    pub fn parameters(&self) -> ([usize; 4], [usize; 4], usize) {
        (self.roc_periods, self.sma_periods, self.signal_period)
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.source_prices.clear();
        self.roc_smas = [
            MovingAverageProvider::new(self.roc_ma_type, self.sma_periods[0]),
            MovingAverageProvider::new(self.roc_ma_type, self.sma_periods[1]),
            MovingAverageProvider::new(self.roc_ma_type, self.sma_periods[2]),
            MovingAverageProvider::new(self.roc_ma_type, self.sma_periods[3]),
        ];
        self.signal_sma = MovingAverageProvider::new(self.signal_ma_type, self.signal_period);
        self.kst_values.clear();
        self.kst_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Sets a new moving average type for ROC smoothing and resets the indicator.
    pub fn set_roc_ma_type(&mut self, ma_type: MovingAverageType) {
        self.roc_ma_type = ma_type;
        self.reset();
    }

    /// Sets a new moving average type for signal line and resets the indicator.
    pub fn set_signal_ma_type(&mut self, ma_type: MovingAverageType) {
        self.signal_ma_type = ma_type;
        self.reset();
    }

    /// Sets both MA types and resets the indicator.
    pub fn set_ma_types(&mut self, roc_ma_type: MovingAverageType, signal_ma_type: MovingAverageType) {
        self.roc_ma_type = roc_ma_type;
        self.signal_ma_type = signal_ma_type;
        self.reset();
    }

    /// Sets a new source field and resets the indicator.
    pub fn set_source(&mut self, source: OhlcvField) {
        self.source = source;
        self.reset();
    }

    /// Returns the current market condition.
    pub fn market_condition(&self) -> &'static str {
        match self.kst_value {
            v if v > 0.0 && self.kst_value > self.signal_value => "Strong Bullish",
            v if v > 0.0 => "Bullish",
            v if v < 0.0 && self.kst_value < self.signal_value => "Strong Bearish",
            v if v < 0.0 => "Bearish",
            _ => "Neutral"
        }
    }
    
    /// Returns trading signal (1 = buy, -1 = sell, 0 = neutral).
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let histogram = self.histogram();
        
        // Сигналы на основе пересечения сигнальной линии
        if self.kst_value > self.signal_value && histogram > 0.1 {
            1  // Покупка - KST выше сигнальной линии
        } else if self.kst_value < self.signal_value && histogram < -0.1 {
            -1 // Продажа - KST ниже сигнальной линии
        } else {
            0  // Нейтрально
        }
    }
    
    /// Returns advanced signal with confirmation.
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.kst_values.len() < 3 {
            return 0;
        }
        
        let len = self.kst_values.len();
        let current = self.kst_value;
        let prev_1 = if len >= 2 { self.kst_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.kst_values[len - 3] } else { 0.0 };

        // Buy signal: zero line crossover from below with signal confirmation
        if prev_2 < 0.0 && prev_1 < 0.0 && current > 0.0 && self.kst_value > self.signal_value {
            return 1;
        }

        // Sell signal: zero line crossover from above with signal confirmation
        if prev_2 > 0.0 && prev_1 > 0.0 && current < 0.0 && self.kst_value < self.signal_value {
            return -1;
        }

        0
    }

    /// Checks for divergence between price and KST.
    ///
    /// Returns: 1 = bullish divergence, -1 = bearish divergence, 0 = none.
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.kst_values.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_kst = self.kst_value;
        let past_kst = self.kst_values[self.kst_values.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let kst_change = current_kst - past_kst;

        // Bullish divergence: price makes new low, but KST rises
        if price_change < 0.0 && kst_change > 0.0 {
            return 1;
        }

        // Bearish divergence: price makes new high, but KST falls
        if price_change > 0.0 && kst_change < 0.0 {
            return -1;
        }

        0
    }

    /// Returns momentum strength (absolute KST value).
    #[inline]
    pub fn momentum_strength(&self) -> f64 {
        self.kst_value.abs()
    }

    /// Returns rate of change of KST over specified periods.
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.kst_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.kst_value;
        let past = self.kst_values[self.kst_values.len() - periods - 1];
        
        if past.abs() < 1e-12 {
            0.0
        } else {
            (current - past) / past * 100.0
        }
    }
    
    /// Returns KST trend direction over specified lookback.
    ///
    /// Returns: 1 = rising, -1 = falling, 0 = sideways.
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.kst_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.kst_value;
        let past = self.kst_values[self.kst_values.len() - lookback - 1];

        if current > past {
            1
        } else if current < past {
            -1
        } else {
            0
        }
    }

    /// Returns individual KST components (raw ROC values) for analysis.
    pub fn get_components(&self) -> Option<[f64; 4]> {
        if !self.is_ready() || self.source_prices.len() < *self.roc_periods.iter().max().unwrap() + 1 {
            return None;
        }

        let mut components = [0.0; 4];
        let current_price = *self.source_prices.last().unwrap();

        for (comp, &roc_period) in components.iter_mut().zip(self.roc_periods.iter()) {
            if self.source_prices.len() > roc_period {
                let past_price = self.source_prices[self.source_prices.len() - roc_period - 1];

                let roc = if past_price.abs() < 1e-12 {
                    0.0
                } else {
                    ((current_price - past_price) / past_price) * 100.0
                };

                *comp = roc;
            }
        }

        Some(components)
    }

    /// Returns information about the indicator state.
    pub fn info(&self) -> String {
        format!(
            "KST: {:.2}, Signal: {:.2}, Histogram: {:.2}, Condition: {}, Strength: {:.2}",
            self.kst_value,
            self.signal_value,
            self.histogram(),
            self.market_condition(),
            self.momentum_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_kst_basic_calculation() {
        let mut kst = KnowSureThing::new();

        // Feed enough data for all ROC periods + smoothing
        for i in 1..=80 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let (kst_val, signal_val) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        assert!(kst_val.is_finite());
        assert!(signal_val.is_finite());
    }

    #[test]
    fn test_kst_uptrend() {
        let mut kst = KnowSureThing::new();

        // Strong uptrend
        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64 * 2.0, 0.0);
        }

        assert!(kst.is_ready());
        let (kst_val, _signal) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        // KST should be positive in uptrend
        assert!(kst_val > 0.0, "KST should be positive in uptrend");
    }

    #[test]
    fn test_kst_downtrend() {
        let mut kst = KnowSureThing::new();

        // Strong downtrend
        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 300.0 - i as f64 * 2.0, 0.0);
        }

        assert!(kst.is_ready());
        let (kst_val, _signal) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        // KST should be negative in downtrend
        assert!(kst_val < 0.0, "KST should be negative in downtrend");
    }

    #[test]
    fn test_kst_signal_line() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let signal = kst.signal_value();
        assert!(signal.is_finite());
    }

    #[test]
    fn test_kst_histogram() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let hist = kst.histogram();
        let (kst_val, _signal) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        assert!((hist - (kst_val - kst.signal_value())).abs() < 1e-10);
    }

    #[test]
    fn test_kst_reset() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(kst.is_ready());

        kst.reset();
        assert!(!kst.is_ready());
    }

    #[test]
    fn test_kst_parameters() {
        let kst = KnowSureThing::with_params([5, 10, 15, 20], [5, 5, 5, 10], 7, MovingAverageType::SMA);
        let (roc, sma, sig) = kst.parameters();
        assert_eq!(roc, [5, 10, 15, 20]);
        assert_eq!(sma, [5, 5, 5, 10]);
        assert_eq!(sig, 7);
    }

    #[test]
    fn test_kst_market_condition() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let condition = kst.market_condition();
        assert!(
            condition == "Strong Bullish"
                || condition == "Bullish"
                || condition == "Neutral"
                || condition == "Bearish"
                || condition == "Strong Bearish"
        );
    }

    #[test]
    fn test_kst_trading_signal() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let signal = kst.trading_signal();
        assert!(signal >= -1 && signal <= 1);
    }

    #[test]
    fn test_kst_momentum_strength() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let strength = kst.momentum_strength();
        assert!(strength >= 0.0);
    }

    #[test]
    fn test_kst_trend_direction() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let direction = kst.trend_direction(5);
        assert!(direction >= -1 && direction <= 1);
    }

    #[test]
    fn test_kst_get_components() {
        let mut kst = KnowSureThing::new();

        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let components = kst.get_components();
        assert!(components.is_some());
        let comp = components.unwrap();
        for c in comp.iter() {
            assert!(c.is_finite());
        }
    }

    #[test]
    fn test_kst_with_source() {
        // Test with HLC3 source
        let mut kst_hlc3 = KnowSureThing::with_source(
            [10, 15, 20, 30],
            [10, 10, 10, 15],
            9,
            OhlcvField::HLC3
        );

        // Test with Close source for comparison
        let mut kst_close = KnowSureThing::new();

        // Feed data where High and Low create a different HLC3 than Close
        for i in 1..=100 {
            let close = 100.0 + i as f64;
            let high = close + 10.0;  // High is above close
            let low = close - 5.0;    // Low is below close
            // HLC3 = (high + low + close) / 3 != close

            kst_hlc3.update_bar(0.0, high, low, close, 0.0);
            kst_close.update_bar(0.0, high, low, close, 0.0);
        }

        assert!(kst_hlc3.is_ready());
        assert!(kst_close.is_ready());

        // Values should be different because different sources
        let (hlc3_val, _) = match kst_hlc3.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        let (close_val, _) = match kst_close.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };

        // Both should be positive (uptrend) but values differ
        assert!(hlc3_val > 0.0);
        assert!(close_val > 0.0);
        assert_ne!(hlc3_val, close_val);
    }

    #[test]
    fn test_kst_with_ma_types() {
        // Test with different MA types for ROC and signal
        let mut kst_ema = KnowSureThing::with_ma_types(
            [10, 15, 20, 30],
            [10, 10, 10, 15],
            9,
            MovingAverageType::EMA,
            MovingAverageType::SMA
        );

        let mut kst_sma = KnowSureThing::new();

        for i in 1..=100 {
            kst_ema.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
            kst_sma.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst_ema.is_ready());
        assert!(kst_sma.is_ready());

        // Values should be different due to different MA types
        let (ema_val, _) = match kst_ema.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        let (sma_val, _) = match kst_sma.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };

        assert!(ema_val.is_finite());
        assert!(sma_val.is_finite());
        assert_ne!(ema_val, sma_val);
    }

    #[test]
    fn test_kst_with_full_config() {
        let mut kst = KnowSureThing::with_full_config(
            [10, 15, 20, 30],
            [10, 10, 10, 15],
            9,
            MovingAverageType::EMA,
            MovingAverageType::RMA,
            OhlcvField::HLC3
        );

        for i in 1..=100 {
            kst.update_bar(100.0, 110.0, 90.0, 105.0 + i as f64, 1000.0);
        }

        assert!(kst.is_ready());
        let (kst_val, signal_val) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        assert!(kst_val.is_finite());
        assert!(signal_val.is_finite());
    }

    #[test]
    fn test_kst_set_source() {
        let mut kst = KnowSureThing::new();

        // Feed initial data
        for i in 1..=50 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        // Change source and reset
        kst.set_source(OhlcvField::HL2);
        assert!(!kst.is_ready());

        // Feed new data with different high/low
        for i in 1..=100 {
            let close = 100.0 + i as f64;
            kst.update_bar(close, close + 10.0, close - 10.0, close, 0.0);
        }

        assert!(kst.is_ready());
        let (kst_val, _) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        assert!(kst_val.is_finite());
    }

    #[test]
    fn test_kst_set_ma_types() {
        let mut kst = KnowSureThing::new();

        // Feed initial data
        for i in 1..=50 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        // Change MA types and reset
        kst.set_ma_types(MovingAverageType::EMA, MovingAverageType::WMA);
        assert!(!kst.is_ready());

        // Feed new data
        for i in 1..=100 {
            kst.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(kst.is_ready());
        let (kst_val, signal_val) = match kst.value() {
            IndicatorValue::Double(k, s) => (k, s),
            _ => panic!("Expected Double value"),
        };
        assert!(kst_val.is_finite());
        assert!(signal_val.is_finite());
    }
}















