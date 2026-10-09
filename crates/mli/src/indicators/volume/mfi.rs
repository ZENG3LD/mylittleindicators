//! Money Flow Index (MFI) - индикатор денежного потока
//! Комбинирует цену и объем для определения давления покупки/продажи
//! Формула: MFI = 100 - (100 / (1 + Money Flow Ratio))
//! где Money Flow Ratio = Positive Money Flow / Negative Money Flow

/// Money Flow Index индикатор
#[derive(Clone, Debug)]
pub struct Mfi {
    period: usize,
    money_flows: Vec<f64>,             // Positive/Negative money flows
    flow_types: Vec<bool>,             // true = positive, false = negative
    typical_prices: Vec<f64>,          // Для сравнения направления
    count: usize,
    value: f64,
}

impl Mfi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            money_flows: Vec::with_capacity(period),
            flow_types: Vec::with_capacity(period),
            typical_prices: Vec::with_capacity(period),
            count: 0,
            value: 50.0, // Нейтральное значение
        }
    }

    /// Feed the resolved `[high, low, close, volume]` lanes (in `SOURCE` order). MFI uses
    /// typical_price (H/L/C) weighted by volume.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        let typical_price = (high + low + close) / 3.0;
        let raw_money_flow = typical_price * volume;
        
        // Определяем направление потока
        let is_positive = if self.count > 0 {
            typical_price > self.typical_prices[self.typical_prices.len() - 1]
        } else {
            true // Первый бар считаем положительным
        };
        
        // Добавляем данные в буферы
        if self.count < self.period {
            self.money_flows.push(raw_money_flow);
            self.flow_types.push(is_positive);
            self.typical_prices.push(typical_price);
            self.count += 1;
        } else {
            // Сдвигаем буферы
            self.money_flows.remove(0);
            self.flow_types.remove(0);
            self.typical_prices.remove(0);
            
            self.money_flows.push(raw_money_flow);
            self.flow_types.push(is_positive);
            self.typical_prices.push(typical_price);
        }
        
        // Рассчитываем MFI если есть достаточно данных
        if self.count >= self.period {
            let mut positive_flow = 0.0;
            let mut negative_flow = 0.0;
            
            for i in 0..self.money_flows.len() {
                if self.flow_types[i] {
                    positive_flow += self.money_flows[i];
                } else {
                    negative_flow += self.money_flows[i];
                }
            }
            
            if negative_flow == 0.0 {
                self.value = 100.0; // Все потоки положительные
            } else if positive_flow == 0.0 {
                self.value = 0.0;   // Все потоки отрицательные
            } else {
                let money_flow_ratio = positive_flow / negative_flow;
                self.value = 100.0 - (100.0 / (1.0 + money_flow_ratio));
            }
        }
        
        self.value
    }

    /// Получить текущее значение MFI
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }

    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.money_flows.clear();
        self.flow_types.clear();
        self.typical_prices.clear();
        self.count = 0;
        self.value = 50.0;
    }

    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.value {
            v if v >= 80.0 => "Overbought",
            v if v >= 60.0 => "Bullish",
            v if v >= 40.0 => "Neutral",
            v if v >= 20.0 => "Bearish", 
            _ => "Oversold",
        }
    }

    /// Получить силу денежного потока
    pub fn flow_strength(&self) -> f64 {
        if self.value > 50.0 {
            (self.value - 50.0) / 50.0  // 0.0 to 1.0 для бычьего потока
        } else {
            (50.0 - self.value) / 50.0  // 0.0 to 1.0 для медвежьего потока
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfi_creation() {
        let mfi = Mfi::new(14);
        assert!(!mfi.is_ready());
        assert_eq!(mfi.period(), 14);
        assert_eq!(mfi.value(), 50.0);
    }

    #[test]
    fn test_mfi_warmup() {
        let mut mfi = Mfi::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mfi.feed(&[price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 10.0]);
        }
        assert!(mfi.is_ready());
    }

    #[test]
    fn test_mfi_range() {
        let mut mfi = Mfi::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mfi.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value >= 0.0 && value <= 100.0, "MFI should be in [0, 100]");
        }
    }

    #[test]
    fn test_mfi_reset() {
        let mut mfi = Mfi::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            mfi.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        mfi.reset();
        assert!(!mfi.is_ready());
        assert_eq!(mfi.value(), 50.0);
    }
}

impl Default for Mfi {
    fn default() -> Self {
        Self::new(14)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Mfi`] — period for the rolling money-flow window.
/// Source fixed to H/L/C (typical price) + volume.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MfiConfig {
    pub period: Param<usize>,
}

impl Indicator for Mfi {
    const ID: IndicatorId = IndicatorId::Mfi;
    /// Volume-weighted oscillator, not a smoothing kernel — not pluggable as a family member.
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: High, Low, Close (for typical price) + Volume.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close, OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(period) per bar — money-flow buffers are scanned each update.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Mfi)];
    type Config = MfiConfig;
    type Runtime = Mfi;

    fn create(cfg: MfiConfig) -> Mfi {
        Mfi::new(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for MfiConfig {
    fn defaults() -> Self {
        MfiConfig { period: Param::Solo(14) }
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


impl Render for Mfi {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Mfi, "MFI", Color::hex(0x009688))
            .bounds(0.0, 100.0)
            .precision(2)
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
        let mut f = IndicatorOrder::Mfi(<<Mfi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |high: f64, low: f64, close: f64, volume: f64| MarketSample::Bar {
            open: 9999.0, high, low, close, volume,
        };
        for i in 0..20 {
            let p = 100.0 + i as f64 * 0.5;
            f.feed(0, bar(p + 1.0, p - 1.0, p, 1000.0));
        }
        let v = f.read(IndicatorOutputId::Mfi);
        assert!(v >= 0.0 && v <= 100.0, "MFI must be in [0, 100], got {v}");
    }
}






















