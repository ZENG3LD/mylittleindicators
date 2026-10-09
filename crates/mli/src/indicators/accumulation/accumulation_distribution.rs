//! Accumulation/Distribution Line (A/D Line) - индикатор накопления/распределения
//! Показывает, происходит ли накопление (покупка) или распределение (продажа)
//! AD = Previous AD + Money Flow Multiplier × Volume
//! Money Flow Multiplier = ((Close - Low) - (High - Close)) / (High - Low)
//! Диапазон MFM: от -1 до +1

/// Accumulation/Distribution Line индикатор
#[derive(Debug, Clone)]
pub struct AccumulationDistribution {
    // Текущее значение A/D Line
    ad_value: f64,
    
    // История значений для анализа
    ad_history: Vec<f64>,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl Default for AccumulationDistribution {
    /// Factory default: no params — same as `new()`.
    fn default() -> Self {
        Self::new()
    }
}

impl AccumulationDistribution {
    /// Создать новый индикатор A/D Line
    pub fn new() -> Self {
        Self {
            ad_value: 0.0,
            ad_history: Vec::with_capacity(512),
            bars_count: 0,
            is_ready: false,
        }
    }
    
    
    /// Получить текущее значение A/D Line
    pub fn value(&self) -> f64 {
        self.ad_value
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.ad_value = 0.0;
        self.ad_history.clear();
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Получить направление движения A/D Line (тренд накопления/распределения)
    /// 1 = накопление (растет), -1 = распределение (падает), 0 = боковое движение
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.ad_history.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.ad_value;
        let past = self.ad_history[self.ad_history.len() - lookback - 1];
        
        let change = current - past;
        let threshold = past.abs() * 0.001; // 0.1% порог для определения значимого изменения
        
        if change > threshold {
            1  // Накопление
        } else if change < -threshold {
            -1 // Распределение
        } else {
            0  // Боковое движение
        }
    }
    
    /// Получить скорость изменения A/D Line
    pub fn rate_of_change(&self, lookback: usize) -> f64 {
        if !self.is_ready() || self.ad_history.len() < lookback + 1 {
            return 0.0;
        }
        
        let current = self.ad_value;
        let past = self.ad_history[self.ad_history.len() - lookback - 1];
        
        if past.abs() < 1e-12 {
            return 0.0;
        }
        
        ((current - past) / past.abs()) * 100.0
    }
    
    /// Проверить дивергенцию между ценой и A/D Line
    /// Возвращает: 1 = бычья дивергенция, -1 = медвежья дивергенция, 0 = нет дивергенции
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.ad_history.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_ad = self.ad_value;
        let past_ad = self.ad_history[self.ad_history.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let ad_change = current_ad - past_ad;
        
        // Бычья дивергенция: цена делает новый минимум, но A/D Line растет
        if price_change < 0.0 && ad_change > 0.0 {
            let price_change_pct = (price_change / past_price).abs();
            let ad_change_pct = (ad_change / past_ad.abs()).abs();
            
            // Проверяем значимость изменений (минимум 0.5%)
            if price_change_pct > 0.005 && ad_change_pct > 0.005 {
                return 1;
            }
        }
        
        // Медвежья дивергенция: цена делает новый максимум, но A/D Line падает
        if price_change > 0.0 && ad_change < 0.0 {
            let price_change_pct = price_change / past_price;
            let ad_change_pct = (ad_change / past_ad.abs()).abs();
            
            // Проверяем значимость изменений (минимум 0.5%)
            if price_change_pct > 0.005 && ad_change_pct > 0.005 {
                return -1;
            }
        }
        
        0
    }
    
    /// Получить торговый сигнал на основе направления A/D Line
    /// 1 = покупка (сильное накопление), -1 = продажа (сильное распределение), 0 = нейтрально
    pub fn trading_signal(&self, price_history: &[f64]) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Анализируем тренд за последние 5 и 20 периодов
        let short_trend = self.trend_direction(5);
        let long_trend = self.trend_direction(20);
        
        // Проверяем дивергенцию
        let divergence = self.check_divergence(price_history, 10);
        
        // Сигнал покупки: накопление + бычья дивергенция
        if short_trend == 1 && long_trend == 1 && divergence == 1 {
            return 1;
        }
        
        // Сигнал продажи: распределение + медвежья дивергенция  
        if short_trend == -1 && long_trend == -1 && divergence == -1 {
            return -1;
        }
        
        // Слабые сигналы на основе только направления тренда
        if short_trend == 1 && long_trend == 1 {
            return 1;
        } else if short_trend == -1 && long_trend == -1 {
            return -1;
        }
        
        0
    }
    
    /// Получить силу сигнала накопления/распределения
    pub fn signal_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        // Рассчитываем среднюю скорость изменения за разные периоды
        let roc_5 = self.rate_of_change(5).abs();
        let roc_10 = self.rate_of_change(10).abs();
        let roc_20 = self.rate_of_change(20).abs();
        
        // Взвешенная средняя скорость изменения
        let weighted_roc = (roc_5 * 0.5 + roc_10 * 0.3 + roc_20 * 0.2) / 100.0;
        
        // Нормализуем к диапазону [0, 1]
        weighted_roc.min(1.0)
    }
    
    /// Получить состояние рынка на основе A/D Line
    pub fn market_condition(&self) -> &'static str {
        let short_trend = self.trend_direction(5);
        let long_trend = self.trend_direction(20);
        
        match (short_trend, long_trend) {
            (1, 1) => "Strong Accumulation",      // Сильное накопление
            (1, 0) | (1, -1) => "Accumulation",  // Накопление
            (-1, -1) => "Strong Distribution",   // Сильное распределение
            (-1, 0) | (-1, 1) => "Distribution", // Распределение
            _ => "Neutral"                       // Нейтрально
        }
    }
    
    /// Получить последние N значений A/D Line для анализа
    pub fn history(&self, n: usize) -> Vec<f64> {
        let len = self.ad_history.len();
        if n >= len {
            self.ad_history.iter().cloned().collect()
        } else {
            self.ad_history[len - n..].to_vec()
        }
    }
    
    /// Рассчитать простую скользящую среднюю A/D Line
    pub fn moving_average(&self, period: usize) -> f64 {
        if !self.is_ready() || self.ad_history.len() < period {
            return self.ad_value;
        }
        
        let len = self.ad_history.len();
        let start_idx = len - period;
        
        let sum: f64 = self.ad_history[start_idx..].iter().sum();
        sum / period as f64
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "A/D Line: {:.2}, Condition: {}, Strength: {:.3}",
            self.ad_value,
            self.market_condition(),
            self.signal_strength()
        )
    }


    /// Feed the resolved input lanes `[high, low, close, volume]`.
    /// Returns current A/D value.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];

        self.bars_count += 1;

        let range = high - low;
        let money_flow_multiplier = if range.abs() < 1e-12 {
            0.0
        } else {
            ((close - low) - (high - close)) / range
        };

        let money_flow_volume = money_flow_multiplier * volume;
        self.ad_value += money_flow_volume;

        if self.ad_history.len() >= 512 {
            self.ad_history.remove(0);
        }
        self.ad_history.push(self.ad_value);

        if self.bars_count >= 1 {
            self.is_ready = true;
        }

        self.ad_value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AdConfig;

impl Indicator for AccumulationDistribution {
    const ID: IndicatorId = IndicatorId::Ad;
    /// No family — a cumulative accumulation/distribution PRODUCER, not a pluggable
    /// oscillator or MA member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close, volume — the standard A/D formula.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(1) cumulative update. Keeps a `Vec` history for divergence analytics, but the
    /// core `feed` path is a pure running sum — constant update.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Ad)];
    type Config = AdConfig;
    type Runtime = AccumulationDistribution;

    fn create(_cfg: AdConfig) -> AccumulationDistribution {
        AccumulationDistribution::new()
    }
}

impl crate::contract::Config for AdConfig {
    fn defaults() -> Self {
        AdConfig
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AccumulationDistribution {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ad, "A/D Line", Color::hex(0x4CAF50))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulation_distribution_creation() {
        let ad = AccumulationDistribution::new();
        assert!(!ad.is_ready());
        assert_eq!(ad.value(), 0.0);
    }

    #[test]
    fn test_accumulation_distribution_warmup() {
        let mut ad = AccumulationDistribution::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ad.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(ad.is_ready());
    }

    #[test]
    fn test_accumulation_distribution_values_finite() {
        let mut ad = AccumulationDistribution::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ad.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_accumulation_distribution_reset() {
        let mut ad = AccumulationDistribution::new();
        for _i in 0..20 {
            ad.feed(&[105.0, 95.0, 101.0, 1000.0]);
        }
        ad.reset();
        assert!(!ad.is_ready());
        assert_eq!(ad.value(), 0.0);
    }

    /// Factory resolves the fixed H/L/C/Volume lanes (open and timestamp are ignored).
    /// A bar where close == midpoint of high+low yields MFM = 0 → AD stays 0.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Ad(<<AccumulationDistribution as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Close at midpoint → MFM = 0 → AD accumulates nothing
        for _ in 0..5 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 110.0, low: 90.0, close: 100.0, volume: 5000.0,
            });
        }
        assert!(f.is_ready());
        assert_eq!(f.read(IndicatorOutputId::Ad), 0.0, "MFM=0 at midpoint -> AD=0, got {}", f.read(IndicatorOutputId::Ad));
        // Bullish bar: close near high → positive AD
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 110.0, low: 90.0, close: 108.0, volume: 1000.0,
        });
        assert!(f.read(IndicatorOutputId::Ad) > 0.0, "bullish bar should accumulate positive AD");
    }
}






















