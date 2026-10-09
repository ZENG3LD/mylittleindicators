//! Chaikin Money Flow (CMF) - индикатор денежного потока Марка Чайкина
//! Измеряет количество накопления или распределения за период
//! CMF = Σ(Money Flow Volume) / Σ(Volume) за N периодов
//! Money Flow Volume = Money Flow Multiplier × Volume
//! Money Flow Multiplier = ((Close - Low) - (High - Close)) / (High - Low)
//!
//! OPTIMIZED: O(1) running sum instead of O(n) iter().sum()

use std::collections::VecDeque;

/// Chaikin Money Flow индикатор
#[derive(Debug, Clone)]
pub struct ChaikinMoneyFlow {
    period: usize,

    // Буферы для денежного потока и объема (VecDeque for O(1) pop_front)
    money_flow_volumes: VecDeque<f64>,
    volumes: VecDeque<f64>,

    // Running sums for O(1) calculation
    sum_money_flow: f64,
    sum_volume: f64,

    // Текущее значение
    cmf_value: f64,

    // Состояние
    index: usize,
    filled: bool,
}

impl Default for ChaikinMoneyFlow {
    /// Factory default: period = 20.
    fn default() -> Self {
        Self::new(20)
    }
}

impl ChaikinMoneyFlow {
    /// Создать новый Chaikin Money Flow с заданным периодом
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "Period must be > 0");

        Self {
            period,
            money_flow_volumes: VecDeque::with_capacity(period),
            volumes: VecDeque::with_capacity(period),
            sum_money_flow: 0.0,
            sum_volume: 0.0,
            cmf_value: 0.0,
            index: 0,
            filled: false,
        }
    }

    /// Feed resolved input lanes `[high, low, close, volume]`.
    /// Returns current CMF value.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];

        let range = high - low;
        let money_flow_multiplier = if range.abs() < 1e-12 {
            0.0
        } else {
            ((close - low) - (high - close)) / range
        };

        let money_flow_volume = money_flow_multiplier * volume;

        if self.money_flow_volumes.len() >= self.period {
            let old_mf = self.money_flow_volumes.pop_front().unwrap_or(0.0);
            self.sum_money_flow -= old_mf;
        }
        self.money_flow_volumes.push_back(money_flow_volume);
        self.sum_money_flow += money_flow_volume;

        if self.volumes.len() >= self.period {
            let old_vol = self.volumes.pop_front().unwrap_or(0.0);
            self.sum_volume -= old_vol;
        }
        self.volumes.push_back(volume);
        self.sum_volume += volume;

        self.index += 1;

        if self.money_flow_volumes.len() >= self.period {
            self.filled = true;
        }

        if self.filled {
            self.cmf_value = if self.sum_volume.abs() < 1e-12 {
                0.0
            } else {
                self.sum_money_flow / self.sum_volume
            };
        }

        self.cmf_value
    }

    /// Получить текущее значение CMF
    pub fn value(&self) -> f64 {
        self.cmf_value
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Получить период индикатора
    pub fn period(&self) -> usize {
        self.period
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.money_flow_volumes.clear();
        self.volumes.clear();
        self.sum_money_flow = 0.0;
        self.sum_volume = 0.0;
        self.cmf_value = 0.0;
        self.index = 0;
        self.filled = false;
    }

    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.cmf_value {
            v if v >= 0.2 => "Strong Accumulation",
            v if v >= 0.05 => "Accumulation",
            v if v <= -0.2 => "Strong Distribution",
            v if v <= -0.05 => "Distribution",
            _ => "Neutral"
        }
    }

    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        match self.cmf_value {
            v if v >= 0.1 => 1,
            v if v <= -0.1 => -1,
            _ => 0
        }
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`ChaikinMoneyFlow`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CmfConfig {
    pub period: Param<usize>,
}

impl Indicator for ChaikinMoneyFlow {
    const ID: IndicatorId = IndicatorId::Cmf;
    /// No family — Chaikin Money Flow is an oscillator-style money-flow PRODUCER, not
    /// interchangeable with a generic oscillator family.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close, volume — the standard CMF formula.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(1) via running sums; two period-deep deques.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque), Store::window(StoreKind::Deque)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cmf)];
    type Config = CmfConfig;
    type Runtime = ChaikinMoneyFlow;

    fn create(cfg: CmfConfig) -> ChaikinMoneyFlow {
        ChaikinMoneyFlow::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for CmfConfig {
    fn defaults() -> Self {
        CmfConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (lookback window) → auto range(2,4048,1). No other axes.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ChaikinMoneyFlow {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cmf, "CMF", Color::hex(0x009688))
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaikin_money_flow_creation() {
        let cmf = ChaikinMoneyFlow::new(20);
        assert!(!cmf.is_ready());
        assert_eq!(cmf.value(), 0.0);
    }

    #[test]
    fn test_chaikin_money_flow_warmup() {
        let mut cmf = ChaikinMoneyFlow::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cmf.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(cmf.is_ready());
    }

    #[test]
    fn test_chaikin_money_flow_range() {
        let mut cmf = ChaikinMoneyFlow::new(20);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = cmf.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value >= -1.0 && value <= 1.0, "CMF should be in [-1, 1]");
        }
    }

    #[test]
    fn test_chaikin_money_flow_reset() {
        let mut cmf = ChaikinMoneyFlow::new(20);
        for i in 0..25 {
            cmf.feed(&[105.0, 95.0, 100.0 + i as f64, 1000.0]);
        }
        cmf.reset();
        assert!(!cmf.is_ready());
        assert_eq!(cmf.value(), 0.0);
    }

    /// Factory resolves H/L/C/Volume lanes; open is ignored (set to 9999 as proof).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Cmf(<<ChaikinMoneyFlow as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..25 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 105.0, low: 95.0, close: 100.0, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        // Close at midpoint → MFM = 0 → CMF = 0
        assert_eq!(f.read(IndicatorOutputId::Cmf), 0.0);
    }
}
