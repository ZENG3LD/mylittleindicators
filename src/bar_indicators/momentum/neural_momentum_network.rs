//! Neural Momentum Network - Advanced momentum analysis using neural network concepts
//! 
//! This indicator applies neural network-inspired techniques to momentum analysis:
//! - Multi-layer perceptron-like structure for feature processing
//! - Activation functions for signal transformation
//! - Backpropagation-inspired adaptive weights
//! - Ensemble of momentum indicators
//! 
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;

/// Тип активационной функции
#[derive(Debug, Clone, Copy)]
pub enum ActivationFunction {
    Sigmoid,    // Сигмоида
    Tanh,       // Гиперболический тангенс
    ReLU,       // Rectified Linear Unit
    Leaky,      // Leaky ReLU
    Swish,      // Swish (x * sigmoid(x))
}

impl ActivationFunction {
    /// Применить активационную функцию
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            ActivationFunction::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            ActivationFunction::Tanh => x.tanh(),
            ActivationFunction::ReLU => x.max(0.0),
            ActivationFunction::Leaky => if x > 0.0 { x } else { 0.01 * x },
            ActivationFunction::Swish => x * (1.0 / (1.0 + (-x).exp())),
        }
    }
    
    /// Производная активационной функции
    pub fn derivative(&self, x: f64) -> f64 {
        match self {
            ActivationFunction::Sigmoid => {
                let s = self.apply(x);
                s * (1.0 - s)
            },
            ActivationFunction::Tanh => 1.0 - x.tanh().powi(2),
            ActivationFunction::ReLU => if x > 0.0 { 1.0 } else { 0.0 },
            ActivationFunction::Leaky => if x > 0.0 { 1.0 } else { 0.01 },
            ActivationFunction::Swish => {
                let sigmoid = 1.0 / (1.0 + (-x).exp());
                sigmoid + x * sigmoid * (1.0 - sigmoid)
            },
        }
    }
}

/// Нейрон в сети
#[derive(Debug, Clone)]
struct Neuron {
    weights: ArrayVec<f64, 8>,      // Веса входов
    bias: f64,                      // Смещение
    activation: ActivationFunction, // Функция активации
    output: f64,                    // Последний выход
    learning_rate: f64,             // Скорость обучения
}

impl Neuron {
    fn new(num_inputs: usize, activation: ActivationFunction, learning_rate: f64) -> Self {
        let mut weights = ArrayVec::new();
        
        // Инициализация весов (Xavier initialization)
        let limit = (6.0 / num_inputs as f64).sqrt();
        for _ in 0..num_inputs {
            weights.push((rand::random::<f64>() - 0.5) * 2.0 * limit);
        }
        
        Self {
            weights,
            bias: 0.0,
            activation,
            output: 0.0,
            learning_rate,
        }
    }
    
    fn forward(&mut self, inputs: &[f64]) -> f64 {
        let mut sum = self.bias;
        
        for (i, &input) in inputs.iter().enumerate() {
            if i < self.weights.len() {
                sum += input * self.weights[i];
            }
        }
        
        self.output = self.activation.apply(sum);
        self.output
    }
    
    fn update_weights(&mut self, inputs: &[f64], error: f64) {
        let gradient = error * self.activation.derivative(self.output);
        
        // Обновляем веса
        for (i, &input) in inputs.iter().enumerate() {
            if i < self.weights.len() {
                self.weights[i] += self.learning_rate * gradient * input;
            }
        }
        
        // Обновляем смещение
        self.bias += self.learning_rate * gradient;
    }
}

/// Результат Neural Momentum Network
#[derive(Debug, Clone, Copy)]
pub struct NeuralMomentumNetworkResult {
    pub momentum_score: f64,         // Общая оценка momentum (-1.0 до 1.0)
    pub layer1_output: [f64; 4],     // Выходы первого слоя
    pub layer2_output: [f64; 2],     // Выходы второго слоя
    pub confidence: f64,             // Уверенность в сигнале (0.0-1.0)
    pub feature_importance: [f64; 6], // Важность входных признаков
    pub network_error: f64,          // Ошибка сети
    pub adaptation_rate: f64,        // Скорость адаптации
    pub momentum_trend: f64,         // Тренд momentum
    pub signal_strength: f64,        // Сила сигнала
    pub neural_signal: i8,           // Сигнал: 1 (покупка), -1 (продажа), 0 (нет)
}

impl NeuralMomentumNetworkResult {
    pub fn empty() -> Self {
        Self {
            momentum_score: 0.0,
            layer1_output: [0.0; 4],
            layer2_output: [0.0; 2],
            confidence: 0.0,
            feature_importance: [0.0; 6],
            network_error: 0.0,
            adaptation_rate: 0.0,
            momentum_trend: 0.0,
            signal_strength: 0.0,
            neural_signal: 0,
        }
    }
}

/// Neural Momentum Network индикатор
#[derive(Clone)]
pub struct NeuralMomentumNetwork {
    // Переиспользуем существующие компоненты для расчета входных признаков
    rsi_ma: MovingAverageProvider,              // RSI-подобный индикатор
    roc_ma: MovingAverageProvider,              // Rate of Change
    momentum_ma: MovingAverageProvider,         // Momentum
    volume_ma: MovingAverageProvider,           // Volume momentum
    atr: Atr,                          // ATR для волатильности
    
    // Нейронная сеть
    layer1: ArrayVec<Neuron, 4>,       // Первый скрытый слой (4 нейрона)
    layer2: ArrayVec<Neuron, 2>,       // Второй скрытый слой (2 нейрона)
    output_neuron: Neuron,             // Выходной нейрон
    
    // Буферы для данных
    prices: ArrayVec<f64, 32>,         // Цены
    volumes: ArrayVec<f64, 16>,        // Объемы
    features: ArrayVec<[f64; 6], 16>,  // Входные признаки
    targets: ArrayVec<f64, 16>,        // Целевые значения для обучения
    
    // Параметры
    learning_rate: f64,                // Скорость обучения
    
    // Результат
    current_result: NeuralMomentumNetworkResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl NeuralMomentumNetwork {
    /// Создать новую Neural Momentum Network с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_learning_rate(0.01)
    }
    
    /// Создать с настраиваемой скоростью обучения
    pub fn with_learning_rate(learning_rate: f64) -> Self {
        assert!(learning_rate > 0.0 && learning_rate <= 1.0, 
                "Learning rate must be between 0.0 and 1.0");
        
        // Создаем нейроны для каждого слоя
        let mut layer1 = ArrayVec::new();
        layer1.push(Neuron::new(6, ActivationFunction::Tanh, learning_rate));
        layer1.push(Neuron::new(6, ActivationFunction::Sigmoid, learning_rate));
        layer1.push(Neuron::new(6, ActivationFunction::ReLU, learning_rate));
        layer1.push(Neuron::new(6, ActivationFunction::Swish, learning_rate));
        
        let mut layer2 = ArrayVec::new();
        layer2.push(Neuron::new(4, ActivationFunction::Tanh, learning_rate));
        layer2.push(Neuron::new(4, ActivationFunction::Sigmoid, learning_rate));
        
        let output_neuron = Neuron::new(2, ActivationFunction::Tanh, learning_rate);
        
        Self {
            // Переиспользуем MovingAverage для разных целей
            rsi_ma: MovingAverageProvider::new(MovingAverageType::EMA, 14),
            roc_ma: MovingAverageProvider::new(MovingAverageType::EMA, 10),
            momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 12),
            volume_ma: MovingAverageProvider::new(MovingAverageType::SMA, 20),
            atr: Atr::new(14, MovingAverageType::RMA),
            
            layer1,
            layer2,
            output_neuron,
            
            prices: ArrayVec::new(),
            volumes: ArrayVec::new(),
            features: ArrayVec::new(),
            targets: ArrayVec::new(),
            
            learning_rate,
            
            current_result: NeuralMomentumNetworkResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> NeuralMomentumNetworkResult {
        // Сохраняем данные
        if self.prices.len() >= 32 {
            self.prices.remove(0);
        }
        self.prices.push(close);
        
        if self.volumes.len() >= 16 {
            self.volumes.remove(0);
        }
        self.volumes.push(volume);
        
        // Обновляем вспомогательные индикаторы
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 1. Рассчитываем входные признаки
        let features = self.calculate_features(open, high, low, close, volume, atr_value);
        
        // 2. Прямое распространение через сеть
        let network_output = self.forward_pass(&features);
        
        // 3. Рассчитываем целевое значение для обучения
        let target = self.calculate_target(close);
        
        // 4. Обратное распространение ошибки
        if !self.targets.is_empty() {
            self.backward_pass(&features, target);
        }
        
        // 5. Анализируем важность признаков
        self.analyze_feature_importance();
        
        // 6. Рассчитываем дополнительные метрики
        self.calculate_confidence_and_trend();
        
        // 7. Генерируем торговые сигналы
        self.generate_neural_signals();
        
        // Сохраняем данные для следующей итерации
        if self.features.len() >= 16 {
            self.features.remove(0);
        }
        self.features.push(features);
        
        if self.targets.len() >= 16 {
            self.targets.remove(0);
        }
        self.targets.push(target);
        
        // Обновляем результат
        self.current_result.momentum_score = network_output;
        
        // Проверяем готовность
        if self.prices.len() >= 20 && self.features.len() >= 5 {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Рассчитать входные признаки
    fn calculate_features(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64, atr: f64) -> [f64; 6] {
        if self.prices.len() < 2 {
            return [0.0; 6];
        }
        
        let len = self.prices.len();
        let prev_close = self.prices[len - 2];
        
        // Признак 1: Нормализованная доходность
        let return_rate = if prev_close != 0.0 {
            (close - prev_close) / prev_close
        } else {
            0.0
        };
        
        // Признак 2: RSI-подобный индикатор
        let rsi_like = self.rsi_ma.update_bar(0.0, 0.0, 0.0, return_rate, 0.0);
        
        // Признак 3: Rate of Change
        let roc = if self.prices.len() >= 10 {
            let old_price = self.prices[len - 10];
            if old_price != 0.0 {
                (close - old_price) / old_price
            } else {
                0.0
            }
        } else {
            0.0
        };
        let roc_smoothed = self.roc_ma.update_bar(0.0, 0.0, 0.0, roc, 0.0);
        
        // Признак 4: Momentum
        let momentum = if self.prices.len() >= 5 {
            close - self.prices[len - 5]
        } else {
            0.0
        };
        let momentum_smoothed = self.momentum_ma.update_bar(0.0, 0.0, 0.0, momentum, 0.0);
        
        // Признак 5: Volume momentum
        let volume_momentum = if self.volumes.len() >= 2 {
            let prev_volume = self.volumes[self.volumes.len() - 2];
            if prev_volume != 0.0 {
                (volume - prev_volume) / prev_volume
            } else {
                0.0
            }
        } else {
            0.0
        };
        let volume_smoothed = self.volume_ma.update_bar(0.0, 0.0, 0.0, volume_momentum, 0.0);
        
        // Признак 6: Нормализованная волатильность
        let volatility = if close != 0.0 { atr / close } else { 0.0 };
        
        // Нормализация признаков в диапазон [-1, 1]
        [
            return_rate.tanh(),           // Нормализованная доходность
            rsi_like.tanh(),              // RSI-like
            roc_smoothed.tanh(),          // ROC
            (momentum_smoothed * 0.01).tanh(), // Momentum
            volume_smoothed.tanh(),       // Volume momentum
            (volatility * 10.0).tanh(),   // Volatility
        ]
    }
    
    /// Прямое распространение через сеть
    fn forward_pass(&mut self, features: &[f64; 6]) -> f64 {
        // Первый слой
        let mut layer1_outputs = [0.0; 4];
        for (i, neuron) in self.layer1.iter_mut().enumerate() {
            layer1_outputs[i] = neuron.forward(features);
        }
        self.current_result.layer1_output = layer1_outputs;
        
        // Второй слой
        let mut layer2_outputs = [0.0; 2];
        for (i, neuron) in self.layer2.iter_mut().enumerate() {
            layer2_outputs[i] = neuron.forward(&layer1_outputs);
        }
        self.current_result.layer2_output = layer2_outputs;
        
        // Выходной слой
        self.output_neuron.forward(&layer2_outputs)
    }
    
    /// Рассчитать целевое значение для обучения
    fn calculate_target(&self, current_price: f64) -> f64 {
        if self.prices.len() < 5 {
            return 0.0;
        }
        
        // Простая стратегия: направление движения цены в следующем периоде
        // (в реальности это будущая информация, но для демонстрации алгоритма)
        let len = self.prices.len();
        let prev_price = self.prices[len - 5];
        
        if prev_price != 0.0 {
            let direction = (current_price - prev_price) / prev_price;
            direction.tanh() // Нормализация
        } else {
            0.0
        }
    }
    
    /// Обратное распространение ошибки
    fn backward_pass(&mut self, features: &[f64; 6], target: f64) {
        let prediction = self.current_result.momentum_score;
        let error = target - prediction;
        self.current_result.network_error = error.abs();
        
        // Обновляем выходной нейрон
        self.output_neuron.update_weights(&self.current_result.layer2_output, error);
        
        // Обратное распространение к слою 2
        let output_gradient = error * self.output_neuron.activation.derivative(prediction);
        for (i, neuron) in self.layer2.iter_mut().enumerate() {
            let layer2_error = output_gradient * self.output_neuron.weights.get(i).copied().unwrap_or(0.0);
            neuron.update_weights(&self.current_result.layer1_output, layer2_error);
        }
        
        // Обратное распространение к слою 1
        for (i, neuron) in self.layer1.iter_mut().enumerate() {
            let mut layer1_error = 0.0;
            for (j, layer2_neuron) in self.layer2.iter().enumerate() {
                let layer2_gradient = output_gradient * layer2_neuron.weights.get(j).copied().unwrap_or(0.0);
                layer1_error += layer2_gradient * layer2_neuron.weights.get(i).copied().unwrap_or(0.0);
            }
            neuron.update_weights(features, layer1_error);
        }
    }
    
    /// Анализировать важность признаков
    fn analyze_feature_importance(&mut self) {
        // Важность признака = среднее абсолютное значение весов, связанных с ним
        for i in 0..6 {
            let mut importance = 0.0;
            let mut count = 0;
            
            for neuron in &self.layer1 {
                if let Some(&weight) = neuron.weights.get(i) {
                    importance += weight.abs();
                    count += 1;
                }
            }
            
            self.current_result.feature_importance[i] = if count > 0 {
                importance / count as f64
            } else {
                0.0
            };
        }
    }
    
    /// Рассчитать уверенность и тренд
    fn calculate_confidence_and_trend(&mut self) {
        // Уверенность основана на стабильности выходов
        if self.targets.len() >= 5 {
            let recent_errors: Vec<f64> = self.targets.iter()
                .rev()
                .take(5)
                .map(|&target| (target - self.current_result.momentum_score).abs())
                .collect();
            
            let avg_error = recent_errors.iter().sum::<f64>() / recent_errors.len() as f64;
            self.current_result.confidence = (1.0 - avg_error).clamp(0.0, 1.0);
        }
        
        // Тренд momentum
        if self.features.len() >= 3 {
            let len = self.features.len();
            let recent_momentum = self.current_result.momentum_score;
            let prev_momentum = if len >= 2 {
                // Примерная оценка предыдущего momentum
                (self.features[len - 2][0] + self.features[len - 2][2]) / 2.0
            } else {
                0.0
            };
            
            self.current_result.momentum_trend = (recent_momentum - prev_momentum).tanh();
        }
        
        // Сила сигнала
        self.current_result.signal_strength = (self.current_result.momentum_score.abs() * 
                                              self.current_result.confidence).min(1.0);
        
        // Скорость адаптации
        self.current_result.adaptation_rate = self.learning_rate * (1.0 + self.current_result.network_error);
    }
    
    /// Генерировать торговые сигналы
    fn generate_neural_signals(&mut self) {
        if !self.is_ready {
            self.current_result.neural_signal = 0;
            return;
        }
        
        let momentum = self.current_result.momentum_score;
        let confidence = self.current_result.confidence;
        let signal_strength = self.current_result.signal_strength;
        
        // Пороги для сигналов
        let confidence_threshold = 0.6;
        let strength_threshold = 0.5;
        let momentum_threshold = 0.3;
        
        if confidence > confidence_threshold && signal_strength > strength_threshold {
            if momentum > momentum_threshold {
                self.current_result.neural_signal = 1; // Покупка
            } else if momentum < -momentum_threshold {
                self.current_result.neural_signal = -1; // Продажа
            } else {
                self.current_result.neural_signal = 0; // Нейтрально
            }
        } else {
            self.current_result.neural_signal = 0;
        }
    }
    
    /// Получить текущую оценку momentum
    pub fn momentum_score(&self) -> f64 {
        self.current_result.momentum_score
    }
    
    /// Получить полный результат
    pub fn result(&self) -> NeuralMomentumNetworkResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Получить текущее значение как IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.momentum_score)
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.rsi_ma.reset();
        self.roc_ma.reset();
        self.momentum_ma.reset();
        self.volume_ma.reset();
        self.atr.reset();
        
        // Переинициализируем нейроны
        self.layer1.clear();
        self.layer1.push(Neuron::new(6, ActivationFunction::Tanh, self.learning_rate));
        self.layer1.push(Neuron::new(6, ActivationFunction::Sigmoid, self.learning_rate));
        self.layer1.push(Neuron::new(6, ActivationFunction::ReLU, self.learning_rate));
        self.layer1.push(Neuron::new(6, ActivationFunction::Swish, self.learning_rate));
        
        self.layer2.clear();
        self.layer2.push(Neuron::new(4, ActivationFunction::Tanh, self.learning_rate));
        self.layer2.push(Neuron::new(4, ActivationFunction::Sigmoid, self.learning_rate));
        
        self.output_neuron = Neuron::new(2, ActivationFunction::Tanh, self.learning_rate);
        
        self.prices.clear();
        self.volumes.clear();
        self.features.clear();
        self.targets.clear();
        
        self.current_result = NeuralMomentumNetworkResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        self.current_result.neural_signal
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        
        format!(
            "Neural Momentum: {:.3}, Confidence: {:.2}, Strength: {:.2}, Error: {:.4}, Trend: {:.2}",
            result.momentum_score,
            result.confidence,
            result.signal_strength,
            result.network_error,
            result.momentum_trend
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        let result = self.current_result;
        
        values.insert("momentum_score".to_string(), result.momentum_score);
        values.insert("confidence".to_string(), result.confidence);
        values.insert("network_error".to_string(), result.network_error);
        values.insert("adaptation_rate".to_string(), result.adaptation_rate);
        values.insert("momentum_trend".to_string(), result.momentum_trend);
        values.insert("signal_strength".to_string(), result.signal_strength);
        
        // Добавляем выходы слоев
        for (i, &output) in result.layer1_output.iter().enumerate() {
            values.insert(format!("layer1_neuron_{}", i), output);
        }
        for (i, &output) in result.layer2_output.iter().enumerate() {
            values.insert(format!("layer2_neuron_{}", i), output);
        }
        
        // Добавляем важность признаков
        for (i, &importance) in result.feature_importance.iter().enumerate() {
            values.insert(format!("feature_importance_{}", i), importance);
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

// Простая реализация генератора случайных чисел для весов
mod rand {
    use std::cell::Cell;
    
    thread_local! {
        static SEED: Cell<u64> = const { Cell::new(1) };
    }
    
    pub fn random<T>() -> T 
    where 
        T: From<f64>
    {
        SEED.with(|seed| {
            let current = seed.get();
            let next = current.wrapping_mul(1103515245).wrapping_add(12345);
            seed.set(next);
            T::from((next as f64) / (u64::MAX as f64))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neural_momentum_network_creation() {
        let nmn = NeuralMomentumNetwork::new();
        assert!(!nmn.is_ready());
        assert_eq!(nmn.learning_rate(), 0.01);
    }
    
    #[test]
    fn test_neural_momentum_network_with_learning_rate() {
        let nmn = NeuralMomentumNetwork::with_learning_rate(0.05);
        assert_eq!(nmn.learning_rate(), 0.05);
    }
    
    #[test]
    fn test_activation_functions() {
        assert!(ActivationFunction::Sigmoid.apply(0.0) > 0.4 && ActivationFunction::Sigmoid.apply(0.0) < 0.6);
        assert_eq!(ActivationFunction::Tanh.apply(0.0), 0.0);
        assert_eq!(ActivationFunction::ReLU.apply(-1.0), 0.0);
        assert_eq!(ActivationFunction::ReLU.apply(1.0), 1.0);
        assert_eq!(ActivationFunction::Leaky.apply(-1.0), -0.01);
    }
    
    #[test]
    fn test_neural_momentum_network_update() {
        let mut nmn = NeuralMomentumNetwork::new();
        
        // Добавляем трендовые данные для обучения
        for i in 0..30 {
            let price = 100.0 + i as f64 * 0.5;
            let volume = 1000.0 + (i as f64 * 10.0).sin() * 100.0;
            let result = nmn.update_bar(price, price + 1.0, price - 1.0, price, volume);
            
            if i > 25 {
                assert!(nmn.is_ready());
                assert!(result.momentum_score.is_finite());
                assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
                assert!(result.signal_strength >= 0.0 && result.signal_strength <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_neural_momentum_network_signals() {
        let mut nmn = NeuralMomentumNetwork::new();
        
        // Генерируем данные с четким трендом
        for i in 0..40 {
            let price = 100.0 + i as f64 * 0.1;
            nmn.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
        }
        
        assert!(nmn.is_ready());
        let signal = nmn.trading_signal();
        assert!(signal >= -1 && signal <= 1);
    }
    
    #[test]
    fn test_neural_momentum_network_reset() {
        let mut nmn = NeuralMomentumNetwork::new();
        
        for i in 0..25 {
            let price = 100.0 + i as f64;
            nmn.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        
        nmn.reset();
        assert!(!nmn.is_ready());
        assert_eq!(nmn.update_count(), 0);
    }
} 






















