// High-performance Efficiency Ratio (Kaufman Efficiency)
// (c) 2024

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct EfficiencyRatioFullHistory {
    pub period: usize,
    values: Vec<f64>,    // динамический вектор всех значений (как в Nautilus)
    deltas: Vec<f64>,    // динамический вектор всех дельт (как в Nautilus)
    value: f64,
    initialized: bool,
}




impl EfficiencyRatioFullHistory {
    /// Returns up to the last n price values (newest first)
    pub fn get_last_prices(&self, n: usize) -> Vec<f64> {
        let len = self.values.len();
        let n = n.min(len);
        self.values[len - n..].iter().rev().cloned().collect()
    }
    /// Returns up to the last n delta values (newest first)
    pub fn get_last_deltas(&self, n: usize) -> Vec<f64> {
        let len = self.deltas.len();
        let n = n.min(len);
        self.deltas[len - n..].iter().rev().cloned().collect()
    }
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            values: Vec::with_capacity(512), // pre-allocate для ускорения
            deltas: Vec::with_capacity(512),
            value: 0.0,
            initialized: false,
        }
    }
    /// Обновить Efficiency Ratio новым баром (используется close)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        self.update_raw(close)
    }
    /// Обновить Efficiency Ratio новым значением (аналог Nautilus)
    pub fn update_raw(&mut self, value: f64) -> f64 {
        self.values.push(value);
        if self.values.len() < 2 {
            self.value = 0.0;
            self.initialized = false;
            return self.value;
        }
        let last_diff = (self.values[self.values.len() - 1] - self.values[self.values.len() - 2]).abs();
        self.deltas.push(last_diff);
        if !self.initialized && self.values.len() >= self.period {
            self.initialized = true;
        }
        let net_diff = (self.values[self.values.len() - 1] - self.values[0]).abs();
        let sum_deltas: f64 = self.deltas.iter().sum();
        self.value = if sum_deltas == 0.0 { 0.0 } else { net_diff / sum_deltas };
        self.value
    }

    /// Возвращает первые n значений буфера (от старого к новому)
    pub fn get_first_n_prices(&self, n: usize) -> Vec<f64> {
        let n = n.min(self.values.len());
        self.values.iter().take(n).cloned().collect()
    }
    /// Возвращает первые n дельт (от старого к новому)
    pub fn get_first_n_deltas(&self, n: usize) -> Vec<f64> {
        let n = n.min(self.deltas.len());
        self.deltas.iter().take(n).cloned().collect()
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    // --- Debug getters ---
    pub fn get_buf(&self) -> &[f64] {
        &self.values
    }
    pub fn get_deltas(&self) -> &[f64] {
        &self.deltas
    }
    // get_buf_start, get_buf_len, get_deltas_start, get_deltas_len удалены как нерелевантные для Vec

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    pub fn reset(&mut self) {
        self.values.clear();
        self.deltas.clear();
        self.value = 0.0;
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_ratio_creation() {
        let ind = EfficiencyRatioFullHistory::new(10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_efficiency_ratio_warmup() {
        let mut ind = EfficiencyRatioFullHistory::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_efficiency_ratio_values() {
        let mut ind = EfficiencyRatioFullHistory::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.value().main() >= 0.0);
    }

    #[test]
    fn test_efficiency_ratio_reset() {
        let mut ind = EfficiencyRatioFullHistory::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}






















