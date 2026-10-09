//! DeMark Pivot Points - пивот уровни по методологии Тома ДеМарка
//!
//! DeMark Pivots используют специальную логику для расчета X значения:
//!
//! Если Close < Open: X = High + (2 × Low) + Close
//! Если Close > Open: X = (2 × High) + Low + Close
//! Если Close = Open: X = High + Low + (2 × Close)
//!
//! Затем рассчитываются уровни:
//! Pivot Point (PP) = X / 4
//! Support (S1) = X / 2 - High
//! Resistance (R1) = X / 2 - Low

use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Row labels for the 3-level DeMark ladder (ascending: S1, PP, R1).
const DEMARK_RUNGS: &[&'static str] = &["S1", "PP", "R1"];

/// Уровни DeMark Pivot Points
#[derive(Debug, Clone, Copy)]
pub struct DeMarkPivotLevels {
    pub pivot: f64,       // Основной пивот уровень
    pub resistance_1: f64, // Первое сопротивление (R1)
    pub support_1: f64,    // Первая поддержка (S1)
    pub x_value: f64,      // Промежуточное X значение для анализа
}

impl DeMarkPivotLevels {
    /// Создать пустые уровни
    pub fn empty() -> Self {
        Self {
            pivot: 0.0,
            resistance_1: 0.0,
            support_1: 0.0,
            x_value: 0.0,
        }
    }
    
    /// Получить все уровни отсортированные по возрастанию
    pub fn all_levels(&self) -> [f64; 3] {
        [self.support_1, self.pivot, self.resistance_1]
    }
    
    /// Получить диапазон торговли (между S1 и R1)
    pub fn trading_range(&self) -> f64 {
        self.resistance_1 - self.support_1
    }
    
    /// Проверить, находится ли цена в диапазоне
    pub fn is_in_range(&self, price: f64) -> bool {
        price >= self.support_1 && price <= self.resistance_1
    }
}

/// DeMark Pivot Points индикатор
#[derive(Debug, Clone)]
pub struct DeMarkPivots {
    // Текущие уровни
    current_levels: DeMarkPivotLevels,
    
    // История уровней
    levels_history: Vec<DeMarkPivotLevels>,

    // Буферы для расчета
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    
    // Настройки
    calculation_period: usize, // Период для расчета
    bars_since_update: usize,
    
    // Состояние
    is_ready: bool,
    update_count: usize,

    /// The 3 levels as a fixed `3 × 1` price VECTOR (row 0 = S1, row 1 = PP, row 2 = R1).
    /// Read via `levels_grid` / `ContractFactory::grid`.
    grid: MatrixGrid,
}

impl Default for DeMarkPivots {
    /// Factory default: `with_period(1)` — daily update (calc_period = `unwrap_or(1).max(1)`).
    fn default() -> Self {
        Self::with_period(1)
    }
}

impl DeMarkPivots {
    /// Создать новый индикатор DeMark Pivots (ежедневное обновление)
    pub fn new() -> Self {
        Self::with_period(1)
    }
    
    /// Создать новый индикатор с настраиваемым периодом обновления
    pub fn with_period(calculation_period: usize) -> Self {
        assert!(calculation_period > 0, "Period must be greater than 0");
        
        Self {
            current_levels: DeMarkPivotLevels::empty(),
            levels_history: Vec::with_capacity(100),
            opens: Vec::with_capacity(32),
            highs: Vec::with_capacity(32),
            lows: Vec::with_capacity(32),
            closes: Vec::with_capacity(32),
            calculation_period,
            bars_since_update: 0,
            is_ready: false,
            update_count: 0,
            grid: MatrixGrid::new(3, 1, false)
                .with_labels(AxisLabels::Named(DEMARK_RUNGS), AxisLabels::Bars),
        }
    }

    /// Обновить индикатор новым баром
    fn recompute(&mut self, open: f64, high: f64, low: f64, close: f64) -> DeMarkPivotLevels {
        // Добавляем данные в буферы
        self.opens.push(open);
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);
        
        self.bars_since_update += 1;
        
        // Проверяем, нужно ли пересчитать уровни
        if self.bars_since_update >= self.calculation_period {
            self.calculate_levels();
            self.bars_since_update = 0;
        }
        
        self.current_levels
    }
    
    /// Принудительно пересчитать уровни на основе накопленных данных
    pub fn calculate_levels(&mut self) {
        if self.opens.is_empty() || self.highs.is_empty() || 
           self.lows.is_empty() || self.closes.is_empty() {
            return;
        }
        
        // Находим данные за период
        let period_open = self.opens[0]; // Открытие первого бара периода
        let period_high = self.highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let period_low = self.lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let period_close = self.closes[self.closes.len() - 1]; // Закрытие последнего бара
        
        // Рассчитываем X по методологии DeMark
        let x_value = if period_close < period_open {
            // Медвежий бар: X = High + (2 × Low) + Close
            period_high + (2.0 * period_low) + period_close
        } else if period_close > period_open {
            // Бычий бар: X = (2 × High) + Low + Close
            (2.0 * period_high) + period_low + period_close
        } else {
            // Нейтральный бар (Close = Open): X = High + Low + (2 × Close)
            period_high + period_low + (2.0 * period_close)
        };
        
        // Рассчитываем уровни DeMark
        let pivot = x_value / 4.0;
        let support_1 = (x_value / 2.0) - period_high;
        let resistance_1 = (x_value / 2.0) - period_low;
        
        // Создаем новые уровни
        let new_levels = DeMarkPivotLevels {
            pivot,
            resistance_1,
            support_1,
            x_value,
        };
        
        // Сохраняем в историю
        if self.levels_history.len() >= 100 {
            self.levels_history.remove(0);
        }
        self.levels_history.push(self.current_levels);
        
        // Обновляем текущие уровни
        self.current_levels = new_levels;

        // Snapshot the 3 levels into the fixed price vector (row 0 = S1, row 1 = PP, row 2 = R1).
        self.grid.reset();
        for (row, &lvl) in new_levels.all_levels().iter().enumerate() {
            self.grid.set_direction(MatrixCell::new(row as u16, 0), lvl);
        }

        // Очищаем буферы для следующего периода
        self.opens.clear();
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        
        self.update_count += 1;
        self.is_ready = true;
    }
    
    /// Получить текущие уровни DeMark Pivots
    pub fn levels(&self) -> DeMarkPivotLevels {
        self.current_levels
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// The 3 levels as a fixed `3 × 1` price vector — the matrix output behind
    /// `IndicatorOutputId::DemarkLevelsGrid`.
    pub fn levels_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.current_levels = DeMarkPivotLevels::empty();
        self.levels_history.clear();
        self.opens.clear();
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.bars_since_update = 0;
        self.is_ready = false;
        self.update_count = 0;
        self.grid.reset();
    }
    
    /// Получить ближайший уровень поддержки
    pub fn nearest_support(&self, current_price: f64) -> f64 {
        if current_price > self.current_levels.support_1 {
            self.current_levels.support_1
        } else {
            // Если цена ниже S1, возвращаем S1 как ближайший уровень сопротивления
            self.current_levels.support_1
        }
    }
    
    /// Получить ближайший уровень сопротивления
    pub fn nearest_resistance(&self, current_price: f64) -> f64 {
        if current_price < self.current_levels.resistance_1 {
            self.current_levels.resistance_1
        } else {
            // Если цена выше R1, возвращаем R1 как ближайший уровень поддержки
            self.current_levels.resistance_1
        }
    }
    
    /// Определить позицию цены относительно уровней
    /// Возвращает: -1 (ниже поддержки), 0 (в диапазоне), 1 (выше сопротивления)
    pub fn price_position(&self, current_price: f64) -> i8 {
        if current_price < self.current_levels.support_1 {
            -1 // Ниже поддержки
        } else if current_price > self.current_levels.resistance_1 {
            1 // Выше сопротивления
        } else {
            0 // В диапазоне
        }
    }
    
    /// Генерировать торговый сигнал на основе пробоя уровней
    /// Возвращает: -1 (продажа), 0 (нет сигнала), 1 (покупка)
    pub fn trading_signal(&self, current_price: f64, prev_price: f64) -> i8 {
        let current_pos = self.price_position(current_price);
        let prev_pos = self.price_position(prev_price);
        
        // Пробой вверх через сопротивление
        if prev_pos <= 0 && current_pos == 1 {
            return 1; // Сигнал покупки
        }
        
        // Пробой вниз через поддержку
        if prev_pos >= 0 && current_pos == -1 {
            return -1; // Сигнал продажи
        }
        
        0 // Нет сигнала
    }
    
    /// Получить расстояние до ближайшего уровня
    pub fn distance_to_nearest_level(&self, current_price: f64) -> f64 {
        let dist_to_support = (current_price - self.current_levels.support_1).abs();
        let dist_to_resistance = (current_price - self.current_levels.resistance_1).abs();
        let dist_to_pivot = (current_price - self.current_levels.pivot).abs();
        
        dist_to_support.min(dist_to_resistance).min(dist_to_pivot)
    }
    
    /// Получить силу уровня (количество касаний)
    pub fn level_strength(&self, level: f64, price_history: &[f64], tolerance_pct: f64) -> u32 {
        let tolerance = level * tolerance_pct / 100.0;
        
        price_history.iter()
            .filter(|&&price| (price - level).abs() <= tolerance)
            .count() as u32
    }
    
    /// Получить историю уровней
    pub fn levels_history(&self) -> Vec<DeMarkPivotLevels> {
        self.levels_history.iter().cloned().collect()
    }
    
    /// Получить информацию о текущих уровнях
    pub fn info(&self, current_price: f64) -> String {
        let levels = self.current_levels;
        let position = match self.price_position(current_price) {
            -1 => "Ниже поддержки",
            0 => "В диапазоне",
            1 => "Выше сопротивления",
            _ => "Неизвестно",
        };
        
        format!(
            "DeMark Pivots: PP={:.4}, R1={:.4}, S1={:.4}, X={:.4}, Позиция: {}, Диапазон: {:.4}",
            levels.pivot,
            levels.resistance_1,
            levels.support_1,
            levels.x_value,
            position,
            levels.trading_range()
        )
    }
    
    /// Получить период расчета
    pub fn period(&self) -> usize {
        self.calculation_period
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
}

// Реализация стандартных методов для совместимости с системой индикаторов
impl DeMarkPivots {
    /// Получить основное значение (пивот точку)
    pub fn value(&self) -> f64 {
        self.current_levels.pivot
    }

    /// Feed resolved `[open, high, low, close]` lanes.
    pub fn feed(&mut self, lanes: &[f64]) {
        let open = lanes[0];
        let high = lanes[1];
        let low = lanes[2];
        let close = lanes[3];
        self.recompute(open, high, low, close);
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param,
    Render, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`DeMarkPivots`] — update period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DeMarkConfig {
    pub period: Param<usize>,
}

impl Indicator for DeMarkPivots {
    const ID: IndicatorId = IndicatorId::Demark;
    /// Not a pluggable family member — DeMark pivot level producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed O/H/L/C fields — X formula depends on close vs open relationship.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(period) per update — rescans O/H/L/C buffers to find period extremes.
    /// Four period-deep Vec buffers.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::Demark),
        Output::matrix(IndicatorOutputId::DemarkLevelsGrid, ValueDomain::Price),
    ];
    type Config = DeMarkConfig;
    type Runtime = DeMarkPivots;

    fn create(cfg: DeMarkConfig) -> DeMarkPivots {
        DeMarkPivots::with_period(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for DeMarkConfig {
    fn defaults() -> Self {
        DeMarkConfig { period: Param::Solo(1) }
    }
    fn machine_defaults() -> Self {
        // period: bars accumulated before recalculating DeMark levels. Default=1 (every bar).
        // Treated as Class A period; auto range(2,4048,1). `create` calls `.max(1)`.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for DeMarkPivots {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Demark, "DeMark", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_demark() {
        let mut f = IndicatorOrder::Demark(<<DeMarkPivots as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // defaults() → DeMarkConfig { period: Param::Solo(1) }
        // bullish bar: close > open => X = 2*H + L + C; PP in range
        f.feed(0, MarketSample::Bar {
            open: 100.0,
            high: 105.0,
            low: 98.0,
            close: 103.0,
            volume: 9999.0,
        });
        assert!(f.is_ready());
        let v = f.primary();
        // X = 210+98+103=411, PP=102.75
        assert!((v - 102.75).abs() < 0.1, "demark pivot unexpected: {v}");
    }

    #[test]
    fn demark_direct_levels_grid_ascending() {
        use crate::engine::matrix_grid::MatrixCell;
        let mut ind = DeMarkPivots::with_period(1);
        // Bullish bar: close > open => X = 2*105 + 98 + 103 = 411
        // PP = 102.75, S1 = 100.5, R1 = 107.5
        ind.recompute(100.0, 105.0, 98.0, 103.0);
        let g = ind.levels_grid();
        assert_eq!(g.cols(), 1);
        assert_eq!(g.rows(), 3);
        let s1 = g.read_direction(MatrixCell::new(0, 0));
        let pp = g.read_direction(MatrixCell::new(1, 0));
        let r1 = g.read_direction(MatrixCell::new(2, 0));
        assert!(s1 < pp && pp < r1, "demark levels must ascend S1 {s1} < PP {pp} < R1 {r1}");
    }

    #[test]
    fn factory_exposes_demark_levels_vector() {
        use crate::engine::matrix_grid::MatrixCell;
        let mut f = IndicatorOrder::Demark(<<DeMarkPivots as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 100.0,
            high: 105.0,
            low: 98.0,
            close: 103.0,
            volume: 9999.0,
        });
        let g = f
            .grid(IndicatorOutputId::DemarkLevelsGrid)
            .expect("DeMark emits its 3-level vector");
        assert_eq!(g.rows(), 3);
        assert_eq!(g.cols(), 1);
        let s1 = g.read_direction(MatrixCell::new(0, 0));
        let pp = g.read_direction(MatrixCell::new(1, 0));
        let r1 = g.read_direction(MatrixCell::new(2, 0));
        assert!(s1 < pp && pp < r1, "levels ascend S1 {s1} < PP {pp} < R1 {r1}");

        // A scalar producer returns None.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::DemarkLevelsGrid).is_none());
    }

    #[test]
    fn demark_grid_row_labels() {
        use crate::engine::matrix_grid::{Label, MatrixCell};
        let mut f = IndicatorOrder::Demark(<<DeMarkPivots as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Bullish bar: close > open
        f.feed(0, MarketSample::Bar {
            open: 100.0, high: 105.0, low: 98.0, close: 103.0, volume: 9999.0,
        });
        let g = f.grid(IndicatorOutputId::DemarkLevelsGrid).unwrap();
        assert_eq!(g.row_label(0), Label::Name("S1"));
        assert_eq!(g.row_label(1), Label::Name("PP"));
        assert_eq!(g.row_label(2), Label::Name("R1"));
        let s1 = g.read_direction(MatrixCell::new(0, 0));
        let pp  = g.read_direction(MatrixCell::new(1, 0));
        let r1  = g.read_direction(MatrixCell::new(2, 0));
        assert!(s1 < pp && pp < r1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demark_pivots_basic() {
        let mut demark = DeMarkPivots::new();
        
        // Тестируем бычий бар (Close > Open)
        let levels = demark.recompute(100.0, 105.0, 98.0, 103.0);
        
        // X = (2 × 105) + 98 + 103 = 210 + 98 + 103 = 411
        // PP = 411 / 4 = 102.75
        // S1 = (411 / 2) - 105 = 205.5 - 105 = 100.5
        // R1 = (411 / 2) - 98 = 205.5 - 98 = 107.5
        
        assert!((levels.x_value - 411.0).abs() < 0.001);
        assert!((levels.pivot - 102.75).abs() < 0.001);
        assert!((levels.support_1 - 100.5).abs() < 0.001);
        assert!((levels.resistance_1 - 107.5).abs() < 0.001);
    }
    
    #[test]
    fn test_demark_pivots_bearish() {
        let mut demark = DeMarkPivots::new();
        
        // Тестируем медвежий бар (Close < Open)
        let levels = demark.recompute(103.0, 105.0, 98.0, 100.0);
        
        // X = 105 + (2 × 98) + 100 = 105 + 196 + 100 = 401
        // PP = 401 / 4 = 100.25
        // S1 = (401 / 2) - 105 = 200.5 - 105 = 95.5
        // R1 = (401 / 2) - 98 = 200.5 - 98 = 102.5
        
        assert!((levels.x_value - 401.0).abs() < 0.001);
        assert!((levels.pivot - 100.25).abs() < 0.001);
        assert!((levels.support_1 - 95.5).abs() < 0.001);
        assert!((levels.resistance_1 - 102.5).abs() < 0.001);
    }
    
    #[test]
    fn test_demark_pivots_neutral() {
        let mut demark = DeMarkPivots::new();
        
        // Тестируем нейтральный бар (Close = Open)
        let levels = demark.recompute(101.0, 105.0, 98.0, 101.0);
        
        // X = 105 + 98 + (2 × 101) = 105 + 98 + 202 = 405
        // PP = 405 / 4 = 101.25
        // S1 = (405 / 2) - 105 = 202.5 - 105 = 97.5
        // R1 = (405 / 2) - 98 = 202.5 - 98 = 104.5
        
        assert!((levels.x_value - 405.0).abs() < 0.001);
        assert!((levels.pivot - 101.25).abs() < 0.001);
        assert!((levels.support_1 - 97.5).abs() < 0.001);
        assert!((levels.resistance_1 - 104.5).abs() < 0.001);
    }
    
    #[test]
    fn test_price_position() {
        let mut demark = DeMarkPivots::new();
        let _levels = demark.recompute(100.0, 105.0, 98.0, 103.0);
        
        // Тестируем позиции цены
        assert_eq!(demark.price_position(95.0), -1); // Ниже поддержки
        assert_eq!(demark.price_position(104.0), 0);  // В диапазоне
        assert_eq!(demark.price_position(110.0), 1);  // Выше сопротивления
    }
    
    #[test]
    fn test_trading_signals() {
        let mut demark = DeMarkPivots::new();
        let _levels = demark.recompute(100.0, 105.0, 98.0, 103.0);
        
        // Тестируем сигналы
        assert_eq!(demark.trading_signal(108.0, 104.0), 1);  // Пробой вверх
        assert_eq!(demark.trading_signal(99.0, 104.0), -1); // Пробой вниз
        assert_eq!(demark.trading_signal(104.0, 103.0), 0); // Нет сигнала
    }
} 






















