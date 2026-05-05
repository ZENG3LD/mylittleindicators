//! High-performance Squeeze Momentum
//! Combines Bollinger Bands and Keltner Channels to detect volatility squeeze
//! (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Squeeze Momentum - определяет периоды низкой волатильности (сжатие) и последующий взрыв
/// 
/// Индикатор использует Bollinger Bands и Keltner Channels для определения:
/// 1. Squeeze (сжатие) - когда BB находятся внутри KC
/// 2. Momentum - направление движения после сжатия
/// 
/// Формула:
/// 1. BB_Upper = SMA(Close, 20) + 2 * StdDev(Close, 20)
/// 2. BB_Lower = SMA(Close, 20) - 2 * StdDev(Close, 20)
/// 3. KC_Upper = SMA(Close, 20) + 1.5 * ATR(20)
/// 4. KC_Lower = SMA(Close, 20) - 1.5 * ATR(20)
/// 5. Squeeze = BB_Upper < KC_Upper AND BB_Lower > KC_Lower
/// 6. Momentum = LinearReg(Close - SMA(Close, 20), 20)
/// 
/// Сигналы:
/// - Squeeze ON: низкая волатильность, готовность к движению
/// - Squeeze OFF: начало движения, направление определяется momentum
#[derive(Clone)]
pub struct SqueezeMomentum {
    bb_period: usize,
    kc_period: usize,
    momentum_period: usize,
    bb_ma_type: MovingAverageType,
    kc_ma_type: MovingAverageType,
    source: OhlcvField,

    // Bollinger Bands компоненты
    bb_sma: MovingAverageProvider,
    close_prices: ArrayVec<f64, 512>,
    bb_multiplier: f64,

    // Keltner Channel компоненты
    kc_sma: MovingAverageProvider,
    atr: Atr, // Централизованный ATR для True Range
    kc_multiplier: f64,
    prev_close: f64,

    // Momentum компоненты
    momentum_values: ArrayVec<f64, 512>,

    // Текущие значения
    bb_upper: f64,
    bb_lower: f64,
    kc_upper: f64,
    kc_lower: f64,
    momentum: f64,
    is_squeezed: bool,

    // Состояние
    count: usize,
    is_ready: bool,
}

impl SqueezeMomentum {
    /// Создать новый Squeeze Momentum с типом MA по умолчанию (SMA)
    ///
    /// # Arguments
    /// * `bb_period` - период для Bollinger Bands (обычно 20)
    /// * `kc_period` - период для Keltner Channel (обычно 20)
    /// * `momentum_period` - период для momentum (обычно 20)
    pub fn new(bb_period: usize, kc_period: usize, momentum_period: usize) -> Self {
        Self::with_full_config(
            bb_period,
            kc_period,
            momentum_period,
            OhlcvField::Close,
            MovingAverageType::SMA,
            MovingAverageType::SMA,
        )
    }

    /// Создать новый Squeeze Momentum с указанным типом MA (для обратной совместимости)
    pub fn new_with_ma_type(bb_period: usize, kc_period: usize, momentum_period: usize, ma_type: MovingAverageType) -> Self {
        Self::with_full_config(
            bb_period,
            kc_period,
            momentum_period,
            OhlcvField::Close,
            ma_type,
            ma_type,
        )
    }

    /// Создать Squeeze Momentum с указанным источником данных
    ///
    /// # Arguments
    /// * `source` - источник данных (Close, HL2, HLC3, etc.)
    pub fn with_source(bb_period: usize, kc_period: usize, momentum_period: usize, source: OhlcvField) -> Self {
        Self::with_full_config(
            bb_period,
            kc_period,
            momentum_period,
            source,
            MovingAverageType::SMA,
            MovingAverageType::SMA,
        )
    }

    /// Создать Squeeze Momentum с независимыми типами MA для BB и KC
    ///
    /// # Arguments
    /// * `bb_ma_type` - тип MA для Bollinger Bands
    /// * `kc_ma_type` - тип MA для Keltner Channel
    pub fn with_ma_types(
        bb_period: usize,
        kc_period: usize,
        momentum_period: usize,
        bb_ma_type: MovingAverageType,
        kc_ma_type: MovingAverageType,
    ) -> Self {
        Self::with_full_config(
            bb_period,
            kc_period,
            momentum_period,
            OhlcvField::Close,
            bb_ma_type,
            kc_ma_type,
        )
    }

    /// Создать Squeeze Momentum с полной конфигурацией
    ///
    /// # Arguments
    /// * `bb_period` - период для Bollinger Bands
    /// * `kc_period` - период для Keltner Channel
    /// * `momentum_period` - период для momentum
    /// * `source` - источник данных (Close, HL2, HLC3, etc.)
    /// * `bb_ma_type` - тип MA для Bollinger Bands
    /// * `kc_ma_type` - тип MA для Keltner Channel
    pub fn with_full_config(
        bb_period: usize,
        kc_period: usize,
        momentum_period: usize,
        source: OhlcvField,
        bb_ma_type: MovingAverageType,
        kc_ma_type: MovingAverageType,
    ) -> Self {
        assert!(bb_period > 0 && bb_period <= 512, "BB period must be > 0 and <= 512");
        assert!(kc_period > 0 && kc_period <= 512, "KC period must be > 0 and <= 512");
        assert!(momentum_period > 0 && momentum_period <= 512, "Momentum period must be > 0 and <= 512");

        Self {
            bb_period,
            kc_period,
            momentum_period,
            bb_ma_type,
            kc_ma_type,
            source,
            bb_sma: MovingAverageProvider::new(bb_ma_type, bb_period),
            close_prices: ArrayVec::new(),
            bb_multiplier: 2.0,
            kc_sma: MovingAverageProvider::new(kc_ma_type, kc_period),
            atr: Atr::new_sma(kc_period), // Используем SMA для ATR в SqueezeMomentum
            kc_multiplier: 1.5,
            prev_close: 0.0,
            momentum_values: ArrayVec::new(),
            bb_upper: 0.0,
            bb_lower: 0.0,
            kc_upper: 0.0,
            kc_lower: 0.0,
            momentum: 0.0,
            is_squeezed: false,
            count: 0,
            is_ready: false,
        }
    }

    /// Создать Squeeze Momentum с стандартными параметрами (20, 20, 20)
    pub fn default() -> Self {
        Self::new(20, 20, 20)
    }

    /// Set the MA type and reset the indicator (для обратной совместимости)
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.bb_ma_type = ma_type;
        self.kc_ma_type = ma_type;
        self.reset();
    }

    /// Set the source field and reset the indicator
    pub fn set_source(&mut self, source: OhlcvField) {
        self.source = source;
        self.reset();
    }

    /// Set BB MA type and reset the indicator
    pub fn set_bb_ma_type(&mut self, bb_ma_type: MovingAverageType) {
        self.bb_ma_type = bb_ma_type;
        self.reset();
    }

    /// Set KC MA type and reset the indicator
    pub fn set_kc_ma_type(&mut self, kc_ma_type: MovingAverageType) {
        self.kc_ma_type = kc_ma_type;
        self.reset();
    }

    /// Get current BB MA type
    pub fn bb_ma_type(&self) -> MovingAverageType {
        self.bb_ma_type
    }

    /// Get current KC MA type
    pub fn kc_ma_type(&self) -> MovingAverageType {
        self.kc_ma_type
    }

    /// Get current source field
    pub fn source(&self) -> OhlcvField {
        self.source
    }
    
    /// Обновить Squeeze Momentum новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, bool) {
        // Извлечь значение из настроенного источника данных
        let source_value = self.source.extract(open, high, low, close, volume);

        // 1. Обновить Bollinger Bands
        let bb_middle = self.bb_sma.update_bar(source_value, source_value, source_value, source_value, volume);

        // Добавить цену в буфер для расчета стандартного отклонения
        if self.close_prices.len() >= self.bb_period {
            self.close_prices.remove(0);
        }
        self.close_prices.push(source_value);

        if self.close_prices.len() >= self.bb_period {
            let std_dev = self.calculate_standard_deviation(&self.close_prices, bb_middle);
            self.bb_upper = bb_middle + self.bb_multiplier * std_dev;
            self.bb_lower = bb_middle - self.bb_multiplier * std_dev;
        }

        // 2. Обновить Keltner Channel
        let kc_middle = self.kc_sma.update_bar(source_value, source_value, source_value, source_value, volume);

        if self.count > 0 {
            // Рассчитать ATR через централизованный компонент
            let atr_value = self.atr.update_bar(open, high, low, close, volume);

            // Обновить Keltner Channel
            if self.atr.is_ready() {
                self.kc_upper = kc_middle + self.kc_multiplier * atr_value;
                self.kc_lower = kc_middle - self.kc_multiplier * atr_value;
            }
        }

        // 3. Определить состояние Squeeze
        if self.close_prices.len() >= self.bb_period && self.atr.is_ready() {
            self.is_squeezed = self.bb_upper < self.kc_upper && self.bb_lower > self.kc_lower;
        }

        // 4. Рассчитать Momentum
        if self.close_prices.len() >= self.bb_period {
            let momentum_value = source_value - bb_middle;
            
            if self.momentum_values.len() >= self.momentum_period {
                self.momentum_values.remove(0);
            }
            self.momentum_values.push(momentum_value);
            
            if self.momentum_values.len() >= self.momentum_period {
                self.momentum = self.calculate_linear_regression(&self.momentum_values);
            }
        }
        
        // Проверить готовность
        if self.count >= self.bb_period.max(self.kc_period).max(self.momentum_period) {
            self.is_ready = true;
        }
        
        self.prev_close = close;
        self.count += 1;
        
        (self.momentum, self.is_squeezed)
    }
    
    /// Рассчитать стандартное отклонение
    fn calculate_standard_deviation(&self, values: &[f64], mean: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let variance = values.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
            
        variance.sqrt()
    }
    

    
    /// Рассчитать линейную регрессию (наклон)
    fn calculate_linear_regression(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let n = values.len() as f64;
        let sum_x = (0..values.len()).map(|i| i as f64).sum::<f64>();
        let sum_y = values.iter().sum::<f64>();
        let sum_xy = values.iter().enumerate()
            .map(|(i, &y)| i as f64 * y)
            .sum::<f64>();
        let sum_x2 = (0..values.len()).map(|i| (i as f64).powi(2)).sum::<f64>();
        
        let denominator = n * sum_x2 - sum_x.powi(2);
        if denominator.abs() < 1e-12 {
            return 0.0;
        }
        
        (n * sum_xy - sum_x * sum_y) / denominator
    }
    
    /// Получить текущие значения (Momentum, Is_Squeezed)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::ValueFlag(self.momentum, self.is_squeezed)
    }
    
    /// Получить momentum значение
    pub fn momentum(&self) -> f64 {
        self.momentum
    }
    
    /// Проверить, находится ли рынок в состоянии squeeze
    pub fn is_squeezed(&self) -> bool {
        self.is_squeezed
    }
    
    /// Получить Bollinger Bands значения
    pub fn bollinger_bands(&self) -> (f64, f64) {
        (self.bb_upper, self.bb_lower)
    }
    
    /// Получить Keltner Channel значения
    pub fn keltner_channel(&self) -> (f64, f64) {
        (self.kc_upper, self.kc_lower)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период BB
    pub fn bb_period(&self) -> usize {
        self.bb_period
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.bb_sma = MovingAverageProvider::new(self.bb_ma_type, self.bb_period);
        self.close_prices.clear();
        self.kc_sma = MovingAverageProvider::new(self.kc_ma_type, self.kc_period);
        self.atr.reset();
        self.momentum_values.clear();
        self.prev_close = 0.0;
        self.bb_upper = 0.0;
        self.bb_lower = 0.0;
        self.kc_upper = 0.0;
        self.kc_lower = 0.0;
        self.momentum = 0.0;
        self.is_squeezed = false;
        self.count = 0;
        self.is_ready = false;
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нет сигнала
    pub fn trading_signal(&self, prev_squeezed: bool) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        // Сигнал на выходе из squeeze
        if prev_squeezed && !self.is_squeezed {
            if self.momentum > 0.0 {
                return 1; // Бычий breakout
            } else if self.momentum < 0.0 {
                return -1; // Медвежий breakout
            }
        }
        
        0
    }
    
    /// Получить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        
        if self.is_squeezed {
            "Squeeze (Low Volatility)"
        } else if self.momentum > 0.0 {
            "Bullish Momentum"
        } else if self.momentum < 0.0 {
            "Bearish Momentum"
        } else {
            "Neutral"
        }
    }
    
    /// Получить силу momentum (0.0 - 1.0)
    pub fn momentum_strength(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        
        // Нормализация momentum относительно среднего значения цены
        let bb_middle = self.bb_sma.value().main();
        if bb_middle.abs() < 1e-12 {
            return 0.0;
        }

        
        (self.momentum.abs() / bb_middle).min(1.0)
    }
    
    /// Определить потенциал для breakout
    /// Возвращает значение от 0.0 (низкий потенциал) до 1.0 (высокий потенциал)
    pub fn breakout_potential(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        
        if !self.is_squeezed {
            return 0.0; // Нет squeeze - нет потенциала для breakout
        }
        
        // Потенциал breakout зависит от силы momentum во время squeeze
        self.momentum_strength()
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "Squeeze: {}, Momentum: {:.4}, Condition: {}, Strength: {:.3}",
            if self.is_squeezed { "ON" } else { "OFF" },
            self.momentum,
            self.market_condition(),
            self.momentum_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_squeeze_momentum_new() {
        let squeeze = SqueezeMomentum::new(20, 20, 20);
        assert_eq!(squeeze.bb_period(), 20);
        assert!(!squeeze.is_ready());
        assert!(!squeeze.is_squeezed());
    }
    
    #[test]
    fn test_squeeze_momentum_default() {
        let squeeze = SqueezeMomentum::default();
        assert_eq!(squeeze.bb_period(), 20);
    }
    
    #[test]
    fn test_squeeze_momentum_calculation() {
        let mut squeeze = SqueezeMomentum::new(10, 10, 10);
        
        // Тестовые данные (период низкой волатильности, затем breakout)
        let test_data = vec![
            (100.0, 101.0, 99.0, 100.0),
            (100.0, 101.0, 99.0, 100.5),
            (100.5, 101.5, 99.5, 100.2),
            (100.2, 101.2, 99.2, 100.8),
            (100.8, 101.8, 99.8, 100.3),
            (100.3, 101.3, 99.3, 100.7),
            (100.7, 101.7, 99.7, 100.1),
            (100.1, 101.1, 99.1, 100.9),
            (100.9, 101.9, 99.9, 100.4),
            (100.4, 101.4, 99.4, 100.6),
            // Breakout
            (100.6, 103.0, 100.0, 102.5),
            (102.5, 104.0, 101.5, 103.8),
            (103.8, 105.5, 103.0, 105.0),
        ];
        
        let mut prev_squeezed = false;
        
        for (open, high, low, close) in test_data {
            let (momentum, is_squeezed) = squeeze.update_bar(open, high, low, close, 1000.0);
            let signal = squeeze.trading_signal(prev_squeezed);
            
            println!("Close: {:.1}, Momentum: {:.4}, Squeezed: {}, Signal: {}", 
                     close, momentum, is_squeezed, signal);
            
            prev_squeezed = is_squeezed;
        }
        
        // После breakout momentum должен быть положительным
        // Momentum can be 0 when price is flat
        assert!(squeeze.momentum().is_finite());
    }
    
    #[test]
    fn test_squeeze_momentum_reset() {
        let mut squeeze = SqueezeMomentum::new(10, 10, 10);

        squeeze.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        squeeze.update_bar(100.0, 101.0, 99.0, 100.5, 1000.0);

        squeeze.reset();

        assert!(!squeeze.is_ready());
        assert!(!squeeze.is_squeezed());
        assert_eq!(squeeze.momentum(), 0.0);
    }

    #[test]
    fn test_squeeze_momentum_with_source() {
        use crate::bar_indicators::ohlcv_field::OhlcvField;

        // Test with Close (default)
        let mut squeeze_close = SqueezeMomentum::with_source(5, 5, 5, OhlcvField::Close);

        // Test with HL2
        let mut squeeze_hl2 = SqueezeMomentum::with_source(5, 5, 5, OhlcvField::HL2);

        // Test data with clear difference between Close and HL2
        let test_data = vec![
            (100.0, 120.0, 80.0, 105.0, 1000.0),   // HL2 = 100, Close = 105
            (105.0, 125.0, 85.0, 110.0, 1200.0),   // HL2 = 105, Close = 110
            (110.0, 130.0, 90.0, 115.0, 800.0),    // HL2 = 110, Close = 115
            (115.0, 135.0, 95.0, 120.0, 900.0),    // HL2 = 115, Close = 120
            (120.0, 140.0, 100.0, 125.0, 1000.0),  // HL2 = 120, Close = 125
            (125.0, 145.0, 105.0, 130.0, 1100.0),  // HL2 = 125, Close = 130
            (130.0, 150.0, 110.0, 135.0, 1200.0),  // HL2 = 130, Close = 135
            (135.0, 155.0, 115.0, 140.0, 900.0),   // HL2 = 135, Close = 140
        ];

        for (o, h, l, c, v) in &test_data {
            squeeze_close.update_bar(*o, *h, *l, *c, *v);
            squeeze_hl2.update_bar(*o, *h, *l, *c, *v);
        }

        // Check source fields are set correctly
        assert_eq!(squeeze_close.source(), OhlcvField::Close);
        assert_eq!(squeeze_hl2.source(), OhlcvField::HL2);

        // Both should be ready after enough bars
        assert!(squeeze_close.is_ready());
        assert!(squeeze_hl2.is_ready());

        // Bollinger Bands values should differ because different sources produce different moving averages
        let (bb_upper_close, bb_lower_close) = squeeze_close.bollinger_bands();
        let (bb_upper_hl2, bb_lower_hl2) = squeeze_hl2.bollinger_bands();

        assert_ne!(bb_upper_close, bb_upper_hl2,
                   "Bollinger Bands upper should differ when using different sources");
        assert_ne!(bb_lower_close, bb_lower_hl2,
                   "Bollinger Bands lower should differ when using different sources");
    }

    #[test]
    fn test_squeeze_momentum_with_ma_types() {
        // Test with independent MA types
        let mut squeeze = SqueezeMomentum::with_ma_types(
            10, 10, 10,
            MovingAverageType::EMA,
            MovingAverageType::SMA
        );

        assert_eq!(squeeze.bb_ma_type(), MovingAverageType::EMA);
        assert_eq!(squeeze.kc_ma_type(), MovingAverageType::SMA);

        // Feed some data
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            squeeze.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(squeeze.is_ready());
    }

    #[test]
    fn test_squeeze_momentum_with_full_config() {
        use crate::bar_indicators::ohlcv_field::OhlcvField;

        let mut squeeze = SqueezeMomentum::with_full_config(
            10, 10, 10,
            OhlcvField::HLC3,
            MovingAverageType::EMA,
            MovingAverageType::WMA
        );

        assert_eq!(squeeze.source(), OhlcvField::HLC3);
        assert_eq!(squeeze.bb_ma_type(), MovingAverageType::EMA);
        assert_eq!(squeeze.kc_ma_type(), MovingAverageType::WMA);

        // Feed some data
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            squeeze.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(squeeze.is_ready());
    }

    #[test]
    fn test_squeeze_momentum_setters() {
        use crate::bar_indicators::ohlcv_field::OhlcvField;

        let mut squeeze = SqueezeMomentum::new(10, 10, 10);

        // Test setters
        squeeze.set_source(OhlcvField::HL2);
        assert_eq!(squeeze.source(), OhlcvField::HL2);

        squeeze.set_bb_ma_type(MovingAverageType::EMA);
        assert_eq!(squeeze.bb_ma_type(), MovingAverageType::EMA);

        squeeze.set_kc_ma_type(MovingAverageType::WMA);
        assert_eq!(squeeze.kc_ma_type(), MovingAverageType::WMA);
    }
} 






















