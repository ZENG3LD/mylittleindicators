// Sweep Reversion Index (SRI)
// Быстрый индикатор детекции свипов ликвидности и возврата внутрь диапазона
// Возвращает непрерывный сигнал в диапазоне [-1.0, 1.0]:
//  - Отрицательный (до -1.0) = бычий свип снизу → лонг-уклон
//  - Положительный (до +1.0) = медвежий свип сверху → шорт-уклон
// При включенном подтверждении (confirm_next_bar) сигнал может задерживаться до следующего бара

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::momentum::highest::Highest;
use crate::bar_indicators::momentum::lowest::Lowest;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy)]
pub struct SweepReversionParams {
    /// Длина окна экстремумов (обычно 20-60)
    pub lookback_period: usize,
    /// Доля диапазона бара для проверки возврата (0.25 означает нижняя/верхняя квартиль)
    pub close_quartile: f64,
    /// Период ATR (для нормализации силы сигнала)
    pub atr_period: usize,
    /// Масштаб для нормализации: чем больше k, тем слабее вес
    pub weight_k: f64,
    /// Требовать подтверждение направлением следующего бара
    pub confirm_next_bar: bool,
    /// Тип MA для ATR сглаживания
    pub atr_ma_type: MovingAverageType,
}

impl Default for SweepReversionParams {
    fn default() -> Self {
        Self {
            lookback_period: 40,
            close_quartile: 0.35,
            atr_period: 14,
            weight_k: 1.0,
            confirm_next_bar: false,
            atr_ma_type: MovingAverageType::RMA,
        }
    }
}

/// Индикатор свипов/возвратов
#[derive(Clone)]
pub struct SweepReversionIndex {
    params: SweepReversionParams,
    highest: Highest,
    lowest: Lowest,
    atr: Atr,

    // Последнее вычисленное значение индикатора
    value: f64,

    // Отложенное подтверждение
    pending_signal: Option<i8>, // -1 = long bias (bullish sweep), +1 = short bias (bearish sweep)
    pending_ref_close: f64,

    is_ready: bool,
}

impl SweepReversionIndex {
    pub fn new(params: SweepReversionParams) -> Self {
        Self {
            highest: Highest::new(params.lookback_period),
            lowest: Lowest::new(params.lookback_period),
            atr: Atr::new(params.atr_period, params.atr_ma_type),
            params,
            value: 0.0,
            pending_signal: None,
            pending_ref_close: 0.0,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highest.reset();
        self.lowest.reset();
        self.atr.reset();
        self.value = 0.0;
        self.pending_signal = None;
        self.pending_ref_close = 0.0;
        self.is_ready = false;
    }

    /// Обновить индикатор новым баром. Возвращает значение [-1.0, 1.0].
    /// Положительное → свип сверху и возврат вниз (short bias)
    /// Отрицательное → свип снизу и возврат вверх (long bias)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        // Получаем предыдущие экстремумы ДО включения текущего бара
        let prev_highest = if self.highest.is_ready() {
            Some(self.highest.value())
        } else {
            None
        };
        let prev_lowest = if self.lowest.is_ready() {
            Some(self.lowest.value())
        } else {
            None
        };

        // Обновляем ATR сначала (он не зависит от экстремумов)
        let _atr_now = self.atr.update_bar(0.0, high, low, close, 0.0);

        // Теперь обновляем экстремумы текущими high/low
        let _ = self.highest.update_bar(0.0, high, 0.0, 0.0, 0.0);
        let _ = self.lowest.update_bar(0.0, 0.0, low, 0.0, 0.0);

        // Готовность после заполнения всех окон
        self.is_ready = self.highest.is_ready() && self.lowest.is_ready() && self.atr.is_ready();
        if !self.is_ready {
            self.value = 0.0;
            return self.value;
        }

        // Нормализованная позиция закрытия внутри бара [0..1]
        let range = (high - low).max(1e-12);
        let pos = (close - low) / range;

        // Базовые флаги свипа относительно предыдущих экстремумов
        let swept_top = prev_highest.map(|h| high > h.main()).unwrap_or(false);
        let swept_bottom = prev_lowest.map(|l| low < l.main()).unwrap_or(false);

        // Возврат внутрь квартиля
        let in_lower_quartile = pos <= self.params.close_quartile;
        let in_upper_quartile = pos >= 1.0 - self.params.close_quartile;

        // Сырые сигналы текущего бара
        let raw_signal: i8 = if swept_top && in_lower_quartile {
            // Медвежий свип сверху → шорт-уклон
            1
        } else if swept_bottom && in_upper_quartile {
            // Бычий свип снизу → лонг-уклон
            -1
        } else {
            0
        };

        // Обработка подтверждения следующего бара (если требуется)
        let confirmed_signal = if self.params.confirm_next_bar {
            match self.pending_signal {
                Some(pend) => {
                    // Проверяем подтверждение направления
                    let is_confirmed = if pend > 0 {
                        // шорт-уклон
                        close < self.pending_ref_close
                    } else {
                        // лонг-уклон
                        close > self.pending_ref_close
                    };
                    // Сбрасываем ожидание вне зависимости от результата
                    self.pending_signal = None;
                    if is_confirmed {
                        pend
                    } else {
                        0
                    }
                }
                None => {
                    // Записываем потенциальный сигнал, но не выдаем его сейчас
                    if raw_signal != 0 {
                        self.pending_signal = Some(raw_signal);
                        self.pending_ref_close = close;
                    }
                    0
                }
            }
        } else {
            raw_signal
        };

        // Вес по ATR: насколько сильно закрытие отклоняется от середины бара относительно ATR
        let atr = self.atr.value().main().abs().max(1e-12);
        let mid = (high + low) * 0.5;
        let dev = (close - mid).abs();
        let mut weight = (dev / (self.params.weight_k.max(1e-12) * atr)).min(1.0);
        if weight.is_nan() || !weight.is_finite() {
            weight = 0.0;
        }

        self.value = (confirmed_signal as f64) * weight;
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Дискретный сигнал на основе последнего значения: sign(value)
    #[inline]
    pub fn discrete_signal(&self) -> i8 {
        if self.value > 0.0 {
            1
        } else if self.value < 0.0 {
            -1
        } else {
            0
        }
    }

    #[inline]
    pub fn params(&self) -> SweepReversionParams {
        self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sweep_reversion_creation() {
        let sri = SweepReversionIndex::new(SweepReversionParams::default());
        assert!(!sri.is_ready());
        assert_eq!(sri.value().main(), 0.0);
        assert_eq!(sri.discrete_signal(), 0);
    }

    #[test]
    fn test_sweep_reversion_basic() {
        let mut sri = SweepReversionIndex::new(SweepReversionParams::default());
        for i in 1..=60 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            sri.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(sri.is_ready());
        assert!(sri.value().main().is_finite());
    }

    #[test]
    fn test_sweep_reversion_range() {
        let mut sri = SweepReversionIndex::new(SweepReversionParams::default());
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 15.0;
            let value = sri.update_bar(price, price + 3.0, price - 3.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "SRI should be in [-1, 1], got {}", value);
        }
    }

    #[test]
    fn test_sweep_reversion_reset() {
        let mut sri = SweepReversionIndex::new(SweepReversionParams::default());
        for i in 1..=60 {
            let price = 100.0 + i as f64;
            sri.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(sri.is_ready());
        sri.reset();
        assert!(!sri.is_ready());
        assert_eq!(sri.value().main(), 0.0);
    }
}
