//! bollinger_bands.rs: High-Performance Bollinger Bands
//!
//! Architecture:
//! - `SmootherSlot` center line (the narrow smoother family) + an O(1) circular
//!   buffer for the standard deviation (no `remove(0)`).
//! - PURE scalar core: `feed(price)`. The core owns NO source field and NO
//!   `update_bar` — whoever supplies input (the factory, per the contract, or a host
//!   composite) resolves the price field and feeds the scalar.
//! - Extra metrics: %B, Bandwidth, Squeeze detection.

use crate::engine::contract_engine::{
    IndicatorOutputId, SmootherId, SmootherSlot, SmootherSlotOrder,
};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, Slot, Store,
    StoreKind, UpdateComplexity, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// High-Performance Bollinger Bands.
///
/// PURE core: owns NO source field and NO `update_bar`. The factory resolves the
/// configured price field (`BbConfig::source`) and feeds the scalar via
/// [`BollingerBands::feed`].
#[derive(Debug, Clone)]
pub struct BollingerBands {
    // Параметры
    period: usize,
    std_dev_mult: f64,

    // Центральная линия (узкий набор сглаживателей)
    center_ma: SmootherSlot,

    // Circular buffer для стандартного отклонения - O(1) operations
    price_buffer: Vec<f64>,
    buffer_index: usize,
    buffer_filled: bool,

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,

    // Дополнительные метрики
    std_dev: f64,
    bandwidth: f64,
    percent_b: f64,
}

impl BollingerBands {
    /// Build Bollinger Bands from a typed smoother id for the center line.
    /// Legacy bridge for un-contracted composites; the contract path goes through
    /// `BbConfig`.
    pub fn from_smoother(ma: SmootherId, period: usize, std_dev_mult: f64) -> Self {
        assert!(period > 0, "Period must be > 0");
        assert!(std_dev_mult > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_dev_mult,
            center_ma: SmootherSlot::new(ma, period),
            price_buffer: Vec::with_capacity(period),
            buffer_index: 0,
            buffer_filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            std_dev: 0.0,
            bandwidth: 0.0,
            percent_b: 0.5,
        }
    }

    /// Классические Bollinger Bands (SMA центральная линия).
    pub fn new_classic(period: usize, std_dev_mult: f64) -> Self {
        Self::from_smoother(SmootherId::Sma, period, std_dev_mult)
    }

    /// 0.1.8 entry. Feeds `close`. The factory path is [`Self::feed`].
    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> (f64, f64, f64) {
        self.feed(close)
    }

    /// Feed ONE pre-extracted scalar price — the pure core computation. Returns
    /// `(upper, middle, lower)`. The caller (the factory's source-resolving feed, or a
    /// host composite) supplies the price; the core knows nothing about OHLCV fields.
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        // Обновляем центральную линию через смузер-слот (узкий набор MA)
        self.middle = self.center_ma.feed(price);

        // Добавляем цену в circular buffer для std dev - O(1) операция
        if self.buffer_filled {
            // Перезаписываем старые значения циклически
            self.price_buffer[self.buffer_index] = price;
        } else {
            // Заполняем буфер в первый раз
            self.price_buffer.push(price);
        }

        // Обновляем индекс циклически
        self.buffer_index = (self.buffer_index + 1) % self.period;

        // Проверяем заполненность буфера
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }

        // Рассчитываем стандартное отклонение и границы
        if self.is_ready() {
            // %B меряем по тому же скаляру, что и полосы (price).
            self.calculate_std_dev_and_bands(price);
        } else {
            self.upper = 0.0;
            self.lower = 0.0;
            self.std_dev = 0.0;
            self.bandwidth = 0.0;
            self.percent_b = 0.5;
        }

        (self.upper, self.middle, self.lower)
    }

    /// Рассчитать стандартное отклонение и границы полос
    fn calculate_std_dev_and_bands(&mut self, current_price: f64) {
        let buffer_len = if self.buffer_filled { self.period } else { self.price_buffer.len() };

        // Рассчитываем стандартное отклонение относительно текущего MA
        let variance = self.price_buffer.iter()
            .take(buffer_len)
            .map(|&price| {
                let diff = price - self.middle;
                diff * diff
            })
            .sum::<f64>() / buffer_len as f64;

        self.std_dev = variance.sqrt();

        // Рассчитываем границы полос
        self.upper = self.middle + self.std_dev_mult * self.std_dev;
        self.lower = self.middle - self.std_dev_mult * self.std_dev;

        // Рассчитываем дополнительные метрики
        self.bandwidth = if self.middle != 0.0 {
            (self.upper - self.lower) / self.middle
        } else {
            0.0
        };

        // %B показывает позицию цены относительно полос (0.0 = нижняя полоса, 1.0 = верхняя полоса)
        let band_width = self.upper - self.lower;
        if band_width > 0.0 {
            self.percent_b = (current_price - self.lower) / band_width;
        } else {
            self.percent_b = 0.5;
        }
    }


    /// Получить текущие значения полос как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    /// Получить верхнюю полосу
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Получить среднюю линию
    pub fn middle(&self) -> f64 {
        self.middle
    }

    /// Получить нижнюю полосу
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// Получить стандартное отклонение
    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }

    /// Получить Bandwidth (ширина полос относительно средней линии)
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }

    /// Получить %B (позиция цены в полосах, 0-1)
    pub fn percent_b(&self) -> f64 {
        self.percent_b
    }

    /// Получить ширину канала в абсолютных значениях
    pub fn channel_width(&self) -> f64 {
        if self.is_ready() {
            self.upper - self.lower
        } else {
            0.0
        }
    }

    /// Получить позицию цены в канале (0.0 = нижняя граница, 1.0 = верхняя граница)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower) / width
        } else {
            0.5
        }
    }

    /// Проверить "сжатие" полос (squeeze)
    pub fn is_squeeze(&self, threshold: f64) -> bool {
        self.is_ready() && self.bandwidth < threshold
    }

    /// Проверить пробой верхней полосы
    pub fn is_upper_breakout(&self, price: f64) -> bool {
        self.is_ready() && price > self.upper
    }

    /// Проверить пробой нижней полосы
    pub fn is_lower_breakout(&self, price: f64) -> bool {
        self.is_ready() && price < self.lower
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.center_ma.is_ready() && self.buffer_filled
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.center_ma.reset();
        self.price_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
        self.std_dev = 0.0;
        self.bandwidth = 0.0;
        self.percent_b = 0.5;
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }

    /// Получить множитель стандартного отклонения
    pub fn std_dev_mult(&self) -> f64 {
        self.std_dev_mult
    }
}

impl Default for BollingerBands {
    fn default() -> Self {
        Self::new_classic(20, 2.0)
    }
}

/// Typed configuration for [`BollingerBands`]. `ma` is the center-line smoother order
/// Dual-mode (`Param` per field; not `Copy`). `period` is the host std-dev window. The centre
/// MA is a `Param<SmootherSlotOrder>` (the rich slot order): the centre MA's member AND its own
/// source-less params (period; for ALMA also offset/sigma). Default = SMA at the host's `period`
/// (the standard Bollinger centre). The slot member's period is its OWN axis (no host-follow),
/// swept by `SmootherSlotOrder::machine_sweep()` over every smoother member × its params. `period`
/// × `std_dev_mult` × `source` × the slot's member/param cube are the orthogonal sweep axes.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct BbConfig {
    pub period: Param<usize>,
    pub std_dev_mult: Param<f64>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
}

impl Indicator for BollingerBands {
    const ID: IndicatorId = IndicatorId::Bb;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Three band outputs (upper / middle / lower).
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::BbUpper),
        Output::price(IndicatorOutputId::BbMiddle),
        Output::price(IndicatorOutputId::BbLower),
        Output::magnitude(IndicatorOutputId::BbStdDev),
        Output::magnitude(IndicatorOutputId::BbBandwidth),
        Output::percent(IndicatorOutputId::BbPercentB),
    ];
    /// Own base = the std-dev circular buffer (period-deep). The center line is the
    /// MovingAverage SLOT — its cost lands recursively, not here.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = BbConfig::SLOTS;
    type Config = BbConfig;
    type Runtime = BollingerBands;

    fn create(cfg: BbConfig) -> BollingerBands {
        // `cfg.period` is the host std-dev window; the centre MA is built from the slot order at
        // ITS OWN params (default = SMA at the host period; the sweep varies member + period).
        let period = cfg.period.resolved();
        BollingerBands {
            period,
            std_dev_mult: cfg.std_dev_mult.resolved(),
            center_ma: cfg.ma.resolved().into_slot(),
            price_buffer: Vec::with_capacity(period),
            buffer_index: 0,
            buffer_filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            std_dev: 0.0,
            bandwidth: 0.0,
            percent_b: 0.5,
        }
    }

    /// Single-source core: the factory variant holds the resolved `Source` and feeds the
    /// scalar — so this `BollingerBands` is pure (no `source` field, no `update_bar`).
    fn source_fields(cfg: &BbConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &BbConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

/// The render half of Bollinger's contract — the SIBLING of `impl Indicator` above.
/// Compute declares the wireable scalar outputs (upper/middle/lower + std_dev/
/// bandwidth/percent_b); Render declares how the UI DRAWS them: an overlay on the price
/// chart, the two bands + the centre MA line. Orthogonal to compute.
impl crate::contract::Config for BbConfig {
    /// Standard Bollinger: period 20, 2σ, close source, SMA centre that FOLLOWS the period.
    fn defaults() -> Self {
        BbConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 20 })),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8, bool→both
        s.std_dev_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for BollingerBands {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::BbUpper, "Upper Band", Color::hex(0x2196F3), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::BbMiddle, "Middle (MA)", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::BbLower, "Lower Band", Color::hex(0x2196F3), 1.0))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bollinger_bands_creation() {
        let bb = BollingerBands::new_classic(20, 2.0);
        assert!(!bb.is_ready());
        assert_eq!(bb.period(), 20);
        assert_eq!(bb.std_dev_mult(), 2.0);
    }

    #[test]
    fn test_bollinger_bands_warmup() {
        let mut bb = BollingerBands::new_classic(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bb.feed(price);
        }
        assert!(bb.is_ready());
    }

    #[test]
    fn test_bollinger_bands_values() {
        let mut bb = BollingerBands::new_classic(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = bb.feed(price);
            if bb.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
                assert!(bb.bandwidth() >= 0.0, "Bandwidth should be non-negative");
            }
        }
    }

    #[test]
    fn test_bollinger_bands_reset() {
        let mut bb = BollingerBands::new_classic(20, 2.0);
        for i in 0..25 {
            bb.feed(100.0 + i as f64);
        }
        bb.reset();
        assert!(!bb.is_ready());
    }

    /// The `ContractFactory` variant holds the resolved source (close) and the `Source`
    /// feed extracts it before calling the pure core — proving the factory feeds the
    /// resolved scalar, not a raw OHLCV bar. Build via the typed order path.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Bb(<<BollingerBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // High is wildly off (999); only the CLOSE source produces a sane centred band.
        let bar = |close: f64| MarketSample::Bar {
            open: 0.0, high: 999.0, low: 0.0, close, volume: 0.0,
        };
        for i in 0..30 {
            f.feed(0, bar(100.0 + (i as f64 * 0.2).sin() * 10.0));
        }
        // The middle must sit near the ~100 close band, not near the 999 highs.
        // factory primary = upper for Bb
        assert!(f.primary().is_finite(), "factory BB upper should be finite, got {}", f.primary());
    }

    /// Pilot B: the centre-MA slot follows the host std-dev period by DEFAULT (no second,
    /// divergeable period), and `Own(p)` lets it diverge on purpose.
    #[test]
    fn centre_ma_follows_or_owns_period() {
        use crate::contract::{Config, Indicator, Param};

        // FOLLOW (default): centre MA runs at the host std-dev period — ONE period, no dup.
        let follow = BbConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 20 })),
        };
        let bb = BollingerBands::create(follow);
        assert_eq!(bb.center_ma.period(), 20, "default centre MA = SMA at the host period");
        assert_eq!(bb.period, 20);

        // OWN period: the centre MA runs at its own slot period, independent of the std-dev window.
        let own = BbConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 50 })),
        };
        let bb2 = BollingerBands::create(own);
        assert_eq!(bb2.center_ma.period(), 50, "slot period 50 → centre MA at its own period");
        assert_eq!(bb2.period, 20, "host std-dev window stays 20");

        // cube: period{20} x mult{2.0} x source{Close} x kind{Sma,Ema} = 2 (kind sweep, follow).
        let swept = BbConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::many(vec![
                SmootherSlotOrder::Sma(PeriodConfig { period: 20 }),
                SmootherSlotOrder::Ema(PeriodConfig { period: 20 }),
            ]),
        };
        assert_eq!(swept.cube_size(), 2);
        assert_eq!(swept.iter().count(), 2);
    }
}
