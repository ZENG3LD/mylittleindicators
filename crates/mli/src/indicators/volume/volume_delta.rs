//! Volume Delta — per-step aggressor delta (`buy - sell`).
//!
//! A bar's OHLCV has no taker side, so a bar sample does not move the
//! ring. Real updates are [`VolumeDelta::update_with_delta`] and ticks.

/// Volume Delta индикатор с фиксированным окном
#[derive(Debug, Clone)]
pub struct VolumeDelta {
    /// Период для расчета средних
    period: usize,
    /// Буфер дельт (фиксированный размер)
    buffer: Vec<f64>,
    /// Индекс для циклического буфера
    idx: usize,
    /// Сумма дельт в буфере
    sum: f64,
    /// Количество обработанных баров
    count: usize,
    /// Кумулятивная дельта за весь период
    cumulative_delta: f64,
    /// Текущая дельта
    current_delta: f64,
    /// Флаг готовности
    ready: bool,
}

impl VolumeDelta {
    /// Создать новый Volume Delta индикатор
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: Vec::with_capacity(period),
            idx: 0,
            sum: 0.0,
            count: 0,
            cumulative_delta: 0.0,
            current_delta: 0.0,
            ready: false,
        }
    }

    /// Обновить индикатор с готовыми buy/sell объёмами
    pub fn update_with_delta(&mut self, buy_volume: f64, sell_volume: f64) -> f64 {
        let delta = buy_volume - sell_volume;
        self.process_delta(delta)
    }

    /// Обработка дельты (общая логика)
    fn process_delta(&mut self, delta: f64) -> f64 {
        self.current_delta = delta;
        self.cumulative_delta += delta;

        if self.count < self.period {
            self.buffer.push(delta);
            self.sum += delta;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            let old = self.buffer[self.idx];
            self.sum += delta - old;
            self.buffer[self.idx] = delta;
            self.idx = (self.idx + 1) % self.period;
        }

        self.ready = self.count >= self.period;
        delta
    }

    pub fn current_delta(&self) -> f64 { self.current_delta }
    pub fn cumulative_delta(&self) -> f64 { self.cumulative_delta }
    pub fn average_delta(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.sum / self.count.min(self.period) as f64 }
    }
    pub fn is_ready(&self) -> bool { self.ready }
    pub fn count(&self) -> usize { self.count }
    pub fn period(&self) -> usize { self.period }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.idx = 0;
        self.sum = 0.0;
        self.count = 0;
        self.cumulative_delta = 0.0;
        self.current_delta = 0.0;
        self.ready = false;
    }

    pub fn value(&self) -> f64 {
        self.current_delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_delta_creation() {
        let vd = VolumeDelta::new(20);
        assert!(!vd.is_ready());
        assert_eq!(vd.period(), 20);
        assert_eq!(vd.current_delta(), 0.0);
    }

    #[test]
    fn test_volume_delta_warmup() {
        let mut vd = VolumeDelta::new(10);
        for _ in 0..15 {
            vd.update_with_delta(600.0, 400.0);
        }
        assert!(vd.is_ready());
    }

    #[test]
    fn test_volume_delta_values() {
        let mut vd = VolumeDelta::new(10);
        let delta = vd.update_with_delta(1000.0, 0.0);
        assert!((delta - 1000.0).abs() < 1e-9, "buy-only step is +1000, got {delta}");

        let delta = vd.update_with_delta(0.0, 1000.0);
        assert!((delta - -1000.0).abs() < 1e-9, "sell-only step is -1000, got {delta}");
    }

    #[test]
    fn test_volume_delta_cumulative() {
        let mut vd = VolumeDelta::new(10);
        for _ in 0..10 {
            vd.update_with_delta(1000.0, 0.0);
        }
        assert!((vd.cumulative_delta() - 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_volume_delta_reset() {
        let mut vd = VolumeDelta::new(10);
        for _ in 0..15 {
            vd.update_with_delta(1000.0, 0.0);
        }
        vd.reset();
        assert!(!vd.is_ready());
        assert_eq!(vd.current_delta(), 0.0);
        assert_eq!(vd.cumulative_delta(), 0.0);
    }
}

impl Default for VolumeDelta {
    fn default() -> Self {
        Self::new(50)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Own config for [`VolumeDelta`] — rolling window size for the cumulative delta.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VdeltaConfig {
    pub period: Param<usize>,
}

impl Indicator for VolumeDelta {
    const ID: IndicatorId = IndicatorId::Vdelta;
    /// Standalone per-bar volume delta — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    /// O(1) per bar — simple ring buffer with running sum.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Vdelta)];
    type Config = VdeltaConfig;
    type Runtime = VolumeDelta;

    fn create(cfg: VdeltaConfig) -> VolumeDelta {
        VolumeDelta::new(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for VdeltaConfig {
    fn defaults() -> Self {
        VdeltaConfig { period: Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl TickConsumer for VolumeDelta {
    fn update_tick(&mut self, tick: &Tick) {
        let delta = if tick.is_buy { tick.size } else { -tick.size };
        self.process_delta(delta);
    }

    fn reset(&mut self) {
        VolumeDelta::reset(self);
    }

    fn is_ready(&self) -> bool {
        VolumeDelta::is_ready(self)
    }
}

impl Render for VolumeDelta {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::Vdelta, "Vol Delta", Color::hex(0x009688)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn on_wide_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(f)
            .expect("spawn")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    }

    #[test]
    fn bar_feed_does_not_fabricate_delta() {
        on_wide_stack(|| {
            let mut f = IndicatorOrder::Vdelta(<<VolumeDelta as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
            f.feed(0, MarketSample::Bar { open: 100.0, high: 110.0, low: 90.0, close: 109.0, volume: 500.0 });
            assert!(!f.is_ready());
            let v = f.read(IndicatorOutputId::Vdelta);
            assert!(v.abs() < 1e-9, "bar OHLCV must not move Vdelta, got {v}");
        });
    }

    #[test]
    fn tick_signed_size_is_the_delta() {
        on_wide_stack(|| {
            let mut f = IndicatorOrder::Vdelta(<<VolumeDelta as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
            let buy = crate::core::types::Tick::new(0, 100.0, 500.0, true);
            let sell = crate::core::types::Tick::new(1, 100.0, 200.0, false);
            f.feed(0, MarketSample::Tick(&buy));
            f.feed(1, MarketSample::Tick(&sell));
            let v = f.read(IndicatorOutputId::Vdelta);
            assert!((v - -200.0).abs() < 1e-9, "last tick is the current delta, got {v}");
        });
    }
}
