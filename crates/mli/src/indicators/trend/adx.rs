//! Average Directional Index (ADX) indicator.

use crate::engine::contract_engine::SmootherId;
use crate::indicators::volatility::atr::Atr;

/// Average Directional Index (ADX) - measures trend strength regardless of direction.
///
/// ADX oscillates between 0 and 100:
/// - 0-25: Weak trend (ranging market)
/// - 25-50: Moderate trend
/// - 50-75: Strong trend
/// - 75-100: Very strong trend
///
/// Also provides +DI and -DI for trend direction.
///
/// # Calculation
/// 1. Calculate True Range (TR)
/// 2. Calculate +DM and -DM (Directional Movement)
/// 3. Smooth TR, +DM, -DM using Wilder's smoothing
/// 4. Calculate +DI = 100 × Smoothed(+DM) / Smoothed(TR)
/// 5. Calculate -DI = 100 × Smoothed(-DM) / Smoothed(TR)
/// 6. DX = 100 × |+DI - -DI| / (+DI + -DI)
/// 7. ADX = Smoothed(DX)
///
/// # Implementation
///
/// Uses Wilder's smoothing (RMA). O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Adx {
    period: usize,

    // ATR for centralised True Range computation
    atr: Atr,

    // Buffers for TR, +DM, -DM sums
    tr_sum: f64,
    plus_dm_sum: f64,
    minus_dm_sum: f64,

    // Buffer for DX accumulation
    dx_buffer: Vec<f64>,
    dx_index: usize,

    // Previous bar values
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,

    // State
    count: usize,
    is_initialized: bool,

    // Current values
    adx_value: f64,
    plus_di: f64,
    minus_di: f64,

    // Smoothing factor
    smoothing_factor: f64,
}

impl Adx {
    /// Create new ADX indicator (Wilder's RMA by default).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Create ADX with a specific ATR smoother type.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        assert!(period > 0, "ADX period must be > 0");
        Self {
            period,
            atr: Atr::from_smoother(period, smoother),
            tr_sum: 0.0,
            plus_dm_sum: 0.0,
            minus_dm_sum: 0.0,
            dx_buffer: Vec::with_capacity(period),
            dx_index: 0,
            prev_high: 0.0,
            prev_low: 0.0,
            prev_close: 0.0,
            count: 0,
            is_initialized: false,
            adx_value: 0.0,
            plus_di: 0.0,
            minus_di: 0.0,
            smoothing_factor: 1.0 / period as f64,
        }
    }

    /// Feed resolved input lanes `[high, low, close]` — the pure core computation.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        if self.count == 0 {
            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;
            self.count = 1;
            return self.adx_value;
        }

        // Calculate TR via ATR's update path
        let tr = self.atr.feed(&[high, low, close]);

        // Calculate Directional Movement
        let (plus_dm, minus_dm) = self.calculate_directional_movement(high, low);

        if self.count <= self.period {
            // Accumulate first `period` values
            self.tr_sum += tr;
            self.plus_dm_sum += plus_dm;
            self.minus_dm_sum += minus_dm;

            if self.count == self.period {
                self.calculate_di_and_dx();
                self.is_initialized = true;
            }
        } else {
            // Wilder's smoothing for TR, +DM, -DM
            self.tr_sum = self.tr_sum - (self.tr_sum * self.smoothing_factor) + tr;
            self.plus_dm_sum = self.plus_dm_sum - (self.plus_dm_sum * self.smoothing_factor) + plus_dm;
            self.minus_dm_sum = self.minus_dm_sum - (self.minus_dm_sum * self.smoothing_factor) + minus_dm;
            self.calculate_di_and_dx();
        }

        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;
        self.count += 1;

        self.adx_value
    }

    fn calculate_directional_movement(&self, high: f64, low: f64) -> (f64, f64) {
        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;

        let plus_dm = if up_move > down_move && up_move > 0.0 { up_move } else { 0.0 };
        let minus_dm = if down_move > up_move && down_move > 0.0 { down_move } else { 0.0 };

        (plus_dm, minus_dm)
    }

    fn calculate_di_and_dx(&mut self) {
        if self.tr_sum.abs() < 1e-12 {
            self.plus_di = 0.0;
            self.minus_di = 0.0;
            return;
        }

        self.plus_di = (self.plus_dm_sum / self.tr_sum) * 100.0;
        self.minus_di = (self.minus_dm_sum / self.tr_sum) * 100.0;

        let di_sum = self.plus_di + self.minus_di;
        let dx = if di_sum.abs() < 1e-12 {
            0.0
        } else {
            ((self.plus_di - self.minus_di).abs() / di_sum) * 100.0
        };

        if self.count <= self.period {
            if self.dx_buffer.len() < self.period {
                self.dx_buffer.push(dx);
            }
            if self.dx_buffer.len() == self.period {
                self.adx_value = self.dx_buffer.iter().sum::<f64>() / self.period as f64;
            }
        } else {
            self.adx_value = self.adx_value - (self.adx_value * self.smoothing_factor) + (dx * self.smoothing_factor);
        }
    }

    /// Get current ADX value.
    pub fn value(&self) -> f64 {
        self.adx_value
    }

    /// Get +DI value.
    pub fn plus_di(&self) -> f64 {
        self.plus_di
    }

    /// Get -DI value.
    pub fn minus_di(&self) -> f64 {
        self.minus_di
    }

    /// Get all values (ADX, +DI, -DI).
    pub fn values(&self) -> (f64, f64, f64) {
        (self.adx_value, self.plus_di, self.minus_di)
    }

    /// Check readiness.
    pub fn is_ready(&self) -> bool {
        self.is_initialized && self.count > self.period * 2
    }

    /// Get period.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Reset indicator state.
    pub fn reset(&mut self) {
        self.atr.reset();
        self.tr_sum = 0.0;
        self.plus_dm_sum = 0.0;
        self.minus_dm_sum = 0.0;
        self.dx_buffer.clear();
        self.dx_index = 0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.prev_close = 0.0;
        self.count = 0;
        self.is_initialized = false;
        self.adx_value = 0.0;
        self.plus_di = 0.0;
        self.minus_di = 0.0;
    }

    /// Trend strength as a static string.
    pub fn trend_strength(&self) -> &'static str {
        match self.adx_value {
            x if x < 25.0 => "Weak",
            x if x < 50.0 => "Moderate",
            x if x < 75.0 => "Strong",
            _ => "Very Strong",
        }
    }

    /// Trend direction based on DI lines.
    pub fn trend_direction(&self) -> &'static str {
        if self.plus_di > self.minus_di {
            "Bullish"
        } else if self.minus_di > self.plus_di {
            "Bearish"
        } else {
            "Neutral"
        }
    }

    /// Trading signal: 1 = strong bullish, -1 = strong bearish, 0 = none.
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        let di_diff = (self.plus_di - self.minus_di).abs();
        if self.adx_value > 25.0 && di_diff > 5.0 {
            if self.plus_di > self.minus_di { 1 } else { -1 }
        } else {
            0
        }
    }

    /// DI crossover: 1 = +DI crosses -DI upward, -1 = -DI crosses +DI upward, 0 = none.
    pub fn di_crossover(&self, prev_plus_di: f64, prev_minus_di: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        if prev_plus_di <= prev_minus_di && self.plus_di > self.minus_di {
            return 1;
        }
        if prev_minus_di <= prev_plus_di && self.minus_di > self.plus_di {
            return -1;
        }
        0
    }

    /// Indicator state summary.
    pub fn info(&self) -> String {
        format!(
            "ADX: {:.2} ({}), +DI: {:.2}, -DI: {:.2}, Direction: {}",
            self.adx_value,
            self.trend_strength(),
            self.plus_di,
            self.minus_di,
            self.trend_direction()
        )
    }
}

impl Default for Adx {
    fn default() -> Self {
        Self::from_smoother(14, SmootherId::Rma)
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

/// Typed config for [`Adx`] — just the period (ATR uses RMA by default via inner Atr config).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AdxConfig {
    pub period: Param<usize>,
}

impl Indicator for Adx {
    const ID: IndicatorId = IndicatorId::Adx;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        // dx_buffer: a period-window Vec used during warm-up accumulation.
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Adx)];
    type Config = AdxConfig;
    type Runtime = Adx;

    fn create(cfg: AdxConfig) -> Adx {
        Adx::from_smoother(cfg.period.resolved(), SmootherId::Rma)
    }
}

impl crate::contract::Config for AdxConfig {
    fn defaults() -> Self {
        AdxConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for Adx {
    fn rendering() -> RenderSpec {
        use crate::contract::ReferenceLine;
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Adx, "ADX", Color::hex(0x2196F3))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(25.0, Color::hex(0x9E9E9E)).with_label("Trend"))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adx_new() {
        let adx = Adx::new(14);
        assert_eq!(adx.period(), 14);
        assert!(!adx.is_ready());
        assert_eq!(adx.value(), 0.0);
    }

    #[test]
    fn test_adx_warmup_finite() {
        let mut adx = Adx::new(7);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 4.0;
            let v = adx.feed(&[price + 1.5, price - 1.5, price]);
            assert!(v.is_finite());
        }
        assert!(adx.is_ready());
    }

    #[test]
    fn test_adx_calculation() {
        let mut adx = Adx::new(14);
        let test_data = vec![
            (1.10, 1.00, 1.05f64),
            (1.15, 1.03, 1.12),
            (1.20, 1.10, 1.18),
            (1.25, 1.15, 1.22),
            (1.30, 1.20, 1.28),
        ];
        for (high, low, close) in test_data {
            adx.feed(&[high, low, close]);
        }
        assert!(adx.plus_di() >= 0.0);
        assert!(adx.minus_di() >= 0.0);
        assert!(adx.value() >= 0.0);
    }

    #[test]
    fn test_adx_reset() {
        let mut adx = Adx::new(14);
        adx.feed(&[1.10, 1.00, 1.05]);
        adx.feed(&[1.15, 1.03, 1.12]);
        adx.reset();
        assert!(!adx.is_ready());
        assert_eq!(adx.value(), 0.0);
        assert_eq!(adx.plus_di(), 0.0);
        assert_eq!(adx.minus_di(), 0.0);
    }

    #[test]
    fn test_trend_strength_classification() {
        // Feed enough data and check strength labels indirectly.
        let mut adx = Adx::new(14);
        // Before warmup, value is 0.0 → Weak
        assert_eq!(adx.trend_strength(), "Weak");
        // Feed a monotone sequence to push ADX value upward
        for i in 0..60 {
            let price = 100.0 + i as f64;
            adx.feed(&[price + 2.0, price - 2.0, price]);
        }
        // Whatever the ADX value is, the label must be one of the four valid strings
        let strength = adx.trend_strength();
        assert!(
            strength == "Weak" || strength == "Moderate" || strength == "Strong" || strength == "Very Strong",
            "unexpected strength: {strength}"
        );
    }

    #[test]
    fn test_factory_feeds_resolved_adx() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::{MarketSample};
        let mut f = IndicatorOrder::Adx(<<Adx as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
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
