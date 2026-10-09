//! Volume Rate of Change (VROC) - скорость изменения объема
//! VROC = (Current Volume - Volume N periods ago) / Volume N periods ago * 100
//! Показывает процентное изменение объема по сравнению с предыдущими периодами

use crate::engine::contract_engine::SmootherSlot;
use std::collections::VecDeque;

/// Volume Rate of Change индикатор
#[derive(Debug, Clone)]
pub struct VolumeRateOfChange {
    period: usize,
    signal_period: usize,

    // Кольцо последних (period+1) объёмов — ровно столько нужно для VROC-гэпа.
    volume_buffer: VecDeque<f64>,

    // Сигнальная линия (MA от VROC)
    signal_ma: SmootherSlot,

    // Текущие значения
    vroc_value: f64,
    signal_value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl VolumeRateOfChange {
    /// Создать новый VROC с параметрами по умолчанию (14, 9), SMA signal.
    pub fn new() -> Self {
        Self::with_params(14, 9)
    }

    /// Создать новый VROC с настраиваемыми параметрами (SMA signal).
    pub fn with_params(period: usize, signal_period: usize) -> Self {
        Self::from_signal_smoother(period, signal_period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for the signal smoother + the periods.
    /// Legacy bridge; the contract path goes through `VrocConfig`.
    pub fn from_signal_smoother(period: usize, signal_period: usize, signal_smoother: SmootherId) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");
        Self {
            period,
            signal_period,
            volume_buffer: VecDeque::with_capacity(period + 1),
            signal_ma: SmootherSlot::new(signal_smoother, signal_period),
            vroc_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Обновить индикатор новым баром
    /// Feed ONE resolved scalar (volume) — the factory extracts the Volume field.
    pub fn feed(&mut self, volume: f64) -> f64 {
        self.bars_count += 1;

        // Добавляем объём в кольцо (держим только последние period+1).
        self.volume_buffer.push_back(volume);
        if self.volume_buffer.len() > self.period + 1 {
            self.volume_buffer.pop_front();
        }

        // Проверяем, можем ли рассчитать VROC
        if self.volume_buffer.len() > self.period {
            let current_volume = volume;
            // Объём `period` баров назад = фронт кольца.
            let past_volume = *self.volume_buffer.front().unwrap_or(&volume);

            // Рассчитываем VROC
            self.vroc_value = if past_volume.abs() > 1e-12 {
                (current_volume - past_volume) / past_volume * 100.0
            } else {
                0.0
            };
        }

        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_ma.feed(self.vroc_value);

        // Проверяем готовность
        if self.bars_count >= self.period + self.signal_period {
            self.is_ready = true;
        }

        self.vroc_value
    }

    /// Получить значение VROC
    pub fn value(&self) -> f64 {
        self.vroc_value
    }

    /// Получить значение сигнальной линии
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }

    /// Получить разность между VROC и сигнальной линией
    pub fn histogram(&self) -> f64 {
        self.vroc_value - self.signal_value
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Получить периоды индикатора
    pub fn periods(&self) -> (usize, usize) {
        (self.period, self.signal_period)
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.volume_buffer.clear();
        self.signal_ma.reset();
        self.vroc_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Определить состояние объемной активности
    pub fn volume_activity(&self) -> &'static str {
        match self.vroc_value {
            v if v > 50.0 => "Very High Volume",
            v if v > 20.0 => "High Volume",
            v if v > 0.0 => "Above Average Volume",
            v if v > -20.0 => "Below Average Volume",
            v if v > -50.0 => "Low Volume",
            _ => "Very Low Volume"
        }
    }

    /// Получить торговый сигнал (1 = покупка, -1 = продажа, 0 = нейтрально)
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        if self.vroc_value > self.signal_value && self.vroc_value > 0.0 {
            1
        } else if self.vroc_value < self.signal_value && self.vroc_value < 0.0 {
            -1
        } else {
            0
        }
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`VolumeRateOfChange`] — % change of volume over `period`.
/// `period` is the VROC lookback, `signal_period` is the host period field, `signal` is
/// a `Param<SmootherSlotOrder>` carrying the MA member + its own period. Default = SMA
/// at `signal_period` (9). The slot period is swept by `SmootherSlotOrder::machine_sweep()`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct VrocConfig {
    pub period: Param<usize>,
    pub signal_period: Param<usize>,
    #[slot]
    pub signal: Param<SmootherSlotOrder>,
}

impl Indicator for VolumeRateOfChange {
    const ID: IndicatorId = IndicatorId::Vroc;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed Volume series — the factory extracts the Volume field and feeds the scalar.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(1) volume-gap over a (period+1)-deep `VecDeque` ring; the signal smoother's
    /// buffer lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    const SLOTS: &'static [Slot] = VrocConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Vroc)];
    type Config = VrocConfig;
    type Runtime = VolumeRateOfChange;

    fn create(cfg: VrocConfig) -> VolumeRateOfChange {
        let period = cfg.period.resolved().max(1);
        // The slot order carries its own period — no host-period resolve needed.
        let signal_slot = cfg.signal.resolved().into_slot();
        let sig_period = signal_slot.period().max(1);
        VolumeRateOfChange {
            period,
            signal_period: sig_period,
            volume_buffer: VecDeque::with_capacity(period + 1),
            signal_ma: signal_slot,
            vroc_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    fn slot_members(cfg: &VrocConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for VrocConfig {
    fn defaults() -> Self {
        VrocConfig {
            period: Param::Solo(14),
            signal_period: Param::Solo(9),
            signal: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 9 })),
        }
    }
    fn machine_defaults() -> Self {
        // period/signal_period: Class A — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumeRateOfChange {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vroc, "Vol ROC", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vroc_creation() {
        let vroc = VolumeRateOfChange::new();
        assert!(!vroc.is_ready());
        assert_eq!(vroc.value(), 0.0);
    }

    #[test]
    fn test_vroc_with_params() {
        let vroc = VolumeRateOfChange::with_params(10, 5);
        assert!(!vroc.is_ready());
        assert_eq!(vroc.periods(), (10, 5));
    }

    #[test]
    fn test_vroc_warmup() {
        let mut vroc = VolumeRateOfChange::with_params(10, 5);
        for i in 0..20 {
            let volume = 1000.0 + (i as f64 * 0.1).sin() * 100.0;
            vroc.feed(volume);
        }
        assert!(vroc.is_ready());
    }

    #[test]
    fn test_vroc_values_finite() {
        let mut vroc = VolumeRateOfChange::new();
        for i in 0..30 {
            let volume = 1000.0 + i as f64 * 50.0;
            let value = vroc.feed(volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vroc_reset() {
        let mut vroc = VolumeRateOfChange::new();
        for i in 0..30 {
            vroc.feed(1000.0 + i as f64 * 10.0);
        }
        vroc.reset();
        assert!(!vroc.is_ready());
        assert_eq!(vroc.value(), 0.0);
    }

    #[test]
    fn test_vroc_contract_create() {
        let cfg = <<VolumeRateOfChange as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.signal_period, Param::Solo(9));
        let mut vroc = <VolumeRateOfChange as Indicator>::create(cfg);
        for i in 0..40 {
            let volume = 1000.0 + i as f64 * 50.0;
            vroc.feed(volume);
        }
        assert!(vroc.is_ready());
    }
}
