// Модуль дивергенций: классическая и скрытая (обратная) дивергенция/конвергенция
// (c) 2024

use arrayvec::ArrayVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceType {
    Bullish,
    Bearish,
    HiddenBullish,
    HiddenBearish,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct DivergenceSignal {
    pub bar_idx: usize,
    pub swing_idx: usize,
    pub price: f64,
    pub indicator: f64,
    pub dtype: DivergenceType,
}

#[derive(Debug, Clone)]
pub struct DivergenceDetector {
    pub signals: ArrayVec<DivergenceSignal, 512>,
    pub last_signal: Option<DivergenceSignal>,
}

impl Default for DivergenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DivergenceDetector {
    pub fn new() -> Self {
        Self {
            signals: ArrayVec::new(),
            last_signal: None,
        }
    }

    /// Проверяет и записывает дивергенции между swing (например, ZigZag) и индикатором на текущем баре
    /// swings: список (индекс, цена) экстремумов (например, ZigZag)
    /// indicator: функция, возвращающая значение индикатора на заданном индексе
    pub fn check_and_record<F: Fn(usize) -> f64>(
        &mut self,
        swings: &[(usize, f64)],
        indicator: F,
        bar_idx: usize,
    ) {
        if swings.len() < 2 {
            return;
        }
        let (idx1, price1) = swings[swings.len() - 2];
        let (idx2, price2) = swings[swings.len() - 1];
        let ind1 = indicator(idx1);
        let ind2 = indicator(idx2);
        let dtype = Self::detect(price1, price2, ind1, ind2);
        if dtype != DivergenceType::None {
            let signal = DivergenceSignal {
                bar_idx,
                swing_idx: idx2,
                price: price2,
                indicator: ind2,
                dtype,
            };
            self.last_signal = Some(signal);
            if self.signals.len() == 512 {
                self.signals.remove(0);
            }
            self.signals.push(signal);
        }
    }

    /// Классическая и скрытая дивергенция/конвергенция
    /// Возвращает тип дивергенции
    pub fn detect(price1: f64, price2: f64, ind1: f64, ind2: f64) -> DivergenceType {
        // Классическая бычья: цена ниже, индикатор выше
        if price2 < price1 && ind2 > ind1 {
            DivergenceType::Bullish
        // Классическая медвежья: цена выше, индикатор ниже
        } else if price2 > price1 && ind2 < ind1 {
            DivergenceType::Bearish
        // Скрытая бычья: цена выше, индикатор ниже
        } else if price2 > price1 && ind2 > ind1 {
            DivergenceType::HiddenBullish
        // Скрытая медвежья: цена ниже, индикатор выше
        } else if price2 < price1 && ind2 < ind1 {
            DivergenceType::HiddenBearish
        } else {
            DivergenceType::None
        }
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.signals.len() >= 2
    }

    pub fn reset(&mut self) {
        self.signals.clear();
        self.last_signal = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_detector_creation() {
        let detector = DivergenceDetector::new();
        assert!(!detector.is_ready());
        assert!(detector.last_signal.is_none());
    }

    #[test]
    fn test_divergence_detect_bullish() {
        // Price lower, indicator higher -> Bullish
        let dtype = DivergenceDetector::detect(100.0, 90.0, 30.0, 40.0);
        assert_eq!(dtype, DivergenceType::Bullish);
    }

    #[test]
    fn test_divergence_detect_bearish() {
        // Price higher, indicator lower -> Bearish
        let dtype = DivergenceDetector::detect(100.0, 110.0, 70.0, 60.0);
        assert_eq!(dtype, DivergenceType::Bearish);
    }

    #[test]
    fn test_divergence_detect_hidden_bullish() {
        // Price higher, indicator higher -> HiddenBullish
        let dtype = DivergenceDetector::detect(100.0, 110.0, 30.0, 40.0);
        assert_eq!(dtype, DivergenceType::HiddenBullish);
    }

    #[test]
    fn test_divergence_detect_hidden_bearish() {
        // Price lower, indicator lower -> HiddenBearish
        let dtype = DivergenceDetector::detect(100.0, 90.0, 70.0, 60.0);
        assert_eq!(dtype, DivergenceType::HiddenBearish);
    }

    #[test]
    fn test_divergence_detect_none() {
        // Same price -> None
        let dtype = DivergenceDetector::detect(100.0, 100.0, 50.0, 50.0);
        assert_eq!(dtype, DivergenceType::None);
    }

    #[test]
    fn test_divergence_detector_reset() {
        let mut detector = DivergenceDetector::new();
        detector.reset();
        assert!(!detector.is_ready());
        assert!(detector.last_signal.is_none());
    }
}






















