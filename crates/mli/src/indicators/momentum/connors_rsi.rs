//! Connors RSI indicator.

use crate::indicators::momentum::rsi::Rsi;
use crate::indicators::utils::math::percentile::percentile_rank;

/// Connors RSI result containing all component values.
#[derive(Debug, Clone, Copy)]
pub struct ConnorsRsiResult {
    /// Final Connors RSI value (0-100).
    pub connors_rsi: f64,
    /// RSI component (0-100).
    pub rsi_component: f64,
    /// RSI of UpDown Length (0-100).
    pub updown_rsi: f64,
    /// ROC Percentile (0-100).
    pub roc_percentile: f64,
    /// Current streak length (positive = up, negative = down).
    pub updown_length: i32,
}

impl ConnorsRsiResult {
    /// Creates an empty result with neutral values.
    pub fn empty() -> Self {
        Self {
            connors_rsi: 50.0,
            rsi_component: 50.0,
            updown_rsi: 50.0,
            roc_percentile: 50.0,
            updown_length: 0,
        }
    }

    /// Returns the current market condition.
    pub fn market_condition(&self) -> &'static str {
        match self.connors_rsi {
            x if x <= 10.0 => "Extremely Oversold",
            x if x <= 20.0 => "Strongly Oversold",
            x if x <= 30.0 => "Oversold",
            x if x <= 40.0 => "Mildly Oversold",
            x if x <= 60.0 => "Neutral",
            x if x <= 70.0 => "Mildly Overbought",
            x if x <= 80.0 => "Overbought",
            x if x <= 90.0 => "Strongly Overbought",
            _ => "Extremely Overbought",
        }
    }
}

/// Connors RSI - enhanced RSI indicator by Larry Connors.
///
/// Connors RSI = (RSI + RSI of UpDown Length + ROC Percentile) / 3
///
/// Components:
/// 1. RSI(3) - standard RSI with period 3
/// 2. RSI of UpDown Length - RSI of consecutive up/down day count
/// 3. ROC Percentile(100) - percentile rank of ROC over 100 periods
///
/// More sensitive to short-term changes and better at identifying
/// extreme overbought/oversold conditions.
///
/// Interpretation:
/// - CRSI <= 10: Extremely oversold (buy)
/// - CRSI <= 20: Strongly oversold
/// - CRSI >= 80: Strongly overbought
/// - CRSI >= 90: Extremely overbought (sell)
///
/// # Parameters
/// - `rsi_period`: RSI calculation period (typically 3)
/// - `updown_period`: UpDown RSI period (typically 2)
/// - `roc_period`: ROC percentile lookback (typically 100)
///
/// # Implementation
///
/// Combines three normalized components. O(1) per update with rolling buffers.
#[derive(Debug, Clone)]
pub struct ConnorsRsi {
    updown_period: usize,
    roc_period: usize,

    rsi: Rsi,

    prices: Vec<f64>,

    updown_lengths: Vec<i32>,
    current_updown_length: i32,
    last_direction: i8,

    updown_gains: Vec<f64>,
    updown_losses: Vec<f64>,
    updown_avg_gain: f64,
    updown_avg_loss: f64,

    roc_values: Vec<f64>,

    current_result: ConnorsRsiResult,

    is_ready: bool,
    update_count: usize,
}

impl ConnorsRsi {
    /// Creates a new Connors RSI with default parameters (3, 2, 100).
    pub fn new() -> Self {
        Self::with_periods(3, 2, 100)
    }

    /// Creates a new Connors RSI with custom parameters.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI calculation period (typically 3)
    /// * `updown_period` - UpDown RSI period (typically 2)
    /// * `roc_period` - ROC percentile lookback (typically 100)
    pub fn with_periods(rsi_period: usize, updown_period: usize, roc_period: usize) -> Self {
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        assert!(updown_period > 0, "UpDown period must be greater than 0");
        assert!(roc_period > 0, "ROC period must be greater than 0");

        Self {
            updown_period,
            roc_period,
            rsi: Rsi::new(rsi_period),
            prices: Vec::with_capacity(roc_period),
            updown_lengths: Vec::with_capacity(updown_period),
            current_updown_length: 0,
            last_direction: 0,
            updown_gains: Vec::with_capacity(32),
            updown_losses: Vec::with_capacity(32),
            updown_avg_gain: 0.0,
            updown_avg_loss: 0.0,
            roc_values: Vec::with_capacity(roc_period),
            current_result: ConnorsRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed a pre-resolved scalar price — the contracted entry point.
    pub fn feed(&mut self, price: f64) -> ConnorsRsiResult {
        self.update_price(price)
    }

    /// Updates the indicator with a new price.
    pub fn update_price(&mut self, price: f64) -> ConnorsRsiResult {
        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.prices.len() >= 2 {
            // 1. Calculate RSI component
            let rsi_component = self.calculate_rsi_component();

            // 2. Calculate UpDown Length and its RSI
            let updown_rsi = self.calculate_updown_rsi();

            // 3. Calculate ROC Percentile
            let roc_percentile = self.calculate_roc_percentile();

            // 4. Combine components
            let connors_rsi = (rsi_component + updown_rsi + roc_percentile) / 3.0;

            // Update result
            self.current_result = ConnorsRsiResult {
                connors_rsi,
                rsi_component,
                updown_rsi,
                roc_percentile,
                updown_length: self.current_updown_length,
            };

            // Check readiness
            if self.prices.len() >= self.rsi.period().max(self.roc_period) {
                self.is_ready = true;
            }
        }

        self.update_count += 1;
        self.current_result
    }

    /// Calculates the RSI component.
    fn calculate_rsi_component(&mut self) -> f64 {
        if self.prices.is_empty() {
            return 50.0;
        }

        let len = self.prices.len();
        let current_price = self.prices[len - 1];

        // Update standard RSI - already returns 0-100
        self.rsi.feed(current_price)
    }

    /// Calculates the UpDown Length and its RSI.
    fn calculate_updown_rsi(&mut self) -> f64 {
        if self.prices.len() < 2 {
            return 50.0;
        }

        let len = self.prices.len();
        let current_price = self.prices[len - 1];
        let prev_price = self.prices[len - 2];

        // Determine direction
        let current_direction = if current_price > prev_price {
            1  // Up
        } else if current_price < prev_price {
            -1 // Down
        } else {
            0  // Unchanged
        };

        // Update streak length
        if current_direction != 0 {
            if current_direction == self.last_direction {
                // Continuation of streak
                if self.current_updown_length > 0 && current_direction == 1 {
                    self.current_updown_length += 1;
                } else if self.current_updown_length < 0 && current_direction == -1 {
                    self.current_updown_length -= 1;
                } else {
                    // Direction change
                    self.current_updown_length = current_direction as i32;
                }
            } else {
                // Direction change
                self.current_updown_length = current_direction as i32;
            }
            self.last_direction = current_direction;
        }

        // Add length to buffer
        if self.updown_lengths.len() >= 512 {
            self.updown_lengths.remove(0);
        }
        self.updown_lengths.push(self.current_updown_length);

        // Calculate RSI from UpDown Length
        if self.updown_lengths.len() >= 2 {
            let current_length = self.current_updown_length as f64;
            let prev_length = if self.updown_lengths.len() >= 2 {
                self.updown_lengths[self.updown_lengths.len() - 2] as f64
            } else {
                0.0
            };

            let change = current_length - prev_length;
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { -change } else { 0.0 };

            // Add to UpDown RSI buffers
            if self.updown_gains.len() >= self.updown_period {
                self.updown_gains.remove(0);
            }
            self.updown_gains.push(gain);

            if self.updown_losses.len() >= self.updown_period {
                self.updown_losses.remove(0);
            }
            self.updown_losses.push(loss);

            // Calculate averages
            if self.updown_gains.len() == self.updown_period {
                if self.updown_avg_gain == 0.0 && self.updown_avg_loss == 0.0 {
                    self.updown_avg_gain = self.updown_gains.iter().sum::<f64>() / self.updown_period as f64;
                    self.updown_avg_loss = self.updown_losses.iter().sum::<f64>() / self.updown_period as f64;
                } else {
                    let alpha = 1.0 / self.updown_period as f64;
                    self.updown_avg_gain = alpha * gain + (1.0 - alpha) * self.updown_avg_gain;
                    self.updown_avg_loss = alpha * loss + (1.0 - alpha) * self.updown_avg_loss;
                }

                // Calculate RSI
                if self.updown_avg_loss == 0.0 {
                    return 100.0;
                }

                let rs = self.updown_avg_gain / self.updown_avg_loss;
                return 100.0 - (100.0 / (1.0 + rs));
            }
        }

        50.0
    }

    /// Calculates the ROC Percentile.
    fn calculate_roc_percentile(&mut self) -> f64 {
        if self.prices.len() < 2 {
            return 50.0;
        }

        let len = self.prices.len();
        let current_price = self.prices[len - 1];
        let prev_price = self.prices[len - 2];

        // Calculate ROC (Rate of Change)
        let roc = if prev_price != 0.0 {
            ((current_price - prev_price) / prev_price) * 100.0
        } else {
            0.0
        };

        // Add ROC to buffer
        if self.roc_values.len() >= self.roc_period {
            self.roc_values.remove(0);
        }
        self.roc_values.push(roc);

        // Calculate percentile
        if self.roc_values.len() >= self.roc_period {
            percentile_rank(&self.roc_values, roc)
        } else {
            50.0
        }
    }

    /// Returns the current Connors RSI value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.current_result.connors_rsi
    }

    /// Returns the full result with all components.
    #[inline]
    pub fn result(&self) -> ConnorsRsiResult {
        self.current_result
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.prices.clear();
        self.rsi.reset();
        self.updown_lengths.clear();
        self.current_updown_length = 0;
        self.last_direction = 0;
        self.updown_gains.clear();
        self.updown_losses.clear();
        self.updown_avg_gain = 0.0;
        self.updown_avg_loss = 0.0;
        self.roc_values.clear();
        self.current_result = ConnorsRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    /// Returns the RSI period.
    pub fn period(&self) -> usize {
        self.rsi.period()
    }

    /// Returns the trading signal: -1 (sell), 0 (none), 1 (buy).
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        let crsi = self.current_result.connors_rsi;
        if crsi <= 10.0 {
            return 1;
        } else if crsi >= 90.0 {
            return -1;
        }
        0
    }

    /// Returns the advanced trading signal using all three components.
    pub fn advanced_trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        let result = self.current_result;
        let rsi_oversold = result.rsi_component <= 20.0;
        let rsi_overbought = result.rsi_component >= 80.0;
        let updown_extreme = result.updown_rsi <= 15.0 || result.updown_rsi >= 85.0;
        if result.connors_rsi <= 15.0 && rsi_oversold && updown_extreme && result.roc_percentile <= 20.0 {
            return 1;
        }
        if result.connors_rsi >= 85.0 && rsi_overbought && updown_extreme && result.roc_percentile >= 80.0 {
            return -1;
        }
        if result.connors_rsi <= 25.0 {
            return 1;
        } else if result.connors_rsi >= 75.0 {
            return -1;
        }
        0
    }

    /// Returns a human-readable status string.
    pub fn info(&self) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal() {
            1 => "Buy",
            -1 => "Sell",
            _ => "No signal",
        };
        format!(
            "Connors RSI: {:.1} ({}), RSI: {:.1}, UpDown RSI: {:.1}, ROC%: {:.1}, UpDown Length: {}, Signal: {}",
            result.connors_rsi,
            result.market_condition(),
            result.rsi_component,
            result.updown_rsi,
            result.roc_percentile,
            result.updown_length,
            signal
        )
    }

    /// Returns all component values.
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("connors_rsi".to_string(), self.current_result.connors_rsi);
        values.insert("rsi_component".to_string(), self.current_result.rsi_component);
        values.insert("updown_rsi".to_string(), self.current_result.updown_rsi);
        values.insert("roc_percentile".to_string(), self.current_result.roc_percentile);
        values.insert("updown_length".to_string(), self.current_result.updown_length as f64);
        values
    }

    /// Returns the number of bars processed.
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Returns (rsi_period, updown_period, roc_period).
    pub fn parameters(&self) -> (usize, usize, usize) {
        (self.rsi.period(), self.updown_period, self.roc_period)
    }
}

impl Default for ConnorsRsi {
    fn default() -> Self {
        Self::with_periods(3, 2, 100)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, StoreKind, Store, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, LineStyle, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`ConnorsRsi`] — period triplet + source + RSI smoother slot.
///
/// `rsi_smoother` is a `#[slot]` that defaults to `follow(Rma)` (Wilder's RMA riding the
/// `rsi_period` axis). The UpDown RSI and ROC percentile are period-only parameters.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ConnorsRsiConfig {
    /// RSI component period (typically 3).
    pub rsi_period: Param<usize>,
    /// UpDown streak RSI period (typically 2).
    pub updown_period: Param<usize>,
    /// ROC percentile lookback (typically 100).
    pub roc_period: Param<usize>,
    /// Configurable price source (default close).
    pub source: Param<OhlcvField>,
    /// RSI gain/loss smoother choice — default `follow(Rma)` (Wilder's RMA at `rsi_period`).
    #[slot]
    pub rsi_smoother: Param<SmootherChoice>,
}

impl Indicator for ConnorsRsi {
    const ID: IndicatorId = IndicatorId::ConnorsRsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Multiple rolling Vec buffers; inner RSI node declared as a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[
            Store::window(StoreKind::Vec),  // prices
            Store::window(StoreKind::Vec),  // roc_values
            Store::window(StoreKind::Vec),  // updown_lengths
            Store::window(StoreKind::Vec),  // updown_gains
            Store::window(StoreKind::Vec),  // updown_losses
        ],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [Slot] = ConnorsRsiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::ConnorsRsi)];

    type Config = ConnorsRsiConfig;
    type Runtime = ConnorsRsi;

    fn create(cfg: ConnorsRsiConfig) -> ConnorsRsi {
        let rsi_period = cfg.rsi_period.resolved();
        let updown_period = cfg.updown_period.resolved();
        let roc_period = cfg.roc_period.resolved();
        let choice = cfg.rsi_smoother.resolved();
        assert!(rsi_period > 0 && updown_period > 0 && roc_period > 0);
        let rsi = Rsi::from_choice(choice, rsi_period);
        ConnorsRsi {
            updown_period,
            roc_period,
            rsi,
            prices: Vec::with_capacity(roc_period),
            updown_lengths: Vec::with_capacity(updown_period),
            current_updown_length: 0,
            last_direction: 0,
            updown_gains: Vec::with_capacity(32),
            updown_losses: Vec::with_capacity(32),
            updown_avg_gain: 0.0,
            updown_avg_loss: 0.0,
            roc_values: Vec::with_capacity(roc_period),
            current_result: ConnorsRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    fn source_fields(cfg: &ConnorsRsiConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &ConnorsRsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ConnorsRsiConfig {
    fn defaults() -> Self {
        ConnorsRsiConfig {
            rsi_period: Param::Solo(3),
            updown_period: Param::Solo(2),
            roc_period: Param::Solo(100),
            source: Param::Solo(OhlcvField::Close),
            rsi_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/updown_period/roc_period: Class A → auto range(2,4048,1).
        // source: Class O → auto all-8.
        // rsi_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for ConnorsRsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ConnorsRsi, "Connors RSI", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0xF44336)).with_style(LineStyle::Dashed))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x4CAF50)).with_style(LineStyle::Dashed))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connors_rsi_creation() {
        let crsi = ConnorsRsi::new();
        assert!(!crsi.is_ready());
        assert_eq!(crsi.parameters(), (3, 2, 100));
        assert_eq!(crsi.value(), 50.0);
    }

    #[test]
    fn test_connors_rsi_with_periods() {
        let crsi = ConnorsRsi::with_periods(5, 3, 50);
        assert_eq!(crsi.parameters(), (5, 3, 50));
    }

    #[test]
    fn test_connors_rsi_update() {
        let mut crsi = ConnorsRsi::new();

        // Ascending prices
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            let result = crsi.feed(price);

            if i > 10 {
                assert!(result.connors_rsi >= 0.0 && result.connors_rsi <= 100.0);
                assert!(result.rsi_component >= 0.0 && result.rsi_component <= 100.0);
                assert!(result.updown_rsi >= 0.0 && result.updown_rsi <= 100.0);
                assert!(result.roc_percentile >= 0.0 && result.roc_percentile <= 100.0);
            }
        }

        // With ascending prices Connors RSI should be high
        assert!(crsi.value() > 70.0);
    }

    #[test]
    fn test_updown_length() {
        let mut crsi = ConnorsRsi::new();

        for i in 1..=5 {
            crsi.feed(100.0 + i as f64);
        }
        assert!(crsi.current_result.updown_length > 0);

        for i in 1..=5 {
            crsi.feed(105.0 - i as f64);
        }
        assert!(crsi.current_result.updown_length < 0);
    }

    #[test]
    fn test_trading_signals() {
        let mut crsi = ConnorsRsi::new();

        let mut price = 100.0;
        for _ in 0..15 {
            price -= 1.0;
            crsi.feed(price);
        }

        if crsi.is_ready() {
            let signal = crsi.trading_signal();
            assert!(signal >= -1 && signal <= 1);
        }
    }

    #[test]
    fn test_market_condition() {
        let result = ConnorsRsiResult {
            connors_rsi: 15.0,
            rsi_component: 20.0,
            updown_rsi: 10.0,
            roc_percentile: 15.0,
            updown_length: -3,
        };
        assert_eq!(result.market_condition(), "Strongly Oversold");

        let result2 = ConnorsRsiResult {
            connors_rsi: 85.0,
            rsi_component: 80.0,
            updown_rsi: 90.0,
            roc_percentile: 85.0,
            updown_length: 4,
        };
        assert_eq!(result2.market_condition(), "Strongly Overbought");
    }

    #[test]
    fn test_connors_rsi_reset() {
        let mut crsi = ConnorsRsi::new();
        for i in 1..=20 {
            crsi.feed(100.0 + i as f64);
        }
        crsi.reset();
        assert!(!crsi.is_ready());
        assert_eq!(crsi.value(), 50.0);
    }

    #[test]
    fn test_connors_rsi_period() {
        let crsi = ConnorsRsi::new();
        assert_eq!(crsi.period(), 3);
    }

    #[test]
    fn test_connors_rsi_richer_ctor() {
        let mut crsi = ConnorsRsi::with_periods(5, 3, 50);
        assert_eq!(crsi.parameters(), (5, 3, 50));
        for i in 1..=60 {
            let p = 100.0 + i as f64 * 0.5;
            let r = crsi.feed(p);
            assert!(r.connors_rsi.is_finite());
        }
        assert!(crsi.is_ready());
    }

    /// Factory resolves the close field (not the wild 9999 high) and feeds the scalar.
    /// With ascending close prices Connors RSI climbs above the neutral 50.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::{MarketSample};
        let cfg = <<ConnorsRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::ConnorsRsi(cfg).build_solo().unwrap();
        for i in 1..=120 {
            let price = 100.0 + i as f64 * 0.5;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        // Ascending close → CRSI should be well above neutral
        assert!(
            f.primary() > 50.0,
            "factory ConnorsRsi on ascending close should be > 50, got {}",
            f.primary()
        );
    }
}
