//! MESA Adaptive Moving Average - адаптивная скользящая средняя от Джона Эхлерса
//!
//! MESA (Maximum Entropy Spectral Analysis) адаптивно изменяет период сглаживания
//! на основе анализа доминирующего цикла в данных.
//!
//! Основано на работе John Ehlers "MESA and Trading Market Cycles"
//!
//! Hilbert-стадии (fast/slow/period) используют фиксированные EMA по Эхлерсу;
//! выходное сглаживание — адаптивная EMA «на месте», alpha = 2/(period+1) с
//! доминирующим циклом в качестве периода.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;
use std::f64::consts::PI;

/// Результат MESA Adaptive MA
#[derive(Debug, Clone, Copy)]
pub struct MesaAdaptiveResult {
    pub value: f64,              // Адаптивное среднее
    pub period: f64,             // Текущий адаптивный период
    pub phase: f64,              // Фаза цикла
    pub i_component: f64,        // In-Phase компонент
    pub q_component: f64,        // Quadrature компонент
    pub cycle_strength: f64,     // Сила цикла (0.0 до 1.0)
}

impl MesaAdaptiveResult {
    pub fn empty() -> Self {
        Self {
            value: 0.0,
            period: 20.0,
            phase: 0.0,
            i_component: 0.0,
            q_component: 0.0,
            cycle_strength: 0.0,
        }
    }
}

/// MESA Adaptive Moving Average индикатор
#[derive(Debug, Clone)]
pub struct MesaAdaptiveMA {
    // Hilbert-стадии: фиксированные EMA-смузеры (константы Эхлерса)
    fast_ma: SmootherSlot,      // Быстрая MA для I компонента
    slow_ma: SmootherSlot,      // Медленная MA для Q компонента
    period_ma: SmootherSlot,    // MA для сглаживания периода

    // Буферы для расчетов
    prices: Vec<f64>,
    i_components: Vec<f64>,
    q_components: Vec<f64>,
    periods: Vec<f64>,

    // Параметры
    min_period: f64,
    max_period: f64,

    // Текущие значения
    current_period: f64,
    // Выход: адаптивная EMA «на месте» (alpha из current_period каждый бар,
    // накопитель НЕ пересоздаётся — иначе адаптивный период не влиял бы на выход).
    mama: f64,
    seeded: bool,

    // Результат
    current_result: MesaAdaptiveResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl MesaAdaptiveMA {
    /// Создать новый MESA Adaptive MA (base 20, полоса ×0.4..×2.5 → скан 8..50).
    pub fn new() -> Self {
        Self::with_parameters(20.0, 0.4, 2.5)
    }

    /// Создать с нормальным `base_period` и ОТНОСИТЕЛЬНОЙ полосой скана
    /// (`min_mult ≤ max_mult`), выводимой внутри: `min = base·min_mult`, `max = base·max_mult`.
    /// Относительные границы делают `min < max` структурным инвариантом — свободных
    /// абсолютных min/max, которые можно расставить не в том порядке, нет.
    pub fn with_parameters(base_period: f64, min_mult: f64, max_mult: f64) -> Self {
        assert!(base_period > 0.0, "base_period must be > 0");
        let min_period = (base_period * min_mult).max(1.0);
        let max_period = (base_period * max_mult).max(min_period + 1.0);

        Self {
            // Hilbert-стадии фиксированы (EMA 6/12/10 по Эхлерсу).
            fast_ma: SmootherSlot::new(SmootherId::Ema, 6),
            slow_ma: SmootherSlot::new(SmootherId::Ema, 12),
            period_ma: SmootherSlot::new(SmootherId::Ema, 10),

            prices: Vec::with_capacity(64),
            i_components: Vec::with_capacity(32),
            q_components: Vec::with_capacity(32),
            periods: Vec::with_capacity(32),

            min_period,
            max_period,
            current_period: (min_period + max_period) / 2.0,
            mama: 0.0,
            seeded: false,

            current_result: MesaAdaptiveResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Build with explicit min/max periods (natural parameterization).
    ///
    /// Identical to [`with_parameters`] except the scan band is taken directly —
    /// the `base_period × mult` derivation is skipped.
    pub fn with_periods(min_period: f64, max_period: f64) -> Self {
        assert!(min_period > 0.0, "min_period must be > 0");
        assert!(max_period > min_period, "max_period must be > min_period");

        Self {
            fast_ma: SmootherSlot::new(SmootherId::Ema, 6),
            slow_ma: SmootherSlot::new(SmootherId::Ema, 12),
            period_ma: SmootherSlot::new(SmootherId::Ema, 10),

            prices: Vec::with_capacity(64),
            i_components: Vec::with_capacity(32),
            q_components: Vec::with_capacity(32),
            periods: Vec::with_capacity(32),

            min_period,
            max_period,
            current_period: (min_period + max_period) / 2.0,
            mama: 0.0,
            seeded: false,

            current_result: MesaAdaptiveResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Обновить индикатор новым баром
    pub fn feed(&mut self, value: f64) -> MesaAdaptiveResult {
        let price = value;
        self.update_price(price)
    }

    /// Обновить индикатор новой ценой
    pub fn update_price(&mut self, price: f64) -> MesaAdaptiveResult {
        // Добавляем цену в буфер
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        // Нужно минимум данных для расчетов
        if self.prices.len() >= 7 {
            // 1. Рассчитываем Hilbert Transform компоненты
            self.calculate_hilbert_components();

            // 2. Определяем адаптивный период
            self.calculate_adaptive_period();

            // 3. Обновляем адаптивную MA адаптивным периодом
            self.update_adaptive_ma(price);

            self.is_ready = true;
        }

        self.update_count += 1;
        self.current_result
    }

    /// Рассчитать компоненты Hilbert Transform
    fn calculate_hilbert_components(&mut self) {
        let len = self.prices.len();
        if len < 7 {
            return;
        }

        // Используем существующие MA для сглаживания
        let current_price = self.prices[len - 1];

        // Обновляем быстрые и медленные MA
        let fast_value = self.fast_ma.feed(current_price);
        let slow_value = self.slow_ma.feed(current_price);

        // I компонент (In-Phase) - разность быстрой и медленной MA
        let i_component = fast_value - slow_value;

        // Q компонент (Quadrature) - используем сдвиг фазы
        let q_component = if len >= 4 {
            let prev_fast = if self.i_components.len() >= 2 {
                self.i_components[self.i_components.len() - 2]
            } else {
                i_component
            };

            // Приближение Hilbert Transform через сдвиг
            (i_component + prev_fast) / 2.0
        } else {
            i_component
        };

        // Сохраняем компоненты
        if self.i_components.len() >= 32 {
            self.i_components.remove(0);
        }
        self.i_components.push(i_component);

        if self.q_components.len() >= 32 {
            self.q_components.remove(0);
        }
        self.q_components.push(q_component);

        // Обновляем результат
        self.current_result.i_component = i_component;
        self.current_result.q_component = q_component;

        // Рассчитываем фазу
        if i_component != 0.0 {
            self.current_result.phase = (q_component / i_component).atan();
        }
    }

    /// Рассчитать адаптивный период
    fn calculate_adaptive_period(&mut self) {
        if self.i_components.len() < 2 || self.q_components.len() < 2 {
            return;
        }

        let i_len = self.i_components.len();
        let q_len = self.q_components.len();

        let i_curr = self.i_components[i_len - 1];
        let i_prev = self.i_components[i_len - 2];
        let q_curr = self.q_components[q_len - 1];
        let q_prev = self.q_components[q_len - 2];

        // Рассчитываем изменение фазы
        let phase_curr = if i_curr != 0.0 { (q_curr / i_curr).atan() } else { 0.0 };
        let phase_prev = if i_prev != 0.0 { (q_prev / i_prev).atan() } else { 0.0 };

        let mut delta_phase = phase_curr - phase_prev;

        // Нормализуем изменение фазы
        if delta_phase < -PI {
            delta_phase += 2.0 * PI;
        } else if delta_phase > PI {
            delta_phase -= 2.0 * PI;
        }

        // Рассчитываем мгновенный период
        let inst_period = if delta_phase.abs() > 0.01 {
            let period = 2.0 * PI / delta_phase.abs();
            period.max(self.min_period).min(self.max_period)
        } else {
            self.current_period
        };

        // Сглаживаем период используя существующую MA
        let smoothed_period = self.period_ma.feed(inst_period);

        // Сохраняем период
        if self.periods.len() >= 32 {
            self.periods.remove(0);
        }
        self.periods.push(smoothed_period);

        self.current_period = smoothed_period.max(self.min_period).min(self.max_period);
        self.current_result.period = self.current_period;

        // Рассчитываем силу цикла
        self.calculate_cycle_strength();
    }

    /// Рассчитать силу цикла
    fn calculate_cycle_strength(&mut self) {
        if self.periods.len() < 5 {
            self.current_result.cycle_strength = 0.5;
            return;
        }

        // Анализируем стабильность периода
        let recent_periods = &self.periods[self.periods.len() - 5..];
        let mean: f64 = recent_periods.iter().sum::<f64>() / recent_periods.len() as f64;

        let variance: f64 = recent_periods.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / recent_periods.len() as f64;

        let std_dev = variance.sqrt();
        let cv = if mean > 0.0 { std_dev / mean } else { 1.0 };

        // Чем меньше коэффициент вариации, тем сильнее цикл
        let strength = (1.0 - cv.min(1.0)).max(0.0);
        self.current_result.cycle_strength = strength;
    }

    /// Обновить адаптивную MA адаптивным периодом (in-place adaptive EMA)
    fn update_adaptive_ma(&mut self, price: f64) {
        // alpha из доминирующего цикла. Накопитель MAMA сохраняется между барами —
        // период меняет ТОЛЬКО скорость сглаживания, а не сбрасывает состояние. Раньше
        // здесь стоял фиксированный EMA, и весь Hilbert-период не влиял на выход (баг).
        let alpha = (2.0 / (self.current_period + 1.0)).clamp(0.0, 1.0);
        self.mama = if self.seeded {
            alpha * price + (1.0 - alpha) * self.mama
        } else {
            self.seeded = true;
            price
        };
        self.current_result.value = self.mama;
    }

    /// Получить текущее значение
    pub fn value(&self) -> f64 {
        self.current_result.value
    }

    /// Получить полный результат
    pub fn result(&self) -> MesaAdaptiveResult {
        self.current_result
    }

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready && self.seeded
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.fast_ma = SmootherSlot::new(SmootherId::Ema, 6);
        self.slow_ma = SmootherSlot::new(SmootherId::Ema, 12);
        self.period_ma = SmootherSlot::new(SmootherId::Ema, 10);
        self.mama = 0.0;
        self.seeded = false;

        self.prices.clear();
        self.i_components.clear();
        self.q_components.clear();
        self.periods.clear();

        self.current_period = (self.min_period + self.max_period) / 2.0;
        self.current_result = MesaAdaptiveResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.current_period as usize
    }

    /// Генерировать торговый сигнал на основе пересечения цены и адаптивной MA
    pub fn trading_signal(&self, current_price: f64, prev_price: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        let ma_value = self.current_result.value;
        let strength = self.current_result.cycle_strength;

        // Сигналы только при достаточной силе цикла
        if strength < 0.3 {
            return 0;
        }

        // Пересечение вверх
        if prev_price <= ma_value && current_price > ma_value {
            return 1;
        }

        // Пересечение вниз
        if prev_price >= ma_value && current_price < ma_value {
            return -1;
        }

        0
    }

    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        let result = self.current_result;
        let trend = if current_price > result.value { "Восходящий" } else { "Нисходящий" };

        format!(
            "MESA Adaptive MA: {:.4}, Период: {:.1}, Фаза: {:.3}, Сила цикла: {:.2}, Тренд: {}",
            result.value,
            result.period,
            result.phase,
            result.cycle_strength,
            trend
        )
    }

    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("mesa_ma".to_string(), self.current_result.value);
        values.insert("adaptive_period".to_string(), self.current_result.period);
        values.insert("phase".to_string(), self.current_result.phase);
        values.insert("i_component".to_string(), self.current_result.i_component);
        values.insert("q_component".to_string(), self.current_result.q_component);
        values.insert("cycle_strength".to_string(), self.current_result.cycle_strength);
        values
    }

    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }

    /// Получить параметры (min_period, max_period)
    pub fn parameters(&self) -> (f64, f64) {
        (self.min_period, self.max_period)
    }
}

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`MesaAdaptiveMA`] (MAMA): the adaptive period scan band as
/// absolute `min_period`/`max_period`, and the price source. MAMA is, by definition, an
/// adaptive EMA whose smoothing rate follows the dominant cycle — there is no
/// configurable MA-type axis (a non-EMA smoother has no single-alpha recurrence to
/// adapt per bar).
///
/// `valid_params` enforces `min_period < max_period`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MesaConfig {
    /// Absolute lower scan period bound (default 8.0). Sweepable.
    pub min_period: Param<f64>,
    /// Absolute upper scan period bound (default 50.0, must be > min_period). Sweepable.
    pub max_period: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl crate::contract::Config for MesaConfig {
    fn defaults() -> Self {
        // Old defaults: base_period=20.0, min_mult=0.4, max_mult=2.5.
        // Derived absolutes (from old with_parameters logic):
        //   min_period = (20.0*0.4).max(1.0) = 8.0
        //   max_period = (20.0*2.5).max(8.0+1.0) = 50.0
        MesaConfig {
            min_period: Param::Solo(8.0),
            max_period: Param::Solo(50.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn valid_params(&self) -> Result<(), String> {
        let min = self.min_period.resolved();
        let max = self.max_period.resolved();
        if min >= max {
            return Err(format!("min_period({min}) >= max_period({max})"));
        }
        Ok(())
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // source: Class O → auto all-8.
        let mut s = Self::machine_defaults_auto();
        // min_period: absolute lower scan bound — sweep_f64(1.0,40.0,1.0).
        s.min_period = Param::many(sweep_f64(1.0, 40.0, 1.0));
        // max_period: absolute upper scan bound — sweep_f64(2.0,200.0,2.0).
        s.max_period = Param::many(sweep_f64(2.0, 200.0, 2.0));
        s
    }
}

impl Indicator for MesaAdaptiveMA {
    const ID: IndicatorId = IndicatorId::Mama;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(period): per-bar Hilbert in-phase/quadrature components + dominant-cycle
    /// period scan over rolling price/component buffers. Three fixed Hilbert-stage EMA
    /// smoothers (fast/slow/period) plus an in-place adaptive-EMA output recurrence
    /// (no inner indicator edge). A heavy adaptive MA — like KAMA/FRAMA, it is kept out
    /// of low-level smoother slots by the weight cap.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Mama)];
    type Config = MesaConfig;
    type Runtime = MesaAdaptiveMA;

    fn create(cfg: MesaConfig) -> MesaAdaptiveMA {
        MesaAdaptiveMA::with_periods(
            cfg.min_period.resolved(),
            cfg.max_period.resolved(),
        )
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &Self::Config) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for MesaAdaptiveMA {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Mama, "MESA Adaptive MA", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesa_adaptive_ma_creation() {
        let mesa = MesaAdaptiveMA::new();
        assert!(!mesa.is_ready());
        assert_eq!(mesa.parameters().0, 8.0);
        assert_eq!(mesa.parameters().1, 50.0);
    }

    #[test]
    fn test_mesa_with_parameters() {
        // base 20 × 0.5..1.5 → scan band 10..30
        let mesa = MesaAdaptiveMA::with_parameters(20.0, 0.5, 1.5);
        assert_eq!(mesa.parameters(), (10.0, 30.0));
    }

    #[test]
    fn test_mesa_update() {
        let mut mesa = MesaAdaptiveMA::new();

        // Добавляем синусоидальные данные
        for i in 0..250 {
            let price = 100.0 + 10.0 * (i as f64 * 0.2).sin();
            let result = mesa.update_price(price);

            if i > 10 {
                assert!(result.period >= mesa.parameters().0);
                assert!(result.period <= mesa.parameters().1);
                assert!(result.cycle_strength >= 0.0 && result.cycle_strength <= 1.0);
                assert!(result.value.is_finite());
            }
        }
    }

    #[test]
    fn test_mesa_constant_converges() {
        let mut mesa = MesaAdaptiveMA::new();
        for _ in 0..120 {
            mesa.update_price(100.0);
        }
        assert!(mesa.is_ready());
        // The in-place adaptive EMA must converge to a constant input.
        assert!((mesa.value() - 100.0).abs() < 1e-3, "MAMA on constant price should converge, got {}", mesa.value());
    }

    #[test]
    fn test_mesa_tracks_uptrend() {
        let mut mesa = MesaAdaptiveMA::new();
        for i in 1..=120 {
            mesa.update_price(100.0 + i as f64);
        }
        assert!(mesa.is_ready());
        // The adaptive output must move up with a sustained trend (not stay near the seed).
        assert!(mesa.value() > 150.0, "MAMA should track the uptrend, got {}", mesa.value());
    }

    #[test]
    fn test_mesa_reset() {
        let mut mesa = MesaAdaptiveMA::new();
        for i in 0..30 {
            mesa.update_price(100.0 + i as f64);
        }
        mesa.reset();
        assert!(!mesa.is_ready());
        assert_eq!(mesa.value(), 0.0);
    }

    #[test]
    fn test_mesa_contract_create() {
        let cfg = <<MesaAdaptiveMA as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut mesa = <MesaAdaptiveMA as Indicator>::create(cfg);
        for i in 1..=60 {
            mesa.feed(100.0 + i as f64);
        }
        assert!(mesa.is_ready());
    }
}
