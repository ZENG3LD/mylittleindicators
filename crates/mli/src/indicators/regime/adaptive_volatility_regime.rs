//! Adaptive Volatility Regime - Advanced volatility classification using ML concepts
//!
//! This indicator uses machine learning-inspired techniques to classify volatility regimes:
//! - K-means clustering for regime identification
//! - Hidden Markov Model concepts for state transitions
//! - Support Vector Machine-like classification
//! - Ensemble of volatility measures
//!
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::volatility::atr::Atr;

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
    points: Vec<[f64; 4]>,      // Точки в кластере
    regime: VolatilityRegime,   // Соответствующий режим
}

impl VolatilityCluster {
    fn new(regime: VolatilityRegime) -> Self {
        Self {
            centroid: [0.0; 4],
            points: Vec::with_capacity(32),
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
#[derive(Debug, Clone)]
pub struct AdaptiveVolatilityRegime {
    // Переиспользуем существующие компоненты для расчета волатильности
    atr: Atr,                               // ATR — smoothing controlled by #[slot]
    short_vol_ma: SmootherSlot,             // Краткосрочная волатильность (EMA/10 baked)
    long_vol_ma: SmootherSlot,              // Долгосрочная волатильность (EMA/30 baked)
    range_ma: SmootherSlot,                 // Средний диапазон (SMA/20 baked)
    volume_vol_ma: SmootherSlot,            // Волатильность объема (EMA/15 baked)

    // K-means кластеры для классификации режимов
    clusters: Vec<VolatilityCluster>,

    // Буферы для данных
    prices: Vec<f64>,               // История цен
    returns: Vec<f64>,              // Доходности
    volatilities: Vec<f64>,         // История волатильностей
    regimes: Vec<VolatilityRegime>,  // История режимов

    // Матрица переходов (Hidden Markov Model)
    transition_matrix: [[f64; 6]; 6],

    // Параметры адаптации
    learning_rate: f64,
    adaptation_period: usize,

    // Результат
    current_result: AdaptiveVolatilityRegimeResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
    current_regime_duration: usize,
}

impl AdaptiveVolatilityRegime {
    /// Создать новую Adaptive Volatility Regime с параметрами по умолчанию.
    /// ATR smoothed with EMA (original default).
    pub fn new() -> Self {
        Self::from_smoothers(0.1, 50, SmootherId::Ema)
    }

    /// Создать с настраиваемыми параметрами.
    ///
    /// `atr_smoother` — smoothing applied to the internal ATR(14).
    /// Original default: `SmootherId::Ema`.
    pub fn from_smoothers(learning_rate: f64, adaptation_period: usize, atr_smoother: SmootherId) -> Self {
        assert!(learning_rate > 0.0 && learning_rate <= 1.0,
                "Learning rate must be between 0.0 and 1.0");
        assert!(adaptation_period > 0, "Adaptation period must be positive");

        let mut clusters = Vec::with_capacity(6);
        clusters.push(VolatilityCluster::new(VolatilityRegime::Quiet));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Normal));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Elevated));
        clusters.push(VolatilityCluster::new(VolatilityRegime::High));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Extreme));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Transitional));

        clusters[0].centroid = [0.1, 0.05, 0.1, 0.2];
        clusters[1].centroid = [0.3, 0.15, 0.3, 0.4];
        clusters[2].centroid = [0.5, 0.25, 0.5, 0.6];
        clusters[3].centroid = [0.7, 0.35, 0.7, 0.8];
        clusters[4].centroid = [0.9, 0.45, 0.9, 1.0];
        clusters[5].centroid = [0.6, 0.8, 0.6, 0.9];

        let mut transition_matrix = [[0.0; 6]; 6];
        for (i, row) in transition_matrix.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = if i == j { 0.7 } else { 0.06 };
            }
        }

        Self {
            atr: Atr::from_smoother(14, atr_smoother),
            short_vol_ma: SmootherSlot::new(SmootherId::Ema, 10),
            long_vol_ma: SmootherSlot::new(SmootherId::Ema, 30),
            range_ma: SmootherSlot::new(SmootherId::Sma, 20),
            volume_vol_ma: SmootherSlot::new(SmootherId::Ema, 15),

            clusters,

            prices: Vec::with_capacity(64),
            returns: Vec::with_capacity(32),
            volatilities: Vec::with_capacity(32),
            regimes: Vec::with_capacity(16),

            transition_matrix,

            learning_rate,
            adaptation_period,

            current_result: AdaptiveVolatilityRegimeResult::empty(),
            is_ready: false,
            update_count: 0,
            current_regime_duration: 0,
        }
    }

    /// Feed resolved input lanes — `[open, high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> AdaptiveVolatilityRegimeResult {
        let high   = lanes[1];
        let low    = lanes[2];
        let close  = lanes[3];
        let volume = lanes[4];

        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(close);

        if self.prices.len() >= 2 {
            let len = self.prices.len();
            let return_rate = (self.prices[len - 1] - self.prices[len - 2]) / self.prices[len - 2];

            if self.returns.len() >= 32 {
                self.returns.remove(0);
            }
            self.returns.push(return_rate);
        }

        let atr_value = self.atr.feed(&[high, low, close]);

        let features = self.extract_volatility_features(high, low, close, volume, atr_value);

        let regime = self.classify_regime(&features);

        self.update_clusters(&features, regime);

        self.calculate_regime_probabilities(&features);

        self.analyze_regime_transitions(regime);

        self.calculate_additional_metrics(&features, atr_value);

        if self.volatilities.len() >= 32 {
            self.volatilities.remove(0);
        }
        self.volatilities.push(atr_value);

        if self.regimes.len() >= 16 {
            self.regimes.remove(0);
        }
        self.regimes.push(regime);

        self.current_result.current_regime = regime;
        self.current_result.volatility_features = features;

        if self.prices.len() >= 30 && self.returns.len() >= 10 {
            self.is_ready = true;
        }

        self.update_count += 1;
        self.current_result
    }


    /// Извлечь признаки волатильности
    fn extract_volatility_features(&mut self, high: f64, low: f64, close: f64, volume: f64, atr: f64) -> [f64; 4] {
        let normalized_volatility = if close > 0.0 { atr / close } else { 0.0 };

        let intraday_volatility = if close > 0.0 { (high - low) / close } else { 0.0 };
        let intraday_smooth = self.short_vol_ma.feed(intraday_volatility);

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

        let volume_volatility = if self.prices.len() >= 2 {
            let prev_volume = volume;
            let volume_change = if prev_volume > 0.0 {
                (volume - prev_volume) / prev_volume
            } else {
                0.0
            };
            self.volume_vol_ma.feed(volume_change.abs())
        } else {
            0.0
        };

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
        for cluster in &mut self.clusters {
            if cluster.regime == regime {
                cluster.add_point(*features);
                break;
            }
        }

        if self.update_count.is_multiple_of(self.adaptation_period) {
            self.adapt_clusters();
        }
    }

    /// Адаптировать кластеры
    fn adapt_clusters(&mut self) {
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

        for (i, cluster) in self.clusters.iter().enumerate() {
            distances[i] = cluster.distance_to(features);
            self.current_result.cluster_distances[i] = distances[i];

            let inverse_distance = 1.0 / (distances[i] + 1e-6);
            total_inverse_distance += inverse_distance;
        }

        for (prob, &d) in self.current_result.regime_probability.iter_mut().zip(distances.iter()) {
            let inverse_distance = 1.0 / (d + 1e-6);
            *prob = inverse_distance / total_inverse_distance;
        }

        self.current_result.regime_confidence = self.current_result.regime_probability.iter()
            .fold(0.0, |acc, &prob| acc.max(prob));
    }

    /// Анализировать переходы между режимами
    fn analyze_regime_transitions(&mut self, new_regime: VolatilityRegime) {
        if let Some(&last_regime) = self.regimes.last() {
            if last_regime == new_regime {
                self.current_regime_duration += 1;
            } else {
                self.current_regime_duration = 1;

                let from_idx = self.regime_to_index(last_regime);
                let to_idx = self.regime_to_index(new_regime);

                self.transition_matrix[from_idx][to_idx] =
                    self.transition_matrix[from_idx][to_idx] * (1.0 - self.learning_rate) +
                    self.learning_rate;

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

        if let Some(&last_regime) = self.regimes.last() {
            let from_idx = self.regime_to_index(last_regime);
            let to_idx = self.regime_to_index(new_regime);
            self.current_result.transition_probability = self.transition_matrix[from_idx][to_idx];
        }

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
        self.current_result.volatility_score = (features[0] * 0.3 +
                                               features[1] * 0.3 +
                                               features[2] * 0.25 + features[3] * 0.15).clamp(0.0, 1.0);

        if self.volatilities.len() >= 5 {
            let recent_vol: Vec<f64> = self.volatilities.iter().rev().take(5).copied().collect();
            let vol_trend = (recent_vol[0] - recent_vol[4]) / recent_vol[4];
            self.current_result.trend_volatility = vol_trend.tanh();
        }

        if self.volatilities.len() >= 10 {
            let short_avg = self.volatilities.iter().rev().take(5).sum::<f64>() / 5.0;
            let long_avg = self.volatilities.iter().rev().take(10).sum::<f64>() / 10.0;

            self.current_result.volatility_trend = if long_avg > 0.0 {
                (short_avg - long_avg) / long_avg
            } else {
                0.0
            };
        }

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

    /// Получить текущее значение
    pub fn value(&self) -> f64 {
        self.current_result.volatility_score
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.short_vol_ma.reset();
        self.long_vol_ma.reset();
        self.range_ma.reset();
        self.volume_vol_ma.reset();

        for cluster in &mut self.clusters {
            cluster.points.clear();
        }

        self.clusters[0].centroid = [0.1, 0.05, 0.1, 0.2];
        self.clusters[1].centroid = [0.3, 0.15, 0.3, 0.4];
        self.clusters[2].centroid = [0.5, 0.25, 0.5, 0.6];
        self.clusters[3].centroid = [0.7, 0.35, 0.7, 0.8];
        self.clusters[4].centroid = [0.9, 0.45, 0.9, 1.0];
        self.clusters[5].centroid = [0.6, 0.8, 0.6, 0.9];

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

    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Получить скорость обучения
    pub fn learning_rate(&self) -> f64 {
        self.learning_rate
    }
}

impl Default for AdaptiveVolatilityRegime {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`AdaptiveVolatilityRegime`].
/// Five period fields + five follow slots: ATR(14/Ema), short-vol(10/Ema), long-vol(30/Ema),
/// range(20/Sma), volume-vol(15/Ema). Scalars `learning_rate` and `adaptation_period` are `Param`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AdaptiveVolatilityRegimeConfig {
    pub learning_rate: Param<f64>,
    pub adaptation_period: Param<usize>,
    pub atr_period: Param<usize>,
    pub short_vol_period: Param<usize>,
    pub long_vol_period: Param<usize>,
    pub range_period: Param<usize>,
    pub volume_vol_period: Param<usize>,
    #[slot]
    pub atr_ma: Param<SmootherChoice>,
    #[slot]
    pub short_vol_ma: Param<SmootherChoice>,
    #[slot]
    pub long_vol_ma: Param<SmootherChoice>,
    #[slot]
    pub range_ma: Param<SmootherChoice>,
    #[slot]
    pub volume_vol_ma: Param<SmootherChoice>,
}

impl Indicator for AdaptiveVolatilityRegime {
    const ID: IndicatorId = IndicatorId::Avr;
    /// Volatility regime classifier — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Open/High/Low/Close/Volume — all five needed (volume drives volume_vol_ma feature).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = AdaptiveVolatilityRegimeConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Avr)];
    type Config = AdaptiveVolatilityRegimeConfig;
    type Runtime = AdaptiveVolatilityRegime;

    fn create(cfg: AdaptiveVolatilityRegimeConfig) -> AdaptiveVolatilityRegime {
        let learning_rate = cfg.learning_rate.resolved();
        let adaptation_period = cfg.adaptation_period.resolved();
        let atr_period = cfg.atr_period.resolved();

        let mut clusters = Vec::with_capacity(6);
        clusters.push(VolatilityCluster::new(VolatilityRegime::Quiet));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Normal));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Elevated));
        clusters.push(VolatilityCluster::new(VolatilityRegime::High));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Extreme));
        clusters.push(VolatilityCluster::new(VolatilityRegime::Transitional));
        clusters[0].centroid = [0.1, 0.05, 0.1, 0.2];
        clusters[1].centroid = [0.3, 0.15, 0.3, 0.4];
        clusters[2].centroid = [0.5, 0.25, 0.5, 0.6];
        clusters[3].centroid = [0.7, 0.35, 0.7, 0.8];
        clusters[4].centroid = [0.9, 0.45, 0.9, 1.0];
        clusters[5].centroid = [0.6, 0.8, 0.6, 0.9];

        let mut transition_matrix = [[0.0_f64; 6]; 6];
        for (i, row) in transition_matrix.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = if i == j { 0.7 } else { 0.06 };
            }
        }

        AdaptiveVolatilityRegime {
            atr: Atr::from_smoother(atr_period, cfg.atr_ma.resolved().kind),
            short_vol_ma: cfg.short_vol_ma.resolved().build(cfg.short_vol_period.resolved()),
            long_vol_ma: cfg.long_vol_ma.resolved().build(cfg.long_vol_period.resolved()),
            range_ma: cfg.range_ma.resolved().build(cfg.range_period.resolved()),
            volume_vol_ma: cfg.volume_vol_ma.resolved().build(cfg.volume_vol_period.resolved()),

            clusters,
            prices: Vec::with_capacity(64),
            returns: Vec::with_capacity(32),
            volatilities: Vec::with_capacity(32),
            regimes: Vec::with_capacity(16),
            transition_matrix,
            learning_rate,
            adaptation_period,
            current_result: AdaptiveVolatilityRegimeResult::empty(),
            is_ready: false,
            update_count: 0,
            current_regime_duration: 0,
        }
    }

    fn slot_members(cfg: &AdaptiveVolatilityRegimeConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AdaptiveVolatilityRegimeConfig {
    fn defaults() -> Self {
        AdaptiveVolatilityRegimeConfig {
            learning_rate: Param::Solo(0.1),
            adaptation_period: Param::Solo(50),
            atr_period: Param::Solo(14),
            short_vol_period: Param::Solo(10),
            long_vol_period: Param::Solo(30),
            range_period: Param::Solo(20),
            volume_vol_period: Param::Solo(15),
            atr_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            short_vol_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            long_vol_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            range_ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            volume_vol_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn valid_params(&self) -> Result<(), String> {
        let short = self.short_vol_period.resolved();
        let long = self.long_vol_period.resolved();
        if short >= long {
            return Err(format!("short_vol_period({short}) >= long_vol_period({long})"));
        }
        Ok(())
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // learning_rate: Class E alpha/decay — sweep_f64(0.01, 0.99, 0.01).
        s.learning_rate = Param::many(sweep_f64(0.01, 0.99, 0.01));
        // adaptation_period: Class A usize — auto range(2,4048,1) covers it.
        // short_vol_period/long_vol_period: Class A usize — auto range(2,4048,1) EACH, but
        // both then resolve to the same min (2), failing this config's OWN `valid_params`
        // (short < long) at the min corner (2026-07-03 fix). Split into disjoint ranges so
        // `resolved()` stays ordered; atr_period/range_period/volume_vol_period (independent
        // lanes) keep the full auto range.
        s.short_vol_period = Param::range(1, 100, 1);
        s.long_vol_period = Param::range(101, 10000, 1);
        // atr_ma / short_vol_ma / long_vol_ma / range_ma / volume_vol_ma:
        //   #[slot] SmootherChoice — deferred wave, leave Solo.
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AdaptiveVolatilityRegime {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Avr, "Avg Vol Range", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_adaptive_volatility_regime_creation() {
        let avr = AdaptiveVolatilityRegime::new();
        assert!(!avr.is_ready());
        assert_eq!(avr.learning_rate(), 0.1);
    }

    #[test]
    fn test_with_parameters_rma_type() {
        let mut avr = AdaptiveVolatilityRegime::from_smoothers(0.1, 50, SmootherId::Rma);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 2.0;
            let result = avr.feed(&[price, price + 1.0, price - 1.0, price, 1000.0]);
            assert!(result.volatility_score.is_finite());
        }
        assert!(avr.is_ready());
    }

    #[test]
    fn test_volatility_regime_conversion() {
        assert_eq!(VolatilityRegime::Normal.to_value(), 0.2);
        assert_eq!(VolatilityRegime::from_value(0.05), VolatilityRegime::Quiet);
        assert_eq!(VolatilityRegime::from_value(0.1), VolatilityRegime::Normal);
        assert_eq!(VolatilityRegime::from_value(0.5), VolatilityRegime::High);
        assert_eq!(VolatilityRegime::from_value(0.7), VolatilityRegime::Extreme);
    }

    #[test]
    fn test_adaptive_volatility_regime_update() {
        let mut avr = AdaptiveVolatilityRegime::new();

        for i in 0..40 {
            let base_price = 100.0;
            let volatility_factor = if i < 20 { 0.5 } else { 2.0 };

            let price = base_price + (i as f64 * 0.1).sin() * volatility_factor;
            let high = price + volatility_factor;
            let low = price - volatility_factor;
            let volume = 1000.0;

            let result = avr.feed(&[price, high, low, price, volume]);

            if i > 35 {
                assert!(avr.is_ready());
                assert!(result.volatility_score >= 0.0 && result.volatility_score <= 1.0);
                assert!(result.regime_confidence >= 0.0 && result.regime_confidence <= 1.0);
                assert!(result.regime_stability >= 0.0 && result.regime_stability <= 1.0);
            }
        }
    }

    #[test]
    fn test_adaptive_volatility_regime_reset() {
        let mut avr = AdaptiveVolatilityRegime::new();

        for i in 0..35 {
            let price = 100.0 + i as f64;
            avr.feed(&[price, price + 1.0, price - 1.0, price, 1000.0]);
        }

        avr.reset();
        assert!(!avr.is_ready());
        assert_eq!(avr.update_count(), 0);
    }

    #[test]
    fn factory_feeds_resolved_avr() {
        let mut f = IndicatorOrder::Avr(<<AdaptiveVolatilityRegime as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price - 0.5,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "AVR score should be [0,1], got {v}");
    }
}
