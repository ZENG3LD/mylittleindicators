//! Floor Trader Pivots - классические уровни пивот трейдеров
//! Основаны на данных предыдущего периода (обычно дневных данных)
//! PP = (High + Low + Close) / 3
//! R1 = 2 * PP - Low
//! R2 = PP + (High - Low)
//! R3 = High + 2 * (PP - Low)
//! S1 = 2 * PP - High
//! S2 = PP - (High - Low)
//! S3 = Low - 2 * (High - PP)

use std::collections::HashMap;

use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Row labels for the 7-level Floor Trader pivot ladder (ascending: S3 … R3).
const FLOOR_RUNGS: &[&'static str] = &["S3", "S2", "S1", "PP", "R1", "R2", "R3"];

/// Уровни Floor Trader пивот трейдеров
#[derive(Debug, Clone)]
pub struct FloorTraderPivotLevels {
    pub pivot_point: f64,
    pub resistance_1: f64,
    pub resistance_2: f64,
    pub resistance_3: f64,
    pub support_1: f64,
    pub support_2: f64,
    pub support_3: f64,
}

impl FloorTraderPivotLevels {
    /// Создать новые уровни пивот
    pub fn new(high: f64, low: f64, close: f64) -> Self {
        let pivot_point = (high + low + close) / 3.0;
        let range = high - low;
        
        Self {
            pivot_point,
            resistance_1: 2.0 * pivot_point - low,
            resistance_2: pivot_point + range,
            resistance_3: high + 2.0 * (pivot_point - low),
            support_1: 2.0 * pivot_point - high,
            support_2: pivot_point - range,
            support_3: low - 2.0 * (high - pivot_point),
        }
    }
    
    /// Получить все уровни как вектор
    pub fn all_levels(&self) -> Vec<f64> {
        vec![
            self.support_3,
            self.support_2,
            self.support_1,
            self.pivot_point,
            self.resistance_1,
            self.resistance_2,
            self.resistance_3,
        ]
    }
    
    /// Получить ближайший уровень к заданной цене
    pub fn nearest_level(&self, price: f64) -> f64 {
        self.all_levels()
            .into_iter()
            .min_by(|a, b| (a - price).abs().partial_cmp(&(b - price).abs()).unwrap())
            .unwrap_or(self.pivot_point)
    }
    
    /// Получить расстояние до ближайшего уровня
    pub fn distance_to_nearest(&self, price: f64) -> f64 {
        let nearest = self.nearest_level(price);
        (price - nearest).abs()
    }
    
    /// Определить, между какими уровнями находится цена
    pub fn price_zone(&self, price: f64) -> &'static str {
        match price {
            p if p >= self.resistance_3 => "Above R3",
            p if p >= self.resistance_2 => "R2-R3",
            p if p >= self.resistance_1 => "R1-R2",
            p if p >= self.pivot_point => "PP-R1",
            p if p >= self.support_1 => "S1-PP",
            p if p >= self.support_2 => "S2-S1",
            p if p >= self.support_3 => "S3-S2",
            _ => "Below S3",
        }
    }
}

/// Floor Trader Pivots индикатор
#[derive(Debug, Clone)]
pub struct FloorTraderPivots {
    // Текущие уровни пивот
    current_levels: Option<FloorTraderPivotLevels>,
    
    // Период для обновления (количество баров)
    update_period: usize,
    
    // Буферы для расчетов
    period_high: f64,
    period_low: f64,
    period_close: f64,
    
    // Счетчики
    bars_in_period: usize,
    
    // История уровней
    levels_history: Vec<FloorTraderPivotLevels>,
    
    // Статистика взаимодействий с уровнями
    level_touches: HashMap<String, u32>,
    
    // Состояние
    is_ready: bool,

    /// The 7 levels as a fixed `7 × 1` price VECTOR (row 0 = S3 … row 3 = PP … row 6 = R3).
    /// Read via `levels_grid` / `ContractFactory::grid`.
    grid: MatrixGrid,
}

impl Default for FloorTraderPivots {
    /// Factory default: `with_period(24)` — daily candles (period = `unwrap_or(24).max(1)`).
    fn default() -> Self {
        Self::with_period(24)
    }
}

impl FloorTraderPivots {
    /// Создать новый индикатор с дневными пивотами (по умолчанию)
    pub fn new() -> Self {
        Self::with_period(24) // 24 часа для дневных пивотов
    }
    
    /// Создать новый индикатор с заданным периодом
    pub fn with_period(period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        
        Self {
            current_levels: None,
            update_period: period,
            period_high: f64::NEG_INFINITY,
            period_low: f64::INFINITY,
            period_close: 0.0,
            bars_in_period: 0,
            levels_history: Vec::with_capacity(100),
            level_touches: HashMap::new(),
            is_ready: false,
            grid: MatrixGrid::new(7, 1, false)
                .with_labels(AxisLabels::Named(FLOOR_RUNGS), AxisLabels::Bars),
        }
    }

    /// Обновить индикатор новым баром
    fn recompute(&mut self, high: f64, low: f64, close: f64) -> Option<FloorTraderPivotLevels> {
        // Обновляем данные для текущего периода
        self.period_high = self.period_high.max(high);
        self.period_low = self.period_low.min(low);
        self.period_close = close;
        self.bars_in_period += 1;
        
        // Проверяем касания уровней
        if let Some(levels) = self.current_levels.clone() {
            self.check_level_touches(high, low, &levels);
        }
        
        // Проверяем, нужно ли обновить пивот уровни
        if self.bars_in_period >= self.update_period {
            // Рассчитываем новые уровни
            let new_levels = FloorTraderPivotLevels::new(self.period_high, self.period_low, self.period_close);

            // Snapshot the 7 levels into the fixed price vector (row 0 = S3 … row 6 = R3).
            self.grid.reset();
            for (row, &lvl) in new_levels.all_levels().iter().enumerate() {
                self.grid.set_direction(MatrixCell::new(row as u16, 0), lvl);
            }

            // Сохраняем в истории
            if self.levels_history.len() >= 100 {
                self.levels_history.remove(0);
            }
            self.levels_history.push(new_levels.clone());
            
            // Обновляем текущие уровни
            self.current_levels = Some(new_levels);
            
            // Сбрасываем данные периода
            self.period_high = f64::NEG_INFINITY;
            self.period_low = f64::INFINITY;
            self.period_close = 0.0;
            self.bars_in_period = 0;
            
            self.is_ready = true;
        }
        
        self.current_levels.clone()
    }
    
    /// Проверить касания уровней
    fn check_level_touches(&mut self, high: f64, low: f64, levels: &FloorTraderPivotLevels) {
        let tolerance = 0.001; // Допуск для касания уровня
        
        let level_names = vec![
            ("S3", levels.support_3),
            ("S2", levels.support_2),
            ("S1", levels.support_1),
            ("PP", levels.pivot_point),
            ("R1", levels.resistance_1),
            ("R2", levels.resistance_2),
            ("R3", levels.resistance_3),
        ];
        
        for (name, level) in level_names {
            if low <= level * (1.0 + tolerance) && high >= level * (1.0 - tolerance) {
                *self.level_touches.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    /// Получить текущие уровни пивот
    pub fn current_levels(&self) -> Option<&FloorTraderPivotLevels> {
        self.current_levels.as_ref()
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// The 7 levels as a fixed `7 × 1` price vector — the matrix output behind
    /// `IndicatorOutputId::FloorpivotLevelsGrid`.
    pub fn levels_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    /// Получить период обновления
    pub fn period(&self) -> usize {
        self.update_period
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.current_levels = None;
        self.period_high = f64::NEG_INFINITY;
        self.period_low = f64::INFINITY;
        self.period_close = 0.0;
        self.bars_in_period = 0;
        self.levels_history.clear();
        self.level_touches.clear();
        self.is_ready = false;
        self.grid.reset();
    }
    
    /// Получить торговый сигнал на основе взаимодействия с уровнями
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        if let Some(levels) = &self.current_levels {
            let tolerance = 0.002; // 0.2% допуск
            
            // Покупка при отскоке от поддержки
            if (current_price - levels.support_1).abs() <= levels.support_1 * tolerance ||
               (current_price - levels.support_2).abs() <= levels.support_2 * tolerance ||
               (current_price - levels.support_3).abs() <= levels.support_3 * tolerance {
                return 1;
            }
            
            // Продажа при отскоке от сопротивления
            if (current_price - levels.resistance_1).abs() <= levels.resistance_1 * tolerance ||
               (current_price - levels.resistance_2).abs() <= levels.resistance_2 * tolerance ||
               (current_price - levels.resistance_3).abs() <= levels.resistance_3 * tolerance {
                return -1;
            }
            
            // Покупка при пробое сопротивления
            if current_price > levels.resistance_1 * 1.001 {
                return 1;
            }
            
            // Продажа при пробое поддержки
            if current_price < levels.support_1 * 0.999 {
                return -1;
            }
        }
        
        0
    }
    
    /// Получить силу уровня (количество касаний)
    pub fn level_strength(&self, level_name: &str) -> u32 {
        self.level_touches.get(level_name).copied().unwrap_or(0)
    }
    
    /// Получить все касания уровней
    pub fn all_level_touches(&self) -> &HashMap<String, u32> {
        &self.level_touches
    }
    
    /// Получить самый сильный уровень
    pub fn strongest_level(&self) -> Option<String> {
        self.level_touches
            .iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(name, _)| name.clone())
    }
    
    /// Получить историю уровней
    pub fn levels_history(&self) -> &[FloorTraderPivotLevels] {
        &self.levels_history
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        if let Some(levels) = &self.current_levels {
            format!(
                "FTP - PP: {:.2}, Zone: {}, Nearest: {:.2}, Distance: {:.2}",
                levels.pivot_point,
                levels.price_zone(current_price),
                levels.nearest_level(current_price),
                levels.distance_to_nearest(current_price)
            )
        } else {
            "FTP - Not Ready".to_string()
        }
    }

    #[inline]
    pub fn value(&self) -> f64 {
        
            self.current_levels
                .as_ref()
                .map(|l| l.pivot_point)
                .unwrap_or(0.0)
        
    }

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
    Color, Cost, Family, Indicator, Output, Param,
    Render, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`FloorTraderPivots`] — update period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FloorTraderPivotsConfig {
    pub period: Param<usize>,
}

impl Indicator for FloorTraderPivots {
    const ID: IndicatorId = IndicatorId::Floorpivot;
    /// Not a pluggable family member — floor-trader pivot level producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C fields.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1) amortized when period=24; accumulates period H/L/C in scalars, no window buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Vec, 100)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::Floorpivot),
        Output::matrix(IndicatorOutputId::FloorpivotLevelsGrid, ValueDomain::Price),
    ];
    type Config = FloorTraderPivotsConfig;
    type Runtime = FloorTraderPivots;

    fn create(cfg: FloorTraderPivotsConfig) -> FloorTraderPivots {
        FloorTraderPivots::with_period(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for FloorTraderPivotsConfig {
    fn defaults() -> Self {
        FloorTraderPivotsConfig { period: Param::Solo(24) }
    }
    fn machine_defaults() -> Self {
        // period: bars-per-pivot-period (update frequency). Treated as Class A period;
        // auto range(2,4048,1) covers the sweep space. `create` calls `.max(1)` so
        // the floor-2 from auto is safe.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for FloorTraderPivots {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Floorpivot, "Floor Pivot", Color::hex(0x2196F3))
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
    fn factory_feeds_resolved_floorpivot() {
        use crate::contract::Param;
        let cfg = FloorTraderPivotsConfig { period: Param::Solo(5) };
        let mut f = IndicatorOrder::Floorpivot(cfg).build_solo().unwrap();
        for i in 1..=6 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 3.0,
                low: base - 3.0,
                close: base,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v > 90.0 && v < 120.0, "floor pivot out of range: {v}");
    }

    #[test]
    fn floor_direct_levels_grid_ascending() {
        use crate::engine::matrix_grid::MatrixCell;
        let mut ind = FloorTraderPivots::with_period(1);
        // PP = (105 + 95 + 100) / 3 = 100
        ind.recompute(105.0, 95.0, 100.0);
        let g = ind.levels_grid();
        assert_eq!(g.cols(), 1);
        assert_eq!(g.rows(), 7);
        let s3 = g.read_direction(MatrixCell::new(0, 0));
        let pp = g.read_direction(MatrixCell::new(3, 0));
        let r3 = g.read_direction(MatrixCell::new(6, 0));
        assert!(s3 < pp && pp < r3, "floor levels must ascend S3 {s3} < PP {pp} < R3 {r3}");
    }

    #[test]
    fn factory_exposes_floor_levels_vector() {
        use crate::engine::matrix_grid::MatrixCell;
        use crate::contract::Param;
        let cfg = FloorTraderPivotsConfig { period: Param::Solo(5) };
        let mut f = IndicatorOrder::Floorpivot(cfg).build_solo().unwrap();
        for i in 1..=6 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 3.0,
                low: base - 3.0,
                close: base,
                volume: 9999.0,
            });
        }
        let g = f
            .grid(IndicatorOutputId::FloorpivotLevelsGrid)
            .expect("FloorTrader emits its 7-level vector");
        assert_eq!(g.rows(), 7);
        assert_eq!(g.cols(), 1);
        let s3 = g.read_direction(MatrixCell::new(0, 0));
        let pp = g.read_direction(MatrixCell::new(3, 0));
        let r3 = g.read_direction(MatrixCell::new(6, 0));
        assert!(s3 < pp && pp < r3, "levels ascend S3 {s3} < PP {pp} < R3 {r3}");

        // A scalar producer returns None.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::FloorpivotLevelsGrid).is_none());
    }

    #[test]
    fn floor_grid_row_labels() {
        use crate::engine::matrix_grid::{Label, MatrixCell};
        use crate::contract::Param;
        let cfg = FloorTraderPivotsConfig { period: Param::Solo(5) };
        let mut f = IndicatorOrder::Floorpivot(cfg).build_solo().unwrap();
        for i in 1..=6 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: base + 3.0, low: base - 3.0, close: base, volume: 9999.0,
            });
        }
        let g = f.grid(IndicatorOutputId::FloorpivotLevelsGrid).unwrap();
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
    fn test_floor_trader_pivots_creation() {
        let ftp = FloorTraderPivots::new();
        assert!(!ftp.is_ready());
        assert!(ftp.current_levels().is_none());
    }

    #[test]
    fn test_floor_trader_pivot_levels() {
        let levels = FloorTraderPivotLevels::new(105.0, 95.0, 100.0);
        // PP = (105 + 95 + 100) / 3 = 100
        assert!((levels.pivot_point - 100.0).abs() < 0.001);
        // R1 = 2 * 100 - 95 = 105
        assert!((levels.resistance_1 - 105.0).abs() < 0.001);
        // S1 = 2 * 100 - 105 = 95
        assert!((levels.support_1 - 95.0).abs() < 0.001);
    }

    #[test]
    fn test_floor_trader_pivots_warmup() {
        let mut ftp = FloorTraderPivots::with_period(5);
        for i in 0..6 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ftp.recompute(price + 1.0, price - 1.0, price);
        }
        assert!(ftp.is_ready());
    }

    #[test]
    fn test_floor_trader_pivots_reset() {
        let mut ftp = FloorTraderPivots::with_period(5);
        for _i in 0..6 {
            ftp.recompute(101.0, 99.0, 100.0);
        }
        ftp.reset();
        assert!(!ftp.is_ready());
        assert!(ftp.current_levels().is_none());
    }
}






















