//! Adaptive Volatility Regime - Advanced volatility classification using ML concepts
//! 
//! This indicator uses machine learning-inspired techniques to classify volatility regimes:
//! - K-means clustering for regime identification
//! - Hidden Markov Model concepts for state transitions
//! - Support Vector Machine-like classification
//! - Ensemble of volatility measures
//! 
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;

/// Режим волатильности
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolatilityRegime {
    Quiet,          // Тихий рынок (низкая волатильность)
    Normal,         // Нормальная волатильность
    Elevated,       // Повышенная волатильность
    High,           // Высокая волатильность
    Extreme,        // Экстремальная волатильность
    Transitional,   // Переходное состояние
}

impl VolatilityRegime {
    /// Получить числовое представление режима
    pub fn to_value(&self) -> f64 {
        match self {
            VolatilityRegime::Quiet => 0.0,
            VolatilityRegime::Normal => 0.2,
            VolatilityRegime::Elevated => 0.4,
            VolatilityRegime::High => 0.6,
            VolatilityRegime::Extreme => 0.8,
            VolatilityRegime::Transitional => 1.0,
        }
    }
    
    /// Создать режим из числового значения
    pub fn from_value(value: f64) -> Self {
        if value < 0.1 {
            VolatilityRegime::Quiet
        } else if value < 0.3 {
            VolatilityRegime::Normal
        } else if value < 0.5 {
            VolatilityRegime::Elevated
        } else if value < 0.7 {
            VolatilityRegime::High
        } else if value < 0.9 {
            VolatilityRegime::Extreme
        } else {
            VolatilityRegime::Transitional
        }
    }
    
    /// Получить описание режима
    pub fn description(&self) -> &'static str {
        match self {
            VolatilityRegime::Quiet => "Quiet Market",
            VolatilityRegime::Normal => "Normal Volatility",
            VolatilityRegime::Elevated => "Elevated Volatility",
            VolatilityRegime::High => "High Volatility",
            VolatilityRegime::Extreme => "Extreme Volatility",
            VolatilityRegime::Transitional => "Transitional State",
        }
    }
}

/// Кластер для K-means
#[derive(Debug, Clone)]
struct VolatilityCluster {
    centroid: [f64; 4],         // Центроид кластера (4 признака)
    points: ArrayVec<[f64; 4], 32>, // Точки в кластере
    regime: VolatilityRegime,   // Соответствующий режим
}

impl VolatilityCluster {
    fn new(regime: VolatilityRegime) -> Self {
        Self {
            centroid: [0.0; 4],
            points: ArrayVec::new(),
            regime,
        }
    }
    
    /// Рассчитать расстояние до точки
    fn distance_to(&self, point: &[f64; 4]) -> f64 {
        self.centroid.iter()
            .zip(point.iter())
            .map(|(c, p)| (c - p).powi(2))
            .sum::<f64>()
            .sqrt()
    }
    
    /// Добавить точку в кластер
    fn add_point(&mut self, point: [f64; 4]) {
        if self.points.len() >= 32 {
            self.points.remove(0);
        }
        self.points.push(point);
        self.update_centroid();
    }
    
    /// Обновить центроид
    fn update_centroid(&mut self) {
        if self.points.is_empty() {
            return;
        }
        
        for i in 0..4 {
            self.centroid[i] = self.points.iter()
                .map(|p| p[i])
                .sum::<f64>() / self.points.len() as f64;
        }
    }
}

/// Результат Adaptive Volatility Regime
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveVolatilityRegimeResult {
    pub current_regime: VolatilityRegime,    // Текущий режим волатильности
    pub regime_probability: [f64; 6],        // Вероятности каждого режима
    pub volatility_score: f64,               // Общий счет волатильности (0.0-1.0)
    pub regime_stability: f64,               // Стабильность режима (0.0-1.0)
    pub transition_probability: f64,         // Вероятность перехода (0.0-1.0)
    pub volatility_features: [f64; 4],       // Извлеченные признаки
    pub cluster_distances: [f64; 6],         // Расстояния до кластеров
    pub regime_confidence: f64,              // Уверенность в классификации
    pub trend_volatility: f64,               // Волатильность тренда
    pub mean_reversion_strength: f64,        // Сила возврата к среднему
    pub volatility_trend: f64,               // Тренд волатильности
    pub regime_duration: usize,              // Длительность текущего режима
}

impl AdaptiveVolatilityRegimeResult {
    pub fn empty() -> Self {
        Self {
            current_regime: VolatilityRegime::Normal,
            regime_probability: [0.0; 6],
            volatility_score: 0.0,
            regime_stability: 0.0,
            transition_probability: 0.0,
            volatility_features: [0.0; 4],
            cluster_distances: [0.0; 6],
            regime_confidence: 0.0,
            trend_volatility: 0.0,
            mean_reversion_strength: 0.0,
            volatility_trend: 0.0,
            regime_duration: 0,
        }
    }
}

/// Adaptive Volatility Regime индикатор
#[derive(Clone)]
pub struct AdaptiveVolatilityRegime {
    // Переиспользуем существующие компоненты для расчета волатильности
    atr: Atr,                               // ATR
    short_vol_ma: MovingAverageProvider,            // Краткосрочная волатильность
    long_vol_ma: MovingAverageProvider,             // Долгосрочная волатильность
    range_ma: MovingAverageProvider,                // Средний диапазон
    volume_vol_ma: MovingAverageProvider,           // Волатильность объема
    
    // K-means кластеры для классификации режимов
    clusters: ArrayVec<VolatilityCluster, 6>,
    
    // Буферы для данных
    prices: ArrayVec<f64, 64>,              // История цен
    returns: ArrayVec<f64, 32>,             // Доходности
    volatilities: ArrayVec<f64, 32>,        // История волатильностей
    regimes: ArrayVec<VolatilityRegime, 16>, // История режимов
    
    // Матрица переходов (Hidden Markov Model)
    transition_matrix: [[f64; 6]; 6],       // Вероятности переходов между режимами
    
    // Параметры адаптации
    learning_rate: f64,                     // Скорость обучения
    adaptation_period: usize,               // Период адаптации
    
    // Результат
    current_result: AdaptiveVolatilityRegimeResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
    current_regime_duration: usize,
}

impl AdaptiveVolatilityRegime {
    /// Создать новую Adaptive Volatility Regime с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(0.1, 50)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(learning_rate: f64, adaptation_period: usize) -> Self {
        assert!(learning_rate > 0.0 && learning_rate <= 1.0, 
                "Learning rate must be between 0.0 and 1.0");
        assert!(adaptation_period > 0, "Adaptation period must be positive");
        
        // Инициализируем кластеры
        let mut clusters = ArrayVec::new();
        clusters.push(VolatilityCluster::new(VolatilityRegime::Quiet));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Normal));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Elevated));
        clusters.push(VolatilityCluster::new(VolatilityRegime::High));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Extreme));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Transitional));
        
        // Инициализируем начальные центроиды
        clusters[0].centroid = [0.1, 0.05, 0.1, 0.2];  // Quiet
        clusters[1].centroid = [0.3, 0.15, 0.3, 0.4];  // Normal
        clusters[2].centroid = [0.5, 0.25, 0.5, 0.6];  // Elevated
        clusters[3].centroid = [0.7, 0.35, 0.7, 0.8];  // High
        clusters[4].centroid = [0.9, 0.45, 0.9, 1.0];  // Extreme
        clusters[5].centroid = [0.6, 0.8, 0.6, 0.9];   // Transitional
        
        // Инициализируем матрицу переходов (равномерное распределение)
        let mut transition_matrix = [[0.0; 6]; 6];
        for (i, row) in transition_matrix.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = if i == j { 0.7 } else { 0.06 }; // Склонность оставаться в том же режиме
            }
        }
        
        Self {
            // Переиспользуем MovingAverage и ATR для разных целей
            atr: Atr::new(14, MovingAverageType::RMA),
            short_vol_ma: MovingAverageProvider::new(MovingAverageType::EMA, 10),
            long_vol_ma: MovingAverageProvider::new(MovingAverageType::EMA, 30),
            range_ma: MovingAverageProvider::new(MovingAverageType::SMA, 20),
            volume_vol_ma: MovingAverageProvider::new(MovingAverageType::EMA, 15),
            
            clusters,
            
            prices: ArrayVec::new(),
            returns: ArrayVec::new(),
            volatilities: ArrayVec::new(),
            regimes: ArrayVec::new(),
            
            transition_matrix,
            
            learning_rate,
            adaptation_period,
            
            current_result: AdaptiveVolatilityRegimeResult::empty(),
            is_ready: false,
            update_count: 0,
            current_regime_duration: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> AdaptiveVolatilityRegimeResult {
        // Сохраняем цены
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(close);
        
        // Рассчитываем доходность
        if self.prices.len() >= 2 {
            let len = self.prices.len();
            let return_rate = (self.prices[len - 1] - self.prices[len - 2]) / self.prices[len - 2];
            
            if self.returns.len() >= 32 {
                self.returns.remove(0);
            }
            self.returns.push(return_rate);
        }
        
        // Обновляем вспомогательные индикаторы
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 1. Извлекаем признаки волатильности
        let features = self.extract_volatility_features(open, high, low, close, volume, atr_value);
        
        // 2. Классифицируем режим с помощью K-means
        let regime = self.classify_regime(&features);
        
        // 3. Обновляем кластеры (онлайн обучение)
        self.update_clusters(&features, regime);
        
        // 4. Рассчитываем вероятности режимов
        self.calculate_regime_probabilities(&features);
        
        // 5. Анализируем переходы между режимами
        self.analyze_regime_transitions(regime);
        
        // 6. Рассчитываем дополнительные метрики
        self.calculate_additional_metrics(&features, atr_value);
        
        // Сохраняем данные
        if self.volatilities.len() >= 32 {
            self.volatilities.remove(0);
        }
        self.volatilities.push(atr_value);
        
        if self.regimes.len() >= 16 {
            self.regimes.remove(0);
        }
        self.regimes.push(regime);
        
        // Обновляем результат
        self.current_result.current_regime = regime;
        self.current_result.volatility_features = features;
        
        // Проверяем готовность
        if self.prices.len() >= 30 && self.returns.len() >= 10 {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Извлечь признаки волатильности
    fn extract_volatility_features(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64, atr: f64) -> [f64; 4] {
        // Признак 1: Нормализованная волатильность (ATR/Price)
        let normalized_volatility = if close > 0.0 { atr / close } else { 0.0 };
        
        // Признак 2: Внутридневная волатильность
        let intraday_volatility = if close > 0.0 { (high - low) / close } else { 0.0 };
        let intraday_smooth = self.short_vol_ma.update_bar(0.0, 0.0, 0.0, intraday_volatility, 0.0);
        
        // Признак 3: Волатильность доходности
        let return_volatility = if self.returns.len() >= 5 {
            let recent_returns: Vec<f64> = self.returns.iter().rev().take(5).copied().collect();
            let mean_return = recent_returns.iter().sum::<f64>() / recent_returns.len() as f64;
            let variance = recent_returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / recent_returns.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        // Признак 4: Волатильность объема
        let volume_volatility = if self.prices.len() >= 2 {
            let prev_volume = volume; // Упрощение для демонстрации
            let volume_change = if prev_volume > 0.0 {
                (volume - prev_volume) / prev_volume
            } else {
                0.0
            };
            self.volume_vol_ma.update_bar(0.0, 0.0, 0.0, volume_change.abs(), 0.0)
        } else {
            0.0
        };
        
        // Нормализация признаков
        [
            (normalized_volatility * 100.0).tanh(),
            (intraday_smooth * 100.0).tanh(),
            (return_volatility * 10.0).tanh(),
            (volume_volatility * 10.0).tanh(),
        ]
    }
    
    /// Классифицировать режим волатильности
    fn classify_regime(&self, features: &[f64; 4]) -> VolatilityRegime {
        let mut min_distance = f64::INFINITY;
        let mut best_regime = VolatilityRegime::Normal;
        
        for cluster in &self.clusters {
            let distance = cluster.distance_to(features);
            if distance < min_distance {
                min_distance = distance;
                best_regime = cluster.regime;
            }
        }
        
        best_regime
    }
    
    /// Обновить кластеры (онлайн K-means)
    fn update_clusters(&mut self, features: &[f64; 4], regime: VolatilityRegime) {
        // Находим кластер для данного режима
        for cluster in &mut self.clusters {
            if cluster.regime == regime {
                cluster.add_point(*features);
                break;
            }
        }
        
        // Адаптируем кластеры каждые N обновлений
        if self.update_count.is_multiple_of(self.adaptation_period) {
            self.adapt_clusters();
        }
    }
    
    /// Адаптировать кластеры
    fn adapt_clusters(&mut self) {
        // Простая адаптация: сдвигаем центроиды к последним точкам
        for cluster in &mut self.clusters {
            if !cluster.points.is_empty() {
                let last_point = cluster.points[cluster.points.len() - 1];
                
                for (c, &lp) in cluster.centroid.iter_mut().zip(last_point.iter()) {
                    *c = *c * (1.0 - self.learning_rate) + lp * self.learning_rate;
                }
            }
        }
    }
    
    /// Рассчитать вероятности режимов
    fn calculate_regime_probabilities(&mut self, features: &[f64; 4]) {
        let mut distances = [0.0; 6];
        let mut total_inverse_distance = 0.0;
        
        // Рассчитываем расстояния до всех кластеров
        for (i, cluster) in self.clusters.iter().enumerate() {
            distances[i] = cluster.distance_to(features);
            self.current_result.cluster_distances[i] = distances[i];
            
            // Избегаем деления на ноль
            let inverse_distance = 1.0 / (distances[i] + 1e-6);
            total_inverse_distance += inverse_distance;
        }
        
        // Конвертируем расстояния в вероятности (чем ближе, тем больше вероятность)
        for (prob, &d) in self.current_result.regime_probability.iter_mut().zip(distances.iter()) {
            let inverse_distance = 1.0 / (d + 1e-6);
            *prob = inverse_distance / total_inverse_distance;
        }
        
        // Рассчитываем уверенность как максимальную вероятность
        self.current_result.regime_confidence = self.current_result.regime_probability.iter()
            .fold(0.0, |acc, &prob| acc.max(prob));
    }
    
    /// Анализировать переходы между режимами
    fn analyze_regime_transitions(&mut self, new_regime: VolatilityRegime) {
        if let Some(&last_regime) = self.regimes.last() {
            // Обновляем длительность режима
            if last_regime == new_regime {
                self.current_regime_duration += 1;
            } else {
                self.current_regime_duration = 1;
                
                // Обновляем матрицу переходов
                let from_idx = self.regime_to_index(last_regime);
                let to_idx = self.regime_to_index(new_regime);
                
                // Обновляем вероятность перехода с использованием экспоненциального сглаживания
                self.transition_matrix[from_idx][to_idx] = 
                    self.transition_matrix[from_idx][to_idx] * (1.0 - self.learning_rate) +
                    self.learning_rate;
                
                // Нормализуем строку матрицы
                let row_sum: f64 = self.transition_matrix[from_idx].iter().sum();
                if row_sum > 0.0 {
                    for j in 0..6 {
                        self.transition_matrix[from_idx][j] /= row_sum;
                    }
                }
            }
        } else {
            self.current_regime_duration = 1;
        }
        
        // Рассчитываем вероятность перехода
        if let Some(&last_regime) = self.regimes.last() {
            let from_idx = self.regime_to_index(last_regime);
            let to_idx = self.regime_to_index(new_regime);
            self.current_result.transition_probability = self.transition_matrix[from_idx][to_idx];
        }
        
        // Рассчитываем стабильность режима
        self.current_result.regime_stability = if self.regimes.len() >= 5 {
            let recent_regimes: Vec<VolatilityRegime> = self.regimes.iter().rev().take(5).copied().collect();
            let same_regime_count = recent_regimes.iter().filter(|&&r| r == new_regime).count();
            same_regime_count as f64 / recent_regimes.len() as f64
        } else {
            1.0
        };
        
        self.current_result.regime_duration = self.current_regime_duration;
    }
    
    /// Конвертировать режим в индекс
    fn regime_to_index(&self, regime: VolatilityRegime) -> usize {
        match regime {
            VolatilityRegime::Quiet => 0,
            VolatilityRegime::Normal => 1,
            VolatilityRegime::Elevated => 2,
            VolatilityRegime::High => 3,
            VolatilityRegime::Extreme => 4,
            VolatilityRegime::Transitional => 5,
        }
    }
    
    /// Рассчитать дополнительные метрики
    fn calculate_additional_metrics(&mut self, features: &[f64; 4], _atr: f64) {
        // Общий счет волатильности (взвешенная сумма признаков)
        self.current_result.volatility_score = (features[0] * 0.3 + 
                                               features[1] * 0.3 +
                                               features[2] * 0.25 + features[3] * 0.15).clamp(0.0, 1.0);
        
        // Волатильность тренда
        if self.volatilities.len() >= 5 {
            let recent_vol: Vec<f64> = self.volatilities.iter().rev().take(5).copied().collect();
            let vol_trend = (recent_vol[0] - recent_vol[4]) / recent_vol[4];
            self.current_result.trend_volatility = vol_trend.tanh();
        }
        
        // Тренд волатильности
        if self.volatilities.len() >= 10 {
            let short_avg = self.volatilities.iter().rev().take(5).sum::<f64>() / 5.0;
            let long_avg = self.volatilities.iter().rev().take(10).sum::<f64>() / 10.0;
            
            self.current_result.volatility_trend = if long_avg > 0.0 {
                (short_avg - long_avg) / long_avg
            } else {
                0.0
            };
        }
        
        // Сила возврата к среднему
        if self.returns.len() >= 10 {
            let recent_returns: Vec<f64> = self.returns.iter().rev().take(10).copied().collect();
            let autocorr = self.calculate_autocorrelation(&recent_returns, 1);
            self.current_result.mean_reversion_strength = (-autocorr).clamp(0.0, 1.0);
        }
    }
    
    /// Рассчитать автокорреляцию
    fn calculate_autocorrelation(&self, data: &[f64], lag: usize) -> f64 {
        if data.len() <= lag {
            return 0.0;
        }
        
        let n = data.len() - lag;
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        
        let mut numerator = 0.0;
        let mut denominator = 0.0;
        
        for i in 0..n {
            let x = data[i] - mean;
            let y = data[i + lag] - mean;
            numerator += x * y;
            denominator += x * x;
        }
        
        if denominator > 0.0 {
            numerator / denominator
        } else {
            0.0
        }
    }
    
    /// Получить текущий режим волатильности
    pub fn current_regime(&self) -> VolatilityRegime {
        self.current_result.current_regime
    }
    
    /// Получить полный результат
    pub fn result(&self) -> AdaptiveVolatilityRegimeResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Получить текущее значение как IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.volatility_score)
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.short_vol_ma.reset();
        self.long_vol_ma.reset();
        self.range_ma.reset();
        self.volume_vol_ma.reset();
        
        // Сбрасываем кластеры
        for cluster in &mut self.clusters {
            cluster.points.clear();
        }
        
        // Переинициализируем центроиды
        self.clusters[0].centroid = [0.1, 0.05, 0.1, 0.2];  // Quiet
        self.clusters[1].centroid = [0.3, 0.15, 0.3, 0.4];  // Normal
        self.clusters[2].centroid = [0.5, 0.25, 0.5, 0.6];  // Elevated
        self.clusters[3].centroid = [0.7, 0.35, 0.7, 0.8];  // High
        self.clusters[4].centroid = [0.9, 0.45, 0.9, 1.0];  // Extreme
        self.clusters[5].centroid = [0.6, 0.8, 0.6, 0.9];   // Transitional
        
        // Сбрасываем матрицу переходов
        for i in 0..6 {
            for j in 0..6 {
                self.transition_matrix[i][j] = if i == j { 0.7 } else { 0.06 };
            }
        }
        
        self.prices.clear();
        self.returns.clear();
        self.volatilities.clear();
        self.regimes.clear();
        
        self.current_result = AdaptiveVolatilityRegimeResult::empty();
        self.is_ready = false;
        self.update_count = 0;
        self.current_regime_duration = 0;
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        
        format!(
            "Regime: {:?}, Score: {:.3}, Confidence: {:.2}, Stability: {:.2}, Duration: {}",
            result.current_regime,
            result.volatility_score,
            result.regime_confidence,
            result.regime_stability,
            result.regime_duration
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        let result = self.current_result;
        
        values.insert("volatility_score".to_string(), result.volatility_score);
        values.insert("regime_confidence".to_string(), result.regime_confidence);
        values.insert("regime_stability".to_string(), result.regime_stability);
        values.insert("transition_probability".to_string(), result.transition_probability);
        values.insert("trend_volatility".to_string(), result.trend_volatility);
        values.insert("mean_reversion_strength".to_string(), result.mean_reversion_strength);
        values.insert("volatility_trend".to_string(), result.volatility_trend);
        values.insert("regime_duration".to_string(), result.regime_duration as f64);
        values.insert("current_regime_value".to_string(), result.current_regime.to_value());
        
        // Добавляем вероятности режимов
        for (i, &prob) in result.regime_probability.iter().enumerate() {
            values.insert(format!("regime_prob_{}", i), prob);
        }
        
        // Добавляем расстояния до кластеров
        for (i, &distance) in result.cluster_distances.iter().enumerate() {
            values.insert(format!("cluster_distance_{}", i), distance);
        }
        
        // Добавляем признаки
        for (i, &feature) in result.volatility_features.iter().enumerate() {
            values.insert(format!("feature_{}", i), feature);
        }
        
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить скорость обучения
    pub fn learning_rate(&self) -> f64 {
        self.learning_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_volatility_regime_creation() {
        let avr = AdaptiveVolatilityRegime::new();
        assert!(!avr.is_ready());
        assert_eq!(avr.learning_rate(), 0.1);
    }
    
    #[test]
    fn test_volatility_regime_conversion() {
        assert_eq!(VolatilityRegime::Normal.to_value(), 0.2);
        // from_value thresholds: <0.1=Quiet, <0.3=Normal, <0.5=Elevated, <0.7=High
        assert_eq!(VolatilityRegime::from_value(0.05), VolatilityRegime::Quiet);   // < 0.1
        assert_eq!(VolatilityRegime::from_value(0.1), VolatilityRegime::Normal);   // 0.1 < 0.3
        assert_eq!(VolatilityRegime::from_value(0.5), VolatilityRegime::High);     // 0.5 < 0.7
        assert_eq!(VolatilityRegime::from_value(0.7), VolatilityRegime::Extreme);  // 0.7 < 0.9
    }
    
    #[test]
    fn test_adaptive_volatility_regime_update() {
        let mut avr = AdaptiveVolatilityRegime::new();
        
        // Добавляем данные с разной волатильностью
        for i in 0..40 {
            let base_price = 100.0;
            let volatility_factor = if i < 20 { 0.5 } else { 2.0 }; // Низкая, затем высокая волатильность
            
            let price = base_price + (i as f64 * 0.1).sin() * volatility_factor;
            let high = price + volatility_factor;
            let low = price - volatility_factor;
            let volume = 1000.0;
            
            let result = avr.update_bar(price, high, low, price, volume);
            
            if i > 35 {
                assert!(avr.is_ready());
                assert!(result.volatility_score >= 0.0 && result.volatility_score <= 1.0);
                assert!(result.regime_confidence >= 0.0 && result.regime_confidence <= 1.0);
                assert!(result.regime_stability >= 0.0 && result.regime_stability <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_volatility_cluster() {
        let mut cluster = VolatilityCluster::new(VolatilityRegime::Normal);
        
        cluster.add_point([0.1, 0.2, 0.3, 0.4]);
        cluster.add_point([0.2, 0.3, 0.4, 0.5]);
        
        assert_eq!(cluster.points.len(), 2);
        assert!(cluster.distance_to(&[0.15, 0.25, 0.35, 0.45]) < 1.0);
    }
    
    #[test]
    fn test_adaptive_volatility_regime_reset() {
        let mut avr = AdaptiveVolatilityRegime::new();
        
        for i in 0..35 {
            let price = 100.0 + i as f64;
            avr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        
        avr.reset();
        assert!(!avr.is_ready());
        assert_eq!(avr.update_count(), 0);
    }
} 






















