//! Signal Detectors - utilities for detecting signals
//!
//! Detectors are stateful objects that track state and emit signals when
//! conditions are met. Each `update()` returns `Option<(SignalKind, Direction)>`.
//! Direction is always determined by the detector, never derived from kind.

use crate::signals::{
    SignalKind, ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, CompositeSub,
    CrossoverType, DivergenceType, ChannelPosition, VolatilityRegime, VolumeCharacter,
};
use crate::signals::signal::Direction;
use arrayvec::ArrayVec;

// ============================================================================
// CROSSOVER DETECTOR
// ============================================================================

/// Detects crossovers between two lines or a line and a level.
#[derive(Debug, Clone)]
pub struct CrossoverDetector {
    prev_a: Option<f64>,
    prev_b: Option<f64>,
    #[allow(dead_code)]
    use_level: bool,
    level: f64,
}

impl CrossoverDetector {
    /// Create a detector for crossover between two lines.
    pub fn new() -> Self {
        Self {
            prev_a: None,
            prev_b: None,
            use_level: false,
            level: 0.0,
        }
    }

    /// Create a detector for crossover with a fixed level.
    pub fn with_level(level: f64) -> Self {
        Self {
            prev_a: None,
            prev_b: None,
            use_level: true,
            level,
        }
    }

    /// Update with two lines. Returns `(Crossover, Up)` or `(Crossover, Down)`.
    pub fn update(&mut self, a: f64, b: f64) -> Option<(SignalKind, Direction)> {
        let result = if let (Some(pa), Some(pb)) = (self.prev_a, self.prev_b) {
            if CrossoverType::CrossUp.check(pa, a, pb, b) {
                Some((SignalKind::Crossover, Direction::Up))
            } else if CrossoverType::CrossDown.check(pa, a, pb, b) {
                Some((SignalKind::Crossover, Direction::Down))
            } else {
                None
            }
        } else {
            None
        };

        self.prev_a = Some(a);
        self.prev_b = Some(b);
        result
    }

    /// Update with a single value vs. the stored level.
    pub fn update_level(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let result = if let Some(prev) = self.prev_a {
            if CrossoverType::CrossUp.check_level(prev, value, self.level) {
                Some((SignalKind::Crossover, Direction::Up))
            } else if CrossoverType::CrossDown.check_level(prev, value, self.level) {
                Some((SignalKind::Crossover, Direction::Down))
            } else {
                None
            }
        } else {
            None
        };

        self.prev_a = Some(value);
        result
    }

    pub fn reset(&mut self) {
        self.prev_a = None;
        self.prev_b = None;
    }
}

impl Default for CrossoverDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// THRESHOLD MONITOR
// ============================================================================

/// Monitors overbought/oversold zone entry and exit.
#[derive(Debug, Clone)]
pub struct ThresholdMonitor {
    upper: f64,
    lower: f64,
    prev_value: Option<f64>,
    in_upper_zone: bool,
    in_lower_zone: bool,
}

impl ThresholdMonitor {
    /// Create a monitor with upper/lower zones (e.g. 70/30 for RSI).
    pub fn new(upper: f64, lower: f64) -> Self {
        Self {
            upper,
            lower,
            prev_value: None,
            in_upper_zone: false,
            in_lower_zone: false,
        }
    }

    /// Update and check zone transitions.
    ///
    /// - Entering upper zone → `(Threshold(Enter), Up)`   (value went UP into overbought)
    /// - Exiting upper zone  → `(Threshold(Exit), Down)`  (value went DOWN out of overbought)
    /// - Entering lower zone → `(Threshold(Enter), Down)` (value went DOWN into oversold)
    /// - Exiting lower zone  → `(Threshold(Exit), Up)`    (value went UP out of oversold)
    pub fn update(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let result = if let Some(_prev) = self.prev_value {
            if !self.in_upper_zone && value > self.upper {
                self.in_upper_zone = true;
                Some((SignalKind::Threshold(ThresholdSub::Enter), Direction::Up))
            } else if self.in_upper_zone && value < self.upper {
                self.in_upper_zone = false;
                Some((SignalKind::Threshold(ThresholdSub::Exit), Direction::Down))
            } else if !self.in_lower_zone && value < self.lower {
                self.in_lower_zone = true;
                Some((SignalKind::Threshold(ThresholdSub::Enter), Direction::Down))
            } else if self.in_lower_zone && value > self.lower {
                self.in_lower_zone = false;
                Some((SignalKind::Threshold(ThresholdSub::Exit), Direction::Up))
            } else {
                None
            }
        } else {
            // Initialize state
            self.in_upper_zone = value > self.upper;
            self.in_lower_zone = value < self.lower;
            None
        };

        self.prev_value = Some(value);
        result
    }

    pub fn is_overbought(&self) -> bool {
        self.in_upper_zone
    }

    pub fn is_oversold(&self) -> bool {
        self.in_lower_zone
    }

    pub fn reset(&mut self) {
        self.prev_value = None;
        self.in_upper_zone = false;
        self.in_lower_zone = false;
    }
}

// ============================================================================
// ZERO CROSS DETECTOR
// ============================================================================

/// Detects zero-line crossings (MACD, CCI, etc.).
#[derive(Debug, Clone)]
pub struct ZeroCrossDetector {
    prev_value: Option<f64>,
    tolerance: f64,
}

impl ZeroCrossDetector {
    pub fn new() -> Self {
        Self {
            prev_value: None,
            tolerance: 0.0,
        }
    }

    pub fn with_tolerance(tolerance: f64) -> Self {
        Self {
            prev_value: None,
            tolerance,
        }
    }

    pub fn update(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let result = if let Some(prev) = self.prev_value {
            if prev <= self.tolerance && value > self.tolerance {
                Some((SignalKind::ZeroCross, Direction::Up))
            } else if prev >= -self.tolerance && value < -self.tolerance {
                Some((SignalKind::ZeroCross, Direction::Down))
            } else {
                None
            }
        } else {
            None
        };

        self.prev_value = Some(value);
        result
    }

    pub fn reset(&mut self) {
        self.prev_value = None;
    }
}

impl Default for ZeroCrossDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HISTOGRAM DETECTOR
// ============================================================================

/// Detects histogram sign changes and momentum shifts.
#[derive(Debug, Clone)]
pub struct HistogramDetector {
    prev_value: Option<f64>,
    prev_prev_value: Option<f64>,
}

impl HistogramDetector {
    pub fn new() -> Self {
        Self {
            prev_value: None,
            prev_prev_value: None,
        }
    }

    pub fn update(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let result = if let Some(prev) = self.prev_value {
            // Sign change
            if prev <= 0.0 && value > 0.0 {
                Some((SignalKind::Histogram(HistogramSub::SignChange), Direction::Up))
            } else if prev >= 0.0 && value < 0.0 {
                Some((SignalKind::Histogram(HistogramSub::SignChange), Direction::Down))
            }
            // Direction change (momentum shift)
            else if let Some(prev_prev) = self.prev_prev_value {
                let prev_diff = prev - prev_prev;
                let curr_diff = value - prev;

                if prev_diff < 0.0 && curr_diff > 0.0 && value > 0.0 {
                    Some((SignalKind::Histogram(HistogramSub::MomentumShift), Direction::Up))
                } else if prev_diff > 0.0 && curr_diff < 0.0 && value < 0.0 {
                    Some((SignalKind::Histogram(HistogramSub::MomentumShift), Direction::Down))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        self.prev_prev_value = self.prev_value;
        self.prev_value = Some(value);
        result
    }

    pub fn reset(&mut self) {
        self.prev_value = None;
        self.prev_prev_value = None;
    }
}

impl Default for HistogramDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CHANNEL DETECTOR
// ============================================================================

/// Detects channel events: touch, break, reenter, mid-cross (Bollinger, Keltner, etc.).
#[derive(Debug, Clone)]
pub struct ChannelDetector {
    prev_position: Option<ChannelPosition>,
    tolerance: f64,
}

impl ChannelDetector {
    pub fn new(tolerance: f64) -> Self {
        Self {
            prev_position: None,
            tolerance,
        }
    }

    pub fn update(&mut self, value: f64, upper: f64, lower: f64) -> Option<(SignalKind, Direction)> {
        let position = ChannelPosition::determine(value, upper, lower, self.tolerance);

        let result = if let Some(prev_pos) = self.prev_position {
            match (prev_pos, position) {
                // Touch upper/lower boundary
                (ChannelPosition::UpperHalf, ChannelPosition::AtUpper) => {
                    Some((SignalKind::Channel(ChannelSub::Touch), Direction::Up))
                }
                (ChannelPosition::LowerHalf, ChannelPosition::AtLower) => {
                    Some((SignalKind::Channel(ChannelSub::Touch), Direction::Down))
                }
                // Break above/below
                (ChannelPosition::AtUpper | ChannelPosition::UpperHalf, ChannelPosition::AboveUpper) => {
                    Some((SignalKind::Channel(ChannelSub::Break), Direction::Up))
                }
                (ChannelPosition::AtLower | ChannelPosition::LowerHalf, ChannelPosition::BelowLower) => {
                    Some((SignalKind::Channel(ChannelSub::Break), Direction::Down))
                }
                // Reenter from above (price came back down)
                (ChannelPosition::AboveUpper, ChannelPosition::AtUpper | ChannelPosition::UpperHalf) => {
                    Some((SignalKind::Channel(ChannelSub::Reenter), Direction::Down))
                }
                // Reenter from below (price came back up)
                (ChannelPosition::BelowLower, ChannelPosition::AtLower | ChannelPosition::LowerHalf) => {
                    Some((SignalKind::Channel(ChannelSub::Reenter), Direction::Up))
                }
                // Mid cross up / down
                (ChannelPosition::LowerHalf | ChannelPosition::AtLower, ChannelPosition::UpperHalf) => {
                    Some((SignalKind::Channel(ChannelSub::MidCross), Direction::Up))
                }
                (ChannelPosition::UpperHalf | ChannelPosition::AtUpper, ChannelPosition::LowerHalf) => {
                    Some((SignalKind::Channel(ChannelSub::MidCross), Direction::Down))
                }
                _ => None,
            }
        } else {
            None
        };

        self.prev_position = Some(position);
        result
    }

    pub fn current_position(&self) -> Option<ChannelPosition> {
        self.prev_position
    }

    pub fn reset(&mut self) {
        self.prev_position = None;
    }
}

// ============================================================================
// DIVERGENCE DETECTOR
// ============================================================================

/// Extremum point used internally by the divergence detector.
#[derive(Debug, Clone, Copy)]
struct ExtremumPoint {
    bar_index: usize,
    price: f64,
    indicator: f64,
    is_high: bool,
}

/// Detects divergences between price and an indicator.
#[derive(Debug, Clone)]
pub struct DivergenceDetector {
    extremums: ArrayVec<ExtremumPoint, 32>,
    min_distance: usize,
    lookback: usize,
    bar_index: usize,
    prev_price: Option<f64>,
    prev_prev_price: Option<f64>,
    prev_indicator: Option<f64>,
    prev_prev_indicator: Option<f64>,
}

impl DivergenceDetector {
    pub fn new(min_distance: usize, lookback: usize) -> Self {
        Self {
            extremums: ArrayVec::new(),
            min_distance: min_distance.max(2),
            lookback: lookback.max(10),
            bar_index: 0,
            prev_price: None,
            prev_prev_price: None,
            prev_indicator: None,
            prev_prev_indicator: None,
        }
    }

    pub fn update(&mut self, price: f64, indicator: f64) -> Option<(SignalKind, Direction)> {
        let mut result = None;

        if let (Some(prev_prev_ind), Some(prev_ind)) = (self.prev_prev_indicator, self.prev_indicator) {
            if let (Some(_prev_prev_price), Some(prev_price)) = (self.prev_prev_price, self.prev_price) {
                // Local indicator high
                if prev_ind > prev_prev_ind && prev_ind > indicator {
                    let point = ExtremumPoint {
                        bar_index: self.bar_index - 1,
                        price: prev_price,
                        indicator: prev_ind,
                        is_high: true,
                    };

                    if let Some(prev_high) = self.find_previous_extremum(true) {
                        if self.bar_index - 1 - prev_high.bar_index >= self.min_distance {
                            if DivergenceType::Bearish.check(
                                prev_high.price, point.price,
                                prev_high.indicator, point.indicator,
                            ) {
                                result = Some((SignalKind::Divergence(DivergenceSub::Regular), Direction::Down));
                            } else if DivergenceType::HiddenBearish.check(
                                prev_high.price, point.price,
                                prev_high.indicator, point.indicator,
                            ) {
                                result = Some((SignalKind::Divergence(DivergenceSub::Hidden), Direction::Down));
                            }
                        }
                    }

                    self.add_extremum(point);
                }
                // Local indicator low
                else if prev_ind < prev_prev_ind && prev_ind < indicator {
                    let point = ExtremumPoint {
                        bar_index: self.bar_index - 1,
                        price: prev_price,
                        indicator: prev_ind,
                        is_high: false,
                    };

                    if let Some(prev_low) = self.find_previous_extremum(false) {
                        if self.bar_index - 1 - prev_low.bar_index >= self.min_distance {
                            if DivergenceType::Bullish.check(
                                prev_low.price, point.price,
                                prev_low.indicator, point.indicator,
                            ) {
                                result = Some((SignalKind::Divergence(DivergenceSub::Regular), Direction::Up));
                            } else if DivergenceType::HiddenBullish.check(
                                prev_low.price, point.price,
                                prev_low.indicator, point.indicator,
                            ) {
                                result = Some((SignalKind::Divergence(DivergenceSub::Hidden), Direction::Up));
                            }
                        }
                    }

                    self.add_extremum(point);
                }
            }
        }

        self.prev_prev_price = self.prev_price;
        self.prev_price = Some(price);
        self.prev_prev_indicator = self.prev_indicator;
        self.prev_indicator = Some(indicator);
        self.bar_index += 1;

        self.cleanup_old_extremums();

        result
    }

    fn find_previous_extremum(&self, is_high: bool) -> Option<&ExtremumPoint> {
        self.extremums.iter().rev().find(|e| e.is_high == is_high)
    }

    fn add_extremum(&mut self, point: ExtremumPoint) {
        if self.extremums.is_full() {
            self.extremums.remove(0);
        }
        self.extremums.push(point);
    }

    fn cleanup_old_extremums(&mut self) {
        let cutoff = self.bar_index.saturating_sub(self.lookback);
        self.extremums.retain(|e| e.bar_index >= cutoff);
    }

    pub fn reset(&mut self) {
        self.extremums.clear();
        self.bar_index = 0;
        self.prev_price = None;
        self.prev_prev_price = None;
        self.prev_indicator = None;
        self.prev_prev_indicator = None;
    }
}

// ============================================================================
// TREND DETECTOR
// ============================================================================

/// Detects Golden Cross, Death Cross, and price-vs-MA crossings.
#[derive(Debug, Clone)]
pub struct TrendDetector {
    fast_prev: Option<f64>,
    slow_prev: Option<f64>,
    price_above_fast: bool,
    price_above_slow: bool,
}

impl TrendDetector {
    pub fn new() -> Self {
        Self {
            fast_prev: None,
            slow_prev: None,
            price_above_fast: false,
            price_above_slow: false,
        }
    }

    /// Update with price, fast MA, and slow MA.
    pub fn update(&mut self, price: f64, fast_ma: f64, slow_ma: f64) -> Option<(SignalKind, Direction)> {
        let result = if let (Some(fp), Some(sp)) = (self.fast_prev, self.slow_prev) {
            // Golden Cross: fast crosses above slow
            if fp <= sp && fast_ma > slow_ma {
                Some((SignalKind::Trend(TrendSub::MaCross), Direction::Up))
            }
            // Death Cross: fast crosses below slow
            else if fp >= sp && fast_ma < slow_ma {
                Some((SignalKind::Trend(TrendSub::MaCross), Direction::Down))
            }
            // Price crosses above fast MA
            else if !self.price_above_fast && price > fast_ma {
                self.price_above_fast = true;
                Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Up))
            }
            // Price crosses below fast MA
            else if self.price_above_fast && price < fast_ma {
                self.price_above_fast = false;
                Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Down))
            } else {
                None
            }
        } else {
            // Initialize
            self.price_above_fast = price > fast_ma;
            self.price_above_slow = price > slow_ma;
            None
        };

        self.fast_prev = Some(fast_ma);
        self.slow_prev = Some(slow_ma);
        result
    }

    pub fn reset(&mut self) {
        self.fast_prev = None;
        self.slow_prev = None;
        self.price_above_fast = false;
        self.price_above_slow = false;
    }
}

impl Default for TrendDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VOLATILITY DETECTOR
// ============================================================================

/// Detects volatility regime changes: breakout, extreme, shift, squeeze.
#[derive(Debug, Clone)]
pub struct VolatilityDetector {
    prev_regime: Option<VolatilityRegime>,
    prev_value: Option<f64>,
    mean: f64,
    std: f64,
    squeeze_threshold: f64,
}

impl VolatilityDetector {
    pub fn new(mean: f64, std: f64, squeeze_threshold: f64) -> Self {
        Self {
            prev_regime: None,
            prev_value: None,
            mean,
            std,
            squeeze_threshold,
        }
    }

    pub fn update(&mut self, volatility: f64) -> Option<(SignalKind, Direction)> {
        let zscore = if self.std > 0.0 {
            (volatility - self.mean) / self.std
        } else {
            0.0
        };

        let regime = VolatilityRegime::from_zscore(zscore);

        let result = if let Some(prev_regime) = self.prev_regime {
            match (prev_regime, regime) {
                (
                    VolatilityRegime::VeryLow | VolatilityRegime::Low,
                    VolatilityRegime::High | VolatilityRegime::VeryHigh,
                ) => Some((SignalKind::Volatility(VolatilitySub::Breakout), Direction::Up)),

                (
                    VolatilityRegime::Normal | VolatilityRegime::High,
                    VolatilityRegime::VeryLow,
                ) => Some((SignalKind::Volatility(VolatilitySub::Extreme), Direction::Down)),

                (
                    VolatilityRegime::Normal,
                    VolatilityRegime::VeryHigh,
                ) => Some((SignalKind::Volatility(VolatilitySub::Extreme), Direction::Up)),

                _ if zscore < -self.squeeze_threshold => {
                    Some((SignalKind::Volatility(VolatilitySub::Squeeze), Direction::Neutral))
                }

                _ => {
                    if let Some(pv) = self.prev_value {
                        if volatility > pv * 1.2 {
                            Some((SignalKind::Volatility(VolatilitySub::Shift), Direction::Up))
                        } else if volatility < pv * 0.8 {
                            Some((SignalKind::Volatility(VolatilitySub::Shift), Direction::Down))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };

        self.prev_regime = Some(regime);
        self.prev_value = Some(volatility);
        result
    }

    pub fn update_stats(&mut self, mean: f64, std: f64) {
        self.mean = mean;
        self.std = std;
    }

    pub fn reset(&mut self) {
        self.prev_regime = None;
        self.prev_value = None;
    }
}

// ============================================================================
// VOLUME DETECTOR
// ============================================================================

/// Detects volume events: spike, climax, level (above/below avg), delta shift.
#[derive(Debug, Clone)]
pub struct VolumeDetector {
    avg_volume: f64,
    prev_character: Option<VolumeCharacter>,
    prev_delta: Option<f64>,
}

impl VolumeDetector {
    pub fn new(avg_volume: f64) -> Self {
        Self {
            avg_volume,
            prev_character: None,
            prev_delta: None,
        }
    }

    pub fn update(&mut self, volume: f64, delta: Option<f64>) -> Option<(SignalKind, Direction)> {
        let ratio = if self.avg_volume > 0.0 {
            volume / self.avg_volume
        } else {
            1.0
        };

        let character = VolumeCharacter::from_ratio(ratio);

        let result = match character {
            VolumeCharacter::Spike => Some((SignalKind::Volume(VolumeSub::Spike), Direction::Neutral)),
            VolumeCharacter::Climax => Some((SignalKind::Volume(VolumeSub::Climax), Direction::Neutral)),
            VolumeCharacter::AboveAverage | VolumeCharacter::High => {
                Some((SignalKind::Volume(VolumeSub::Level), Direction::Up))
            }
            VolumeCharacter::VeryLow => {
                Some((SignalKind::Volume(VolumeSub::Level), Direction::Down))
            }
            _ => None,
        };

        // Delta signal: sign change in buy/sell delta
        let delta_signal = if let (Some(d), Some(prev_d)) = (delta, self.prev_delta) {
            if d > 0.0 && prev_d <= 0.0 {
                Some((SignalKind::Volume(VolumeSub::DeltaShift), Direction::Up))
            } else if d < 0.0 && prev_d >= 0.0 {
                Some((SignalKind::Volume(VolumeSub::DeltaShift), Direction::Down))
            } else {
                None
            }
        } else {
            None
        };

        self.prev_character = Some(character);
        self.prev_delta = delta;

        // Priority: delta_signal, then volume signal
        delta_signal.or(result)
    }

    pub fn update_avg(&mut self, avg_volume: f64) {
        self.avg_volume = avg_volume;
    }

    pub fn reset(&mut self) {
        self.prev_character = None;
        self.prev_delta = None;
    }
}

// ============================================================================
// SWING DETECTOR
// ============================================================================

/// Detects confirmed swing highs and swing lows.
#[derive(Debug, Clone)]
pub struct SwingDetector {
    lookback: usize,
    highs: ArrayVec<f64, 64>,
    lows: ArrayVec<f64, 64>,
    bar_index: usize,
}

impl SwingDetector {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.clamp(1, 30),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            bar_index: 0,
        }
    }

    pub fn update(&mut self, high: f64, low: f64) -> Option<(SignalKind, Direction)> {
        if self.highs.len() > self.lookback * 2 {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        self.bar_index += 1;

        if self.highs.len() < self.lookback * 2 + 1 {
            return None;
        }

        let mid = self.lookback;
        let mid_high = self.highs[mid];
        let mid_low = self.lows[mid];

        // Swing High: all highs left and right are lower than mid
        let is_swing_high = self.highs[..mid].iter().all(|&h| h < mid_high)
            && self.highs[mid + 1..].iter().all(|&h| h < mid_high);

        // Swing Low: all lows left and right are higher than mid
        let is_swing_low = self.lows[..mid].iter().all(|&l| l > mid_low)
            && self.lows[mid + 1..].iter().all(|&l| l > mid_low);

        if is_swing_high {
            Some((SignalKind::Swing, Direction::Up))
        } else if is_swing_low {
            Some((SignalKind::Swing, Direction::Down))
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.bar_index = 0;
    }
}

// ============================================================================
// MULTI-SIGNAL DETECTOR
// ============================================================================

/// Aggregates multiple `(SignalKind, Direction)` signals within a bar and
/// evaluates a composite result.
#[derive(Debug, Clone)]
pub struct MultiSignalDetector {
    signals: ArrayVec<(SignalKind, Direction), 16>,
    bar_index: usize,
}

impl MultiSignalDetector {
    pub fn new() -> Self {
        Self {
            signals: ArrayVec::new(),
            bar_index: 0,
        }
    }

    /// Add a signal for the current bar.
    pub fn add_signal(&mut self, signal: SignalKind, direction: Direction) {
        if !self.signals.is_full() {
            self.signals.push((signal, direction));
        }
    }

    /// Clear signals for the current bar.
    pub fn clear(&mut self) {
        self.signals.clear();
    }

    /// Evaluate accumulated signals and return a composite result.
    pub fn evaluate(&self) -> Option<(SignalKind, Direction)> {
        if self.signals.is_empty() {
            return None;
        }

        let bullish_count = self.signals.iter().filter(|(_, d)| d.as_i8() > 0).count();
        let bearish_count = self.signals.iter().filter(|(_, d)| d.as_i8() < 0).count();

        if bullish_count >= 3 && bearish_count == 0 {
            Some((SignalKind::Composite(CompositeSub::Strong), Direction::Up))
        } else if bearish_count >= 3 && bullish_count == 0 {
            Some((SignalKind::Composite(CompositeSub::Strong), Direction::Down))
        } else if bullish_count >= 2 && bearish_count == 0 {
            Some((SignalKind::Composite(CompositeSub::Confirmed), Direction::Up))
        } else if bearish_count >= 2 && bullish_count == 0 {
            Some((SignalKind::Composite(CompositeSub::Confirmed), Direction::Down))
        } else if bullish_count > 0 && bearish_count > 0 {
            Some((SignalKind::Composite(CompositeSub::Conflict), Direction::Neutral))
        } else if self.signals.len() == 1 {
            Some(self.signals[0])
        } else {
            None
        }
    }

    /// Get all accumulated signals.
    pub fn signals(&self) -> &[(SignalKind, Direction)] {
        &self.signals
    }

    /// Advance to next bar and clear accumulated signals.
    pub fn next_bar(&mut self) {
        self.bar_index += 1;
        self.signals.clear();
    }
}

impl Default for MultiSignalDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossover_detector() {
        let mut detector = CrossoverDetector::new();

        // No signal on first update
        assert!(detector.update(45.0, 50.0).is_none());

        // Cross up
        let signal = detector.update(55.0, 50.0);
        assert_eq!(signal, Some((SignalKind::Crossover, Direction::Up)));

        // No cross
        assert!(detector.update(56.0, 50.0).is_none());

        // Cross down
        let signal = detector.update(48.0, 50.0);
        assert_eq!(signal, Some((SignalKind::Crossover, Direction::Down)));
    }

    #[test]
    fn test_threshold_monitor() {
        let mut monitor = ThresholdMonitor::new(70.0, 30.0);

        // Initial update
        assert!(monitor.update(50.0).is_none());

        // Enter overbought
        let signal = monitor.update(75.0);
        assert_eq!(
            signal,
            Some((SignalKind::Threshold(ThresholdSub::Enter), Direction::Up))
        );
        assert!(monitor.is_overbought());

        // Exit overbought
        let signal = monitor.update(65.0);
        assert_eq!(
            signal,
            Some((SignalKind::Threshold(ThresholdSub::Exit), Direction::Down))
        );
        assert!(!monitor.is_overbought());

        // Enter oversold
        assert!(monitor.update(35.0).is_none());
        let signal = monitor.update(25.0);
        assert_eq!(
            signal,
            Some((SignalKind::Threshold(ThresholdSub::Enter), Direction::Down))
        );
        assert!(monitor.is_oversold());
    }

    #[test]
    fn test_zero_cross_detector() {
        let mut detector = ZeroCrossDetector::new();

        assert!(detector.update(-5.0).is_none());

        let signal = detector.update(5.0);
        assert_eq!(signal, Some((SignalKind::ZeroCross, Direction::Up)));

        let signal = detector.update(-5.0);
        assert_eq!(signal, Some((SignalKind::ZeroCross, Direction::Down)));
    }

    #[test]
    fn test_histogram_detector() {
        let mut detector = HistogramDetector::new();

        assert!(detector.update(-5.0).is_none());
        assert!(detector.update(-3.0).is_none());

        let signal = detector.update(2.0);
        assert_eq!(
            signal,
            Some((SignalKind::Histogram(HistogramSub::SignChange), Direction::Up))
        );

        assert!(detector.update(5.0).is_none());

        let signal = detector.update(-1.0);
        assert_eq!(
            signal,
            Some((SignalKind::Histogram(HistogramSub::SignChange), Direction::Down))
        );
    }

    #[test]
    fn test_trend_detector() {
        let mut detector = TrendDetector::new();

        // Initial
        assert!(detector.update(100.0, 98.0, 100.0).is_none());

        // MA Cross Up (Golden Cross)
        let signal = detector.update(102.0, 101.0, 99.0);
        assert_eq!(
            signal,
            Some((SignalKind::Trend(TrendSub::MaCross), Direction::Up))
        );

        // MA Cross Down (Death Cross)
        let signal = detector.update(95.0, 96.0, 98.0);
        assert_eq!(
            signal,
            Some((SignalKind::Trend(TrendSub::MaCross), Direction::Down))
        );
    }

    #[test]
    fn test_swing_detector() {
        let mut detector = SwingDetector::new(2);

        // Pattern: low - higher - peak - lower - low
        assert!(detector.update(100.0, 95.0).is_none());
        assert!(detector.update(105.0, 98.0).is_none());
        assert!(detector.update(110.0, 100.0).is_none()); // potential swing high
        assert!(detector.update(105.0, 98.0).is_none());

        let signal = detector.update(100.0, 95.0);
        assert_eq!(signal, Some((SignalKind::Swing, Direction::Up)));
    }

    #[test]
    fn test_multi_signal_detector() {
        let mut detector = MultiSignalDetector::new();

        detector.add_signal(SignalKind::Crossover, Direction::Up);
        detector.add_signal(SignalKind::Threshold(ThresholdSub::Exit), Direction::Up);

        let signal = detector.evaluate();
        assert_eq!(
            signal,
            Some((SignalKind::Composite(CompositeSub::Confirmed), Direction::Up))
        );

        detector.clear();
        detector.add_signal(SignalKind::Crossover, Direction::Up);
        detector.add_signal(SignalKind::Crossover, Direction::Down);

        let signal = detector.evaluate();
        assert_eq!(
            signal,
            Some((SignalKind::Composite(CompositeSub::Conflict), Direction::Neutral))
        );
    }
}
