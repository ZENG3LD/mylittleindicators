//! Volume Profile Indicator
//! Анализирует распределение объема по ценовым уровням в рамках сессии
//! НЕ скользящий! Сбрасывается каждую сессию/период

use crate::types::Bar;

/// Ценовой уровень с объемом
#[derive(Debug, Clone, Copy)]
pub struct PriceLevel {
    pub price: f64,
    pub volume: f64,
}

/// Volume Profile индикатор (сессионный)
#[derive(Debug, Clone)]
pub struct VolumeProfile {
    /// Ценовые уровни (фиксированный размер)
    levels: Vec<PriceLevel>,
    /// Размер тика для группировки цен
    tick_size: f64,
    /// Общий объем за сессию
    total_volume: f64,
    /// Время начала сессии
    session_start_time: i64,
    /// Длительность сессии в секундах
    session_duration: i64,
    /// Количество обработанных баров в текущей сессии
    bars_count: usize,
    /// Флаг готовности
    ready: bool,
    /// Множитель для конвертации цены в индекс
    price_multiplier: f64,
    /// Volume-by-price histogram as a `VP_WINDOW × 1` VECTOR (row = price bucket centered on the
    /// POC, value = cumulative volume). Read via `profile_grid` / `ContractFactory::matrix_grid`.
    grid: MatrixGrid,
}

impl VolumeProfile {
    /// Создать новый Volume Profile
    /// tick_size - размер тика для группировки
    /// session_duration - длительность сессии в секундах (0 = без автосброса)
    pub fn new(tick_size: f64, session_duration: i64) -> Self {
        let price_multiplier = 1.0 / tick_size;
        Self {
            levels: Vec::with_capacity(1024),
            tick_size,
            total_volume: 0.0,
            session_start_time: 0,
            session_duration,
            bars_count: 0,
            ready: false,
            price_multiplier,
            grid: MatrixGrid::new(VP_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }
    
    /// Обновить профиль новым Bar
    pub fn update(&mut self, volume_bar: &Bar) -> bool {
        // Проверяем нужно ли начать новую сессию
        if self.should_reset_session(volume_bar.time) {
            self.reset_session(volume_bar.time);
        }
        
        if volume_bar.volume <= 0.0 {
            return false;
        }
        
        // Распределяем объем по ценовым уровням
        self.distribute_volume(volume_bar);
        
        self.total_volume += volume_bar.volume;
        self.bars_count += 1;
        self.ready = true;
        self.snapshot_grid();

        true
    }
    
    /// Проверить нужно ли сбросить сессию
    fn should_reset_session(&self, current_time: i64) -> bool {
        if self.session_duration <= 0 {
            return false; // Без автосброса
        }
        
        if self.session_start_time == 0 {
            return true; // Первая сессия
        }
        
        current_time >= self.session_start_time + self.session_duration
    }
    
    /// Сбросить сессию
    fn reset_session(&mut self, start_time: i64) {
        self.levels.clear();
        self.total_volume = 0.0;
        self.session_start_time = start_time;
        self.bars_count = 0;
        self.ready = false;
    }
    
    /// Распределить объем по ценовым уровням
    fn distribute_volume(&mut self, volume_bar: &Bar) {
        // Простая модель: основной объем на OHLC точках
        let ohlc_volume = volume_bar.volume * 0.25; // 25% на каждую точку
        
        self.add_volume_at_price(volume_bar.open, ohlc_volume);
        self.add_volume_at_price(volume_bar.high, ohlc_volume);
        self.add_volume_at_price(volume_bar.low, ohlc_volume);
        self.add_volume_at_price(volume_bar.close, ohlc_volume);
    }
    
    /// Добавить объем на ценовом уровне
    fn add_volume_at_price(&mut self, price: f64, volume: f64) {
        let rounded_price = self.round_to_tick(price);
        
        // Ищем существующий уровень
        if let Some(level) = self.levels.iter_mut().find(|l| (l.price - rounded_price).abs() < self.tick_size * 0.5) {
            level.volume += volume;
        } else {
            // Добавляем новый уровень
            self.levels.push(PriceLevel {
                price: rounded_price,
                volume,
            });
        }
    }
    
    /// Округлить цену до тика
    fn round_to_tick(&self, price: f64) -> f64 {
        // Используем price_multiplier для лучшей производительности (умножение быстрее деления)
        (price * self.price_multiplier).round() / self.price_multiplier
    }

    /// Получить POC (Point of Control) - уровень с максимальным объемом
    pub fn get_poc(&self) -> Option<PriceLevel> {
        self.levels.iter()
            .max_by(|a, b| a.volume.partial_cmp(&b.volume).unwrap())
            .copied()
    }
    
    /// Получить объем на определенном ценовом уровне
    pub fn volume_at_price(&self, price: f64) -> f64 {
        let rounded_price = self.round_to_tick(price);
        
        self.levels.iter()
            .find(|l| (l.price - rounded_price).abs() < self.tick_size * 0.5)
            .map(|l| l.volume)
            .unwrap_or(0.0)
    }
    
    /// Получить топ N ценовых уровней по объему
    pub fn top_volume_levels(&self, n: usize) -> Vec<PriceLevel> {
        let mut levels = self.levels.to_vec();
        levels.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap());
        levels.truncate(n);
        levels
    }
    
    /// Получить все уровни
    pub fn all_levels(&self) -> &[PriceLevel] {
        &self.levels
    }
    
    /// Получить уровни в ценовом диапазоне
    pub fn levels_in_range(&self, min_price: f64, max_price: f64) -> Vec<PriceLevel> {
        self.levels.iter()
            .filter(|l| l.price >= min_price && l.price <= max_price)
            .copied()
            .collect()
    }
    
    /// Получить общий объем сессии
    pub fn total_volume(&self) -> f64 {
        self.total_volume
    }
    
    /// Получить количество ценовых уровней
    pub fn levels_count(&self) -> usize {
        self.levels.len()
    }
    
    /// Получить ценовой диапазон профиля
    pub fn price_range(&self) -> Option<(f64, f64)> {
        if self.levels.is_empty() {
            return None;
        }
        
        let min_price = self.levels.iter().map(|l| l.price).fold(f64::INFINITY, f64::min);
        let max_price = self.levels.iter().map(|l| l.price).fold(f64::NEG_INFINITY, f64::max);
        
        Some((min_price, max_price))
    }
    
    /// Получить время сессии
    pub fn session_info(&self) -> (i64, i64, usize) {
        (self.session_start_time, self.session_duration, self.bars_count)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    
    /// Принудительно сбросить профиль
    pub fn reset(&mut self) {
        self.levels.clear();
        self.total_volume = 0.0;
        self.session_start_time = 0;
        self.bars_count = 0;
        self.ready = false;
        self.grid.reset();
    }

    /// Получить размер тика
    pub fn get_tick_size(&self) -> f64 {
        self.tick_size
    }

    /// Установить новый размер тика (пересчитывает price_multiplier)
    pub fn set_tick_size(&mut self, new_tick_size: f64) {
        assert!(new_tick_size > 0.0, "Tick size must be positive");
        self.tick_size = new_tick_size;
        self.price_multiplier = 1.0 / new_tick_size;
        // Очищаем уровни, так как они не будут соответствовать новому размеру тика
        self.reset();
    }

    /// Получить множитель цены
    pub fn get_price_multiplier(&self) -> f64 {
        self.price_multiplier
    }

    /// Snapshot the current volume-by-price histogram into the `VP_WINDOW × 1` grid centered on
    /// the POC bucket. Called each time `update` transitions `ready` to true (or while ready).
    fn snapshot_grid(&mut self) {
        let poc_price = match self.get_poc() {
            Some(p) => p.price,
            None => return,
        };
        let bs = self.tick_size;
        let poc_key = (poc_price / bs).round() as i64;

        self.grid.reset();
        let half = (VP_WINDOW / 2) as i64;
        let mut row_prices = Vec::with_capacity(VP_WINDOW as usize);
        for row in 0..VP_WINDOW {
            let key = poc_key + (row as i64 - half);
            let price = key as f64 * bs;
            row_prices.push(price);
            // Sum volumes across all levels that round to this bucket key.
            let vol: f64 = self.levels.iter()
                .filter(|l| (l.price / bs).round() as i64 == key)
                .map(|l| l.volume)
                .sum();
            if vol > 0.0 {
                self.grid.set_direction(MatrixCell::new(row, 0), vol);
            }
        }
        self.grid.set_row_values(&row_prices);
    }

    /// The volume-by-price profile as a `VP_WINDOW × 1` vector — the matrix output behind
    /// `IndicatorOutputId::VprofileProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }

    /// Получить длительность сессии
    pub fn get_session_duration(&self) -> i64 {
        self.session_duration
    }

    /// Установить новую длительность сессии
    pub fn set_session_duration(&mut self, new_duration: i64) {
        self.session_duration = new_duration;
    }

    /// Получить полную конфигурацию
    pub fn get_config(&self) -> VolumeProfileConfig {
        VolumeProfileConfig {
            tick_size: Param::Solo(self.tick_size),
            session_duration: Param::Solo(self.session_duration),
        }
    }

    /// Установить новую конфигурацию
    pub fn set_config(&mut self, config: VolumeProfileConfig) {
        self.tick_size = config.tick_size.resolved().max(1e-9);
        self.session_duration = config.session_duration.resolved();
        self.price_multiplier = 1.0 / self.tick_size;
        self.reset();
    }

    /// Получить подробную статистику профиля
    pub fn get_stats(&self) -> VolumeProfileStats {
        let poc = self.get_poc();
        let range = self.price_range();
        let avg_volume = if !self.levels.is_empty() {
            self.total_volume / self.levels.len() as f64
        } else {
            0.0
        };

        VolumeProfileStats {
            total_volume: self.total_volume,
            levels_count: self.levels.len(),
            price_range: range,
            average_volume_per_level: avg_volume,
            poc,
            session_bars_count: self.bars_count,
            session_start_time: self.session_start_time,
        }
    }

    /// Получить уровни поддержки и сопротивления
    pub fn get_support_resistance_levels(&self, min_volume_threshold: f64) -> Vec<PriceLevel> {
        self.levels.iter()
            .filter(|level| level.volume >= min_volume_threshold)
            .copied()
            .collect()
    }

    /// Получить объемный дисбаланс между покупками и продажами (приблизительно)
    pub fn get_volume_imbalance(&self) -> f64 {
        if let Some(poc) = self.get_poc() {
            // Простая эвристика: объем выше POC vs ниже POC
            let above_poc: f64 = self.levels.iter()
                .filter(|level| level.price > poc.price)
                .map(|level| level.volume)
                .sum();
            let below_poc: f64 = self.levels.iter()
                .filter(|level| level.price < poc.price)
                .map(|level| level.volume)
                .sum();
            
            if above_poc + below_poc > 0.0 {
                (above_poc - below_poc) / (above_poc + below_poc)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

}

/// Статистика Volume Profile
#[derive(Debug, Clone)]
pub struct VolumeProfileStats {
    pub total_volume: f64,
    pub levels_count: usize,
    pub price_range: Option<(f64, f64)>,
    pub average_volume_per_level: f64,
    pub poc: Option<PriceLevel>, // Point of Control
    pub session_bars_count: usize,
    pub session_start_time: i64,
}

use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::Param;

/// Fixed price-window (rows) of the exposed volume-profile VECTOR, centered on the POC bucket.
const VP_WINDOW: u16 = 64;

/// Конфигурация Volume Profile
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolumeProfileConfig {
    pub tick_size: Param<f64>,
    pub session_duration: Param<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Bar;

    #[test]
    fn test_volume_profile_basic() {
        let mut profile = VolumeProfile::new(0.01, 0); // Без автосброса
        
        let bar = Bar {
            time: 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        };
        // Bar is already the correct type
        
        let updated = profile.update(&bar);
        
        assert!(updated);
        assert!(profile.is_ready());
        assert_eq!(profile.total_volume(), 1000.0);
        assert!(profile.levels_count() > 0);
    }
    
    #[test]
    fn test_poc() {
        let mut profile = VolumeProfile::new(0.01, 0);
        
        // Добавляем бар
        let bar = Bar {
            time: 1000,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 1000.0,
        };
        // Bar is already the correct type
        profile.update(&bar);
        
        let poc = profile.get_poc();
        assert!(poc.is_some());
        
        let poc_level = poc.unwrap();
        assert_eq!(poc_level.price, 100.0);
        assert!(poc_level.volume > 0.0);
    }
    
    #[test]
    fn row_label_at_center_is_poc_price() {
        use crate::engine::matrix_grid::Label;
        let tick_size = 1.0_f64;
        let mut vp = VolumeProfile::new(tick_size, 0);
        // All OHLC points at 100.0 → bucket key = round(100.0 / 1.0) = 100; price = 100.0 * 1.0 = 100.0
        let dominant = Bar { time: 1, open: 100.0, high: 100.0, low: 100.0, close: 100.0, volume: 1000.0 };
        vp.update(&dominant);
        let center = VP_WINDOW / 2;
        let label = vp.profile_grid().row_label(center);
        let expected = 100.0_f64;
        if let Label::Value(p) = label {
            assert!((p - expected).abs() < tick_size, "row_label at center = {p}, expected ≈ {expected}");
        } else {
            panic!("expected Label::Value, got {:?}", label);
        }
    }

    #[test]
    fn profile_vector_captures_volume_around_poc() {
        let mut vp = VolumeProfile::new(1.0, 0);
        // Dominant bar at price 100 (all OHLC points land in the same bucket).
        let dominant = Bar { time: 1, open: 100.0, high: 100.0, low: 100.0, close: 100.0, volume: 1000.0 };
        vp.update(&dominant);
        // Noise bars at different prices.
        for price in [101.0_f64, 102.0, 103.0, 104.0] {
            let bar = Bar { time: 2, open: price, high: price, low: price, close: price, volume: 100.0 };
            vp.update(&bar);
        }
        let g = vp.profile_grid();
        assert_eq!(g.cols(), 1, "a 1-D volume-by-price vector");
        assert_eq!(g.rows(), 64, "64 price rows");
        // POC is at price 100 — should sit at the center row with dominant volume.
        let center = VP_WINDOW / 2;
        assert!(
            g.read_direction(MatrixCell::new(center, 0)) >= 1000.0,
            "POC volume at the center row"
        );
    }

    #[test]
    fn test_session_reset() {
        let mut profile = VolumeProfile::new(0.01, 1000); // Сессия 1000 секунд
        
        // Первый бар
        let bar1 = Bar {
            time: 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        };
        // Bar is already the correct type
        profile.update(&bar1);
        
        let first_volume = profile.total_volume();
        
        // Бар через 2000 секунд - новая сессия
        let bar2 = Bar {
            time: 3000,
            open: 102.0,
            high: 103.0,
            low: 101.0,
            close: 102.5,
            volume: 500.0,
        };
        // Bar is already the correct type
        profile.update(&bar2);
        
        // Объем должен сброситься
        assert_eq!(profile.total_volume(), 500.0);
        assert_ne!(first_volume, profile.total_volume());
    }
}

impl Default for VolumeProfile {
    fn default() -> Self {
        Self::new(0.01, 0)
    }
}

// ── BarTimeFull inherent bridge ───────────────────────────────────────────────

impl VolumeProfile {
    /// `(ts, &[O,H,L,C,V])` entry point — the `Fields +time` macro arm. The wall-clock
    /// (canonical MILLISECONDS → UTC seconds) drives the session reset; the OHLCV lanes build
    /// the legacy [`Bar`] and delegate to [`Self::update`] — reset logic preserved byte-for-byte.
    #[inline]
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) {
        let bar = Bar {
            time: ts_ms.div_euclid(1000),
            open: lanes[0],
            high: lanes[1],
            low: lanes[2],
            close: lanes[3],
            volume: lanes[4],
        };
        self.update(&bar);
    }

    /// Contract value accessor: POC price, or `0.0` before the first bar.
    pub fn value(&self) -> f64 {
        self.get_poc().map(|p| p.price).unwrap_or(0.0)
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Color, Cost, Family, Indicator, Output, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::contract::Render;
use crate::engine::stream_kind::StreamKind;

impl Indicator for VolumeProfile {
    const ID: IndicatorId = IndicatorId::Vprofile;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Five-field source: O/H/L/C/V (the OHLC points distributed across price buckets, plus
    /// volume). The orthogonal wall-clock (the `+time` flag) drives the session reset.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(n_levels) per bar (linear scan of the levels vec for bucket lookup).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::fixed(StoreKind::Vec, 1024)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::Vprofile),
        Output::matrix(IndicatorOutputId::VprofileProfileGrid, ValueDomain::Count),
    ];

    type Config = VolumeProfileConfig;
    type Runtime = VolumeProfile;

    fn create(cfg: VolumeProfileConfig) -> VolumeProfile {
        let tick = cfg.tick_size.resolved().max(1e-9);
        VolumeProfile::new(tick, cfg.session_duration.resolved().max(0))
    }
}

impl crate::contract::Config for VolumeProfileConfig {
    fn defaults() -> Self {
        VolumeProfileConfig {
            tick_size: Param::Solo(0.01),
            session_duration: Param::Solo(0),
        }
    }
    fn machine_defaults() -> Self {
        // tick_size (f64): Class I PIN — instrument-relative price tick, leave Solo (auto leaves f64 Solo)
        // session_duration (i64): Class M — sweep canonical session lengths in ms:
        //   1 h = 3 600 000, 4 h = 14 400 000, 8 h = 28 800 000, 24 h = 86 400 000
        let mut s = Self::machine_defaults_auto();
        s.session_duration = Param::many(vec![3_600_000i64, 14_400_000, 28_800_000, 86_400_000]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumeProfile {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vprofile, "POC", Color::hex(0xFF5722))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_timed_bar() {
        let mut f = IndicatorOrder::Vprofile(<<VolumeProfile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Feed two bars at different timestamps
        f.feed(1_699_833_600_000, MarketSample::Bar {
            open: 100.0, high: 101.0, low: 99.0, close: 100.5, volume: 1000.0,
        });
        f.feed(1_699_920_000_000, MarketSample::Bar {
            open: 101.0, high: 102.0, low: 100.0, close: 101.5, volume: 800.0,
        });
        let v = f.read(IndicatorOutputId::Vprofile);
        assert!(v.is_finite() && v > 0.0, "POC should be a positive finite price, got {v}");
    }

    #[test]
    fn factory_exposes_vprofile_vector() {
        let f = IndicatorOrder::Vprofile(
            <<VolumeProfile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let g = f
            .grid(IndicatorOutputId::VprofileProfileGrid)
            .expect("Vprofile emits the volume-profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::VprofileProfileGrid).is_none());
    }
}






















