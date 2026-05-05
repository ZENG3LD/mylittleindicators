//! Market Microstructure - анализатор микроструктуры рынка
//! Анализирует микроструктурные характеристики рынка: ликвидность, эффективность, качество исполнения

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Bar;
use arrayvec::ArrayVec;

/// Метрики ликвидности
#[derive(Debug, Clone)]
pub struct LiquidityMetrics {
    pub bid_ask_spread: f64,
    pub spread_pct: f64,
    pub effective_spread: f64,
    pub market_depth: f64,
    pub price_impact: f64,
    pub liquidity_score: f64, // 0.0 - 1.0
}

/// Метрики эффективности рынка
#[derive(Debug, Clone)]
pub struct EfficiencyMetrics {
    pub price_discovery_speed: f64,
    pub information_ratio: f64,
    pub volatility_clustering: f64,
    pub mean_reversion_strength: f64,
    pub trend_persistence: f64,
    pub efficiency_score: f64, // 0.0 - 1.0
}

/// Метрики качества исполнения
#[derive(Debug, Clone)]
pub struct ExecutionQuality {
    pub slippage_estimate: f64,
    pub timing_risk: f64,
    pub adverse_selection: f64,
    pub order_flow_toxicity: f64,
    pub execution_score: f64, // 0.0 - 1.0
}

/// Анализатор микроструктуры рынка
#[derive(Clone)]
pub struct MarketMicrostructure {
    period: usize,
    
    // Буферы данных
    volume_bars: ArrayVec<Bar, 512>,
    price_changes: ArrayVec<f64, 512>,
    volume_changes: ArrayVec<f64, 512>,
    spreads: ArrayVec<f64, 512>,
    
    // Текущие метрики
    liquidity_metrics: LiquidityMetrics,
    efficiency_metrics: EfficiencyMetrics,
    execution_quality: ExecutionQuality,
    
    // Расчетные переменные
    cumulative_volume: f64,
    cumulative_price_volume: f64,
    volatility_sum: f64,
    autocorrelation_sum: f64,
    
    // Состояние рынка
    market_regime: MarketRegime,
    microstructure_score: f64,
}

/// Режим рынка
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketRegime {
    HighLiquidity,
    MediumLiquidity,
    LowLiquidity,
    StressedMarket,
    NormalMarket,
    VolatileMarket,
}

impl MarketMicrostructure {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            volume_bars: ArrayVec::new(),
            price_changes: ArrayVec::new(),
            volume_changes: ArrayVec::new(),
            spreads: ArrayVec::new(),
            liquidity_metrics: LiquidityMetrics {
                bid_ask_spread: 0.0,
                spread_pct: 0.0,
                effective_spread: 0.0,
                market_depth: 0.0,
                price_impact: 0.0,
                liquidity_score: 0.5,
            },
            efficiency_metrics: EfficiencyMetrics {
                price_discovery_speed: 0.0,
                information_ratio: 0.0,
                volatility_clustering: 0.0,
                mean_reversion_strength: 0.0,
                trend_persistence: 0.0,
                efficiency_score: 0.5,
            },
            execution_quality: ExecutionQuality {
                slippage_estimate: 0.0,
                timing_risk: 0.0,
                adverse_selection: 0.0,
                order_flow_toxicity: 0.0,
                execution_score: 0.5,
            },
            cumulative_volume: 0.0,
            cumulative_price_volume: 0.0,
            volatility_sum: 0.0,
            autocorrelation_sum: 0.0,
            market_regime: MarketRegime::NormalMarket,
            microstructure_score: 0.5,
        }
    }
    
    /// Обновить анализатор новым Bar
    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        // Добавляем в буфер
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);
        
        // Обновляем производные данные
        self.update_derived_data(volume_bar);
        
        // Рассчитываем метрики
        self.calculate_liquidity_metrics();
        self.calculate_efficiency_metrics();
        self.calculate_execution_quality();
        
        // Определяем режим рынка
        self.determine_market_regime();
        
        // Рассчитываем общий скор
        self.calculate_microstructure_score();
        
        self.microstructure_score
    }
    
    /// Обновить производные данные
    fn update_derived_data(&mut self, volume_bar: &Bar) {
        // Изменения цены
        if let Some(prev_bar) = self.volume_bars.get(self.volume_bars.len().saturating_sub(2)) {
            let price_change = (volume_bar.close - prev_bar.close) / prev_bar.close;
            if self.price_changes.len() >= self.period {
                self.price_changes.remove(0);
            }
            self.price_changes.push(price_change);
            
            let volume_change = (volume_bar.volume - prev_bar.volume) / prev_bar.volume.max(1.0);
            if self.volume_changes.len() >= self.period {
                self.volume_changes.remove(0);
            }
            self.volume_changes.push(volume_change);
        }
        
        // Спреды
        if let Some(spread) = None::<f64> {
            if self.spreads.len() >= self.period {
                self.spreads.remove(0);
            }
            self.spreads.push(spread);
        }
        
        // Кумулятивные данные
        self.cumulative_volume += volume_bar.volume;
        let typical_price = (volume_bar.high + volume_bar.low + volume_bar.close) / 3.0;
        self.cumulative_price_volume += typical_price * volume_bar.volume;
    }
    
    /// Рассчитать метрики ликвидности
    fn calculate_liquidity_metrics(&mut self) {
        if self.spreads.is_empty() {
            return;
        }
        
        // Средний спред
        let avg_spread = self.spreads.iter().sum::<f64>() / self.spreads.len() as f64;
        self.liquidity_metrics.bid_ask_spread = avg_spread;
        
        // Спред в процентах
        if let Some(last_bar) = self.volume_bars.last() {
            self.liquidity_metrics.spread_pct = (avg_spread / last_bar.close) * 100.0;
        }
        
        // Эффективный спред (с учетом объема)
        let total_volume: f64 = self.volume_bars.iter().map(|b| b.volume).sum();
        if total_volume > 0.0 {
            let weighted_spread: f64 = self.volume_bars.iter()
                .zip(self.spreads.iter())
                .map(|(bar, spread)| spread * bar.volume)
                .sum();
            self.liquidity_metrics.effective_spread = weighted_spread / total_volume;
        }
        
        // Глубина рынка (приблизительная оценка)
        self.liquidity_metrics.market_depth = total_volume / self.volume_bars.len() as f64;
        
        // Ценовое воздействие
        if self.price_changes.len() > 1 && self.volume_changes.len() > 1 {
            let price_vol_correlation = self.calculate_correlation(&self.price_changes, &self.volume_changes);
            self.liquidity_metrics.price_impact = price_vol_correlation.abs();
        }
        
        // Скор ликвидности
        let spread_score = (1.0 - (self.liquidity_metrics.spread_pct / 1.0).min(1.0)).max(0.0);
        let depth_score = (self.liquidity_metrics.market_depth / 1000.0).min(1.0);
        let impact_score = (1.0 - self.liquidity_metrics.price_impact).max(0.0);
        
        self.liquidity_metrics.liquidity_score = (spread_score + depth_score + impact_score) / 3.0;
    }
    
    /// Рассчитать метрики эффективности
    fn calculate_efficiency_metrics(&mut self) {
        if self.price_changes.len() < 10 {
            return;
        }
        
        // Скорость ценообразования (обратная к автокорреляции)
        let autocorr = self.calculate_autocorrelation(&self.price_changes, 1);
        self.efficiency_metrics.price_discovery_speed = 1.0 - autocorr.abs();
        
        // Информационный коэффициент
        let mean_return = self.price_changes.iter().sum::<f64>() / self.price_changes.len() as f64;
        let volatility = self.calculate_volatility(&self.price_changes);
        if volatility > 0.0 {
            self.efficiency_metrics.information_ratio = mean_return / volatility;
        }
        
        // Кластеризация волатильности
        let vol_changes: ArrayVec<f64, 512> = self.price_changes.windows(2)
            .map(|w| (w[1].abs() - w[0].abs()).abs())
            .collect();
        if vol_changes.len() > 1 {
            self.efficiency_metrics.volatility_clustering = self.calculate_autocorrelation(&vol_changes, 1);
        }
        
        // Сила возврата к среднему
        if self.price_changes.len() > 2 {
            let mean_reversion = self.calculate_mean_reversion_strength();
            self.efficiency_metrics.mean_reversion_strength = mean_reversion;
        }
        
        // Постоянство тренда
        let trend_persistence = self.calculate_trend_persistence();
        self.efficiency_metrics.trend_persistence = trend_persistence;
        
        // Скор эффективности
        let discovery_score = self.efficiency_metrics.price_discovery_speed;
        let clustering_score = 1.0 - self.efficiency_metrics.volatility_clustering.abs();
        let reversion_score = self.efficiency_metrics.mean_reversion_strength.abs();
        
        self.efficiency_metrics.efficiency_score = (discovery_score + clustering_score + reversion_score) / 3.0;
    }
    
    /// Рассчитать качество исполнения
    fn calculate_execution_quality(&mut self) {
        if self.volume_bars.len() < 5 {
            return;
        }
        
        // Оценка проскальзывания
        let volatility = self.calculate_volatility(&self.price_changes);
        self.execution_quality.slippage_estimate = volatility * 0.5; // Приблизительная оценка
        
        // Риск тайминга
        if let Some(last_bar) = self.volume_bars.last() {
            let high_low_ratio = (last_bar.high - last_bar.low) / last_bar.close;
            self.execution_quality.timing_risk = high_low_ratio;
        }
        
        // Неблагоприятный отбор
        let adverse_selection = self.liquidity_metrics.price_impact * 0.7;
        self.execution_quality.adverse_selection = adverse_selection;
        
        // Токсичность ордер флоу
        let flow_toxicity = if self.efficiency_metrics.price_discovery_speed < 0.5 { 0.8 } else { 0.2 };
        self.execution_quality.order_flow_toxicity = flow_toxicity;
        
        // Скор качества исполнения
        let slippage_score = (1.0 - (self.execution_quality.slippage_estimate / 0.01).min(1.0)).max(0.0);
        let timing_score = (1.0 - (self.execution_quality.timing_risk / 0.05).min(1.0)).max(0.0);
        let selection_score = (1.0 - self.execution_quality.adverse_selection).max(0.0);
        let toxicity_score = 1.0 - self.execution_quality.order_flow_toxicity;
        
        self.execution_quality.execution_score = (slippage_score + timing_score + selection_score + toxicity_score) / 4.0;
    }
    
    /// Определить режим рынка
    fn determine_market_regime(&mut self) {
        let liquidity_score = self.liquidity_metrics.liquidity_score;
        let efficiency_score = self.efficiency_metrics.efficiency_score;
        let execution_score = self.execution_quality.execution_score;
        
        let avg_score = (liquidity_score + efficiency_score + execution_score) / 3.0;
        
        self.market_regime = match avg_score {
            s if s >= 0.8 => MarketRegime::HighLiquidity,
            s if s >= 0.6 => MarketRegime::MediumLiquidity,
            s if s >= 0.4 => MarketRegime::LowLiquidity,
            s if s >= 0.2 => MarketRegime::VolatileMarket,
            _ => MarketRegime::StressedMarket,
        };
    }
    
    /// Рассчитать общий скор микроструктуры
    fn calculate_microstructure_score(&mut self) {
        self.microstructure_score = self.liquidity_metrics.liquidity_score * 0.4 +
            self.efficiency_metrics.efficiency_score * 0.3 +
            self.execution_quality.execution_score * 0.3;
    }
    
    /// Вспомогательные функции расчета
    fn calculate_correlation(&self, x: &ArrayVec<f64, 512>, y: &ArrayVec<f64, 512>) -> f64 {
        if x.len() != y.len() || x.len() < 2 {
            return 0.0;
        }
        
        let n = x.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
        let sum_x2: f64 = x.iter().map(|a| a * a).sum();
        let sum_y2: f64 = y.iter().map(|b| b * b).sum();
        
        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();
        
        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }
    
    fn calculate_autocorrelation(&self, data: &ArrayVec<f64, 512>, lag: usize) -> f64 {
        if data.len() <= lag {
            return 0.0;
        }
        
        let n = data.len() - lag;
        let x1: ArrayVec<f64, 512> = data.iter().take(n).cloned().collect();
        let x2: ArrayVec<f64, 512> = data.iter().skip(lag).cloned().collect();
        
        self.calculate_correlation(&x1, &x2)
    }
    
    fn calculate_volatility(&self, data: &ArrayVec<f64, 512>) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }
        
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
        variance.sqrt()
    }
    
    fn calculate_mean_reversion_strength(&self) -> f64 {
        // Упрощенная оценка силы возврата к среднему
        let autocorr_1 = self.calculate_autocorrelation(&self.price_changes, 1);
        let autocorr_5 = self.calculate_autocorrelation(&self.price_changes, 5);
        
        if autocorr_1 < 0.0 && autocorr_5.abs() < autocorr_1.abs() {
            autocorr_1.abs()
        } else {
            0.0
        }
    }
    
    fn calculate_trend_persistence(&self) -> f64 {
        if self.price_changes.len() < 10 {
            return 0.0;
        }
        
        let positive_changes = self.price_changes.iter().filter(|&&x| x > 0.0).count();
        let total_changes = self.price_changes.len();
        
        let trend_ratio = positive_changes as f64 / total_changes as f64;
        (trend_ratio - 0.5).abs() * 2.0 // Нормализуем к [0, 1]
    }
    
    // Геттеры для метрик
    pub fn liquidity_metrics(&self) -> &LiquidityMetrics {
        &self.liquidity_metrics
    }
    
    pub fn efficiency_metrics(&self) -> &EfficiencyMetrics {
        &self.efficiency_metrics
    }
    
    pub fn execution_quality(&self) -> &ExecutionQuality {
        &self.execution_quality
    }
    
    pub fn market_regime(&self) -> MarketRegime {
        self.market_regime
    }
    
    pub fn microstructure_score(&self) -> f64 {
        self.microstructure_score
    }
    
    pub fn is_ready(&self) -> bool {
        self.volume_bars.len() >= (self.period / 2).max(5)
    }

    /// Стандартный update_bar для совместимости с API индикаторов
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let bar = Bar {
            time: 0,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        };
        self.update_volume_bar(&bar)
    }

    /// Получить значение как IndicatorValue
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.microstructure_score)
    }
    
    pub fn reset(&mut self) {
        self.volume_bars.clear();
        self.price_changes.clear();
        self.volume_changes.clear();
        self.spreads.clear();
        self.cumulative_volume = 0.0;
        self.cumulative_price_volume = 0.0;
        self.volatility_sum = 0.0;
        self.autocorrelation_sum = 0.0;
        self.market_regime = MarketRegime::NormalMarket;
        self.microstructure_score = 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_microstructure_creation() {
        let ind = MarketMicrostructure::new(20);
        assert!(!ind.is_ready());
        assert_eq!(ind.microstructure_score(), 0.5);
    }

    #[test]
    fn test_market_microstructure_warmup() {
        let mut ind = MarketMicrostructure::new(10);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0 + (i as f64 * 0.1).sin(),
                high: 101.0 + (i as f64 * 0.1).sin(),
                low: 99.0 + (i as f64 * 0.1).sin(),
                close: 100.5 + (i as f64 * 0.1).sin(),
                volume: 1000.0 + i as f64 * 10.0,
            };
            ind.update_volume_bar(&bar);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_market_microstructure_score_range() {
        let mut ind = MarketMicrostructure::new(10);
        for i in 0..20 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            let score = ind.update_volume_bar(&bar);
            assert!(score >= 0.0 && score <= 1.0);
        }
    }

    #[test]
    fn test_market_microstructure_reset() {
        let mut ind = MarketMicrostructure::new(10);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.microstructure_score(), 0.5);
    }
} 






















