//! High-performance SSL Channel
//! Trend-following indicator using moving averages with dynamic support/resistance
//! (c) 2024

use crate::bar_indicators::average::MovingAverageProvider;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// SSL Channel - использует скользящие средние для определения тренда
/// 
/// SSL (Semaphore Signal Level) Channel создает динамические уровни поддержки и сопротивления
/// на основе скользящих средних от максимумов и минимумов.
/// 
/// Формула:
/// 1. MA_High = MA(High, period)
/// 2. MA_Low = MA(Low, period)  
/// 3. SSL_Up = MA_High если Close > MA_High[1], иначе MA_Low
/// 4. SSL_Down = MA_Low если Close < MA_Low[1], иначе MA_High
/// 
/// Сигналы:
/// - Бычий тренд: SSL_Up > SSL_Down
/// - Медвежий тренд: SSL_Up < SSL_Down
/// - Смена тренда: пересечение SSL_Up и SSL_Down
#[derive(Clone)]
pub struct SslChannel {
    ma_type: MovingAverageType,
    period: usize,

    // Скользящие средние для High и Low
    ma_high: MovingAverageProvider,
    ma_low: MovingAverageProvider,

    // SSL линии
    ssl_up: f64,
    ssl_down: f64,

    // Предыдущие значения для определения тренда
    prev_ssl_up: f64,
    prev_ssl_down: f64,
    prev_close: f64,

    // Состояние тренда
    trend_direction: i8, // 1 = бычий, -1 = медвежий, 0 = неопределен

    // Состояние
    count: usize,
    is_ready: bool,
}

impl SslChannel {
    /// Создать новый SSL Channel
    ///
    /// # Arguments
    /// * `period` - период для скользящих средних (обычно 10-21)
    pub fn new(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::SMA)
    }

    /// Создать SSL Channel с указанным типом скользящей средней
    ///
    /// # Arguments
    /// * `period` - период для скользящих средних (обычно 10-21)
    /// * `ma_type` - тип скользящей средней (SMA, EMA, WMA, и т.д.)
    pub fn new_with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        assert!(period > 0 && period <= 512, "Period must be > 0 and <= 512");

        Self {
            ma_type,
            period,
            ma_high: MovingAverageProvider::new(ma_type, period),
            ma_low: MovingAverageProvider::new(ma_type, period),
            ssl_up: 0.0,
            ssl_down: 0.0,
            prev_ssl_up: 0.0,
            prev_ssl_down: 0.0,
            prev_close: 0.0,
            trend_direction: 0,
            count: 0,
            is_ready: false,
        }
    }

    /// Создать SSL Channel с стандартным периодом (10)
    pub fn default() -> Self {
        Self::new(10)
    }
    
    /// Обновить SSL Channel новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64) {
        // Обновить скользящие средние
        let ma_high_val = self.ma_high.update_bar(high, high, high, high, 0.0);
        let ma_low_val = self.ma_low.update_bar(low, low, low, low, 0.0);
        
        if self.count > 0 {
            // Сохранить предыдущие значения
            self.prev_ssl_up = self.ssl_up;
            self.prev_ssl_down = self.ssl_down;
            
            // Рассчитать SSL линии
            if close > self.ma_high.value().main() {
                self.ssl_up = ma_high_val;
            } else {
                self.ssl_up = ma_low_val;
            }

            if close < self.ma_low.value().main() {
                self.ssl_down = ma_low_val;
            } else {
                self.ssl_down = ma_high_val;
            }
            
            // Определить направление тренда
            if self.ssl_up > self.ssl_down {
                self.trend_direction = 1;  // Бычий тренд
            } else if self.ssl_up < self.ssl_down {
                self.trend_direction = -1; // Медвежий тренд
            }
            
            // Проверить готовность
            if self.count >= self.period {
                self.is_ready = true;
            }
        } else {
            // Первый бар - инициализация
            self.ssl_up = ma_high_val;
            self.ssl_down = ma_low_val;
        }
        
        self.prev_close = close;
        self.count += 1;
        
        (self.ssl_up, self.ssl_down)
    }
    
    /// Получить текущие значения (SSL_Up, SSL_Down)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.ssl_up, self.ssl_down)
    }
    
    /// Получить SSL Up значение
    pub fn ssl_up(&self) -> f64 {
        self.ssl_up
    }
    
    /// Получить SSL Down значение
    pub fn ssl_down(&self) -> f64 {
        self.ssl_down
    }
    
    /// Получить направление тренда
    /// 1 = бычий, -1 = медвежий, 0 = неопределен
    pub fn trend_direction(&self) -> i8 {
        self.trend_direction
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Установить тип скользящей средней
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.ma_high = MovingAverageProvider::new(self.ma_type, self.period);
        self.ma_low = MovingAverageProvider::new(self.ma_type, self.period);
        self.ssl_up = 0.0;
        self.ssl_down = 0.0;
        self.prev_ssl_up = 0.0;
        self.prev_ssl_down = 0.0;
        self.prev_close = 0.0;
        self.trend_direction = 0;
        self.count = 0;
        self.is_ready = false;
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нет сигнала
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        // Сигнал на смену тренда
        if self.trend_direction == 1 && self.prev_ssl_up <= self.prev_ssl_down {
            return 1; // Переход в бычий тренд
        }
        
        if self.trend_direction == -1 && self.prev_ssl_up >= self.prev_ssl_down {
            return -1; // Переход в медвежий тренд
        }
        
        0
    }
    
    /// Проверить пересечение SSL линий
    /// 1 = SSL_Up пересекает SSL_Down вверх, -1 = SSL_Down пересекает SSL_Up вверх, 0 = нет пересечения
    pub fn crossover(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        // Бычье пересечение: SSL_Up пересекает SSL_Down снизу вверх
        if self.prev_ssl_up <= self.prev_ssl_down && self.ssl_up > self.ssl_down {
            return 1;
        }
        
        // Медвежье пересечение: SSL_Down пересекает SSL_Up снизу вверх
        if self.prev_ssl_up >= self.prev_ssl_down && self.ssl_up < self.ssl_down {
            return -1;
        }
        
        0
    }
    
    /// Получить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        
        match self.trend_direction {
            1 => "Bullish Trend",
            -1 => "Bearish Trend",
            _ => "Sideways"
        }
    }
    
    /// Получить силу тренда (0.0 - 1.0)
    pub fn trend_strength(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        
        let ssl_diff = (self.ssl_up - self.ssl_down).abs();
        let ssl_avg = (self.ssl_up + self.ssl_down) / 2.0;
        
        if ssl_avg.abs() < 1e-12 {
            return 0.0;
        }
        
        // Сила тренда как процент разности между SSL линиями
        let strength = ssl_diff / ssl_avg;
        
        // Нормализуем к диапазону [0, 1]
        (strength * 10.0).min(1.0)
    }
    
    /// Определить уровень поддержки/сопротивления
    pub fn support_resistance_level(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        
        match self.trend_direction {
            1 => self.ssl_down,  // В бычьем тренде SSL_Down - поддержка
            -1 => self.ssl_up,   // В медвежьем тренде SSL_Up - сопротивление
            _ => (self.ssl_up + self.ssl_down) / 2.0 // В боковом движении - средний уровень
        }
    }
    
    /// Проверить пробой уровня
    /// 1 = пробой вверх, -1 = пробой вниз, 0 = нет пробоя
    pub fn breakout_signal(&self, close: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let support_resistance = self.support_resistance_level();
        let threshold = support_resistance * 0.001; // 0.1% порог для пробоя
        
        match self.trend_direction {
            1 => {
                // В бычьем тренде проверяем пробой поддержки вниз
                if close < support_resistance - threshold {
                    -1 // Пробой поддержки
                } else {
                    0
                }
            },
            -1 => {
                // В медвежьем тренде проверяем пробой сопротивления вверх
                if close > support_resistance + threshold {
                    1 // Пробой сопротивления
                } else {
                    0
                }
            },
            _ => 0 // В боковом движении нет четких уровней
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "SSL: Up={:.3}, Down={:.3}, Trend: {}, Strength: {:.3}",
            self.ssl_up,
            self.ssl_down,
            self.market_condition(),
            self.trend_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ssl_channel_new() {
        let ssl = SslChannel::new(10);
        assert_eq!(ssl.period(), 10);
        assert!(!ssl.is_ready());
        assert_eq!(ssl.trend_direction(), 0);
    }
    
    #[test]
    fn test_ssl_channel_default() {
        let ssl = SslChannel::default();
        assert_eq!(ssl.period(), 10);
    }
    
    #[test]
    fn test_ssl_channel_calculation() {
        let mut ssl = SslChannel::new(5);

        // Тестовые данные (восходящий тренд)
        let test_data = vec![
            (100.0, 102.0, 98.0, 101.0),
            (101.0, 103.0, 99.0, 102.0),
            (102.0, 104.0, 100.0, 103.0),
            (103.0, 105.0, 101.0, 104.0),
            (104.0, 106.0, 102.0, 105.0),
            (105.0, 107.0, 103.0, 106.0),
            (106.0, 108.0, 104.0, 107.0),
            (107.0, 109.0, 105.0, 108.0),
        ];

        for (open, high, low, close) in test_data {
            ssl.update_bar(open, high, low, close, 1000.0);
        }

        // После прогрева индикатор должен быть готов
        assert!(ssl.is_ready());
        // SSL линии должны иметь значения
        assert!(ssl.ssl_up() > 0.0);
        assert!(ssl.ssl_down() > 0.0);
        // Направление тренда определено (не 0)
        assert_ne!(ssl.trend_direction(), 0);
    }
    
    #[test]
    fn test_ssl_channel_reset() {
        let mut ssl = SslChannel::new(10);
        
        ssl.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0);
        ssl.update_bar(101.0, 103.0, 99.0, 102.0, 1000.0);
        
        ssl.reset();
        
        assert!(!ssl.is_ready());
        assert_eq!(ssl.trend_direction(), 0);
        assert_eq!(ssl.ssl_up(), 0.0);
        assert_eq!(ssl.ssl_down(), 0.0);
    }
    
    #[test]
    fn test_market_conditions() {
        let mut ssl = SslChannel::new(5);
        ssl.is_ready = true;

        ssl.trend_direction = 1;
        assert_eq!(ssl.market_condition(), "Bullish Trend");

        ssl.trend_direction = -1;
        assert_eq!(ssl.market_condition(), "Bearish Trend");

        ssl.trend_direction = 0;
        assert_eq!(ssl.market_condition(), "Sideways");
    }

    #[test]
    fn test_ssl_channel_with_ma_type() {
        let ssl_sma = SslChannel::new_with_ma_type(10, MovingAverageType::SMA);
        assert_eq!(ssl_sma.period(), 10);
        assert!(!ssl_sma.is_ready());

        let ssl_ema = SslChannel::new_with_ma_type(10, MovingAverageType::EMA);
        assert_eq!(ssl_ema.period(), 10);
        assert!(!ssl_ema.is_ready());
    }

    #[test]
    fn test_ssl_channel_set_ma_type() {
        let mut ssl = SslChannel::new(10);

        // Прогрев с SMA
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ssl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ssl.is_ready());

        // Смена типа MA должна сбросить индикатор
        ssl.set_ma_type(MovingAverageType::EMA);
        assert!(!ssl.is_ready());
        assert_eq!(ssl.trend_direction(), 0);
    }

    #[test]
    fn test_ssl_channel_different_ma_types() {
        let mut ssl_sma = SslChannel::new_with_ma_type(5, MovingAverageType::SMA);
        let mut ssl_ema = SslChannel::new_with_ma_type(5, MovingAverageType::EMA);

        let test_data = vec![
            (100.0, 102.0, 98.0, 101.0),
            (101.0, 103.0, 99.0, 102.0),
            (102.0, 104.0, 100.0, 103.0),
            (103.0, 105.0, 101.0, 104.0),
            (104.0, 106.0, 102.0, 105.0),
            (105.0, 107.0, 103.0, 106.0),
        ];

        for (open, high, low, close) in test_data {
            ssl_sma.update_bar(open, high, low, close, 1000.0);
            ssl_ema.update_bar(open, high, low, close, 1000.0);
        }

        // Оба индикатора должны быть готовы
        assert!(ssl_sma.is_ready());
        assert!(ssl_ema.is_ready());

        // Значения должны отличаться из-за разных типов MA
        let (sma_up, sma_down) = (ssl_sma.ssl_up(), ssl_sma.ssl_down());
        let (ema_up, ema_down) = (ssl_ema.ssl_up(), ssl_ema.ssl_down());

        // Проверяем что значения разные (EMA более чувствительна)
        assert_ne!(sma_up, ema_up);
        assert_ne!(sma_down, ema_down);
    }
} 






















