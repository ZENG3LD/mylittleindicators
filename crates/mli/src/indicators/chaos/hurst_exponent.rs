//! Hurst Exponent - показатель Херста для анализа персистентности временного ряда
//! Определяет склонность рынка к трендовости или возврату к среднему
//! Значения: 0.0-1.0, где <0.5 = mean reversion, 0.5 = random walk, >0.5 = trending

/// Показатель Херста (R/S анализ)
#[derive(Debug, Clone)]
pub struct HurstExponent {
    period: usize,
    min_period: usize, // Минимальный период для расчета

    // Буфер данных
    prices: Vec<f64>,
    returns: Vec<f64>,

    // Результаты
    hurst_exponent: f64,
    persistence_score: f64, // -1.0 to 1.0 (-1 = mean reversion, 0 = random, 1 = trending)

    // R/S статистики
    rs_ratio: f64,
    range_value: f64,
    std_dev: f64,

    // Состояние
    is_ready: bool,
}

impl Default for HurstExponent {
    fn default() -> Self {
        Self::new(128)
    }
}

impl HurstExponent {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.min(512),
            min_period: (period / 2).max(10).min(period.saturating_sub(1)), // Reasonable minimum for R/S analysis
            prices: Vec::with_capacity(512),
            returns: Vec::with_capacity(512),
            hurst_exponent: 0.5, // Начальное значение (случайное блуждание)
            persistence_score: 0.0,
            rs_ratio: 1.0,
            range_value: 0.0,
            std_dev: 0.0,
            is_ready: false,
        }
    }

    /// Feed a single close price scalar.
    pub fn feed(&mut self, price: f64) -> f64 {
        // Добавляем цену в буфер
        if self.prices.len() >= self.period {
            self.prices.remove(0);
        }
        self.prices.push(price);

        // Рассчитываем доходности
        self.calculate_returns();

        // Рассчитываем показатель Херста если достаточно данных
        if self.returns.len() >= self.min_period {
            self.calculate_hurst_exponent();
            self.is_ready = true;
        }

        self.hurst_exponent
    }

    /// Рассчитать доходности
    fn calculate_returns(&mut self) {
        if self.prices.len() < 2 {
            return;
        }

        // Очищаем старые доходности
        self.returns.clear();

        // Рассчитываем логарифмические доходности
        for i in 1..self.prices.len() {
            let return_val = (self.prices[i] / self.prices[i - 1]).ln();
            self.returns.push(return_val);
        }
    }

    /// Рассчитать показатель Херста методом R/S анализа
    fn calculate_hurst_exponent(&mut self) {
        if self.returns.len() < self.min_period {
            return;
        }

        // Рассчитываем среднюю доходность
        let mean_return = self.returns.iter().sum::<f64>() / self.returns.len() as f64;

        // Рассчитываем кумулятивные отклонения от среднего
        let mut cumulative_deviations = Vec::new();
        let mut cumsum = 0.0;

        for &ret in &self.returns {
            cumsum += ret - mean_return;
            cumulative_deviations.push(cumsum);
        }

        // Рассчитываем Range (размах)
        let max_cumsum = cumulative_deviations.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_cumsum = cumulative_deviations.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        self.range_value = max_cumsum - min_cumsum;

        // Рассчитываем стандартное отклонение
        let variance = self.returns.iter()
            .map(|&ret| (ret - mean_return).powi(2))
            .sum::<f64>() / self.returns.len() as f64;
        self.std_dev = variance.sqrt();

        // R/S отношение
        if self.std_dev > 1e-10 {
            self.rs_ratio = self.range_value / self.std_dev;

            // Показатель Херста: H = log(R/S) / log(n/2) для R/S анализа
            let n = self.returns.len() as f64;
            if self.rs_ratio > 0.0 && n > 2.0 {
                self.hurst_exponent = self.rs_ratio.ln() / (n / 2.0).ln();

                // Ограничиваем значения в разумных пределах
                self.hurst_exponent = self.hurst_exponent.clamp(0.0, 1.0);

                // Рассчитываем persistence score
                self.persistence_score = (self.hurst_exponent - 0.5) * 2.0; // -1.0 to 1.0
            }
        }
    }

    /// Получить показатель Херста
    pub fn hurst_exponent(&self) -> f64 {
        self.hurst_exponent
    }

    pub fn value(&self) -> f64 {
        self.hurst_exponent
    }

    /// Получить оценку персистентности (-1.0 to 1.0)
    pub fn persistence_score(&self) -> f64 {
        self.persistence_score
    }

    /// Получить R/S отношение
    pub fn rs_ratio(&self) -> f64 {
        self.rs_ratio
    }

    /// Определить тип рыночного поведения
    pub fn market_behavior(&self) -> &'static str {
        match self.hurst_exponent {
            h if h < 0.4 => "Strong Mean Reversion",
            h if h < 0.45 => "Weak Mean Reversion",
            h if h < 0.55 => "Random Walk",
            h if h < 0.6 => "Weak Trending",
            h if h < 0.7 => "Strong Trending",
            _ => "Very Strong Trending",
        }
    }

    /// Получить сигнал торговли на основе персистентности
    pub fn trading_signal(&self) -> i8 {
        match self.hurst_exponent {
            h if h < 0.4 => -1, // Mean reversion - контртренд
            h if h > 0.6 => 1,  // Trending - следование тренду
            _ => 0,             // Нейтрально
        }
    }

    /// Определить силу сигнала
    pub fn signal_strength(&self) -> f64 {
        (self.hurst_exponent - 0.5).abs() * 2.0
    }

    /// Определить оптимальную стратегию
    pub fn optimal_strategy(&self) -> &'static str {
        match self.hurst_exponent {
            h if h < 0.45 => "Mean Reversion Strategy",
            h if h > 0.55 => "Trend Following Strategy",
            _ => "Range Trading Strategy",
        }
    }

    /// Получить рыночную эффективность (близость к 0.5)
    pub fn market_efficiency(&self) -> f64 {
        1.0 - (self.hurst_exponent - 0.5).abs() * 2.0
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }

    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.prices.clear();
        self.returns.clear();
        self.hurst_exponent = 0.5;
        self.persistence_score = 0.0;
        self.rs_ratio = 1.0;
        self.range_value = 0.0;
        self.std_dev = 0.0;
        self.is_ready = false;
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
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`HurstExponent`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HurstConfig {
    pub period: Param<usize>,
}

impl Indicator for HurstExponent {
    const ID: IndicatorId = IndicatorId::Hurst;
    /// Statistical chaos / persistence detector — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default: Close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period) per bar: rescans returns for mean/cumsum/range/std on each update.
    /// Two period-deep Vec stores (prices, returns).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Hurst)];

    type Config = HurstConfig;
    type Runtime = HurstExponent;

    fn create(cfg: HurstConfig) -> HurstExponent {
        HurstExponent::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for HurstConfig {
    fn defaults() -> Self {
        HurstConfig { period: Param::Solo(128) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for HurstExponent {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Hurst, "Hurst", Color::hex(0x9C27B0))
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.5, Color::hex(0x9E9E9E)).with_label("Random Walk"))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hurst_exponent_creation() {
        let ind = HurstExponent::new(50);
        assert!(!ind.is_ready());
        assert_eq!(ind.hurst_exponent(), 0.5);
    }

    #[test]
    fn test_hurst_exponent_warmup() {
        let mut ind = HurstExponent::new(30);
        // min_period is min(max(30/2, 10), 29) = 15, need period bars to fill price buffer
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.feed(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_hurst_exponent_values_range() {
        let mut ind = HurstExponent::new(30);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let h = ind.feed(price);
            assert!(h >= 0.0 && h <= 1.0);
        }
    }

    #[test]
    fn test_hurst_exponent_reset() {
        let mut ind = HurstExponent::new(30);
        for i in 0..40 {
            ind.feed(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.hurst_exponent(), 0.5);
    }

    #[test]
    fn factory_feeds_resolved_hurst() {
        use crate::contract::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::Hurst(<<HurstExponent as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // not used — SOURCE is Close
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Hurst);
        assert!(v >= 0.0 && v <= 1.0);
    }
}
