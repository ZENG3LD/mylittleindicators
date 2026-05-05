//! Chaos Oscillator - комбинированный индикатор хаотичности рынка
//! Объединяет фрактальную размерность, показатель Херста и волатильность
//! для определения степени хаоса в рыночных движениях

use arrayvec::ArrayVec;
use super::fractal_dimension::FractalDimension;
use super::hurst_exponent::HurstExponent;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Осциллятор хаоса
#[derive(Clone)]
pub struct ChaosOscillator {
    period: usize,
    
    // Компоненты анализа хаоса
    fractal_dimension: FractalDimension,
    hurst_exponent: HurstExponent,
    
    // Буферы для дополнительных расчетов
    prices: ArrayVec<f64, 512>,
    volatilities: ArrayVec<f64, 512>,
    
    // Результаты
    chaos_index: f64,      // 0.0-1.0, где 1.0 = максимальный хаос
    predictability: f64,   // 0.0-1.0, где 1.0 = максимальная предсказуемость
    market_regime: MarketRegime,
    
    // Составляющие индекса
    complexity_weight: f64,    // Вес фрактальной размерности
    persistence_weight: f64,   // Вес показателя Херста
    volatility_weight: f64,    // Вес волатильности
    
    // Состояние
    is_ready: bool,
}

/// Режимы рынка на основе анализа хаоса
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketRegime {
    OrderedTrend,      // Упорядоченный тренд
    ChaoticTrend,      // Хаотичный тренд
    OrderedRange,      // Упорядоченный флэт
    ChaoticRange,      // Хаотичный флэт
    TransitionPhase,   // Переходная фаза
}

impl ChaosOscillator {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.min(512),
            fractal_dimension: FractalDimension::new(period, period / 8),
            hurst_exponent: HurstExponent::new(period),
            prices: ArrayVec::new(),
            volatilities: ArrayVec::new(),
            chaos_index: 0.5,
            predictability: 0.5,
            market_regime: MarketRegime::TransitionPhase,
            complexity_weight: 0.4,
            persistence_weight: 0.4,
            volatility_weight: 0.2,
            is_ready: false,
        }
    }
    
    /// Создать с настраиваемыми весами компонентов
    pub fn new_with_weights(
        period: usize,
        complexity_weight: f64,
        persistence_weight: f64,
        volatility_weight: f64,
    ) -> Self {
        // Нормализуем веса
        let total_weight = complexity_weight + persistence_weight + volatility_weight;
        let norm_complexity = complexity_weight / total_weight;
        let norm_persistence = persistence_weight / total_weight;
        let norm_volatility = volatility_weight / total_weight;
        
        Self {
            period: period.min(512),
            fractal_dimension: FractalDimension::new(period, period / 8),
            hurst_exponent: HurstExponent::new(period),
            prices: ArrayVec::new(),
            volatilities: ArrayVec::new(),
            chaos_index: 0.5,
            predictability: 0.5,
            market_regime: MarketRegime::TransitionPhase,
            complexity_weight: norm_complexity,
            persistence_weight: norm_persistence,
            volatility_weight: norm_volatility,
            is_ready: false,
        }
    }
    
    /// Обновить осциллятор новым баром
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
        // Добавляем цену закрытия
        if self.prices.len() >= self.period {
            self.prices.remove(0);
        }
        self.prices.push(close);
        
        // Рассчитываем волатильность (True Range)
        let volatility = if self.prices.len() >= 2 {
            let prev_close = self.prices[self.prices.len() - 2];
            let tr1 = high - low;
            let tr2 = (high - prev_close).abs();
            let tr3 = (low - prev_close).abs();
            tr1.max(tr2).max(tr3)
        } else {
            high - low
        };
        
        if self.volatilities.len() >= self.period {
            self.volatilities.remove(0);
        }
        self.volatilities.push(volatility);
        
        // Обновляем компоненты
        self.fractal_dimension.update(close);
        self.hurst_exponent.update(close);
        
        // Рассчитываем индекс хаоса
        if self.prices.len() >= self.period / 2 {
            self.calculate_chaos_index();
            self.determine_market_regime();
            self.is_ready = true;
        }
        
        self.chaos_index
    }
    
    /// Рассчитать индекс хаоса
    fn calculate_chaos_index(&mut self) {
        // Компонент сложности (фрактальная размерность)
        let complexity_component = self.fractal_dimension.complexity_score();
        
        // Компонент персистентности (обратный показатель Херста)
        // Чем ближе к 0.5, тем больше хаос
        let hurst = self.hurst_exponent.hurst_exponent();
        let persistence_component = 1.0 - (hurst - 0.5).abs() * 2.0;
        
        // Компонент волатильности (нормализованная волатильность)
        let volatility_component = if !self.volatilities.is_empty() && !self.prices.is_empty() {
            let avg_volatility = self.volatilities.iter().sum::<f64>() / self.volatilities.len() as f64;
            let avg_price = self.prices.iter().sum::<f64>() / self.prices.len() as f64;
            if avg_price > 0.0 {
                (avg_volatility / avg_price).min(1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Комбинируем компоненты с весами
        self.chaos_index = (complexity_component * self.complexity_weight +
            persistence_component * self.persistence_weight + volatility_component * self.volatility_weight).clamp(0.0, 1.0);
        
        // Предсказуемость - обратная величина хаоса
        self.predictability = 1.0 - self.chaos_index;
    }
    
    /// Определить режим рынка
    fn determine_market_regime(&mut self) {
        let hurst = self.hurst_exponent.hurst_exponent();
        let is_trending = !(0.45..=0.55).contains(&hurst);
        let is_chaotic = self.chaos_index > 0.6;
        
        self.market_regime = match (is_trending, is_chaotic) {
            (true, false) => MarketRegime::OrderedTrend,
            (true, true) => MarketRegime::ChaoticTrend,
            (false, false) => MarketRegime::OrderedRange,
            (false, true) => MarketRegime::ChaoticRange,
        };
        
        // Переходная фаза при средних значениях
        if self.chaos_index > 0.4 && self.chaos_index < 0.6 {
            self.market_regime = MarketRegime::TransitionPhase;
        }
    }
    
    /// Получить индекс хаоса
    pub fn chaos_index(&self) -> f64 {
        self.chaos_index
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.chaos_index)
    }
    
    /// Получить предсказуемость
    pub fn predictability(&self) -> f64 {
        self.predictability
    }
    
    /// Получить режим рынка
    pub fn market_regime(&self) -> MarketRegime {
        self.market_regime
    }
    
    /// Получить текстовое описание режима рынка
    pub fn market_regime_description(&self) -> &'static str {
        match self.market_regime {
            MarketRegime::OrderedTrend => "Ordered Trend - Predictable directional movement",
            MarketRegime::ChaoticTrend => "Chaotic Trend - Unpredictable directional movement",
            MarketRegime::OrderedRange => "Ordered Range - Predictable sideways movement",
            MarketRegime::ChaoticRange => "Chaotic Range - Unpredictable sideways movement",
            MarketRegime::TransitionPhase => "Transition Phase - Market changing regime",
        }
    }
    
    /// Получить торговый сигнал на основе анализа хаоса
    pub fn trading_signal(&self) -> i8 {
        match self.market_regime {
            MarketRegime::OrderedTrend => {
                // В упорядоченном тренде следуем направлению
                self.hurst_exponent.trading_signal()
            },
            MarketRegime::ChaoticTrend => {
                // В хаотичном тренде осторожность
                if self.chaos_index > 0.8 { 0 } else { self.hurst_exponent.trading_signal() }
            },
            MarketRegime::OrderedRange => {
                // В упорядоченном флэте - контртренд
                -self.hurst_exponent.trading_signal()
            },
            MarketRegime::ChaoticRange => {
                // В хаотичном флэте - ожидание
                0
            },
            MarketRegime::TransitionPhase => {
                // В переходной фазе - ожидание
                0
            },
        }
    }
    
    /// Получить силу сигнала
    pub fn signal_strength(&self) -> f64 {
        match self.market_regime {
            MarketRegime::OrderedTrend => self.predictability,
            MarketRegime::OrderedRange => self.predictability * 0.7,
            _ => self.predictability * 0.3, // Слабые сигналы в хаосе
        }
    }
    
    /// Получить компоненты анализа
    pub fn get_components(&self) -> (f64, f64, f64) {
        let complexity = self.fractal_dimension.complexity_score();
        let persistence = 1.0 - (self.hurst_exponent.hurst_exponent() - 0.5).abs() * 2.0;
        let volatility = if !self.volatilities.is_empty() && !self.prices.is_empty() {
            let avg_vol = self.volatilities.iter().sum::<f64>() / self.volatilities.len() as f64;
            let avg_price = self.prices.iter().sum::<f64>() / self.prices.len() as f64;
            if avg_price > 0.0 { (avg_vol / avg_price).min(1.0) } else { 0.0 }
        } else { 0.0 };
        
        (complexity, persistence, volatility)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> IndicatorValue {
        self.update(high, low, close);
        self.value()
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.fractal_dimension.reset();
        self.hurst_exponent.reset();
        self.prices.clear();
        self.volatilities.clear();
        self.chaos_index = 0.5;
        self.predictability = 0.5;
        self.market_regime = MarketRegime::TransitionPhase;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_oscillator_creation() {
        let ind = ChaosOscillator::new(50);
        assert!(!ind.is_ready());
        assert_eq!(ind.period(), 50);
    }

    #[test]
    fn test_chaos_oscillator_warmup() {
        let mut ind = ChaosOscillator::new(30);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price + 1.0, price - 1.0, price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_chaos_oscillator_values_range() {
        let mut ind = ChaosOscillator::new(30);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let chaos = ind.update(price + 1.0, price - 1.0, price);
            assert!(chaos >= 0.0 && chaos <= 1.0);
            assert!(ind.predictability() >= 0.0 && ind.predictability() <= 1.0);
        }
    }

    #[test]
    fn test_chaos_oscillator_reset() {
        let mut ind = ChaosOscillator::new(30);
        for i in 0..40 {
            ind.update(100.0 + i as f64, 105.0, 101.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.chaos_index(), 0.5);
    }
} 






















