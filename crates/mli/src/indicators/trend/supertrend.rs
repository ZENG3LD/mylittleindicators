//! Supertrend — dynamic trend-following indicator.
//!
//! Supertrend = (High + Low) / 2 ± (Multiplier × ATR)
//! Displays dynamic support/resistance levels and trend direction.

use crate::engine::contract_engine::SmootherId;
use crate::indicators::volatility::atr::Atr;

/// Supertrend indicator.
#[derive(Debug, Clone)]
pub struct Supertrend {
    period: usize,
    multiplier: f64,

    supertrend_values: Vec<f64>,
    trend_direction: Vec<i8>,

    // ATR for volatility computation
    atr: Atr,

    supertrend_value: f64,
    current_trend: i8, // 1 = uptrend, -1 = downtrend

    prev_supertrend: f64,
    prev_trend: i8,

    bars_count: usize,
    is_ready: bool,
}

impl Supertrend {
    /// Create Supertrend with default params (period=10, multiplier=3.0, RMA).
    pub fn new() -> Self {
        Self::with_params(10, 3.0)
    }

    /// Create Supertrend with custom params (RMA by default).
    pub fn with_params(period: usize, multiplier: f64) -> Self {
        Self::from_smoother(period, multiplier, SmootherId::Rma)
    }

    /// Create Supertrend with a specific ATR smoother.
    pub fn from_smoother(period: usize, multiplier: f64, smoother: SmootherId) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        Self {
            period,
            multiplier,
            supertrend_values: Vec::with_capacity(512),
            trend_direction: Vec::with_capacity(512),
            atr: Atr::from_smoother(period, smoother),
            supertrend_value: 0.0,
            current_trend: 1,
            prev_supertrend: 0.0,
            prev_trend: 1,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed resolved input lanes `[high, low, close]` — the pure core computation.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        let atr_value = self.atr.feed(&[high, low, close]);
        let hl2 = (high + low) / 2.0;
        let upper_band = hl2 + (self.multiplier * atr_value);
        let lower_band = hl2 - (self.multiplier * atr_value);

        let final_upper_band = if self.bars_count == 1 {
            upper_band
        } else {
            let prev_final_upper = if !self.supertrend_values.is_empty() && self.prev_trend == 1 {
                self.prev_supertrend
            } else {
                upper_band
            };
            if upper_band < prev_final_upper || close > prev_final_upper {
                upper_band
            } else {
                prev_final_upper
            }
        };

        let final_lower_band = if self.bars_count == 1 {
            lower_band
        } else {
            let prev_final_lower = if !self.supertrend_values.is_empty() && self.prev_trend == -1 {
                self.prev_supertrend
            } else {
                lower_band
            };
            if lower_band > prev_final_lower || close < prev_final_lower {
                lower_band
            } else {
                prev_final_lower
            }
        };

        if self.bars_count == 1 {
            self.current_trend = if close <= final_upper_band { -1 } else { 1 };
        } else if self.prev_trend == 1 && close <= final_lower_band {
            self.current_trend = -1;
        } else if self.prev_trend == -1 && close >= final_upper_band {
            self.current_trend = 1;
        } else {
            self.current_trend = self.prev_trend;
        }

        self.supertrend_value = if self.current_trend == 1 {
            final_lower_band
        } else {
            final_upper_band
        };

        if self.supertrend_values.len() >= 512 {
            self.supertrend_values.remove(0);
        }
        if self.trend_direction.len() >= 512 {
            self.trend_direction.remove(0);
        }

        self.supertrend_values.push(self.supertrend_value);
        self.trend_direction.push(self.current_trend);
        self.prev_supertrend = self.supertrend_value;
        self.prev_trend = self.current_trend;

        if self.bars_count >= self.period + 2 {
            self.is_ready = true;
        }

        self.supertrend_value
    }

    /// Get Supertrend value.
    pub fn value(&self) -> f64 {
        self.supertrend_value
    }

    /// Get trend direction: 1 = uptrend, -1 = downtrend.
    pub fn trend_direction(&self) -> i8 {
        self.current_trend
    }

    /// Get value and trend direction.
    pub fn values(&self) -> (f64, i8) {
        (self.supertrend_value, self.current_trend)
    }

    /// Check readiness.
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Get parameters (period, multiplier).
    pub fn parameters(&self) -> (usize, f64) {
        (self.period, self.multiplier)
    }

    /// Reset indicator state.
    pub fn reset(&mut self) {
        self.supertrend_values.clear();
        self.trend_direction.clear();
        self.atr.reset();
        self.supertrend_value = 0.0;
        self.current_trend = 1;
        self.prev_supertrend = 0.0;
        self.prev_trend = 1;
        self.bars_count = 0;
        self.is_ready = false;
    }

    // Transitional bridge: supertrend_stop.rs still calls these; delegates to feed/from_smoother.
    // Remove when supertrend_stop.rs is contracted.

    /// Transitional bridge: `supertrend_stop.rs` calls `Supertrend::with_atr_ma_type(period, multiplier, ma_type)`.
    /// Delegates to `from_smoother` via `to_smoother()`.
    pub fn with_atr_ma_type(
        period: usize,
        multiplier: f64,
        atr_ma_type: crate::engine::contract_engine::SmootherId,
    ) -> Self {
        Self::from_smoother(period, multiplier, atr_ma_type)
    }


    /// Trend condition as a static string.
    pub fn trend_condition(&self) -> &'static str {
        match self.current_trend {
            1 => "Uptrend",
            -1 => "Downtrend",
            _ => "Neutral",
        }
    }

    /// Trading signal based on price vs Supertrend.
    pub fn trading_signal(&self, close: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        match self.current_trend {
            1 if close > self.supertrend_value => 1,
            -1 if close < self.supertrend_value => -1,
            _ => 0,
        }
    }

    /// Trend change signal: 1 = flipped to uptrend, -1 = flipped to downtrend, 0 = none.
    pub fn trend_change_signal(&self) -> i8 {
        if !self.is_ready() || self.trend_direction.len() < 2 {
            return 0;
        }
        let len = self.trend_direction.len();
        let prev = self.trend_direction[len - 2];
        if prev == -1 && self.current_trend == 1 {
            1
        } else if prev == 1 && self.current_trend == -1 {
            -1
        } else {
            0
        }
    }

    /// Distance to Supertrend as a percentage.
    pub fn distance_to_supertrend(&self, price: f64) -> f64 {
        if !self.is_ready() || self.supertrend_value.abs() < 1e-12 {
            return 0.0;
        }
        (price - self.supertrend_value) / self.supertrend_value * 100.0
    }

    /// Percentage of the last `periods` bars that are in the current trend direction.
    pub fn trend_strength(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.trend_direction.len() < periods {
            return 0.0;
        }
        let start_idx = self.trend_direction.len() - periods;
        let slice = &self.trend_direction[start_idx..];
        let count = slice.iter().filter(|&&x| x == self.current_trend).count();
        count as f64 / periods as f64 * 100.0
    }

    /// Bars the current trend has persisted.
    pub fn trend_duration(&self) -> usize {
        if !self.is_ready() || self.trend_direction.is_empty() {
            return 0;
        }
        let mut duration = 1;
        for i in (0..self.trend_direction.len().saturating_sub(1)).rev() {
            if self.trend_direction[i] == self.current_trend {
                duration += 1;
            } else {
                break;
            }
        }
        duration
    }

    /// Rate of change of Supertrend over `periods` bars.
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.supertrend_values.len() < periods + 1 {
            return 0.0;
        }
        let past = self.supertrend_values[self.supertrend_values.len() - periods - 1];
        if past.abs() > 1e-12 {
            (self.supertrend_value - past) / past * 100.0
        } else {
            0.0
        }
    }

    /// Support or resistance level identification.
    pub fn support_resistance_level(&self) -> (&'static str, f64) {
        if !self.is_ready() {
            return ("Unknown", 0.0);
        }
        match self.current_trend {
            1 => ("Support", self.supertrend_value),
            -1 => ("Resistance", self.supertrend_value),
            _ => ("Neutral", self.supertrend_value),
        }
    }

    /// Trend statistics over last `periods` bars: (uptrend%, downtrend%, change_count).
    pub fn trend_statistics(&self, periods: usize) -> (f64, f64, usize) {
        if !self.is_ready() || self.trend_direction.len() < periods {
            return (0.0, 0.0, 0);
        }
        let start_idx = self.trend_direction.len() - periods;
        let slice = &self.trend_direction[start_idx..];
        let up = slice.iter().filter(|&&x| x == 1).count();
        let down = slice.iter().filter(|&&x| x == -1).count();
        let mut changes = 0usize;
        for i in 1..slice.len() {
            if slice[i] != slice[i - 1] {
                changes += 1;
            }
        }
        (up as f64 / periods as f64 * 100.0, down as f64 / periods as f64 * 100.0, changes)
    }

    /// Indicator state summary.
    pub fn info(&self) -> String {
        let duration = self.trend_duration();
        let strength = self.trend_strength(20);
        let (level_type, level_value) = self.support_resistance_level();
        format!(
            "Supertrend: {:.2}, Trend: {}, Duration: {} bars, Strength: {:.1}%, {} Level: {:.2}",
            self.supertrend_value,
            self.trend_condition(),
            duration,
            strength,
            level_type,
            level_value
        )
    }
}

impl Default for Supertrend {
    fn default() -> Self {
        Self::from_smoother(10, 3.0, SmootherId::Rma)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Supertrend`] — period and ATR multiplier.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SupertrendConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
}

impl Indicator for Supertrend {
    const ID: IndicatorId = IndicatorId::Supertrend;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        // Two bounded Vec buffers (capped at 512): supertrend_values + trend_direction.
        stores: &[Store::fixed(StoreKind::Vec, 512), Store::fixed(StoreKind::Vec, 512)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Supertrend)];
    type Config = SupertrendConfig;
    type Runtime = Supertrend;

    fn create(cfg: SupertrendConfig) -> Supertrend {
        Supertrend::from_smoother(cfg.period.resolved(), cfg.multiplier.resolved(), SmootherId::Rma)
    }
}

impl crate::contract::Config for SupertrendConfig {
    fn defaults() -> Self {
        SupertrendConfig { period: Param::Solo(10), multiplier: Param::Solo(3.0) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        use crate::contract::sweep_f64;
        // period: Class A — auto range(2,4048,1).
        let mut s = Self::machine_defaults_auto();
        // multiplier: Class C (ATR band-width scaler) — sweep_f64(0.1,10.0,0.1).
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
}


impl Render for Supertrend {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Supertrend, "Supertrend", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supertrend_creation() {
        let st = Supertrend::new();
        assert!(!st.is_ready());
        assert_eq!(st.parameters(), (10, 3.0));
    }

    #[test]
    fn test_supertrend_with_params() {
        let st = Supertrend::with_params(14, 2.5);
        assert!(!st.is_ready());
        assert_eq!(st.parameters(), (14, 2.5));
    }

    #[test]
    fn test_supertrend_warmup() {
        let mut st = Supertrend::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            st.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(st.is_ready());
    }

    #[test]
    fn test_supertrend_values_finite() {
        let mut st = Supertrend::new();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = st.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value.is_finite());
        }
        let dir = st.trend_direction();
        assert!(dir == 1 || dir == -1);
    }

    #[test]
    fn test_supertrend_reset() {
        let mut st = Supertrend::new();
        for i in 0..30 {
            st.feed(&[100.0 + i as f64 + 5.0, 100.0 + i as f64 - 5.0, 100.0 + i as f64]);
        }
        st.reset();
        assert!(!st.is_ready());
    }

    #[test]
    fn test_supertrend_warmup_finite() {
        let mut st = Supertrend::from_smoother(10, 3.0, SmootherId::Ema);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            let v = st.feed(&[price + 2.0, price - 2.0, price]);
            assert!(v.is_finite());
        }
        assert!(st.is_ready());
    }

    #[test]
    fn test_factory_feeds_resolved_supertrend() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Supertrend(<<Supertrend as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // not used by KlineSlice[H,L,C]
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0, // not used
            });
        }
        assert!(f.primary().is_finite());
    }
}
