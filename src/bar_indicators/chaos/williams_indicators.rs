//! Williams Indicators - индикаторы Билла Вильямса для анализа хаоса
//! Включает Alligator, Awesome Oscillator, Acceleration/Deceleration и Market Facilitation Index

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Alligator - индикатор "Аллигатор" Билла Вильямса
/// Состоит из трех сглаженных скользящих средних с разными периодами и смещениями
#[derive(Clone)]
pub struct Alligator {
    // Три линии аллигатора
    jaw: MovingAverageProvider,    // Челюсть (синяя линия) - 13-периодная SMA, смещение 8
    teeth: MovingAverageProvider,  // Зубы (красная линия) - 8-периодная SMA, смещение 5  
    lips: MovingAverageProvider,   // Губы (зеленая линия) - 5-периодная SMA, смещение 3
    
    // Буферы для смещений
    jaw_buffer: ArrayVec<f64, 32>,
    teeth_buffer: ArrayVec<f64, 32>,
    lips_buffer: ArrayVec<f64, 32>,
    
    // Смещения
    jaw_offset: usize,
    teeth_offset: usize,
    lips_offset: usize,
    
    // Состояние
    is_ready: bool,
}

impl Default for Alligator {
    fn default() -> Self {
        Self::new()
    }
}

impl Alligator {
    pub fn new() -> Self {
        Self {
            jaw: MovingAverageProvider::new(MovingAverageType::SMA, 13),
            teeth: MovingAverageProvider::new(MovingAverageType::SMA, 8),
            lips: MovingAverageProvider::new(MovingAverageType::SMA, 5),
            jaw_buffer: ArrayVec::new(),
            teeth_buffer: ArrayVec::new(),
            lips_buffer: ArrayVec::new(),
            jaw_offset: 8,
            teeth_offset: 5,
            lips_offset: 3,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новой ценой (обычно медианная цена = (H+L)/2)
    pub fn update(&mut self, price: f64) -> (f64, f64, f64) {
        // Обновляем скользящие средние (используем update_bar с ценой как close)
        let jaw_value = self.jaw.update_bar(price, price, price, price, 1.0);
        let teeth_value = self.teeth.update_bar(price, price, price, price, 1.0);
        let lips_value = self.lips.update_bar(price, price, price, price, 1.0);
        
        // Добавляем в буферы для смещения
        if self.jaw_buffer.len() >= self.jaw_offset {
            self.jaw_buffer.remove(0);
        }
        self.jaw_buffer.push(jaw_value);
        
        if self.teeth_buffer.len() >= self.teeth_offset {
            self.teeth_buffer.remove(0);
        }
        self.teeth_buffer.push(teeth_value);
        
        if self.lips_buffer.len() >= self.lips_offset {
            self.lips_buffer.remove(0);
        }
        self.lips_buffer.push(lips_value);
        
        // Проверяем готовность
        if self.jaw_buffer.len() >= self.jaw_offset && 
           self.teeth_buffer.len() >= self.teeth_offset &&
           self.lips_buffer.len() >= self.lips_offset {
            self.is_ready = true;
        }
        
        self.get_values()
    }
    
    /// Получить текущие значения линий (с учетом смещения)
    /// Offset означает "сколько баров назад" - берём последнее значение в буфере
    pub fn get_values(&self) -> (f64, f64, f64) {
        let jaw = if self.jaw_buffer.len() >= self.jaw_offset {
            // Берём значение с offset баров назад (последнее в буфере = текущее смещённое)
            self.jaw_buffer[self.jaw_buffer.len() - self.jaw_offset]
        } else {
            self.jaw.value().main()
        };

        let teeth = if self.teeth_buffer.len() >= self.teeth_offset {
            self.teeth_buffer[self.teeth_buffer.len() - self.teeth_offset]
        } else {
            self.teeth.value().main()
        };

        let lips = if self.lips_buffer.len() >= self.lips_offset {
            self.lips_buffer[self.lips_buffer.len() - self.lips_offset]
        } else {
            self.lips.value().main()
        };

        (jaw, teeth, lips)
    }
    
    /// Определить состояние аллигатора
    pub fn alligator_state(&self) -> &'static str {
        let (jaw, teeth, lips) = self.get_values();
        
        if lips > teeth && teeth > jaw {
            "Hunting (Uptrend)"
        } else if lips < teeth && teeth < jaw {
            "Hunting (Downtrend)"
        } else {
            "Sleeping (Sideways)"
        }
    }
    
    /// Получить торговый сигнал
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        let (jaw, teeth, lips) = self.get_values();
        
        if lips > teeth && teeth > jaw && current_price > lips {
            1 // Покупка
        } else if lips < teeth && teeth < jaw && current_price < lips {
            -1 // Продажа
        } else {
            0 // Нейтрально
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Update with OHLCV bar - uses median price (H+L)/2
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        let median_price = (high + low) / 2.0;
        self.update(median_price);
        self.value()
    }

    pub fn value(&self) -> IndicatorValue {
        let (jaw, teeth, lips) = self.get_values();
        IndicatorValue::Triple(jaw, teeth, lips)
    }

    pub fn reset(&mut self) {
        self.jaw.reset();
        self.teeth.reset();
        self.lips.reset();
        self.jaw_buffer.clear();
        self.teeth_buffer.clear();
        self.lips_buffer.clear();
        self.is_ready = false;
    }
}

/// Awesome Oscillator - индикатор AO Билла Вильямса
/// Разность между 5-периодной и 34-периодной простыми скользящими средними медианных цен
#[derive(Clone)]
pub struct AwesomeOscillator {
    sma5: MovingAverageProvider,
    sma34: MovingAverageProvider,
    
    // Буфер значений для анализа
    ao_values: ArrayVec<f64, 512>,
    
    // Результат
    ao_value: f64,
    
    is_ready: bool,
}

impl Default for AwesomeOscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl AwesomeOscillator {
    pub fn new() -> Self {
        Self {
            sma5: MovingAverageProvider::new(MovingAverageType::SMA, 5),
            sma34: MovingAverageProvider::new(MovingAverageType::SMA, 34),
            ao_values: ArrayVec::new(),
            ao_value: 0.0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор медианной ценой (H+L)/2
    pub fn update(&mut self, median_price: f64) -> f64 {
        let sma5_val = self.sma5.update_bar(median_price, median_price, median_price, median_price, 1.0);
        let sma34_val = self.sma34.update_bar(median_price, median_price, median_price, median_price, 1.0);
        
        self.ao_value = sma5_val - sma34_val;
        
        // Добавляем в буфер для анализа
        if self.ao_values.len() >= 512 {
            self.ao_values.remove(0);
        }
        self.ao_values.push(self.ao_value);
        
        if self.sma34.is_ready() {
            self.is_ready = true;
        }
        
        self.ao_value
    }
    
    /// Получить текущее значение AO
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.ao_value)
    }
    
    /// Определить сигнал "блюдце" (saucer)
    pub fn saucer_signal(&self) -> i8 {
        if self.ao_values.len() < 3 {
            return 0;
        }
        
        let len = self.ao_values.len();
        let current = self.ao_values[len - 1];
        let prev1 = self.ao_values[len - 2];
        let prev2 = self.ao_values[len - 3];
        
        // Бычье блюдце: все значения выше нуля, средний бар ниже соседних
        if current > 0.0 && prev1 > 0.0 && prev2 > 0.0 &&
           prev1 < prev2 && current > prev1 {
            return 1;
        }
        
        // Медвежье блюдце: все значения ниже нуля, средний бар выше соседних
        if current < 0.0 && prev1 < 0.0 && prev2 < 0.0 &&
           prev1 > prev2 && current < prev1 {
            return -1;
        }
        
        0
    }
    
    /// Определить сигнал пересечения нулевой линии
    pub fn zero_line_cross(&self) -> i8 {
        if self.ao_values.len() < 2 {
            return 0;
        }
        
        let len = self.ao_values.len();
        let current = self.ao_values[len - 1];
        let prev = self.ao_values[len - 2];
        
        if prev <= 0.0 && current > 0.0 {
            1 // Пересечение снизу вверх
        } else if prev >= 0.0 && current < 0.0 {
            -1 // Пересечение сверху вниз
        } else {
            0
        }
    }
    
    /// Получить общий торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        let saucer = self.saucer_signal();
        let zero_cross = self.zero_line_cross();
        
        if saucer != 0 {
            saucer // Приоритет сигналу блюдца
        } else {
            zero_cross
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn reset(&mut self) {
        self.sma5.reset();
        self.sma34.reset();
        self.ao_values.clear();
        self.ao_value = 0.0;
        self.is_ready = false;
    }
}

/// Acceleration/Deceleration - индикатор AC Билла Вильямса
/// Разность между AO и его 5-периодной простой скользящей средней
#[derive(Clone)]
pub struct AccelerationDeceleration {
    awesome_oscillator: AwesomeOscillator,
    ao_sma: MovingAverageProvider,
    
    ac_value: f64,
    ac_values: ArrayVec<f64, 512>,
    
    is_ready: bool,
}

impl Default for AccelerationDeceleration {
    fn default() -> Self {
        Self::new()
    }
}

impl AccelerationDeceleration {
    pub fn new() -> Self {
        Self {
            awesome_oscillator: AwesomeOscillator::new(),
            ao_sma: MovingAverageProvider::new(MovingAverageType::SMA, 5),
            ac_value: 0.0,
            ac_values: ArrayVec::new(),
            is_ready: false,
        }
    }
    
    /// Обновить индикатор медианной ценой
    pub fn update(&mut self, median_price: f64) -> f64 {
        let ao_val = self.awesome_oscillator.update(median_price);
        let ao_sma_val = self.ao_sma.update_bar(ao_val, ao_val, ao_val, ao_val, 1.0);
        
        self.ac_value = ao_val - ao_sma_val;
        
        if self.ac_values.len() >= 512 {
            self.ac_values.remove(0);
        }
        self.ac_values.push(self.ac_value);
        
        if self.ao_sma.is_ready() {
            self.is_ready = true;
        }
        
        self.ac_value
    }
    
    /// Получить текущее значение AC
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.ac_value)
    }
    
    /// Определить сигнал изменения цвета (смена направления)
    pub fn color_change_signal(&self) -> i8 {
        if self.ac_values.len() < 2 {
            return 0;
        }
        
        let len = self.ac_values.len();
        let current = self.ac_values[len - 1];
        let prev = self.ac_values[len - 2];
        
        if prev < 0.0 && current > 0.0 {
            1 // Смена с красного на зеленый
        } else if prev > 0.0 && current < 0.0 {
            -1 // Смена с зеленого на красный
        } else {
            0
        }
    }
    
    /// Получить торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        self.color_change_signal()
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn reset(&mut self) {
        self.awesome_oscillator.reset();
        self.ao_sma.reset();
        self.ac_value = 0.0;
        self.ac_values.clear();
        self.is_ready = false;
    }
}

/// Market Facilitation Index - индикатор MFI Билла Вильямса
/// Измеряет эффективность движения цены на единицу объема
#[derive(Clone)]
pub struct MarketFacilitationIndex {
    mfi_values: ArrayVec<f64, 512>,
    volume_values: ArrayVec<f64, 512>,
    
    mfi_value: f64,
    
    is_ready: bool,
}

impl Default for MarketFacilitationIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketFacilitationIndex {
    pub fn new() -> Self {
        Self {
            mfi_values: ArrayVec::new(),
            volume_values: ArrayVec::new(),
            mfi_value: 0.0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update(&mut self, high: f64, low: f64, volume: f64) -> f64 {
        // MFI = (High - Low) / Volume
        self.mfi_value = if volume > 0.0 {
            (high - low) / volume
        } else {
            0.0
        };
        
        // Сохраняем для анализа
        if self.mfi_values.len() >= 512 {
            self.mfi_values.remove(0);
        }
        self.mfi_values.push(self.mfi_value);
        
        if self.volume_values.len() >= 512 {
            self.volume_values.remove(0);
        }
        self.volume_values.push(volume);
        
        if self.mfi_values.len() >= 2 {
            self.is_ready = true;
        }
        
        self.mfi_value
    }
    
    /// Получить текущее значение MFI
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.mfi_value)
    }
    
    /// Определить тип бара по Вильямсу
    pub fn bar_type(&self) -> &'static str {
        if self.mfi_values.len() < 2 || self.volume_values.len() < 2 {
            return "Insufficient Data";
        }
        
        let len = self.mfi_values.len();
        let current_mfi = self.mfi_values[len - 1];
        let prev_mfi = self.mfi_values[len - 2];
        let current_volume = self.volume_values[len - 1];
        let prev_volume = self.volume_values[len - 2];
        
        let mfi_up = current_mfi > prev_mfi;
        let volume_up = current_volume > prev_volume;
        
        match (mfi_up, volume_up) {
            (true, true) => "Green (Trend Acceleration)",
            (false, false) => "Fade (Trend Deceleration)",
            (true, false) => "Fake (False Breakout)",
            (false, true) => "Squat (Accumulation/Distribution)",
        }
    }
    
    /// Получить торговый сигнал на основе типа бара
    pub fn trading_signal(&self) -> i8 {
        match self.bar_type() {
            "Green (Trend Acceleration)" => 1,
            "Fade (Trend Deceleration)" => 0,
            "Fake (False Breakout)" => -1,
            "Squat (Accumulation/Distribution)" => 0,
            _ => 0,
        }
    }
    
    /// Получить силу сигнала
    pub fn signal_strength(&self) -> f64 {
        match self.bar_type() {
            "Green (Trend Acceleration)" => 1.0,
            "Squat (Accumulation/Distribution)" => 0.7,
            "Fake (False Breakout)" => 0.5,
            "Fade (Trend Deceleration)" => 0.3,
            _ => 0.0,
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn reset(&mut self) {
        self.mfi_values.clear();
        self.volume_values.clear();
        self.mfi_value = 0.0;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alligator_creation() {
        let ind = Alligator::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn test_alligator_warmup() {
        let mut ind = Alligator::new();
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_alligator_values_finite() {
        let mut ind = Alligator::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (jaw, teeth, lips) = ind.update(price);
            assert!(jaw.is_finite());
            assert!(teeth.is_finite());
            assert!(lips.is_finite());
        }
    }

    #[test]
    fn test_alligator_reset() {
        let mut ind = Alligator::new();
        for i in 0..25 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
    }

    #[test]
    fn test_awesome_oscillator_creation() {
        let ind = AwesomeOscillator::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_awesome_oscillator_warmup() {
        let mut ind = AwesomeOscillator::new();
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_awesome_oscillator_values_finite() {
        let mut ind = AwesomeOscillator::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let ao = ind.update(price);
            assert!(ao.is_finite());
        }
    }

    #[test]
    fn test_awesome_oscillator_reset() {
        let mut ind = AwesomeOscillator::new();
        for i in 0..40 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_acceleration_deceleration_creation() {
        let ind = AccelerationDeceleration::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_acceleration_deceleration_warmup() {
        let mut ind = AccelerationDeceleration::new();
        for i in 0..45 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_acceleration_deceleration_values_finite() {
        let mut ind = AccelerationDeceleration::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let ac = ind.update(price);
            assert!(ac.is_finite());
        }
    }

    #[test]
    fn test_acceleration_deceleration_reset() {
        let mut ind = AccelerationDeceleration::new();
        for i in 0..45 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_market_facilitation_index_creation() {
        let ind = MarketFacilitationIndex::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_market_facilitation_index_warmup() {
        let mut ind = MarketFacilitationIndex::new();
        for i in 0..5 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price + 1.0, price - 1.0, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_market_facilitation_index_values_finite() {
        let mut ind = MarketFacilitationIndex::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let mfi = ind.update(price + 1.0, price - 1.0, 1000.0 + i as f64);
            assert!(mfi.is_finite());
        }
    }

    #[test]
    fn test_market_facilitation_index_reset() {
        let mut ind = MarketFacilitationIndex::new();
        for i in 0..10 {
            ind.update(105.0, 95.0, 1000.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 






















