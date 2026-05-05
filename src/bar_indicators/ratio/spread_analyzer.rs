// High-performance Spread Analyzer
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpreadAnalyzer {
    period: usize,
    spreads: ArrayVec<f64, 512>,
    index: usize,
    filled: bool,
    sum: f64,
    current: f64,
    average: f64,
}

impl SpreadAnalyzer {
    pub fn new(period: usize) -> Self {
        assert!(period > 0 && period <= 512, "period must be in 1..=512");
        Self {
            period,
            spreads: ArrayVec::new(),
            index: 0,
            filled: false,
            sum: 0.0,
            current: 0.0,
            average: 0.0,
        }
    }
    /// Обновить SpreadAnalyzer новым тиком (bid, ask)
    pub fn update_bar(&mut self, bid: f64, ask: f64) -> f64 {
        assert!(self.period <= 512, "period must be in 1..=512");
        let spread = ask - bid;
        if self.spreads.len() < self.period {
            self.spreads.push(spread);
            self.sum += spread;
            self.filled = false; // ещё не готов
        } else {
            // Реализация вытеснения: удаляем первый, добавляем новый
            let old = self.spreads.remove(0);
            self.spreads.push(spread);
            self.sum += spread - old;
            self.filled = true; // теперь готов (ровно после capacity+1)
        }
        self.current = spread;
        let len = self.spreads.len();
        self.average = if len == 0 { 0.0 } else { self.sum / len as f64 };
        self.average
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.average)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.spreads.clear();
        self.index = 0;
        self.filled = false;
        self.sum = 0.0;
        self.current = 0.0;
        self.average = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spread_analyzer_creation() {
        let ind = SpreadAnalyzer::new(10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_spread_analyzer_warmup() {
        let mut ind = SpreadAnalyzer::new(10);
        for i in 0..15 {
            let bid = 100.0 + (i as f64 * 0.1);
            let ask = bid + 0.05;
            ind.update_bar(bid, ask);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_spread_analyzer_values() {
        let mut ind = SpreadAnalyzer::new(10);
        for i in 0..20 {
            let bid = 100.0 + i as f64 * 0.1;
            let ask = bid + 0.05;
            ind.update_bar(bid, ask);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.value().main() >= 0.0);
    }

    #[test]
    fn test_spread_analyzer_reset() {
        let mut ind = SpreadAnalyzer::new(10);
        for i in 0..15 {
            let bid = 100.0 + i as f64 * 0.1;
            let ask = bid + 0.05;
            ind.update_bar(bid, ask);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}






















