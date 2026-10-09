//! Detrended Price Oscillator (DPO) indicator.

use crate::engine::contract_engine::SmootherSlot;

/// Detrended Price Oscillator (DPO) - removes trend from price to show cycles.
///
/// DPO = Close[lookback_offset] - SMA(Close, N) centered at that point
///
/// Unlike most oscillators, DPO is not designed to identify overbought/oversold
/// conditions. Instead, it removes the trend component to reveal underlying
/// price cycles.
///
/// Interpretation:
/// - DPO > 0: Price above its historical average (cycle peak)
/// - DPO < 0: Price below its historical average (cycle trough)
/// - Zero crossovers: Potential cycle turning points
///
/// # Parameters
/// - `period`: Lookback period (typically 20)
/// - `source`: OHLCV field to use as input (default Close)
///
/// # Implementation
///
/// Uses sliding window for historical prices. O(period) per update.
#[derive(Debug, Clone)]
pub struct DetrendedPriceOscillator {
    period: usize,
    lookback_offset: usize,

    // Буферы для расчетов
    close_prices: Vec<f64>,
    dpo_values: Vec<f64>,

    // Скользящая средняя для расчета
    sma: SmootherSlot,

    // Текущее значение
    dpo_value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl DetrendedPriceOscillator {
    /// Creates a new DPO with default period (20).
    pub fn new() -> Self {
        Self::with_period(20)
    }

    /// Creates a new DPO with specified period.
    pub fn with_period(period: usize) -> Self {
        Self::from_smoother(SmootherId::Sma, period)
    }

    /// Build from a narrow `SmootherId` for the internal MA.
    pub fn from_smoother(id: SmootherId, period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        let lookback_offset = period / 2 + 1;
        Self {
            period,
            lookback_offset,
            close_prices: Vec::with_capacity(period + lookback_offset + 2),
            dpo_values: Vec::with_capacity(period),
            sma: SmootherSlot::new(id, period),
            dpo_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, price: f64) -> f64 {
        self.bars_count += 1;

        // Добавляем цену в буфер
        if self.close_prices.len() >= 512 {
            self.close_prices.remove(0);
        }
        self.close_prices.push(price);

        // Обновляем SMA
        let _sma_value = self.sma.feed(price);

        // Проверяем, можем ли рассчитать DPO
        if self.close_prices.len() >= self.period + self.lookback_offset {
            // Получаем цену закрытия N/2 + 1 периодов назад
            let historical_close_idx = self.close_prices.len() - self.lookback_offset - 1;
            let historical_close = self.close_prices[historical_close_idx];

            // Получаем SMA для исторической точки
            let historical_sma = self.calculate_historical_sma(historical_close_idx);

            // Рассчитываем DPO
            self.dpo_value = historical_close - historical_sma;

            // Добавляем в буфер
            if self.dpo_values.len() >= 512 {
                self.dpo_values.remove(0);
            }
            self.dpo_values.push(self.dpo_value);

            // Проверяем готовность
            if !self.is_ready && self.dpo_values.len() >= 3 {
                self.is_ready = true;
            }
        }

        self.dpo_value
    }

    /// Calculates historical SMA for the given index.
    fn calculate_historical_sma(&self, center_idx: usize) -> f64 {
        if center_idx < self.period / 2 || center_idx + self.period / 2 >= self.close_prices.len() {
            return 0.0;
        }

        let start_idx = center_idx - self.period / 2;
        let end_idx = start_idx + self.period;

        if end_idx > self.close_prices.len() {
            return 0.0;
        }

        let sum: f64 = self.close_prices[start_idx..end_idx].iter().sum();
        sum / self.period as f64
    }

    /// Returns the current DPO value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.dpo_value
    }

    /// Returns `true` if the DPO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the period of this DPO.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Resets the DPO to its initial state.
    pub fn reset(&mut self) {
        self.close_prices.clear();
        self.dpo_values.clear();
        self.sma.reset();
        self.dpo_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Returns the current cycle condition.
    pub fn cycle_condition(&self) -> &'static str {
        match self.dpo_value {
            v if v > 0.0 => "Above Trend Cycle",
            v if v < 0.0 => "Below Trend Cycle",
            _ => "On Trend Cycle",
        }
    }

    /// Returns trading signal based on cycles.
    /// 1 = buy, -1 = sell, 0 = neutral
    pub fn cycle_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        if self.dpo_value > 0.0 {
            1
        } else if self.dpo_value < 0.0 {
            -1
        } else {
            0
        }
    }

    // Transitional bridge: dpo_percent.rs + dpo_bands.rs still call these; delegates to feed/slot.
    // Remove when those callers are contracted.

    /// Transitional bridge: callers that hold `SmootherId` (e.g. `dpo_bands.rs`) use this
    /// to build a DPO with a specific MA.  Delegates to `from_smoother` via `to_smoother()`.
    pub fn with_period_and_ma_type(period: usize, ma_type: crate::engine::contract_engine::SmootherId) -> Self {
        Self::from_smoother(ma_type, period.max(2))
    }


    /// Returns advanced signal with confirmation.
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.dpo_values.len() < 3 {
            return 0;
        }

        let len = self.dpo_values.len();
        let current = self.dpo_value;
        let prev_1 = if len >= 2 { self.dpo_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.dpo_values[len - 3] } else { 0.0 };

        if prev_2 <= 0.0 && prev_1 <= 0.0 && current > 0.0 {
            return 1;
        }

        if prev_2 >= 0.0 && prev_1 >= 0.0 && current < 0.0 {
            return -1;
        }

        0
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherId};
use crate::engine::contract_engine::SmootherChoice;
use crate::contract::{Cost, Family, Indicator, Output, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Param;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

impl DetrendedPriceOscillator {
    /// Build a DPO from a smoother CHOICE (kind + follow/own period) at the host `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        // Resolve the slot's Follow/Own period against the host period — never discard `Own`.
        let p = choice.period.resolve(period).max(1);
        Self::from_smoother(choice.id(), p)
    }
}

/// Typed contract config for [`DetrendedPriceOscillator`].
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct DpoConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    /// The internal MA smoother — defaults to SMA per the standard DPO definition.
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for DetrendedPriceOscillator {
    const ID: IndicatorId = IndicatorId::Dpo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period) per bar: rescans the price window to compute the centered historical SMA.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const SLOTS: &'static [Slot] = DpoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Dpo)];
    type Config = DpoConfig;
    type Runtime = DetrendedPriceOscillator;

    fn create(cfg: DpoConfig) -> DetrendedPriceOscillator {
        let period = cfg.period.resolved();
        let choice = cfg.smoother.resolved();
        DetrendedPriceOscillator::from_choice(choice, period)
    }

    fn source_fields(cfg: &DpoConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &DpoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for DpoConfig {
    fn defaults() -> Self {
        DpoConfig {
            period: Param::Solo(20),
            source: Param::Solo(OhlcvField::Close),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto; source: Class O → auto all-8.
        // smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for DetrendedPriceOscillator {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dpo, "DPO", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl Default for DetrendedPriceOscillator {
    fn default() -> Self {
        Self::with_period(14)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpo_basic_calculation() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);
        for i in 1..=50 {
            dpo.feed(100.0 + i as f64);
        }
        assert!(dpo.is_ready());
    }

    #[test]
    fn test_dpo_constant_price() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);
        for _ in 1..=50 {
            dpo.feed(100.0);
        }
        assert!(dpo.is_ready());
        assert!(dpo.value().abs() < 0.1, "DPO with constant price should be near 0");
    }

    #[test]
    fn test_dpo_cycle_condition() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);
        for i in 1..=50 {
            dpo.feed(100.0 + (i % 20) as f64);
        }
        assert!(dpo.is_ready());
        let condition = dpo.cycle_condition();
        assert!(
            condition == "Above Trend Cycle"
                || condition == "Below Trend Cycle"
                || condition == "On Trend Cycle"
        );
    }

    #[test]
    fn test_dpo_reset() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);
        for i in 1..=50 {
            dpo.feed(100.0 + i as f64);
        }
        assert!(dpo.is_ready());
        dpo.reset();
        assert!(!dpo.is_ready());
        assert!(dpo.value().abs() < 1e-10);
    }

    #[test]
    fn test_dpo_period_getter() {
        let dpo = DetrendedPriceOscillator::with_period(15);
        assert_eq!(dpo.period(), 15);
    }

    #[test]
    fn test_dpo_cycle_signal() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);
        assert_eq!(dpo.cycle_signal(), 0);
        for i in 1..=50 {
            dpo.feed(100.0 + i as f64);
        }
        assert!(dpo.is_ready());
        let signal = dpo.cycle_signal();
        assert!(signal >= -1 && signal <= 1);
    }

    #[test]
    fn test_dpo_default() {
        let dpo = DetrendedPriceOscillator::new();
        assert_eq!(dpo.period(), 20);
    }

    #[test]
    fn factory_feeds_resolved_dpo() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DetrendedPriceOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 20);
        let mut f = IndicatorOrder::Dpo(cfg).build_solo().unwrap();
        for i in 1..=50 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: 100.0 + i as f64,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
