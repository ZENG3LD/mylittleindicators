//! Particle Filter
//! Фильтр частиц для нелинейных систем с произвольными распределениями
//! Использует метод Монте-Карло для аппроксимации апостериорного распределения

use arrayvec::ArrayVec;
use std::f64::consts::PI;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Частица
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub state: [f64; 2],    // Состояние [позиция, скорость]
    pub weight: f64,        // Вес частицы
}

impl Particle {
    pub fn new(state: [f64; 2], weight: f64) -> Self {
        Self { state, weight }
    }
}

/// Стратегия ресемплирования
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResamplingStrategy {
    Systematic,        // Систематическое ресемплирование
    Stratified,        // Стратифицированное ресемплирование
    Residual,          // Остаточное ресемплирование
    Multinomial,       // Мультиномиальное ресемплирование
}

/// Результат фильтра частиц
#[derive(Debug, Clone)]
pub struct ParticleFilterResult {
    pub filtered_value: f64,        // Отфильтрованное значение
    pub predicted_value: f64,       // Предсказанное значение
    pub velocity: f64,              // Скорость
    pub uncertainty: f64,           // Неопределенность
    pub effective_sample_size: f64, // Эффективный размер выборки
    pub weight_entropy: f64,        // Энтропия весов
    pub particle_diversity: f64,    // Разнообразие частиц
    pub likelihood: f64,            // Правдоподобие
}

impl Default for ParticleFilterResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleFilterResult {
    pub fn new() -> Self {
        Self {
            filtered_value: 0.0,
            predicted_value: 0.0,
            velocity: 0.0,
            uncertainty: 0.0,
            effective_sample_size: 0.0,
            weight_entropy: 0.0,
            particle_diversity: 0.0,
            likelihood: 0.0,
        }
    }
}

/// Генератор псевдослучайных чисел (упрощенный)
#[derive(Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    
    fn next(&mut self) -> f64 {
        // Линейный конгруэнтный генератор
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        (self.state as f64) / (u64::MAX as f64)
    }
    
    fn normal(&mut self) -> f64 {
        // Метод Box-Muller для генерации нормально распределенных чисел
        static mut CACHE: Option<f64> = None;
        static mut HAS_CACHE: bool = false;
        
        unsafe {
            if HAS_CACHE {
                HAS_CACHE = false;
                return CACHE.unwrap();
            }
            
            let u1 = self.next();
            let u2 = self.next();
            
            let mag = (-2.0 * u1.ln()).sqrt();
            let z0 = mag * (2.0 * PI * u2).cos();
            let z1 = mag * (2.0 * PI * u2).sin();
            
            CACHE = Some(z1);
            HAS_CACHE = true;
            
            z0
        }
    }
}

/// Particle Filter
#[derive(Clone)]
pub struct ParticleFilter {
    // Параметры фильтра
    num_particles: usize,
    dt: f64,
    process_noise_std: f64,
    measurement_noise_std: f64,
    resampling_strategy: ResamplingStrategy,
    resampling_threshold: f64,
    
    // Частицы
    particles: ArrayVec<Particle, 1000>,
    temp_particles: ArrayVec<Particle, 1000>,
    
    // Результаты
    current_result: ParticleFilterResult,
    
    // Генератор случайных чисел
    rng: SimpleRng,
    
    // История для анализа
    ess_history: ArrayVec<f64, 100>,                // Effective Sample Size
    likelihood_history: ArrayVec<f64, 100>,        // Правдоподобие
    diversity_history: ArrayVec<f64, 100>,         // Разнообразие
    
    // Статистики
    resampling_count: usize,
    degeneracy_count: usize,
    
    // Состояние
    is_initialized: bool,
}

impl ParticleFilter {
    pub fn new(
        num_particles: usize,
        dt: f64,
        process_noise_std: f64,
        measurement_noise_std: f64,
        resampling_strategy: ResamplingStrategy,
        seed: Option<u64>
    ) -> Self {
        let num_particles = num_particles.clamp(10, 1000); // Ограничиваем размер
        let seed = seed.unwrap_or(42);
        
        Self {
            num_particles,
            dt,
            process_noise_std,
            measurement_noise_std,
            resampling_strategy,
            resampling_threshold: 0.5, // Ресемплируем если ESS < 50% от числа частиц
            particles: ArrayVec::new(),
            temp_particles: ArrayVec::new(),
            current_result: ParticleFilterResult::new(),
            rng: SimpleRng::new(seed),
            ess_history: ArrayVec::new(),
            likelihood_history: ArrayVec::new(),
            diversity_history: ArrayVec::new(),
            resampling_count: 0,
            degeneracy_count: 0,
            is_initialized: false,
        }
    }
    
    /// Обновление фильтра
    pub fn update(&mut self, observation: f64) -> &ParticleFilterResult {
        if !self.is_initialized {
            self.initialize(observation);
            self.current_result.filtered_value = observation;
            return &self.current_result;
        }
        
        // 1. Шаг предсказания (движение частиц)
        self.predict();
        
        // 2. Шаг коррекции (обновление весов)
        self.correct(observation);
        
        // 3. Вычисление статистик
        self.compute_statistics();
        
        // 4. Ресемплирование при необходимости
        if self.current_result.effective_sample_size < self.resampling_threshold * (self.num_particles as f64) {
            self.resample();
            self.resampling_count += 1;
        }
        
        // 5. Вычисление итоговых результатов
        self.compute_results();
        
        // 6. Сохранение истории
        self.save_history();
        
        &self.current_result
    }
    
    /// Инициализация фильтра
    fn initialize(&mut self, observation: f64) {
        self.particles.clear();
        
        // Инициализируем частицы вокруг наблюдения
        for _ in 0..self.num_particles {
            let position = observation + self.rng.normal() * 10.0; // Начальная неопределенность
            let velocity = self.rng.normal() * 1.0;
            let weight = 1.0 / (self.num_particles as f64);
            
            if !self.particles.is_full() {
                self.particles.push(Particle::new([position, velocity], weight));
            }
        }
        
        self.current_result.filtered_value = observation;
        self.current_result.predicted_value = observation;
        self.current_result.velocity = 0.0;
        self.current_result.uncertainty = 10.0;
        
        self.is_initialized = true;
    }
    
    /// Шаг предсказания
    fn predict(&mut self) {
        for particle in &mut self.particles {
            // Применяем модель движения с шумом
            let new_position = particle.state[0] + self.dt * particle.state[1] + 
                              self.rng.normal() * self.process_noise_std;
            let new_velocity = particle.state[1] * 0.95 + // Небольшое затухание
                              self.rng.normal() * self.process_noise_std * 0.1;
            
            particle.state = [new_position, new_velocity];
        }
    }
    
    /// Шаг коррекции
    fn correct(&mut self, observation: f64) {
        let mut total_weight = 0.0;
        
        // Обновляем веса на основе правдоподобия
        for i in 0..self.particles.len() {
            let predicted_obs = self.observation_function(&self.particles[i].state);
            let likelihood = self.compute_likelihood(observation, predicted_obs);
            
            self.particles[i].weight *= likelihood;
            total_weight += self.particles[i].weight;
        }
        
        // Нормализуем веса
        if total_weight > 1e-12 {
            for particle in &mut self.particles {
                particle.weight /= total_weight;
            }
        } else {
            // Если все веса близки к нулю, сбрасываем их
            let uniform_weight = 1.0 / (self.num_particles as f64);
            for particle in &mut self.particles {
                particle.weight = uniform_weight;
            }
            self.degeneracy_count += 1;
        }
    }
    
    /// Функция наблюдения
    fn observation_function(&self, state: &[f64; 2]) -> f64 {
        // Наблюдаем позицию с небольшой нелинейностью
        let position = state[0];
        position + 0.01 * position.powi(2) // Слабая квадратичная нелинейность
    }
    
    /// Вычисление правдоподобия
    fn compute_likelihood(&self, observation: f64, predicted: f64) -> f64 {
        let diff = observation - predicted;
        let variance = self.measurement_noise_std.powi(2);
        
        // Гауссова функция правдоподобия
        let exponent = -0.5 * diff.powi(2) / variance;
        let normalization = 1.0 / (2.0 * PI * variance).sqrt();
        
        normalization * exponent.exp()
    }
    
    /// Вычисление статистик
    fn compute_statistics(&mut self) {
        // Эффективный размер выборки
        let mut sum_weights_sq = 0.0;
        for particle in &self.particles {
            sum_weights_sq += particle.weight.powi(2);
        }
        self.current_result.effective_sample_size = if sum_weights_sq > 0.0 {
            1.0 / sum_weights_sq
        } else {
            0.0
        };
        
        // Энтропия весов
        let mut entropy = 0.0;
        for particle in &self.particles {
            if particle.weight > 1e-12 {
                entropy -= particle.weight * particle.weight.ln();
            }
        }
        self.current_result.weight_entropy = entropy;
        
        // Разнообразие частиц (стандартное отклонение позиций)
        let mean_position: f64 = self.particles.iter()
            .map(|p| p.weight * p.state[0])
            .sum();
        
        let variance: f64 = self.particles.iter()
            .map(|p| p.weight * (p.state[0] - mean_position).powi(2))
            .sum();
        
        self.current_result.particle_diversity = variance.sqrt();
    }
    
    /// Ресемплирование
    fn resample(&mut self) {
        match self.resampling_strategy {
            ResamplingStrategy::Systematic => self.systematic_resample(),
            ResamplingStrategy::Stratified => self.stratified_resample(),
            ResamplingStrategy::Residual => self.residual_resample(),
            ResamplingStrategy::Multinomial => self.multinomial_resample(),
        }
    }
    
    /// Систематическое ресемплирование
    fn systematic_resample(&mut self) {
        self.temp_particles.clear();
        
        let n = self.particles.len();
        let r = self.rng.next() / (n as f64);
        
        let mut c = self.particles[0].weight;
        let mut i = 0;
        
        for j in 0..n {
            let u = r + (j as f64) / (n as f64);
            while u > c && i < n - 1 {
                i += 1;
                c += self.particles[i].weight;
            }
            
            if !self.temp_particles.is_full() && i < self.particles.len() {
                let mut new_particle = self.particles[i];
                new_particle.weight = 1.0 / (n as f64);
                self.temp_particles.push(new_particle);
            }
        }
        
        std::mem::swap(&mut self.particles, &mut self.temp_particles);
    }
    
    /// Стратифицированное ресемплирование
    fn stratified_resample(&mut self) {
        self.temp_particles.clear();
        
        let n = self.particles.len();
        let mut cumulative_weights = ArrayVec::<f64, 1000>::new();
        let mut sum = 0.0;
        
        // Вычисляем кумулятивные веса
        for particle in &self.particles {
            sum += particle.weight;
            if !cumulative_weights.is_full() {
                cumulative_weights.push(sum);
            }
        }
        
        // Ресемплируем
        for j in 0..n {
            let u = (j as f64 + self.rng.next()) / (n as f64);
            
            // Бинарный поиск
            let mut low = 0;
            let mut high = cumulative_weights.len();
            while low < high {
                let mid = (low + high) / 2;
                if cumulative_weights[mid] < u {
                    low = mid + 1;
                } else {
                    high = mid;
                }
            }
            
            if !self.temp_particles.is_full() && low < self.particles.len() {
                let mut new_particle = self.particles[low];
                new_particle.weight = 1.0 / (n as f64);
                self.temp_particles.push(new_particle);
            }
        }
        
        std::mem::swap(&mut self.particles, &mut self.temp_particles);
    }
    
    /// Остаточное ресемплирование
    fn residual_resample(&mut self) {
        self.temp_particles.clear();
        
        let n = self.particles.len() as f64;
        
        // Детерминированная часть
        for particle in &self.particles {
            let count = (n * particle.weight).floor() as usize;
            for _ in 0..count {
                if !self.temp_particles.is_full() {
                    self.temp_particles.push(*particle);
                }
            }
        }
        
        // Случайная часть для остатков
        let remaining = self.num_particles - self.temp_particles.len();
        if remaining > 0 {
            // Создаем веса для остатков
            let mut residual_weights = ArrayVec::<f64, 1000>::new();
            let mut total_residual = 0.0;
            
            for particle in &self.particles {
                let residual = n * particle.weight - (n * particle.weight).floor();
                if !residual_weights.is_full() {
                    residual_weights.push(residual);
                }
                total_residual += residual;
            }
            
            // Нормализуем остаточные веса
            if total_residual > 1e-12 {
                for weight in &mut residual_weights {
                    *weight /= total_residual;
                }
            }
            
            // Ресемплируем остатки
            for _ in 0..remaining {
                let u = self.rng.next();
                let mut cumulative = 0.0;
                for (i, &weight) in residual_weights.iter().enumerate() {
                    cumulative += weight;
                    if u <= cumulative && i < self.particles.len() {
                        if !self.temp_particles.is_full() {
                            self.temp_particles.push(self.particles[i]);
                        }
                        break;
                    }
                }
            }
        }
        
        // Нормализуем веса
        let uniform_weight = 1.0 / (self.temp_particles.len() as f64);
        for particle in &mut self.temp_particles {
            particle.weight = uniform_weight;
        }
        
        std::mem::swap(&mut self.particles, &mut self.temp_particles);
    }
    
    /// Мультиномиальное ресемплирование
    fn multinomial_resample(&mut self) {
        self.temp_particles.clear();
        
        for _ in 0..self.num_particles {
            let u = self.rng.next();
            let mut cumulative = 0.0;
            
            for particle in &self.particles {
                cumulative += particle.weight;
                if u <= cumulative {
                    if !self.temp_particles.is_full() {
                        let mut new_particle = *particle;
                        new_particle.weight = 1.0 / (self.num_particles as f64);
                        self.temp_particles.push(new_particle);
                    }
                    break;
                }
            }
        }
        
        std::mem::swap(&mut self.particles, &mut self.temp_particles);
    }
    
    /// Вычисление итоговых результатов
    fn compute_results(&mut self) {
        // Взвешенное среднее
        let mut weighted_position = 0.0;
        let mut weighted_velocity = 0.0;
        let mut total_weight = 0.0;
        
        for particle in &self.particles {
            weighted_position += particle.weight * particle.state[0];
            weighted_velocity += particle.weight * particle.state[1];
            total_weight += particle.weight;
        }
        
        if total_weight > 1e-12 {
            self.current_result.filtered_value = weighted_position;
            self.current_result.velocity = weighted_velocity;
        }
        
        // Взвешенная дисперсия для неопределенности
        let mut weighted_variance = 0.0;
        for particle in &self.particles {
            let diff = particle.state[0] - self.current_result.filtered_value;
            weighted_variance += particle.weight * diff.powi(2);
        }
        self.current_result.uncertainty = weighted_variance.sqrt();
        
        // Правдоподобие (среднее по частицам)
        self.current_result.likelihood = self.particles.iter()
            .map(|p| p.weight)
            .sum::<f64>() / (self.particles.len() as f64);
    }
    
    /// Сохранение истории
    fn save_history(&mut self) {
        // ESS
        if self.ess_history.len() >= 100 {
            self.ess_history.remove(0);
        }
        if !self.ess_history.is_full() {
            self.ess_history.push(self.current_result.effective_sample_size);
        }
        
        // Правдоподобие
        if self.likelihood_history.len() >= 100 {
            self.likelihood_history.remove(0);
        }
        if !self.likelihood_history.is_full() {
            self.likelihood_history.push(self.current_result.likelihood);
        }
        
        // Разнообразие
        if self.diversity_history.len() >= 100 {
            self.diversity_history.remove(0);
        }
        if !self.diversity_history.is_full() {
            self.diversity_history.push(self.current_result.particle_diversity);
        }
    }
    
    // Публичные методы
    
    pub fn filtered_value(&self) -> f64 {
        self.current_result.filtered_value
    }
    
    pub fn predicted_value(&self) -> f64 {
        self.current_result.predicted_value
    }
    
    pub fn velocity(&self) -> f64 {
        self.current_result.velocity
    }
    
    pub fn uncertainty(&self) -> f64 {
        self.current_result.uncertainty
    }
    
    pub fn effective_sample_size(&self) -> f64 {
        self.current_result.effective_sample_size
    }
    
    pub fn weight_entropy(&self) -> f64 {
        self.current_result.weight_entropy
    }
    
    pub fn particle_diversity(&self) -> f64 {
        self.current_result.particle_diversity
    }
    
    pub fn likelihood(&self) -> f64 {
        self.current_result.likelihood
    }
    
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
    
    pub fn resampling_count(&self) -> usize {
        self.resampling_count
    }
    
    pub fn degeneracy_count(&self) -> usize {
        self.degeneracy_count
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.filtered_value)
    }

    pub fn ess_history(&self) -> &[f64] {
        &self.ess_history
    }
    
    pub fn likelihood_history(&self) -> &[f64] {
        &self.likelihood_history
    }
    
    pub fn diversity_history(&self) -> &[f64] {
        &self.diversity_history
    }
    
    pub fn set_resampling_threshold(&mut self, threshold: f64) {
        self.resampling_threshold = threshold.clamp(0.1, 1.0);
    }
    
    pub fn reset(&mut self) {
        self.particles.clear();
        self.temp_particles.clear();
        self.current_result = ParticleFilterResult::new();
        self.ess_history.clear();
        self.likelihood_history.clear();
        self.diversity_history.clear();
        self.resampling_count = 0;
        self.degeneracy_count = 0;
        self.is_initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_filter_creation() {
        let pf = ParticleFilter::new(100, 1.0, 0.1, 1.0, ResamplingStrategy::Systematic, Some(42));
        assert!(!pf.is_ready());
        assert_eq!(pf.filtered_value(), 0.0);
    }

    #[test]
    fn test_particle_filter_warmup() {
        let mut pf = ParticleFilter::new(100, 1.0, 0.1, 1.0, ResamplingStrategy::Systematic, Some(42));
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pf.update(price);
        }
        assert!(pf.is_ready());
    }

    #[test]
    fn test_particle_filter_values_finite() {
        let mut pf = ParticleFilter::new(100, 1.0, 0.1, 1.0, ResamplingStrategy::Systematic, Some(42));
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let result = pf.update(price);
            assert!(result.filtered_value.is_finite());
            assert!(result.velocity.is_finite());
        }
    }

    #[test]
    fn test_particle_filter_reset() {
        let mut pf = ParticleFilter::new(100, 1.0, 0.1, 1.0, ResamplingStrategy::Systematic, Some(42));
        for i in 0..10 {
            pf.update(100.0 + i as f64);
        }
        pf.reset();
        assert!(!pf.is_ready());
        assert_eq!(pf.resampling_count(), 0);
    }

    #[test]
    fn test_particle_filter_stratified() {
        let mut pf = ParticleFilter::new(100, 1.0, 0.1, 1.0, ResamplingStrategy::Stratified, Some(42));
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let result = pf.update(price);
            assert!(result.filtered_value.is_finite());
        }
    }
} 






















