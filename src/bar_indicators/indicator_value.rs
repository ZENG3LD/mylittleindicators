//! Типизированные значения индикаторов
//!
//! Enum для представления всех типов значений индикаторов без аллокаций.
//! Все варианты Copy/Clone для максимальной производительности.

use std::collections::HashMap;

/// Типизированное значение индикатора
///
/// Все значения хранятся на стеке (Copy types) для максимальной производительности.
/// Используется pattern matching вместо HashMap lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndicatorValue {
    /// Простое значение (RSI, SMA, EMA, momentum, volume indicators)
    Single(f64),

    /// Сигнал-индикатор (-1, 0, 1) для кроссоверов, трендов, торговых сигналов
    Signal(i8),

    /// Булево значение (детекторы, флаги, условия)
    Flag(bool),

    /// Два значения (MACD line + signal, Darvas box upper/lower)
    Double(f64, f64),

    /// Три значения (общий случай)
    Triple(f64, f64, f64),

    /// Канал: (upper, middle, lower) - Bollinger Bands, Keltner, Donchian
    Channel3 {
        upper: f64,
        middle: f64,
        lower: f64,
    },

    /// MACD с гистограммой
    Macd {
        line: f64,
        signal: f64,
        histogram: f64,
    },

    /// Ichimoku Cloud (5 линий)
    Ichimoku {
        tenkan: f64,
        kijun: f64,
        senkou_a: f64,
        senkou_b: f64,
        chikou: f64,
    },

    /// Heikin Ashi candle (OHLC преобразование)
    Candle {
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    },

    /// Расширенный канал с метриками (BB + bandwidth, %B)
    ChannelExtended {
        upper: f64,
        middle: f64,
        lower: f64,
        bandwidth: f64,
        percent_b: f64,
    },

    /// Адаптивный индикатор с параметрами
    Adaptive {
        value: f64,
        period: f64,
        alpha: f64,
    },

    /// Статистический тест (ADF, KPSS, cointegration)
    StatTest {
        statistic: f64,
        p_value: f64,
        is_significant: bool,
    },

    /// Волатильность с компонентами
    Volatility {
        total: f64,
        close_close: f64,
        high_low: f64,
    },

    /// Смешанное значение + флаг
    ValueFlag(f64, bool),

    /// Два флага
    DoubleFlag(bool, bool),

    /// Fuzzy Candle (5 классификаций: direction, size, body_size, upper_wick, lower_wick)
    FuzzyCandle {
        direction: i8,
        size: i8,
        body_size: i8,
        upper_wick: i8,
        lower_wick: i8,
    },

    /// Candle Anatomy (3 ratios + 2 flags)
    CandleAnatomy {
        body: f64,
        upper_wick: f64,
        lower_wick: f64,
        long_upper: bool,
        long_lower: bool,
    },

    /// Hilbert Transform (amplitude, phase, frequency)
    Hilbert {
        amplitude: f64,
        phase: f64,
        frequency: f64,
    },
}

impl IndicatorValue {
    /// Получить главное значение как f64
    ///
    /// Для Single/Signal/Flag возвращает значение напрямую.
    /// Для сложных типов (Channel, Macd, etc.) возвращает "основное" значение:
    /// - Channel → middle
    /// - Macd → line
    /// - Ichimoku → kijun
    /// - Candle → close
    pub fn main(&self) -> f64 {
        match self {
            Self::Single(v) => *v,
            Self::Signal(s) => *s as f64,
            Self::Flag(b) => if *b { 1.0 } else { 0.0 },
            Self::Double(a, _) => *a,
            Self::Triple(a, _, _) => *a,
            Self::Channel3 { middle, .. } => *middle,
            Self::Macd { line, .. } => *line,
            Self::Ichimoku { kijun, .. } => *kijun,
            Self::Candle { close, .. } => *close,
            Self::ChannelExtended { middle, .. } => *middle,
            Self::Adaptive { value, .. } => *value,
            Self::StatTest { statistic, .. } => *statistic,
            Self::Volatility { total, .. } => *total,
            Self::ValueFlag(v, _) => *v,
            Self::DoubleFlag(a, _) => if *a { 1.0 } else { 0.0 },
            Self::FuzzyCandle { direction, .. } => *direction as f64,
            Self::CandleAnatomy { body, .. } => *body,
            Self::Hilbert { amplitude, .. } => *amplitude,
        }
    }

    /// Проверить, является ли значение сигналом
    pub fn is_signal(&self) -> bool {
        matches!(self, Self::Signal(_))
    }

    /// Получить как сигнал (если это сигнал)
    pub fn as_signal(&self) -> Option<i8> {
        match self {
            Self::Signal(s) => Some(*s),
            _ => None,
        }
    }

    /// Проверить, является ли значение каналом
    pub fn is_channel(&self) -> bool {
        matches!(self, Self::Channel3 { .. } | Self::ChannelExtended { .. })
    }

    /// Получить канал как (upper, middle, lower)
    pub fn as_channel(&self) -> Option<(f64, f64, f64)> {
        match self {
            Self::Channel3 { upper, middle, lower } => Some((*upper, *middle, *lower)),
            Self::ChannelExtended { upper, middle, lower, .. } => Some((*upper, *middle, *lower)),
            _ => None,
        }
    }

    /// Получить как MACD
    pub fn as_macd(&self) -> Option<(f64, f64, f64)> {
        match self {
            Self::Macd { line, signal, histogram } => Some((*line, *signal, *histogram)),
            _ => None,
        }
    }

    /// Get upper band (for channels) - convenience method
    pub fn upper(&self) -> Option<f64> {
        match self {
            Self::Channel3 { upper, .. } => Some(*upper),
            Self::ChannelExtended { upper, .. } => Some(*upper),
            Self::Double(a, _) => Some(*a),
            Self::Triple(a, _, _) => Some(*a),
            _ => None,
        }
    }

    /// Get lower band (for channels) - convenience method
    pub fn lower(&self) -> Option<f64> {
        match self {
            Self::Channel3 { lower, .. } => Some(*lower),
            Self::ChannelExtended { lower, .. } => Some(*lower),
            Self::Double(_, b) => Some(*b),
            Self::Triple(_, _, c) => Some(*c),
            _ => None,
        }
    }

    /// Get middle/center value (for channels) - convenience method
    pub fn middle(&self) -> Option<f64> {
        match self {
            Self::Channel3 { middle, .. } => Some(*middle),
            Self::ChannelExtended { middle, .. } => Some(*middle),
            Self::Triple(_, b, _) => Some(*b),
            _ => None,
        }
    }

    /// Get MACD line - convenience method
    pub fn macd_line(&self) -> Option<f64> {
        match self {
            Self::Macd { line, .. } => Some(*line),
            _ => None,
        }
    }

    /// Get MACD signal - convenience method
    pub fn macd_signal(&self) -> Option<f64> {
        match self {
            Self::Macd { signal, .. } => Some(*signal),
            _ => None,
        }
    }

    /// Get MACD histogram - convenience method
    pub fn macd_histogram(&self) -> Option<f64> {
        match self {
            Self::Macd { histogram, .. } => Some(*histogram),
            _ => None,
        }
    }

    /// Check if the main value is finite (not NaN or infinite)
    pub fn is_finite(&self) -> bool {
        self.main().is_finite()
    }

    /// Конвертировать в Vec<f64> (для legacy кода или сериализации)
    pub fn as_vec(&self) -> Vec<f64> {
        match self {
            Self::Single(v) => vec![*v],
            Self::Signal(s) => vec![*s as f64],
            Self::Flag(b) => vec![if *b { 1.0 } else { 0.0 }],
            Self::Double(a, b) => vec![*a, *b],
            Self::Triple(a, b, c) => vec![*a, *b, *c],
            Self::Channel3 { upper, middle, lower } => vec![*upper, *middle, *lower],
            Self::Macd { line, signal, histogram } => vec![*line, *signal, *histogram],
            Self::Ichimoku { tenkan, kijun, senkou_a, senkou_b, chikou } =>
                vec![*tenkan, *kijun, *senkou_a, *senkou_b, *chikou],
            Self::Candle { open, high, low, close } => vec![*open, *high, *low, *close],
            Self::ChannelExtended { upper, middle, lower, bandwidth, percent_b } =>
                vec![*upper, *middle, *lower, *bandwidth, *percent_b],
            Self::Adaptive { value, period, alpha } => vec![*value, *period, *alpha],
            Self::StatTest { statistic, p_value, is_significant } =>
                vec![*statistic, *p_value, if *is_significant { 1.0 } else { 0.0 }],
            Self::Volatility { total, close_close, high_low } =>
                vec![*total, *close_close, *high_low],
            Self::ValueFlag(v, f) => vec![*v, if *f { 1.0 } else { 0.0 }],
            Self::DoubleFlag(a, b) => vec![
                if *a { 1.0 } else { 0.0 },
                if *b { 1.0 } else { 0.0 }
            ],
            Self::FuzzyCandle { direction, size, body_size, upper_wick, lower_wick } =>
                vec![*direction as f64, *size as f64, *body_size as f64, *upper_wick as f64, *lower_wick as f64],
            Self::CandleAnatomy { body, upper_wick, lower_wick, long_upper, long_lower } =>
                vec![*body, *upper_wick, *lower_wick, if *long_upper { 1.0 } else { 0.0 }, if *long_lower { 1.0 } else { 0.0 }],
            Self::Hilbert { amplitude, phase, frequency } =>
                vec![*amplitude, *phase, *frequency],
        }
    }

    /// Конвертировать в HashMap<String, f64> (для обратной совместимости)
    pub fn as_hashmap(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();

        match self {
            Self::Single(v) => {
                map.insert("value".to_string(), *v);
            }
            Self::Signal(s) => {
                map.insert("signal".to_string(), *s as f64);
            }
            Self::Flag(b) => {
                map.insert("flag".to_string(), if *b { 1.0 } else { 0.0 });
            }
            Self::Double(a, b) => {
                map.insert("first".to_string(), *a);
                map.insert("second".to_string(), *b);
            }
            Self::Triple(a, b, c) => {
                map.insert("first".to_string(), *a);
                map.insert("second".to_string(), *b);
                map.insert("third".to_string(), *c);
            }
            Self::Channel3 { upper, middle, lower } => {
                map.insert("upper".to_string(), *upper);
                map.insert("middle".to_string(), *middle);
                map.insert("lower".to_string(), *lower);
            }
            Self::Macd { line, signal, histogram } => {
                map.insert("line".to_string(), *line);
                map.insert("signal".to_string(), *signal);
                map.insert("histogram".to_string(), *histogram);
            }
            Self::Ichimoku { tenkan, kijun, senkou_a, senkou_b, chikou } => {
                map.insert("tenkan".to_string(), *tenkan);
                map.insert("kijun".to_string(), *kijun);
                map.insert("senkou_a".to_string(), *senkou_a);
                map.insert("senkou_b".to_string(), *senkou_b);
                map.insert("chikou".to_string(), *chikou);
            }
            Self::Candle { open, high, low, close } => {
                map.insert("open".to_string(), *open);
                map.insert("high".to_string(), *high);
                map.insert("low".to_string(), *low);
                map.insert("close".to_string(), *close);
            }
            Self::ChannelExtended { upper, middle, lower, bandwidth, percent_b } => {
                map.insert("upper".to_string(), *upper);
                map.insert("middle".to_string(), *middle);
                map.insert("lower".to_string(), *lower);
                map.insert("bandwidth".to_string(), *bandwidth);
                map.insert("percent_b".to_string(), *percent_b);
            }
            Self::Adaptive { value, period, alpha } => {
                map.insert("value".to_string(), *value);
                map.insert("period".to_string(), *period);
                map.insert("alpha".to_string(), *alpha);
            }
            Self::StatTest { statistic, p_value, is_significant } => {
                map.insert("statistic".to_string(), *statistic);
                map.insert("p_value".to_string(), *p_value);
                map.insert("is_significant".to_string(), if *is_significant { 1.0 } else { 0.0 });
            }
            Self::Volatility { total, close_close, high_low } => {
                map.insert("total".to_string(), *total);
                map.insert("close_close".to_string(), *close_close);
                map.insert("high_low".to_string(), *high_low);
            }
            Self::ValueFlag(v, f) => {
                map.insert("value".to_string(), *v);
                map.insert("flag".to_string(), if *f { 1.0 } else { 0.0 });
            }
            Self::DoubleFlag(a, b) => {
                map.insert("first".to_string(), if *a { 1.0 } else { 0.0 });
                map.insert("second".to_string(), if *b { 1.0 } else { 0.0 });
            }
            Self::FuzzyCandle { direction, size, body_size, upper_wick, lower_wick } => {
                map.insert("direction".to_string(), *direction as f64);
                map.insert("size".to_string(), *size as f64);
                map.insert("body_size".to_string(), *body_size as f64);
                map.insert("upper_wick".to_string(), *upper_wick as f64);
                map.insert("lower_wick".to_string(), *lower_wick as f64);
            }
            Self::CandleAnatomy { body, upper_wick, lower_wick, long_upper, long_lower } => {
                map.insert("body".to_string(), *body);
                map.insert("upper_wick".to_string(), *upper_wick);
                map.insert("lower_wick".to_string(), *lower_wick);
                map.insert("long_upper".to_string(), if *long_upper { 1.0 } else { 0.0 });
                map.insert("long_lower".to_string(), if *long_lower { 1.0 } else { 0.0 });
            }
            Self::Hilbert { amplitude, phase, frequency } => {
                map.insert("amplitude".to_string(), *amplitude);
                map.insert("phase".to_string(), *phase);
                map.insert("frequency".to_string(), *frequency);
            }
        }

        map
    }
}


impl std::fmt::Display for IndicatorValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(v) => write!(f, "{:.6}", v),
            Self::Signal(s) => write!(f, "{}", s),
            Self::Flag(b) => write!(f, "{}", b),
            Self::Double(a, b) => write!(f, "({:.6}, {:.6})", a, b),
            Self::Triple(a, b, c) => write!(f, "({:.6}, {:.6}, {:.6})", a, b, c),
            Self::Channel3 { upper, middle, lower } => 
                write!(f, "Channel(upper={:.6}, middle={:.6}, lower={:.6})", upper, middle, lower),
            Self::Macd { line, signal, histogram } => 
                write!(f, "MACD(line={:.6}, signal={:.6}, histogram={:.6})", line, signal, histogram),
            Self::Ichimoku { tenkan, kijun, senkou_a, senkou_b, chikou } => 
                write!(f, "Ichimoku(tenkan={:.6}, kijun={:.6}, senkou_a={:.6}, senkou_b={:.6}, chikou={:.6})", 
                       tenkan, kijun, senkou_a, senkou_b, chikou),
            Self::Candle { open, high, low, close } => 
                write!(f, "Candle(O={:.6}, H={:.6}, L={:.6}, C={:.6})", open, high, low, close),
            Self::ChannelExtended { upper, middle, lower, bandwidth, percent_b } => 
                write!(f, "ChannelExt(upper={:.6}, middle={:.6}, lower={:.6}, bw={:.6}, %B={:.6})", 
                       upper, middle, lower, bandwidth, percent_b),
            Self::Adaptive { value, period, alpha } => 
                write!(f, "Adaptive(value={:.6}, period={:.6}, alpha={:.6})", value, period, alpha),
            Self::StatTest { statistic, p_value, is_significant } => 
                write!(f, "StatTest(stat={:.6}, p={:.6}, sig={})", statistic, p_value, is_significant),
            Self::Volatility { total, close_close, high_low } => 
                write!(f, "Vol(total={:.6}, cc={:.6}, hl={:.6})", total, close_close, high_low),
            Self::ValueFlag(v, flag) => write!(f, "({:.6}, {})", v, flag),
            Self::DoubleFlag(a, b) => write!(f, "({}, {})", a, b),
            Self::FuzzyCandle { direction, size, body_size, upper_wick, lower_wick } => 
                write!(f, "FuzzyCandle(dir={}, size={}, body={}, uw={}, lw={})", 
                       direction, size, body_size, upper_wick, lower_wick),
            Self::CandleAnatomy { body, upper_wick, lower_wick, long_upper, long_lower } =>
                write!(f, "Anatomy(body={:.6}, uw={:.6}, lw={:.6}, lu={}, ll={})",
                       body, upper_wick, lower_wick, long_upper, long_lower),
            Self::Hilbert { amplitude, phase, frequency } =>
                write!(f, "Hilbert(amp={:.6}, phase={:.6}, freq={:.6})", amplitude, phase, frequency),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_value() {
        let val = IndicatorValue::Single(42.5);
        assert_eq!(val.main(), 42.5);
        assert_eq!(val.as_vec(), vec![42.5]);
    }

    #[test]
    fn test_signal_value() {
        let val = IndicatorValue::Signal(1);
        assert_eq!(val.main(), 1.0);
        assert_eq!(val.as_signal(), Some(1));
        assert!(val.is_signal());
    }

    #[test]
    fn test_channel_value() {
        let val = IndicatorValue::Channel3 {
            upper: 110.0,
            middle: 100.0,
            lower: 90.0,
        };
        assert_eq!(val.main(), 100.0);
        assert_eq!(val.as_channel(), Some((110.0, 100.0, 90.0)));
        assert!(val.is_channel());
    }

    #[test]
    fn test_macd_value() {
        let val = IndicatorValue::Macd {
            line: 2.5,
            signal: 2.0,
            histogram: 0.5,
        };
        assert_eq!(val.main(), 2.5);
        assert_eq!(val.as_macd(), Some((2.5, 2.0, 0.5)));
    }

    #[test]
    fn test_as_vec() {
        let val = IndicatorValue::Triple(1.0, 2.0, 3.0);
        assert_eq!(val.as_vec(), vec![1.0, 2.0, 3.0]);
    }
}
