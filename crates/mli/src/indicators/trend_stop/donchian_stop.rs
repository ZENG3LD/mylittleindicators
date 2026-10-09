//! Donchian Stop - индикатор динамических уровней на основе каналов Дончиана
//!
//! Вычисляет уровни на основе каналов Дончиана:
//! - Верхняя линия: максимальный High за N периодов
//! - Нижняя линия: минимальный Low за N периодов
//! - Средняя линия: (верхняя + нижняя) / 2
//!
//! Может использовать разные периоды для верхней и нижней полос,
//! а также добавлять отступы для более консервативного подхода.
//!
//! Индикатор НЕ содержит логику стопов - только возвращает граници каналов.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.


/// Typed dual-mode config for [`DonchianStop`].
/// `upper_period` / `lower_period` = rolling extrema windows;
/// `offset` = absolute or percentage offset from channel bounds;
/// `use_percentage` = true → `offset` is a percentage.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DonchianStopConfig {
    /// Period for upper band (highest highs).
    pub upper_period: Param<usize>,
    /// Period for lower band (lowest lows).
    pub lower_period: Param<usize>,
    /// Offset from channel bounds (absolute points or percentage).
    pub offset: Param<f64>,
    /// If true, `offset` is treated as a percentage of the bound price.
    pub use_percentage: Param<bool>,
}

/// Donchian Stop индикатор - уровни на основе каналов Дончиана
#[derive(Debug, Clone)]
pub struct DonchianStop {
    upper_period: usize,
    lower_period: usize,
    offset: f64,
    use_percentage: bool,

    highs: Vec<f64>,
    lows: Vec<f64>,

    upper_line: f64,
    lower_line: f64,
    middle_line: f64,

    upper_stop: f64,
    lower_stop: f64,

    bars_count: usize,
    is_ready: bool,
}

impl DonchianStop {
    /// Create with default parameters (period=20, offset=0.0, absolute).
    pub fn new() -> Self {
        Self::with_params(20, 0.0, false)
    }

    /// Create with symmetric period for both bands.
    pub fn with_params(period: usize, offset: f64, use_percentage: bool) -> Self {
        Self::with_different_periods(period, period, offset, use_percentage)
    }

    /// Create with asymmetric periods for upper and lower bands.
    pub fn with_different_periods(
        upper_period: usize,
        lower_period: usize,
        offset: f64,
        use_percentage: bool,
    ) -> Self {
        assert!(upper_period > 0, "Upper period must be greater than 0");
        assert!(lower_period > 0, "Lower period must be greater than 0");

        Self {
            upper_period,
            lower_period,
            offset,
            use_percentage,
            highs: Vec::with_capacity(512),
            lows: Vec::with_capacity(512),
            upper_line: 0.0,
            lower_line: 0.0,
            middle_line: 0.0,
            upper_stop: 0.0,
            lower_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed one bar's `[high, low]` lanes (resolved by the factory from `const SOURCE`).
    /// Returns `(lower_stop, upper_stop, middle_line)`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];

        self.bars_count += 1;

        let max_period = self.upper_period.max(self.lower_period);

        if self.highs.len() >= max_period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);

        if self.highs.len() >= self.upper_period {
            let start_idx = self.highs.len().saturating_sub(self.upper_period);
            self.upper_line = self.highs[start_idx..]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
        }

        if self.lows.len() >= self.lower_period {
            let start_idx = self.lows.len().saturating_sub(self.lower_period);
            self.lower_line = self.lows[start_idx..]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
        }

        if self.upper_line > 0.0 && self.lower_line < f64::INFINITY {
            self.middle_line = (self.upper_line + self.lower_line) / 2.0;
        }

        self.calculate_stop_levels();

        let min_period = self.upper_period.min(self.lower_period);
        self.is_ready = self.bars_count >= min_period
            && self.upper_line > 0.0
            && self.lower_line < f64::INFINITY;

        (self.lower_stop, self.upper_stop, self.middle_line)
    }

    fn calculate_stop_levels(&mut self) {
        if self.upper_line > 0.0 {
            if self.use_percentage {
                self.upper_stop = self.upper_line * (1.0 + self.offset / 100.0);
            } else {
                self.upper_stop = self.upper_line + self.offset;
            }
        }

        if self.lower_line < f64::INFINITY {
            if self.use_percentage {
                self.lower_stop = self.lower_line * (1.0 - self.offset / 100.0);
            } else {
                self.lower_stop = self.lower_line - self.offset;
            }
        }
    }

    pub fn upper_line(&self) -> f64 {
        self.upper_line
    }

    pub fn lower_line(&self) -> f64 {
        if self.lower_line == f64::INFINITY { 0.0 } else { self.lower_line }
    }

    pub fn middle_line(&self) -> f64 {
        self.middle_line
    }

    pub fn upper_stop(&self) -> f64 {
        self.upper_stop
    }

    pub fn lower_stop(&self) -> f64 {
        self.lower_stop
    }

    pub fn value(&self) -> f64 {
        self.lower_stop
    }

    pub fn levels(&self) -> (f64, f64, f64) {
        (self.lower_stop, self.upper_stop, self.middle_line)
    }

    pub fn channel_width(&self) -> f64 {
        if self.upper_line > 0.0 && self.lower_line < f64::INFINITY {
            self.upper_line - self.lower_line
        } else {
            0.0
        }
    }

    pub fn price_position(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width == 0.0 {
            return 0.5;
        }
        (price - self.lower_line) / width
    }

    pub fn is_above_upper(&self, price: f64) -> bool {
        price > self.upper_line
    }

    pub fn is_below_lower(&self, price: f64) -> bool {
        price < self.lower_line
    }

    pub fn is_inside_channel(&self, price: f64) -> bool {
        price >= self.lower_line && price <= self.upper_line
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.upper_line = 0.0;
        self.lower_line = 0.0;
        self.middle_line = 0.0;
        self.upper_stop = 0.0;
        self.lower_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    pub fn params(&self) -> (usize, usize, f64, bool) {
        (self.upper_period, self.lower_period, self.offset, self.use_percentage)
    }
}

impl Default for DonchianStop {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
    sweep_f64,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

impl Indicator for DonchianStop {
    const ID: IndicatorId = IndicatorId::Dons;
    /// No family — a trailing-stop level producer (not a pluggable oscillator / channel
    /// family member). Consumed by name via a `Port`, not by family slot.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed High + Low lanes — the Donchian channel is defined by rolling max(High) and
    /// min(Low), neither of which is configurable.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// Primary output is the lower-stop scalar (`value()` returns `Single`).
    /// O(period) per bar — rescans the window for max/min each update. Two period-deep
    /// `Vec` buffers (highs, lows).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Dons)];

    type Config = DonchianStopConfig;
    type Runtime = DonchianStop;

    fn create(cfg: DonchianStopConfig) -> DonchianStop {
        DonchianStop::with_different_periods(
            cfg.upper_period.resolved(),
            cfg.lower_period.resolved(),
            cfg.offset.resolved(),
            cfg.use_percentage.resolved(),
        )
    }
}

impl crate::contract::Config for DonchianStopConfig {
    fn defaults() -> Self {
        DonchianStopConfig {
            upper_period: Param::Solo(20),
            lower_period: Param::Solo(20),
            offset: Param::Solo(0.0),
            use_percentage: Param::Solo(false),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // upper_period/lower_period→range(2,4048,1); use_percentage→both
        s.offset = Param::many(sweep_f64(0.0, 1.0, 0.05)); // Class D ratio/fraction (audit: offset trend_stop)
        s
    }
}


impl Render for DonchianStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Dons, "Donchian Stop", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_stop_creation() {
        let ind = DonchianStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_donchian_stop_with_params() {
        let ind = DonchianStop::with_params(20, 0.5, false);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (20, 20, 0.5, false));
    }

    #[test]
    fn test_donchian_stop_warmup() {
        let mut ind = DonchianStop::with_params(10, 0.0, false);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_donchian_stop_values_finite() {
        let mut ind = DonchianStop::with_params(10, 0.0, false);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (lower, upper, mid) = ind.feed(&[price + 1.0, price - 1.0]);
            assert!(lower.is_finite());
            assert!(upper.is_finite());
            assert!(mid.is_finite());
        }
    }

    #[test]
    fn test_donchian_stop_reset() {
        let mut ind = DonchianStop::with_params(10, 0.0, false);
        for i in 0..20 {
            ind.feed(&[105.0 + i as f64, 95.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    /// Factory resolves the fixed High/Low lanes from `const SOURCE` and feeds them.
    /// A wild close (9999.0) must NOT affect the output — only H/L are consumed.
    /// In a steady uptrend the lower bound (lower_stop = `value()`) is the rolling
    /// min-Low = constant 99.0 (offset=0), so it stays 99.0 after warmup.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Dons(<<DonchianStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        // Feed 25 bars: high=101..125, low=99, close=wild 9999, volume=wild -1.
        // Lower stop = min low over 20 bars = 99.0 (no offset).
        for i in 1..=25usize {
            f.feed(0, MarketSample::Bar {
                open: -1.0,
                high: 100.0 + i as f64,
                low: 99.0,
                close: 9999.0,
                volume: -1.0,
            });
        }
        assert!(f.is_ready());
        // lower_stop = min(low) - 0.0 = 99.0
        assert!(
            (f.primary() - 99.0).abs() < 1e-9,
            "expected lower_stop=99.0, got {}",
            f.primary()
        );
    }
}
