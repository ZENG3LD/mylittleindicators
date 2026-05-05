//! Chande Momentum Oscillator (CMO) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// CMO calculation mode.
#[derive(Clone)]
pub enum CmoMode {
    /// Classic mode using SMA/buffer-based calculation.
    Classic,
    /// Wilder mode using MovingAverage-based calculation (default).
    Wilder,
}

/// Chande Momentum Oscillator (CMO) - measures momentum as ratio of gains vs losses.
///
/// CMO = 100 × (Sum of Gains - Sum of Losses) / (Sum of Gains + Sum of Losses)
///
/// Developed by Tushar Chande. Unlike RSI which uses ratio of gains to losses,
/// CMO uses the difference, making it oscillate between -100 and +100.
///
/// Interpretation:
/// - CMO > 50: Strong upward momentum, overbought
/// - CMO < -50: Strong downward momentum, oversold
/// - Zero crossovers: Trend change signals
/// - CMO near 0: Neutral momentum
///
/// # Parameters
/// - `period`: Lookback period for momentum calculation
/// - `ma_type`: Type of moving average for smoothing (default RMA)
/// - `mode`: Calculation mode (Wilder or Classic)
///
/// # Implementation
///
/// Supports two modes: Wilder (uses MAs) and Classic (uses buffers).
/// O(1) per update in Wilder mode, O(period) in Classic mode.
#[derive(Clone)]
pub struct Cmo {
    period: usize,
    ma_type: MovingAverageType,  // Store catalog type, no pattern matching needed
    gain_ma: MovingAverageProvider,
    loss_ma: MovingAverageProvider,
    gains: ArrayVec<f64, 512>,
    losses: ArrayVec<f64, 512>,
    prev: f64,
    value: f64,
    filled: bool,
    index: usize,
    mode: CmoMode,
} 

impl Cmo {
    /// Creates a new CMO with Wilder mode (default).
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `ma_type` - Optional moving average type (default RMA)
    pub fn new(period: usize, ma_type: Option<MovingAverageType>) -> Self {
        Self::with_mode(period, ma_type, CmoMode::Wilder)
    }

    /// Creates a new CMO with specified mode.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `ma_type` - Optional moving average type (default RMA)
    /// * `mode` - Calculation mode (Wilder or Classic)
    pub fn with_mode(period: usize, ma_type: Option<MovingAverageType>, mode: CmoMode) -> Self {
        let ma_type_resolved = ma_type.unwrap_or(MovingAverageType::RMA);
        Self {
            period,
            ma_type: ma_type_resolved,
            gain_ma: MovingAverageProvider::new(ma_type_resolved, period),
            loss_ma: MovingAverageProvider::new(ma_type_resolved, period),
            gains: ArrayVec::new(),
            losses: ArrayVec::new(),
            prev: 0.0,
            value: 0.0,
            filled: false,
            index: 0,
            mode,
        }
    }
    /// Updates the CMO with a new bar and returns the current value.
    ///
    /// Only the `close` price is used; other OHLCV fields are ignored.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        if self.index == 0 && self.prev == 0.0 {
            self.prev = close;
            self.index = 1;
            return self.value;
        }
        let diff = close - self.prev;
        let (gain, loss) = if diff > 0.0 {
            (diff, 0.0)
        } else {
            (0.0, -diff)
        };
        self.prev = close;
        self.index += 1;
        match self.mode {
            CmoMode::Wilder => {
                let avg_gain = self.gain_ma.update_bar(0.0, 0.0, 0.0, gain, 0.0);
                let avg_loss = self.loss_ma.update_bar(0.0, 0.0, 0.0, loss, 0.0);
                if self.index >= self.period {
                    self.filled = true;
                }
                let denom = avg_gain + avg_loss;
                self.value = if self.filled && denom.abs() >= 1e-12 {
                    100.0 * (avg_gain - avg_loss) / denom
                } else {
                    0.0
                };
            }
            CmoMode::Classic => {
                if self.gains.len() == self.period {
                    self.gains.pop();
                }
                if self.losses.len() == self.period {
                    self.losses.pop();
                }
                self.gains.insert(0, gain);
                self.losses.insert(0, loss);
                if self.gains.len() == self.period && self.losses.len() == self.period {
                    self.filled = true;
                    let avg_gain: f64 = self.gains.iter().sum::<f64>() / self.period as f64;
                    let avg_loss: f64 = self.losses.iter().sum::<f64>() / self.period as f64;
                    let denom = avg_gain + avg_loss;
                    self.value = if denom.abs() >= 1e-12 {
                        100.0 * (avg_gain - avg_loss) / denom
                    } else {
                        0.0
                    };
                }
            }
        }
        self.value
    }
    /// Returns the current CMO value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the CMO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the CMO to its initial state.
    pub fn reset(&mut self) {
        // Use stored ma_type directly - no pattern matching on MovingAverage enum
        self.gain_ma = MovingAverageProvider::new(self.ma_type, self.period);
        self.loss_ma = MovingAverageProvider::new(self.ma_type, self.period);
        self.gains.clear();
        self.losses.clear();
        self.index = 0;
        self.filled = false;
        self.prev = 0.0;
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_cmo_basic_calculation() {
        let mut cmo = Cmo::new(14, None);

        // Feed uptrend data
        for i in 1..=30 {
            cmo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(cmo.is_ready());
        // Pure uptrend = only gains, CMO should be near +100
        assert!(cmo.value().main() > 50.0, "CMO in strong uptrend should be > 50");
    }

    #[test]
    fn test_cmo_downtrend() {
        let mut cmo = Cmo::new(14, None);

        // Feed downtrend data
        for i in 1..=30 {
            cmo.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(cmo.is_ready());
        // Pure downtrend = only losses, CMO should be near -100
        assert!(cmo.value().main() < -50.0, "CMO in strong downtrend should be < -50");
    }

    #[test]
    fn test_cmo_range() {
        let mut cmo = Cmo::new(14, None);

        // Feed oscillating data
        for i in 1..=30 {
            let price = if i % 2 == 0 { 105.0 } else { 95.0 };
            cmo.update_bar(0.0, 0.0, 0.0, price, 0.0);
        }

        assert!(cmo.is_ready());
        // CMO should be between -100 and +100
        assert!(cmo.value().main() >= -100.0 && cmo.value().main() <= 100.0);
    }

    #[test]
    fn test_cmo_constant_price() {
        let mut cmo = Cmo::new(14, None);

        // First bar
        cmo.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);

        // Constant price = no change, CMO should be 0
        for _ in 1..=30 {
            cmo.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        }

        assert!(cmo.is_ready());
        assert!(cmo.value().main().abs() < 1.0, "CMO with constant price should be near 0");
    }

    #[test]
    fn test_cmo_reset() {
        let mut cmo = Cmo::new(14, None);

        for i in 1..=30 {
            cmo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(cmo.is_ready());

        cmo.reset();
        assert!(!cmo.is_ready());
        assert!(cmo.value().main().abs() < 1e-10);
    }

    #[test]
    fn test_cmo_classic_mode() {
        let mut cmo = Cmo::with_mode(10, None, CmoMode::Classic);

        for i in 1..=20 {
            cmo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(cmo.is_ready());
        assert!(cmo.value().main() > 0.0);
    }

    #[test]
    fn test_cmo_with_ema() {
        let mut cmo = Cmo::new(14, Some(MovingAverageType::EMA));

        for i in 1..=30 {
            cmo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(cmo.is_ready());
        assert!(cmo.value().main() > 0.0);
    }
}


















