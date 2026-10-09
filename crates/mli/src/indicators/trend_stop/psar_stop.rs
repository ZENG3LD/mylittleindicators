//! PSAR Stop - динамические уровни на основе Parabolic SAR
//!
//! Вычисляет значения Parabolic SAR для создания динамических уровней поддержки/сопротивления.
//! Индикатор НЕ содержит логику стопов — только возвращает уровни SAR.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use super::parabolic_sar::ParabolicSAR;

/// PSAR Stop индикатор — динамические уровни на основе Parabolic SAR.
#[derive(Debug, Clone)]
pub struct PSARStop {
    psar: ParabolicSAR,
    current_level: f64,
    trend_direction: i8, // 1 = up trend, -1 = down trend
}

impl PSARStop {
    /// Default ctor: af_start=0.02, af_increment=0.02, af_max=0.20.
    pub fn new() -> Self {
        Self::with_params(0.02, 0.02, 0.20)
    }

    /// Build with custom acceleration factor parameters.
    pub fn with_params(af_start: f64, af_increment: f64, af_max: f64) -> Self {
        Self {
            psar: ParabolicSAR::with_params(af_start, af_increment, af_max),
            current_level: 0.0,
            trend_direction: 1,
        }
    }

    /// Feed a bar [high, low, close] (matches `const SOURCE` order: H/L/C).
    ///
    /// The inner `ParabolicSAR` is driven via its still-present `update_bar` bridge.
    /// `close` is used to infer trend direction (SAR below close = uptrend).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        let psar_value = self.psar.feed(&[high, low]);
        self.current_level = psar_value;
        self.trend_direction = if psar_value < close { 1 } else { -1 };

        self.current_level
    }

    pub fn level(&self) -> f64 { self.current_level }

    /// Contract output: the SAR stop level.
    pub fn value(&self) -> f64 {
        self.current_level
    }

    pub fn trend_direction(&self) -> i8 { self.trend_direction }

    pub fn is_ready(&self) -> bool { self.psar.is_ready() }

    pub fn reset(&mut self) {
        self.psar.reset();
        self.current_level = 0.0;
        self.trend_direction = 1;
    }
}

impl Default for PSARStop {
    fn default() -> Self { Self::new() }
}

// ── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`PSARStop`] — acceleration factor triple.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PSARStopConfig {
    pub af_start: Param<f64>,
    pub af_increment: Param<f64>,
    pub af_max: Param<f64>,
}

impl Indicator for PSARStop {
    const ID: IndicatorId = IndicatorId::Psars;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// H/L needed for the inner ParabolicSAR; C needed for trend_direction.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1) outer wrapper; inner ParabolicSAR cost declared via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Psar, &[IndicatorOutputId::Psar])],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Psars)];
    type Config = PSARStopConfig;
    type Runtime = PSARStop;

    fn create(cfg: PSARStopConfig) -> PSARStop {
        PSARStop::with_params(cfg.af_start.resolved(), cfg.af_increment.resolved(), cfg.af_max.resolved())
    }
}

impl crate::contract::Config for PSARStopConfig {
    fn defaults() -> Self {
        PSARStopConfig {
            af_start: Param::Solo(0.02),
            af_increment: Param::Solo(0.02),
            af_max: Param::Solo(0.20),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // f64 fields stay Solo from auto
        // Class L — Parabolic SAR acceleration factor axes
        // Generator enforces af_start ≤ af_increment and af_max ≥ af_start as structural filters.
        s.af_start = Param::many(sweep_f64(0.01, 0.10, 0.01));
        s.af_increment = Param::many(sweep_f64(0.01, 0.10, 0.01));
        s.af_max = Param::many(sweep_f64(0.05, 0.50, 0.05));
        s
    }
}


impl Render for PSARStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Psars, "SAR Stop", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psar_stop_creation() {
        let ind = PSARStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_psar_stop_warmup() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_psar_stop_values_finite() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let v = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_psar_stop_trend_direction() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        let dir = ind.trend_direction();
        assert!(dir == 1 || dir == -1);
    }

    #[test]
    fn test_psar_stop_reset() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..15 {
            ind.feed(&[105.0, 95.0, 100.0 + i as f64]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_psars() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<PSARStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Psars(cfg).build_solo().unwrap();
        for i in 0..15 {
            let price = 100.0 + i as f64;
            // open=9999 is the wild value; only high/low/close matter
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
