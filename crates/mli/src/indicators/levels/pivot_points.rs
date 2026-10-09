//! Classic Pivot Points - классические пивот уровни для определения поддержки и сопротивления
//! Pivot Point (PP) = (High + Low + Close) / 3
//! Support 1 (S1) = (2 × PP) - High
//! Support 2 (S2) = PP - (High - Low)
//! Support 3 (S3) = Low - 2 × (High - PP)
//! Resistance 1 (R1) = (2 × PP) - Low
//! Resistance 2 (R2) = PP + (High - Low)
//! Resistance 3 (R3) = High + 2 × (PP - Low)

use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Row labels for the 7-level Classic Pivot ladder (ascending: S3 … R3).
const PIVOT_RUNGS: &[&'static str] = &["S3", "S2", "S1", "PP", "R1", "R2", "R3"];

/// Number of distinct price levels in the Classic Pivot ladder (S3, S2, S1, PP, R1, R2, R3).
const PIVOT_LEVELS: u16 = 7;


/// Уровни Classic Pivot Points
#[derive(Debug, Clone, Copy)]
pub struct ClassicPivotLevels {
    pub pivot: f64,       // Основной пивот уровень
    pub resistance_1: f64, // Первое сопротивление (R1)
    pub resistance_2: f64, // Второе сопротивление (R2) 
    pub resistance_3: f64, // Третье сопротивление (R3)
    pub support_1: f64,    // Первая поддержка (S1)
    pub support_2: f64,    // Вторая поддержка (S2)
    pub support_3: f64,    // Третья поддержка (S3)
}

impl ClassicPivotLevels {
    /// Создать пустые уровни
    pub fn empty() -> Self {
        Self {
            pivot: 0.0,
            resistance_1: 0.0,
            resistance_2: 0.0,
            resistance_3: 0.0,
            support_1: 0.0,
            support_2: 0.0,
            support_3: 0.0,
        }
    }
    
    /// Получить все уровни сопротивления отсортированные по возрастанию
    pub fn resistance_levels(&self) -> [f64; 3] {
        [self.resistance_1, self.resistance_2, self.resistance_3]
    }
    
    /// Получить все уровни поддержки отсортированные по убыванию
    pub fn support_levels(&self) -> [f64; 3] {
        [self.support_1, self.support_2, self.support_3]
    }
    
    /// Получить все уровни включая пивот
    pub fn all_levels(&self) -> [f64; 7] {
        [
            self.support_3,
            self.support_2, 
            self.support_1,
            self.pivot,
            self.resistance_1,
            self.resistance_2,
            self.resistance_3,
        ]
    }
}

/// Classic Pivot Points индикатор
#[derive(Debug, Clone)]
pub struct PivotPoints {
    // Текущие уровни
    current_levels: ClassicPivotLevels,

    // История уровней
    levels_history: Vec<ClassicPivotLevels>,

    // Буферы для расчета (для периодических обновлений)
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,

    // Настройки
    calculation_period: usize, // Период для расчета (1 = ежедневно, 7 = еженедельно и т.д.)
    bars_since_update: usize,

    // Состояние
    is_ready: bool,
    update_count: usize,

    /// The 7 classic levels as a fixed `7 × 1` price VECTOR (row 0 = S3 … row 3 = PP … row 6 = R3,
    /// value = the level's price). Read via `levels_grid` / `ContractFactory::grid`.
    grid: MatrixGrid,
}

impl Default for PivotPoints {
    /// Factory default: `with_period(1)` — daily update (period = `unwrap_or(1).max(1)`).
    fn default() -> Self {
        Self::with_period(1)
    }
}

impl PivotPoints {
    /// Создать новый индикатор Pivot Points (ежедневное обновление)
    pub fn new() -> Self {
        Self::with_period(1)
    }
    
    /// Создать новый индикатор с настраиваемым периодом обновления
    /// period = 1 (ежедневно), 7 (еженедельно), и т.д.
    pub fn with_period(calculation_period: usize) -> Self {
        assert!(calculation_period > 0, "Period must be greater than 0");
        
        Self {
            current_levels: ClassicPivotLevels::empty(),
            levels_history: Vec::with_capacity(100),
            highs: Vec::with_capacity(32),
            lows: Vec::with_capacity(32),
            closes: Vec::with_capacity(32),
            calculation_period,
            bars_since_update: 0,
            is_ready: false,
            update_count: 0,
            grid: MatrixGrid::new(PIVOT_LEVELS, 1, false)
                .with_labels(AxisLabels::Named(PIVOT_RUNGS), AxisLabels::Bars),
        }
    }

    /// Обновить индикатор новым баром
    fn recompute(&mut self, high: f64, low: f64, close: f64) -> ClassicPivotLevels {
        // Добавляем данные в буферы
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
        if self.highs.is_empty() {
            return;
        }
        
        // Находим максимум, минимум и последнее закрытие за период
        let period_high = self.highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let period_low = self.lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let period_close = self.closes[self.closes.len() - 1];
        
        // Рассчитываем пивот точку
        let pivot = (period_high + period_low + period_close) / 3.0;
        
        // Рассчитываем уровни сопротивления
        let resistance_1 = (2.0 * pivot) - period_low;
        let resistance_2 = pivot + (period_high - period_low);
        let resistance_3 = period_high + 2.0 * (pivot - period_low);
        
        // Рассчитываем уровни поддержки
        let support_1 = (2.0 * pivot) - period_high;
        let support_2 = pivot - (period_high - period_low);
        let support_3 = period_low - 2.0 * (period_high - pivot);
        
        // Создаем новые уровни
        let new_levels = ClassicPivotLevels {
            pivot,
            resistance_1,
            resistance_2,
            resistance_3,
            support_1,
            support_2,
            support_3,
        };
        
        // Populate the 7×1 price vector (row 0 = S3 … row 3 = PP … row 6 = R3, ascending).
        self.grid.reset();
        for (row, &lvl) in new_levels.all_levels().iter().enumerate() {
            self.grid.set_direction(MatrixCell::new(row as u16, 0), lvl);
        }

        // Сохраняем в историю
        if self.levels_history.len() >= 100 {
            self.levels_history.remove(0);
        }
        self.levels_history.push(self.current_levels);

        // Обновляем текущие уровни
        self.current_levels = new_levels;
        
        // Очищаем буферы для следующего периода
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        
        self.update_count += 1;
        self.is_ready = true;
    }
    
    /// Получить текущие уровни Pivot Points
    pub fn levels(&self) -> ClassicPivotLevels {
        self.current_levels
    }

    /// The 7 classic levels as a fixed `7 × 1` price vector — the matrix output behind
    /// `IndicatorOutputId::PivotLevelsGrid`. Row 0 = S3, row 3 = PP, row 6 = R3 (ascending).
    pub fn levels_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.current_levels = ClassicPivotLevels::empty();
        self.levels_history.clear();
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.bars_since_update = 0;
        self.is_ready = false;
        self.update_count = 0;
        self.grid.reset();
    }
    
    /// Определить ближайший уровень поддержки для текущей цены
    pub fn nearest_support(&self, current_price: f64) -> f64 {
        let supports = [
            self.current_levels.support_1,
            self.current_levels.support_2,
            self.current_levels.support_3,
        ];
        
        // Находим ближайший уровень поддержки ниже текущей цены
        supports
            .iter()
            .filter(|&&level| level < current_price)
            .fold(f64::NEG_INFINITY, |acc, &level| acc.max(level))
    }
    
    /// Определить ближайший уровень сопротивления для текущей цены
    pub fn nearest_resistance(&self, current_price: f64) -> f64 {
        let resistances = [
            self.current_levels.resistance_1,
            self.current_levels.resistance_2,
            self.current_levels.resistance_3,
        ];
        
        // Находим ближайший уровень сопротивления выше текущей цены
        resistances
            .iter()
            .filter(|&&level| level > current_price)
            .fold(f64::INFINITY, |acc, &level| acc.min(level))
    }
    
    /// Определить текущую позицию цены относительно пивота
    /// 1 = выше пивота (бычий), -1 = ниже пивота (медвежий), 0 = на пивоте
    pub fn price_position(&self, current_price: f64) -> i8 {
        let pivot = self.current_levels.pivot;
        let threshold = pivot * 0.001; // 0.1% порог
        
        if current_price > pivot + threshold {
            1  // Выше пивота
        } else if current_price < pivot - threshold {
            -1 // Ниже пивота
        } else {
            0  // На пивоте
        }
    }
    
    /// Получить торговый сигнал на основе пересечения уровней
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self, current_price: f64, prev_price: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Проверяем пересечение уровней снизу вверх (покупка)
        let levels = self.current_levels.all_levels();
        for &level in &levels {
            if prev_price <= level && current_price > level {
                // Пересечение снизу вверх - потенциальная покупка
                if level >= self.current_levels.pivot {
                    return 1; // Пересечение пивота или сопротивления вверх
                }
            }
        }
        
        // Проверяем пересечение уровней сверху вниз (продажа)
        for &level in &levels {
            if prev_price >= level && current_price < level {
                // Пересечение сверху вниз - потенциальная продажа
                if level <= self.current_levels.pivot {
                    return -1; // Пересечение пивота или поддержки вниз
                }
            }
        }
        
        0
    }
    
    /// Рассчитать расстояние до ближайшего уровня в процентах
    pub fn distance_to_nearest_level(&self, current_price: f64) -> f64 {
        if current_price.abs() < 1e-12 {
            return 0.0;
        }
        
        let levels = self.current_levels.all_levels();
        let nearest_distance = levels
            .iter()
            .map(|&level| (level - current_price).abs())
            .fold(f64::INFINITY, |acc, dist| acc.min(dist));
        
        (nearest_distance / current_price) * 100.0
    }
    
    /// Определить силу уровня на основе истории
    /// Возвращает количество раз, когда цена отскакивала от уровня
    pub fn level_strength(&self, level: f64, price_history: &[f64], tolerance_pct: f64) -> u32 {
        if price_history.len() < 2 {
            return 0;
        }
        
        let tolerance = level * (tolerance_pct / 100.0);
        let mut touches = 0;
        
        for i in 1..price_history.len() {
            let prev_price = price_history[i - 1];
            let current_price = price_history[i];
            
            // Проверяем касание уровня (цена приближается и отскакивает)
            let near_level = (current_price - level).abs() <= tolerance;
            let moving_away = (current_price - level).abs() > (prev_price - level).abs();
            
            if near_level && moving_away {
                touches += 1;
            }
        }
        
        touches
    }
    
    /// Получить историю уровней для анализа
    pub fn levels_history(&self) -> Vec<ClassicPivotLevels> {
        self.levels_history.iter().cloned().collect()
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self, current_price: f64) -> String {
        format!(
            "PP: {:.4}, Position: {}, Nearest S: {:.4}, Nearest R: {:.4}, Distance: {:.2}%",
            self.current_levels.pivot,
            match self.price_position(current_price) {
                1 => "Above",
                -1 => "Below",
                _ => "At Pivot"
            },
            self.nearest_support(current_price),
            self.nearest_resistance(current_price),
            self.distance_to_nearest_level(current_price)
        )
    }

    /// Named output getter: brace `r1`.
    #[inline]
    pub fn r1(&self) -> f64 { self.current_levels.resistance_1 }

    /// Named output getter: brace `s1`.
    #[inline]
    pub fn s1(&self) -> f64 { self.current_levels.support_1 }
    /// Named output getter: brace `pivot` (the central pivot point).
    pub fn point(&self) -> f64 { self.current_levels.pivot }

    // NOTE(coordinator): brace `value` in universe entry `Pivot { r1 s1 value }` semantically
    // denotes the PIVOT POINT (the centre level), not a generic "main value". The brace must be
    // renamed `pivot` in the contract_universe! manifest so a generated getter `fn pivot()`
    // can be added here. Skipped in this pass per spec.


    /// Feed resolved `[high, low, close]` lanes.
    pub fn feed(&mut self, lanes: &[f64]) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        self.recompute(high, low, close);
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, LineStyle, Output,
    Param, Render, RenderOutput, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity,
    ValueDomain,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`PivotPoints`] — update period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PivotPointsConfig {
    pub period: Param<usize>,
}

impl Indicator for PivotPoints {
    const ID: IndicatorId = IndicatorId::Pivot;
    /// Not a pluggable family member — a fixed-formula level producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C fields — classic pivot formula.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// Triple: R1, S1, pivot.
    /// O(1) per bar in the common case (period=1); O(period) when accumulating
    /// highs/lows over multiple bars. Three Vec buffers.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::PivotR1),
        Output::price(IndicatorOutputId::PivotS1),
        Output::price(IndicatorOutputId::PivotPoint),
        Output::matrix(IndicatorOutputId::PivotLevelsGrid, ValueDomain::Price),
    ];
    type Config = PivotPointsConfig;
    type Runtime = PivotPoints;

    fn create(cfg: PivotPointsConfig) -> PivotPoints {
        PivotPoints::with_period(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for PivotPointsConfig {
    fn defaults() -> Self {
        PivotPointsConfig { period: Param::Solo(1) }
    }
    fn machine_defaults() -> Self {
        // period: bars accumulated before recalculating pivot levels. Default=1 (every bar).
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


impl Render for PivotPoints {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(
                RenderOutput::line(IndicatorOutputId::PivotPoint, "Pivot", Color::hex(0x2196F3), 2.0),
            )
            .output(
                RenderOutput::line(IndicatorOutputId::PivotR1, "R1", Color::hex(0xF44336), 1.0)
                    .with_style(LineStyle::Dashed),
            )
            .output(
                RenderOutput::line(IndicatorOutputId::PivotS1, "S1", Color::hex(0x4CAF50), 1.0)
                    .with_style(LineStyle::Dashed),
            )
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
    fn factory_feeds_resolved_pivot() {
        let mut f = IndicatorOrder::Pivot(<<PivotPoints as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Config::defaults() → PivotPointsConfig { period: Param::Solo(1) }
        for i in 1..=5 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 5.0,
                low: base - 5.0,
                close: base,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // pivot = (H+L+C)/3 — must be in range
        let v = f.primary();
        assert!(v > 90.0 && v < 120.0, "pivot out of range: {v}");
    }

    #[test]
    fn factory_exposes_pivot_levels_vector() {
        use crate::engine::matrix_grid::MatrixCell;
        let cfg = PivotPointsConfig { period: crate::contract::Param::Solo(1) };
        let mut f = IndicatorOrder::Pivot(cfg).build_solo().unwrap();
        // Feed one bar — period=1 triggers a level update on every bar.
        f.feed(0, MarketSample::Bar {
            open: 9999.0,
            high: 110.0,
            low: 90.0,
            close: 100.0,
            volume: 9999.0,
        });
        assert!(f.is_ready());
        let g = f
            .grid(IndicatorOutputId::PivotLevelsGrid)
            .expect("PivotPoints emits its 7-level vector");
        assert_eq!(g.rows(), 7, "rows should be 7 (S3…R3)");
        assert_eq!(g.cols(), 1);
        // The vector is price-ordered: S3 (row 0) < PP (row 3) < R3 (row 6).
        let s3 = g.read_direction(MatrixCell::new(0, 0));
        let pp = g.read_direction(MatrixCell::new(3, 0));
        let r3 = g.read_direction(MatrixCell::new(6, 0));
        assert!(s3 < pp && pp < r3, "levels must ascend: S3 {s3} < PP {pp} < R3 {r3}");

        // A scalar-only producer returns None for this matrix output.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::PivotLevelsGrid).is_none());
    }

    #[test]
    fn pivot_grid_row_labels() {
        use crate::engine::matrix_grid::{Label, MatrixCell};
        let cfg = PivotPointsConfig { period: crate::contract::Param::Solo(1) };
        let mut f = IndicatorOrder::Pivot(cfg).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 110.0, low: 90.0, close: 100.0, volume: 9999.0,
        });
        let g = f.grid(IndicatorOutputId::PivotLevelsGrid).unwrap();
        assert_eq!(g.row_label(0), Label::Name("S3"));
        assert_eq!(g.row_label(3), Label::Name("PP"));
        assert_eq!(g.row_label(6), Label::Name("R3"));
        let s3 = g.read_direction(MatrixCell::new(0, 0));
        let pp  = g.read_direction(MatrixCell::new(3, 0));
        let r3  = g.read_direction(MatrixCell::new(6, 0));
        assert!(s3 < pp && pp < r3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pivot_points_creation() {
        let pp = PivotPoints::new();
        assert!(!pp.is_ready());
    }

    #[test]
    fn test_classic_pivot_levels() {
        let levels = ClassicPivotLevels::empty();
        assert_eq!(levels.pivot, 0.0);
        assert_eq!(levels.resistance_1, 0.0);
        assert_eq!(levels.support_1, 0.0);
    }

    #[test]
    fn test_pivot_points_update() {
        let mut pp = PivotPoints::new();
        let levels = pp.recompute(105.0, 95.0, 102.0);
        assert!(pp.is_ready());
        // PP = (105 + 95 + 102) / 3 = 100.67
        assert!((levels.pivot - 100.67).abs() < 0.1);
    }

    #[test]
    fn test_pivot_points_levels_order() {
        let mut pp = PivotPoints::new();
        let levels = pp.recompute(110.0, 90.0, 100.0);
        // Verify levels are in correct order
        assert!(levels.support_3 < levels.support_2);
        assert!(levels.support_2 < levels.support_1);
        assert!(levels.support_1 < levels.pivot);
        assert!(levels.pivot < levels.resistance_1);
        assert!(levels.resistance_1 < levels.resistance_2);
        assert!(levels.resistance_2 < levels.resistance_3);
    }

    #[test]
    fn test_pivot_points_reset() {
        let mut pp = PivotPoints::new();
        pp.recompute(105.0, 95.0, 102.0);
        pp.reset();
        assert!(!pp.is_ready());
    }

    #[test]
    fn levels_grid_ascending_with_pivot_in_middle() {
        use crate::engine::matrix_grid::MatrixCell;
        let mut pp = PivotPoints::new(); // period = 1
        pp.recompute(110.0, 90.0, 100.0);
        assert!(pp.is_ready());

        let g = pp.levels_grid();
        assert_eq!(g.cols(), 1, "grid must have exactly 1 column");
        assert_eq!(g.rows(), 7, "grid must have exactly 7 rows (S3, S2, S1, PP, R1, R2, R3)");

        // Rows are strictly ascending.
        for row in 1..7_u16 {
            let prev = g.read_direction(MatrixCell::new(row - 1, 0));
            let curr = g.read_direction(MatrixCell::new(row, 0));
            assert!(prev < curr, "row {} ({prev}) must be < row {} ({curr})", row - 1, row);
        }

        // PP is the middle row (row 3) and must lie between the extremes.
        let s3 = g.read_direction(MatrixCell::new(0, 0));
        let pp_val = g.read_direction(MatrixCell::new(3, 0));
        let r3 = g.read_direction(MatrixCell::new(6, 0));
        assert!(s3 < pp_val && pp_val < r3, "PP {pp_val} must be between S3 {s3} and R3 {r3}");
    }
}






















