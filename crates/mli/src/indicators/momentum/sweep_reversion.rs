// Sweep Reversion Index (SRI)
// Быстрый индикатор детекции свипов ликвидности и возврата внутрь диапазона
// Возвращает непрерывный сигнал в диапазоне [-1.0, 1.0]:
//  - Отрицательный (до -1.0) = бычий свип снизу → лонг-уклон
//  - Положительный (до +1.0) = медвежий свип сверху → шорт-уклон
// При включенном подтверждении (confirm_next_bar) сигнал может задерживаться до следующего бара

use crate::engine::contract_engine::SmootherId;
use crate::indicators::swing::highest::Highest;
use crate::indicators::swing::lowest::Lowest;
use crate::indicators::volatility::atr::Atr;

#[derive(Debug, Clone, Copy)]
pub struct SweepReversionParams {
    /// Длина окна экстремумов (обычно 20-60)
    pub lookback_period: usize,
    /// Доля диапазона бара для проверки возврата (0.25 означает нижняя/верхняя квартиль)
    pub close_quartile: f64,
    /// Период ATR (для нормализации силы сигнала)
    pub atr_period: usize,
    /// Масштаб для нормализации: чем больше k, тем слабее вес
    pub weight_k: f64,
    /// Требовать подтверждение направлением следующего бара
    pub confirm_next_bar: bool,
    /// Тип MA для ATR сглаживания
    pub atr_smoother: SmootherId,
}

impl Default for SweepReversionParams {
    fn default() -> Self {
        Self {
            lookback_period: 40,
            close_quartile: 0.35,
            atr_period: 14,
            weight_k: 1.0,
            confirm_next_bar: false,
            atr_smoother: SmootherId::Rma,
        }
    }
}

/// Индикатор свипов/возвратов
#[derive(Debug, Clone)]
pub struct SweepReversionIndex {
    params: SweepReversionParams,
    highest: Highest,
    lowest: Lowest,
    atr: Atr,

    value: f64,

    pending_signal: Option<i8>,
    pending_ref_close: f64,

    is_ready: bool,
}

impl SweepReversionIndex {
    pub fn new(params: SweepReversionParams) -> Self {
        Self {
            highest: Highest::new(params.lookback_period),
            lowest: Lowest::new(params.lookback_period),
            atr: Atr::from_smoother(params.atr_period, params.atr_smoother),
            params,
            value: 0.0,
            pending_signal: None,
            pending_ref_close: 0.0,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highest.reset();
        self.lowest.reset();
        self.atr.reset();
        self.value = 0.0;
        self.pending_signal = None;
        self.pending_ref_close = 0.0;
        self.is_ready = false;
    }

    /// Feed a bar as [high, low, close] (Sources order).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        let prev_highest = if self.highest.is_ready() {
            Some(self.highest.value())
        } else {
            None
        };
        let prev_lowest = if self.lowest.is_ready() {
            Some(self.lowest.value())
        } else {
            None
        };

        // ATR reads H/L/C from a bar; open/volume are ignored.
        self.atr.feed(&[high, low, close]);

        // Highest reads only high; Lowest reads only low.
        self.highest.feed(high);
        self.lowest.feed(low);

        self.is_ready = self.highest.is_ready() && self.lowest.is_ready() && self.atr.is_ready();
        if !self.is_ready {
            self.value = 0.0;
            return self.value;
        }

        let range = (high - low).max(1e-12);
        let pos = (close - low) / range;

        let swept_top = prev_highest.map(|h| high > h).unwrap_or(false);
        let swept_bottom = prev_lowest.map(|l| low < l).unwrap_or(false);

        let in_lower_quartile = pos <= self.params.close_quartile;
        let in_upper_quartile = pos >= 1.0 - self.params.close_quartile;

        let raw_signal: i8 = if swept_top && in_lower_quartile {
            1
        } else if swept_bottom && in_upper_quartile {
            -1
        } else {
            0
        };

        let confirmed_signal = if self.params.confirm_next_bar {
            match self.pending_signal {
                Some(pend) => {
                    let is_confirmed = if pend > 0 {
                        close < self.pending_ref_close
                    } else {
                        close > self.pending_ref_close
                    };
                    self.pending_signal = None;
                    if is_confirmed { pend } else { 0 }
                }
                None => {
                    if raw_signal != 0 {
                        self.pending_signal = Some(raw_signal);
                        self.pending_ref_close = close;
                    }
                    0
                }
            }
        } else {
            raw_signal
        };

        let atr = self.atr.value().abs().max(1e-12);
        let mid = (high + low) * 0.5;
        let dev = (close - mid).abs();
        let mut weight = (dev / (self.params.weight_k.max(1e-12) * atr)).min(1.0);
        if weight.is_nan() || !weight.is_finite() {
            weight = 0.0;
        }

        self.value = (confirmed_signal as f64) * weight;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Дискретный сигнал на основе последнего значения: sign(value)
    #[inline]
    pub fn discrete_signal(&self) -> i8 {
        if self.value > 0.0 {
            1
        } else if self.value < 0.0 {
            -1
        } else {
            0
        }
    }

    #[inline]
    pub fn params(&self) -> SweepReversionParams {
        self.params
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`SweepReversionIndex`].
///
/// All scalar fields are `Param<T>`; `atr_smoother` is a `#[slot]` defaulting to `follow(Rma)`.
/// `confirm_next_bar` is not a sweep axis in practice (it flips behaviour, not a tune).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct SweepRevConfig {
    pub lookback_period: Param<usize>,
    pub close_quartile: Param<f64>,
    /// ATR period for signal normalization.
    pub atr_period: Param<usize>,
    pub weight_k: Param<f64>,
    pub confirm_next_bar: Param<bool>,
    /// ATR smoother — default `follow(Rma)` (Wilder) at `atr_period`.
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for SweepReversionIndex {
    const ID: IndicatorId = IndicatorId::SweepRev;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[
            Port::new(IndicatorId::Highest, &[IndicatorOutputId::Highest]),
            Port::new(IndicatorId::Lowest, &[IndicatorOutputId::Lowest]),
            Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr]),
        ],
    };
    const SLOTS: &'static [Slot] = SweepRevConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::SweepRev)];
    type Config = SweepRevConfig;
    type Runtime = SweepReversionIndex;

    fn create(cfg: SweepRevConfig) -> SweepReversionIndex {
        let atr_period = cfg.atr_period.resolved();
        let choice = cfg.atr_smoother.resolved();
        SweepReversionIndex::new(SweepReversionParams {
            lookback_period: cfg.lookback_period.resolved(),
            close_quartile: cfg.close_quartile.resolved(),
            atr_period,
            weight_k: cfg.weight_k.resolved(),
            confirm_next_bar: cfg.confirm_next_bar.resolved(),
            atr_smoother: choice.kind,
        })
    }

    fn slot_members(cfg: &SweepRevConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for SweepRevConfig {
    fn defaults() -> Self {
        SweepRevConfig {
            lookback_period: Param::Solo(40),
            close_quartile: Param::Solo(0.35),
            atr_period: Param::Solo(14),
            weight_k: Param::Solo(1.0),
            confirm_next_bar: Param::Solo(false),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // lookback_period/atr_period: Class A → auto range(2,4048,1).
        // close_quartile: Class D (quartile fraction 0..1) — sweep_f64(0.0,1.0,0.05).
        // weight_k: Class D (weight_* pattern, 0..1 unit-interval) — sweep_f64(0.0,1.0,0.05).
        // confirm_next_bar: Class R → auto both.
        // atr_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.close_quartile = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s.weight_k = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
}


impl Render for SweepReversionIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SweepRev, "Sweep Rev", Color::hex(0x9C27B0))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sweep_reversion_creation() {
        let sri = SweepReversionIndex::new(SweepReversionParams::default());
        assert!(!sri.is_ready());
        assert_eq!(sri.value(), 0.0);
        assert_eq!(sri.discrete_signal(), 0);
    }

    #[test]
    fn test_sweep_reversion_basic() {
        let mut sri = SweepReversionIndex::new(SweepReversionParams::default());
        for i in 1..=60 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            sri.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(sri.is_ready());
        assert!(sri.value().is_finite());
    }

    #[test]
    fn test_sweep_reversion_range() {
        let mut sri = SweepReversionIndex::new(SweepReversionParams::default());
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 15.0;
            let value = sri.feed(&[price + 3.0, price - 3.0, price]);
            assert!(value >= -1.0 && value <= 1.0, "SRI should be in [-1, 1], got {}", value);
        }
    }

    #[test]
    fn test_sweep_reversion_reset() {
        let mut sri = SweepReversionIndex::new(SweepReversionParams::default());
        for i in 1..=60 {
            let price = 100.0 + i as f64;
            sri.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(sri.is_ready());
        sri.reset();
        assert!(!sri.is_ready());
        assert_eq!(sri.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SweepReversionIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::SweepRev(cfg).build_solo().unwrap();
        for i in 1..=60 {
            let price = 100.0 + i as f64;
            // open=9999.0 and volume=9999.0 prove only H/L/C are used
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 2.0, low: price - 2.0, close: price, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }
}
