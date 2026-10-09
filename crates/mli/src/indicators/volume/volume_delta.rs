//! Volume Delta Indicator
//! Анализирует баланс покупок/продаж

use crate::types::Bar;

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

    /// Обновить индикатор баром (эвристика по price action)
    pub fn update(&mut self, bar: &Bar) -> f64 {
        let delta = self.estimate_delta_from_bar(bar);
        self.process_delta(delta)
    }

    /// Feed resolved lanes `[open, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let open = lanes[0];
        let close = lanes[1];
        let volume = lanes[2];
        let delta = if close > open {
            volume
        } else if close < open {
            -volume
        } else {
            0.0
        };
        self.process_delta(delta)
    }

    /// Оценка дельты из бара (эвристика)
    fn estimate_delta_from_bar(&self, bar: &Bar) -> f64 {
        let price_delta = bar.close - bar.open;
        if price_delta > 0.0 {
            bar.volume // Покупки
        } else if price_delta < 0.0 {
            -bar.volume // Продажи
        } else {
            0.0 // Нейтрально
        }
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
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vd.feed(&[price, price + 0.5, 1000.0]);
        }
        assert!(vd.is_ready());
    }

    #[test]
    fn test_volume_delta_values() {
        let mut vd = VolumeDelta::new(10);
        // close (101) > open (100) → positive delta
        let delta = vd.feed(&[100.0, 101.0, 1000.0]);
        assert!(delta > 0.0, "Rising close should have positive delta");

        // close (100) < open (101) → negative delta
        let delta = vd.feed(&[101.0, 100.0, 1000.0]);
        assert!(delta < 0.0, "Falling close should have negative delta");
    }

    #[test]
    fn test_volume_delta_cumulative() {
        let mut vd = VolumeDelta::new(10);
        for i in 0..10 {
            let price = 100.0 + i as f64;
            vd.feed(&[price, price + 0.5, 1000.0]);
        }
        let cumulative = vd.cumulative_delta();
        assert!(cumulative.is_finite());
    }

    #[test]
    fn test_volume_delta_reset() {
        let mut vd = VolumeDelta::new(10);
        for i in 0..15 {
            vd.feed(&[100.0 + i as f64, 101.0, 1000.0]);
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
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`VolumeDelta`] — rolling window size for the cumulative delta.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VdeltaConfig {
    pub period: Param<usize>,
}

impl Indicator for VolumeDelta {
    const ID: IndicatorId = IndicatorId::Vdelta;
    /// Standalone per-bar volume delta — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed open (direction base) + close (direction) + volume (magnitude) lanes.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
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

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Vdelta(<<VolumeDelta as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Bullish bar: close > open → +volume
        f.feed(0, MarketSample::Bar { open: 100.0, high: 9999.0, low: 9999.0, close: 101.0, volume: 500.0 });
        assert_eq!(f.read(IndicatorOutputId::Vdelta), 500.0, "bullish bar should give +500 delta");
    }
}
