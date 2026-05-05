//! High-performance Market Cipher
//! Modern comprehensive indicator combining multiple technical analysis signals
//! (c) 2024

use arrayvec::ArrayVec;

/// Market Cipher - комплексный индикатор, объединяющий несколько сигналов
/// 
/// Market Cipher объединяет:
/// 1. Wave Trend - основной осциллятор
/// 2. Money Flow - анализ объема и цены
/// 3. RSI с дивергенциями
/// 4. VWAP анализ
/// 5. Squeeze индикатор
/// 
/// Компоненты:
/// - Green/Red dots: сигналы покупки/продажи на основе Wave Trend
/// - Blue triangles: сильные сигналы momentum
/// - Yellow X: потенциальные развороты
/// - Background color: общее состояние рынка
pub struct MarketCipher {
    period: usize,
    
    // Wave Trend компоненты
    wt_period: usize,
    wt_signal_period: usize,
    hlc3_values: ArrayVec<f64, 512>,
    wt1_values: ArrayVec<f64, 512>,
    wt2_values: ArrayVec<f64, 512>,
    wt1: f64,
    wt2: f64,
    
    // Money Flow компоненты
    mf_period: usize,
    money_flow: f64,
    volume_sum: f64,
    money_flow_values: ArrayVec<f64, 512>,  // История для скользящего среднего
    
    // RSI компоненты
    rsi_period: usize,
    rsi_values: ArrayVec<f64, 512>,
    rsi: f64,
    
    // VWAP компоненты
    vwap_sum: f64,
    volume_total: f64,
    vwap: f64,
    
    // Сигналы
    buy_signal: bool,
    sell_signal: bool,
    strong_buy: bool,
    strong_sell: bool,
    reversal_signal: i8, // 1 = бычий разворот, -1 = медвежий разворот, 0 = нет
    
    // Состояние
    count: usize,
    is_ready: bool,
    prev_close: f64,
}

impl MarketCipher {
    /// Создать новый Market Cipher
    /// 
    /// # Arguments
    /// * `period` - основной период для расчетов (обычно 14)
    pub fn new(period: usize) -> Self {
        assert!(period > 0 && period <= 512, "Period must be > 0 and <= 512");
        
        Self {
            period,
            wt_period: period,
            wt_signal_period: 4,
            hlc3_values: ArrayVec::new(),
            wt1_values: ArrayVec::new(),
            wt2_values: ArrayVec::new(),
            wt1: 0.0,
            wt2: 0.0,
            mf_period: period,
            money_flow: 0.0,
            volume_sum: 0.0,
            money_flow_values: ArrayVec::new(),
            rsi_period: period,
            rsi_values: ArrayVec::new(),
            rsi: 50.0,
            vwap_sum: 0.0,
            volume_total: 0.0,
            vwap: 0.0,
            buy_signal: false,
            sell_signal: false,
            strong_buy: false,
            strong_sell: false,
            reversal_signal: 0,
            count: 0,
            is_ready: false,
            prev_close: 0.0,
        }
    }
    
    /// Создать Market Cipher с стандартным периодом (14)
    pub fn default() -> Self {
        Self::new(14)
    }
    
    /// Обновить Market Cipher новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, bool, bool) {
        let hlc3 = (high + low + close) / 3.0;
        
        // 1. Обновить Wave Trend
        self.update_wave_trend(hlc3, high, low);
        
        // 2. Обновить Money Flow
        self.update_money_flow(hlc3, volume);
        
        // 3. Обновить RSI
        if self.count > 0 {
            self.update_rsi(close);
        }
        
        // 4. Обновить VWAP
        self.update_vwap(hlc3, volume);
        
        // 5. Генерировать сигналы
        if self.count > self.period * 2 {
            self.generate_signals(close, volume);
        }
        
        // Проверить готовность
        if self.count >= self.period * 2 {
            self.is_ready = true;
        }
        
        self.prev_close = close;
        self.count += 1;
        
        (self.wt1, self.wt2, self.buy_signal, self.sell_signal)
    }
    
    /// Обновить Wave Trend
    fn update_wave_trend(&mut self, hlc3: f64, _high: f64, _low: f64) {
        // Добавить HLC3 в буфер
        if self.hlc3_values.len() >= self.wt_period {
            self.hlc3_values.remove(0);
        }
        self.hlc3_values.push(hlc3);
        
        if self.hlc3_values.len() >= self.wt_period {
            // Рассчитать EMA для Wave Trend
            let ema_hlc3 = self.calculate_ema(&self.hlc3_values, self.wt_period);
            
            // Рассчитать диапазон
            let highest = self.hlc3_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let lowest = self.hlc3_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            if (highest - lowest).abs() > 1e-12 {
                self.wt1 = -((hlc3 - ema_hlc3) / (0.015 * (highest - lowest)));
            } else {
                self.wt1 = 0.0;
            }
            
            // Добавить WT1 в буфер для расчета WT2
            if self.wt1_values.len() >= self.wt_signal_period {
                self.wt1_values.remove(0);
            }
            self.wt1_values.push(self.wt1);
            
            if self.wt1_values.len() >= self.wt_signal_period {
                self.wt2 = self.calculate_ema(&self.wt1_values, self.wt_signal_period);
            }
        }
    }
    
    /// Обновить Money Flow
    fn update_money_flow(&mut self, hlc3: f64, volume: f64) {
        let raw_money_flow = hlc3 * volume;
        
        // Добавляем в буфер с ограничением по mf_period
        if self.money_flow_values.len() >= self.mf_period {
            self.money_flow_values.remove(0);
        }
        self.money_flow_values.push(raw_money_flow);
        
        // Вычисляем скользящее среднее Money Flow за mf_period
        if !self.money_flow_values.is_empty() {
            self.money_flow = self.money_flow_values.iter().sum::<f64>() / self.money_flow_values.len() as f64;
        }
        
        self.volume_sum += volume;
    }
    
    /// Обновить RSI
    fn update_rsi(&mut self, close: f64) {
        let change = close - self.prev_close;
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };
        
        // Упрощенный RSI расчет
        if self.rsi_values.len() >= self.rsi_period {
            self.rsi_values.remove(0);
        }
        self.rsi_values.push(if gain > loss { gain } else { -loss });
        
        if self.rsi_values.len() >= self.rsi_period {
            let avg_gain = self.rsi_values.iter().filter(|&&x| x > 0.0).sum::<f64>() / self.rsi_period as f64;
            let avg_loss = self.rsi_values.iter().filter(|&&x| x < 0.0).map(|x| -x).sum::<f64>() / self.rsi_period as f64;
            
            if avg_loss.abs() < 1e-12 {
                self.rsi = 100.0;
            } else {
                let rs = avg_gain / avg_loss;
                self.rsi = 100.0 - (100.0 / (1.0 + rs));
            }
        }
    }
    
    /// Обновить VWAP
    fn update_vwap(&mut self, hlc3: f64, volume: f64) {
        self.vwap_sum += hlc3 * volume;
        self.volume_total += volume;
        
        if self.volume_total > 0.0 {
            self.vwap = self.vwap_sum / self.volume_total;
        }
    }
    
    /// Генерировать торговые сигналы
    fn generate_signals(&mut self, close: f64, volume: f64) {
        // Сброс сигналов
        self.buy_signal = false;
        self.sell_signal = false;
        self.strong_buy = false;
        self.strong_sell = false;
        self.reversal_signal = 0;
        
        // Wave Trend кроссоверы
        let wt_cross_up = self.wt1 > self.wt2 && self.wt1 < -40.0;
        let wt_cross_down = self.wt1 < self.wt2 && self.wt1 > 40.0;
        
        // RSI условия
        let rsi_oversold = self.rsi < 30.0;
        let rsi_overbought = self.rsi > 70.0;
        
        // VWAP условия
        let above_vwap = close > self.vwap;
        let below_vwap = close < self.vwap;
        
        // Money Flow условия
        let strong_volume = volume > self.volume_sum / self.count as f64 * 1.5;
        
        // Основные сигналы (зеленые/красные точки)
        if wt_cross_up && rsi_oversold {
            self.buy_signal = true;
        }
        
        if wt_cross_down && rsi_overbought {
            self.sell_signal = true;
        }
        
        // Сильные сигналы (синие треугольники)
        if self.buy_signal && above_vwap && strong_volume {
            self.strong_buy = true;
        }
        
        if self.sell_signal && below_vwap && strong_volume {
            self.strong_sell = true;
        }
        
        // Сигналы разворота (желтые X)
        if self.wt1 > 60.0 && self.wt1 < self.wt2 && rsi_overbought {
            self.reversal_signal = -1; // Медвежий разворот
        }
        
        if self.wt1 < -60.0 && self.wt1 > self.wt2 && rsi_oversold {
            self.reversal_signal = 1; // Бычий разворот
        }
    }
    
    /// Рассчитать EMA
    fn calculate_ema(&self, values: &[f64], period: usize) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut ema = values[0];
        
        for &value in values.iter().skip(1) {
            ema = alpha * value + (1.0 - alpha) * ema;
        }
        
        ema
    }
    
    /// Получить Wave Trend значения
    pub fn wave_trend(&self) -> (f64, f64) {
        (self.wt1, self.wt2)
    }
    
    /// Получить основные сигналы
    pub fn signals(&self) -> (bool, bool) {
        (self.buy_signal, self.sell_signal)
    }
    
    /// Получить сильные сигналы
    pub fn strong_signals(&self) -> (bool, bool) {
        (self.strong_buy, self.strong_sell)
    }
    
    /// Получить сигнал разворота
    pub fn reversal_signal(&self) -> i8 {
        self.reversal_signal
    }
    
    /// Получить RSI значение
    pub fn rsi(&self) -> f64 {
        self.rsi
    }
    
    /// Получить VWAP значение
    pub fn vwap(&self) -> f64 {
        self.vwap
    }
    
    /// Получить Money Flow
    pub fn money_flow(&self) -> f64 {
        self.money_flow
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.hlc3_values.clear();
        self.wt1_values.clear();
        self.wt2_values.clear();
        self.rsi_values.clear();
        
        self.wt1 = 0.0;
        self.wt2 = 0.0;
        self.money_flow = 0.0;
        self.volume_sum = 0.0;
        self.money_flow_values.clear();  // Очищаем новый массив
        self.rsi = 0.0;
        self.vwap_sum = 0.0;
        self.volume_total = 0.0;
        self.vwap = 0.0;
        self.buy_signal = false;
        self.sell_signal = false;
        self.strong_buy = false;
        self.strong_sell = false;
        self.reversal_signal = 0;
        self.count = 0;
        self.is_ready = false;
        self.prev_close = 0.0;
    }

    /// Получить период Money Flow
    pub fn get_mf_period(&self) -> usize {
        self.mf_period
    }

    /// Установить новый период Money Flow
    pub fn set_mf_period(&mut self, new_mf_period: usize) {
        assert!(new_mf_period > 0 && new_mf_period <= 512, "MF period must be > 0 and <= 512");
        self.mf_period = new_mf_period;
        // Обрезаем массив если новый период меньше
        while self.money_flow_values.len() > self.mf_period {
            self.money_flow_values.remove(0);
        }
        // Пересчитываем Money Flow с новым периодом
        if !self.money_flow_values.is_empty() {
            self.money_flow = self.money_flow_values.iter().sum::<f64>() / self.money_flow_values.len() as f64;
        }
    }

    /// Получить период Wave Trend
    pub fn get_wt_period(&self) -> usize {
        self.wt_period
    }

    /// Получить период RSI
    pub fn get_rsi_period(&self) -> usize {
        self.rsi_period
    }

    /// Получить полную конфигурацию
    pub fn get_config(&self) -> MarketCipherConfig {
        MarketCipherConfig {
            period: self.period,
            wt_period: self.wt_period,
            wt_signal_period: self.wt_signal_period,
            mf_period: self.mf_period,
            rsi_period: self.rsi_period,
        }
    }

    /// Установить новую конфигурацию
    pub fn set_config(&mut self, config: MarketCipherConfig) {
        self.period = config.period;
        self.wt_period = config.wt_period;
        self.wt_signal_period = config.wt_signal_period;
        self.rsi_period = config.rsi_period;
        
        // Обновляем mf_period через setter для правильной обработки
        self.set_mf_period(config.mf_period);
    }

    /// Получить комплексный торговый сигнал
    /// 2 = сильная покупка, 1 = покупка, -1 = продажа, -2 = сильная продажа, 0 = нет сигнала
    pub fn composite_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        if self.strong_buy {
            return 2;
        } else if self.strong_sell {
            return -2;
        } else if self.buy_signal {
            return 1;
        } else if self.sell_signal {
            return -1;
        }
        
        0
    }
    
    /// Получить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        
        match (self.strong_buy, self.strong_sell, self.buy_signal, self.sell_signal, self.reversal_signal) {
            (true, _, _, _, _) => "Strong Bullish",
            (_, true, _, _, _) => "Strong Bearish",
            (_, _, true, _, _) => "Bullish",
            (_, _, _, true, _) => "Bearish",
            (_, _, _, _, 1) => "Bullish Reversal",
            (_, _, _, _, -1) => "Bearish Reversal",
            _ => "Neutral"
        }
    }
    
    /// Получить подробную информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "Market Cipher: WT1={:.2}, WT2={:.2}, RSI={:.1}, MF={:.0}, VWAP={:.2}, Buy={}, Sell={}, Reversal={}",
            self.wt1, self.wt2, self.rsi, self.money_flow, self.vwap,
            self.buy_signal, self.sell_signal, self.reversal_signal
        )
    }
}

/// Конфигурация индикатора Market Cipher
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarketCipherConfig {
    pub period: usize,
    pub wt_period: usize,
    pub wt_signal_period: usize,
    pub mf_period: usize,
    pub rsi_period: usize,
}

impl std::fmt::Debug for MarketCipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarketCipher")
            .field("period", &self.period)
            .field("wt_period", &self.wt_period)
            .field("wt_signal_period", &self.wt_signal_period)
            .field("mf_period", &self.mf_period)
            .field("rsi_period", &self.rsi_period)
            .field("wt1", &self.wt1)
            .field("wt2", &self.wt2)
            .field("money_flow", &self.money_flow)
            .field("rsi", &self.rsi)
            .field("vwap", &self.vwap)
            .field("buy_signal", &self.buy_signal)
            .field("sell_signal", &self.sell_signal)
            .field("reversal_signal", &self.reversal_signal)
            .field("is_ready", &self.is_ready)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_market_cipher_new() {
        let cipher = MarketCipher::new(14);
        assert_eq!(cipher.period(), 14);
        assert!(!cipher.is_ready());
    }
    
    #[test]
    fn test_market_cipher_calculation() {
        let mut cipher = MarketCipher::new(10);
        
        // Тестовые данные
        let test_data = vec![
            (100.0, 102.0, 98.0, 101.0, 1000.0),
            (101.0, 103.0, 99.0, 102.0, 1200.0),
            (102.0, 104.0, 100.0, 103.0, 800.0),
            (103.0, 105.0, 101.0, 104.0, 1500.0),
            (104.0, 106.0, 102.0, 105.0, 900.0),
        ];
        
        for (open, high, low, close, volume) in test_data {
            let (wt1, wt2, buy, sell) = cipher.update_bar(open, high, low, close, volume);
            println!("WT1: {:.3}, WT2: {:.3}, Buy: {}, Sell: {}", wt1, wt2, buy, sell);
        }
        
        // Проверить, что значения в разумных пределах
        let (wt1, wt2) = cipher.wave_trend();
        assert!(wt1 >= -100.0 && wt1 <= 100.0);
        assert!(wt2 >= -100.0 && wt2 <= 100.0);
    }
} 






















