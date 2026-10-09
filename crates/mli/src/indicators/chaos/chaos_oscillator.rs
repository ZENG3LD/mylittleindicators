//! Chaos Oscillator - комбинированный индикатор хаотичности рынка
//! Объединяет фрактальную размерность, показатель Херста и волатильность
//! для определения степени хаоса в рыночных движениях

use super::fractal_dimension::FractalDimension;
use super::hurst_exponent::HurstExponent;
/// Осциллятор хаоса
#[derive(Debug, Clone)]
pub struct ChaosOscillator {
    period: usize,
    
    // Компоненты анализа хаоса
    fractal_dimension: FractalDimension,
    hurst_exponent: HurstExponent,
    
    // Буферы для дополнительных расчетов
    prices: Vec<f64>,
    volatilities: Vec<f64>,
    
    // Результаты
    chaos_index: f64,      // 0.0-1.0, где 1.0 = максимальный хаос
    predictability: f64,   // 0.0-1.0, где 1.0 = максимальная предсказуемость
    market_regime: MarketRegime,
    
    // Составляющие индекса
    complexity_weight: f64,    // Вес фрактальной размерности
    persistence_weight: f64,   // Вес показателя Херста
    volatility_weight: f64,    // Вес волатильности
    
    // Состояние
    is_ready: bool,
}

/// Режимы рынка на основе анализа хаоса
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketRegime {
    OrderedTrend,      // Упорядоченный тренд
    ChaoticTrend,      // Хаотичный тренд
    OrderedRange,      // Упорядоченный флэт
    ChaoticRange,      // Хаотичный флэт
    TransitionPhase,   // Переходная фаза
}

impl Default for ChaosOscillator {
    fn default() -> Self {
        Self::new_with_weights(128, 0.4, 0.4, 0.2)
    }
}

impl ChaosOscillator {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.min(512),
            fractal_dimension: FractalDimension::new(period, period / 8),
            hurst_exponent: HurstExponent::new(period),
            prices: Vec::with_capacity(512),
            volatilities: Vec::with_capacity(512),
            chaos_index: 0.5,
            predictability: 0.5,
            market_regime: MarketRegime::TransitionPhase,
            complexity_weight: 0.4,
            persistence_weight: 0.4,
            volatility_weight: 0.2,
            is_ready: false,
        }
    }
    
    /// Создать с настраиваемыми весами компонентов
    pub fn new_with_weights(
        period: usize,
        complexity_weight: f64,
        persistence_weight: f64,
        volatility_weight: f64,
    ) -> Self {
        // Нормализуем веса
        let total_weight = complexity_weight + persistence_weight + volatility_weight;
        let norm_complexity = complexity_weight / total_weight;
        let norm_persistence = persistence_weight / total_weight;
        let norm_volatility = volatility_weight / total_weight;
        
        Self {
            period: period.min(512),
            fractal_dimension: FractalDimension::new(period, period / 8),
            hurst_exponent: HurstExponent::new(period),
            prices: Vec::with_capacity(512),
            volatilities: Vec::with_capacity(512),
            chaos_index: 0.5,
            predictability: 0.5,
            market_regime: MarketRegime::TransitionPhase,
            complexity_weight: norm_complexity,
            persistence_weight: norm_persistence,
            volatility_weight: norm_volatility,
            is_ready: false,
        }
    }
    
    /// Обновить осциллятор новым баром
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
        // Добавляем цену закрытия
        if self.prices.len() >= self.period {
            self.prices.remove(0);
        }
        self.prices.push(close);
        
        // Рассчитываем волатильность (True Range)
        let volatility = if self.prices.len() >= 2 {
            let prev_close = self.prices[self.prices.len() - 2];
            let tr1 = high - low;
            let tr2 = (high - prev_close).abs();
            let tr3 = (low - prev_close).abs();
            tr1.max(tr2).max(tr3)
        } else {
            high - low
        };
        
        if self.volatilities.len() >= self.period {
            self.volatilities.remove(0);
        }
        self.volatilities.push(volatility);
        
        // Обновляем компоненты
        self.fractal_dimension.feed(close);
        self.hurst_exponent.feed(close);
        
        // Рассчитываем индекс хаоса
        if self.prices.len() >= self.period / 2 {
            self.calculate_chaos_index();
            self.determine_market_regime();
            self.is_ready = true;
        }
        
        self.chaos_index
    }
    
    /// Рассчитать индекс хаоса
    fn calculate_chaos_index(&mut self) {
        // Компонент сложности (фрактальная размерность)
        let complexity_component = self.fractal_dimension.complexity_score();
        
        // Компонент персистентности (обратный показатель Херста)
        // Чем ближе к 0.5, тем больше хаос
        let hurst = self.hurst_exponent.hurst_exponent();
        let persistence_component = 1.0 - (hurst - 0.5).abs() * 2.0;
        
        // Компонент волатильности (нормализованная волатильность)
        let volatility_component = if !self.volatilities.is_empty() && !self.prices.is_empty() {
            let avg_volatility = self.volatilities.iter().sum::<f64>() / self.volatilities.len() as f64;
            let avg_price = self.prices.iter().sum::<f64>() / self.prices.len() as f64;
            if avg_price > 0.0 {
                (avg_volatility / avg_price).min(1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Комбинируем компоненты с весами
        self.chaos_index = (complexity_component * self.complexity_weight +
            persistence_component * self.persistence_weight + volatility_component * self.volatility_weight).clamp(0.0, 1.0);
        
        // Предсказуемость - обратная величина хаоса
        self.predictability = 1.0 - self.chaos_index;
    }
    
    /// Определить режим рынка
    fn determine_market_regime(&mut self) {
        let hurst = self.hurst_exponent.hurst_exponent();
        let is_trending = !(0.45..=0.55).contains(&hurst);
        let is_chaotic = self.chaos_index > 0.6;
        
        self.market_regime = match (is_trending, is_chaotic) {
            (true, false) => MarketRegime::OrderedTrend,
            (true, true) => MarketRegime::ChaoticTrend,
            (false, false) => MarketRegime::OrderedRange,
            (false, true) => MarketRegime::ChaoticRange,
        };
        
        // Переходная фаза при средних значениях
        if self.chaos_index > 0.4 && self.chaos_index < 0.6 {
            self.market_regime = MarketRegime::TransitionPhase;
        }
    }
    
    /// Получить индекс хаоса
    pub fn chaos_index(&self) -> f64 {
        self.chaos_index
    }

    pub fn value(&self) -> f64 {
        self.chaos_index
    }
    
    /// Получить предсказуемость
    pub fn predictability(&self) -> f64 {
        self.predictability
    }
    
    /// Получить режим рынка
    pub fn market_regime(&self) -> MarketRegime {
        self.market_regime
    }
    
    /// Получить текстовое описание режима рынка
    pub fn market_regime_description(&self) -> &'static str {
        match self.market_regime {
            MarketRegime::OrderedTrend => "Ordered Trend - Predictable directional movement",
            MarketRegime::ChaoticTrend => "Chaotic Trend - Unpredictable directional movement",
            MarketRegime::OrderedRange => "Ordered Range - Predictable sideways movement",
            MarketRegime::ChaoticRange => "Chaotic Range - Unpredictable sideways movement",
            MarketRegime::TransitionPhase => "Transition Phase - Market changing regime",
        }
    }
    
    /// Получить торговый сигнал на основе анализа хаоса
    pub fn trading_signal(&self) -> i8 {
        match self.market_regime {
            MarketRegime::OrderedTrend => {
                // В упорядоченном тренде следуем направлению
                self.hurst_exponent.trading_signal()
            },
            MarketRegime::ChaoticTrend => {
                // В хаотичном тренде осторожность
                if self.chaos_index > 0.8 { 0 } else { self.hurst_exponent.trading_signal() }
            },
            MarketRegime::OrderedRange => {
                // В упорядоченном флэте - контртренд
                -self.hurst_exponent.trading_signal()
            },
            MarketRegime::ChaoticRange => {
                // В хаотичном флэте - ожидание
                0
            },
            MarketRegime::TransitionPhase => {
                // В переходной фазе - ожидание
                0
            },
        }
    }
    
    /// Получить силу сигнала
    pub fn signal_strength(&self) -> f64 {
        match self.market_regime {
            MarketRegime::OrderedTrend => self.predictability,
            MarketRegime::OrderedRange => self.predictability * 0.7,
            _ => self.predictability * 0.3, // Слабые сигналы в хаосе
        }
    }
    
    /// Получить компоненты анализа
    pub fn get_components(&self) -> (f64, f64, f64) {
        let complexity = self.fractal_dimension.complexity_score();
        let persistence = 1.0 - (self.hurst_exponent.hurst_exponent() - 0.5).abs() * 2.0;
        let volatility = if !self.volatilities.is_empty() && !self.prices.is_empty() {
            let avg_vol = self.volatilities.iter().sum::<f64>() / self.volatilities.len() as f64;
            let avg_price = self.prices.iter().sum::<f64>() / self.prices.len() as f64;
            if avg_price > 0.0 { (avg_vol / avg_price).min(1.0) } else { 0.0 }
        } else { 0.0 };
        
        (complexity, persistence, volatility)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Feed pre-extracted [high, low, close] lanes from the factory (const SOURCE = KlineSlice([High, Low, Close])).
    pub fn feed(&mut self, lanes: &[f64]) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        self.update(high, low, close);
    }


    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.fractal_dimension.reset();
        self.hurst_exponent.reset();
        self.prices.clear();
        self.volatilities.clear();
        self.chaos_index = 0.5;
        self.predictability = 0.5;
        self.market_regime = MarketRegime::TransitionPhase;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_oscillator_creation() {
        let ind = ChaosOscillator::new(50);
        assert!(!ind.is_ready());
        assert_eq!(ind.period(), 50);
    }

    #[test]
    fn test_chaos_oscillator_warmup() {
        let mut ind = ChaosOscillator::new(30);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price + 1.0, price - 1.0, price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_chaos_oscillator_values_range() {
        let mut ind = ChaosOscillator::new(30);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let chaos = ind.update(price + 1.0, price - 1.0, price);
            assert!(chaos >= 0.0 && chaos <= 1.0);
            assert!(ind.predictability() >= 0.0 && ind.predictability() <= 1.0);
        }
    }

    #[test]
    fn test_chaos_oscillator_reset() {
        let mut ind = ChaosOscillator::new(30);
        for i in 0..40 {
            ind.update(100.0 + i as f64, 105.0, 101.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.chaos_index(), 0.5);
    }

    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::ChaosOsc(<<ChaosOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            // volume = 9999.0 is not in SOURCE — proves only H/L/C are consumed.
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 1.5,
                low: base - 1.5,
                close: base,
                volume: 9999.0,
            });
        }
        // After enough bars the chaos index is in [0.0, 1.0].
        let v = f.read(IndicatorOutputId::ChaosOsc);
        assert!(v >= 0.0 && v <= 1.0, "chaos index out of range: {v}");
    }
}

// ─── Contract wiring ──────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, RenderSpec, Render, SourceAxis,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`ChaosOscillator`].
///
/// Mirrors `new_with_weights` — weights are normalised internally.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ChaosOscConfig {
    pub period: Param<usize>,
    pub complexity_weight: Param<f64>,
    pub persistence_weight: Param<f64>,
    pub volatility_weight: Param<f64>,
}

impl Indicator for ChaosOscillator {
    const ID: IndicatorId = IndicatorId::ChaosOsc;
    /// Composite chaos detector — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C fields — all three are always consumed.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(
        &[OhlcvField::High, OhlcvField::Low, OhlcvField::Close],
    ));
    /// Outer own cost: O(period) rescan of the volatility window per bar;
    /// two Vec stores (prices + volatilities).
    /// Inner ports charge the full cost of FractalDimension and HurstExponent.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
        inner: &[
            Port::new(IndicatorId::FractalDim, &[IndicatorOutputId::FractalDim]),
            Port::new(IndicatorId::Hurst, &[IndicatorOutputId::Hurst]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::ChaosOsc)];

    type Config = ChaosOscConfig;
    type Runtime = ChaosOscillator;

    fn create(cfg: ChaosOscConfig) -> ChaosOscillator {
        ChaosOscillator::new_with_weights(
            cfg.period.resolved(),
            cfg.complexity_weight.resolved(),
            cfg.persistence_weight.resolved(),
            cfg.volatility_weight.resolved(),
        )
    }
}

impl crate::contract::Config for ChaosOscConfig {
    fn defaults() -> Self {
        ChaosOscConfig {
            period: Param::Solo(128),
            complexity_weight: Param::Solo(0.4),
            persistence_weight: Param::Solo(0.4),
            volatility_weight: Param::Solo(0.2),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        use crate::contract::sweep_f64;
        let mut s = Self::machine_defaults_auto();
        // period: Class A → auto range(2,4048,1) — leave as-is
        // complexity_weight: Class D ratio/fraction → sweep 0.0..=1.0 step 0.05.
        // A1 2026-07-04: floor from valid_params (runtime — `new_with_weights` normalizes
        // by `total_weight = complexity+persistence+volatility`; the all-0.0 min corner of
        // three independently-swept `sweep_f64(0.0, ..)` axes makes `total_weight == 0.0`,
        // a 0/0 NaN with no `valid_params` gate to reject it). Floor ONE axis at 0.05 so the
        // sum can never be zero; the other two keep the full 0.0..=1.0 sweep.
        s.complexity_weight = Param::many(sweep_f64(0.05, 1.0, 0.05));
        // persistence_weight: Class D ratio/fraction → sweep 0.0..=1.0 step 0.05
        s.persistence_weight = Param::many(sweep_f64(0.0, 1.0, 0.05));
        // volatility_weight: Class D ratio/fraction → sweep 0.0..=1.0 step 0.05
        s.volatility_weight = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
}


impl Render for ChaosOscillator {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ChaosOsc, "Chaos Osc", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}




















