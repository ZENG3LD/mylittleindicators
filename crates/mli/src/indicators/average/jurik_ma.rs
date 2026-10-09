//! Jurik Moving Average (JMA) approximation indicator.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;

/// Jurik Moving Average (JMA) approximation - low-lag adaptive smoother.
///
/// This is an approximation of the proprietary Jurik Moving Average
/// using a blend of fast and slow EMAs controlled by a phase parameter.
///
/// JMA ≈ w × EMA_fast + (1-w) × EMA_slow
///
/// where w = (phase + 100) / 200.
///
/// # Parameters
/// - `period`: Base EMA period (fast uses period, slow uses period×2)
/// - `phase`: Controls weighting between fast/slow (-100 to +100)
///
/// # Implementation
///
/// Uses two O(1) EMA instances, so overall complexity is O(1).
#[derive(Debug, Clone)]
pub struct JurikMa {
    period: usize,
    ema_fast: SmootherSlot,
    ema_slow: SmootherSlot,
    phase: f64,
    value: f64,
}

impl JurikMa {
    /// Creates a new JMA with the specified period and phase.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Base smoothing period
    /// * `phase` - Blend factor (-100 = slow, +100 = fast)
    pub fn new(period: usize, phase: f64) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            ema_fast: SmootherSlot::new(SmootherId::Ema, p),
            ema_slow: SmootherSlot::new(SmootherId::Ema, (p * 2).max(2)),
            phase,
            value: 0.0,
        }
    }

    /// Returns `true` if the JMA has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema_fast.is_ready() && self.ema_slow.is_ready()
    }

    /// Returns the current JMA value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Resets the JMA to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.ema_fast = SmootherSlot::new(SmootherId::Ema, self.period);
        self.ema_slow = SmootherSlot::new(SmootherId::Ema, (self.period * 2).max(2));
        self.value = 0.0;
    }

    /// Updates the JMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn feed(&mut self, value: f64) -> f64 {
        let value = value;
        let f = self.ema_fast.feed(value);
        let s = self.ema_slow.feed(value);
        let w = (self.phase.clamp(-100.0, 100.0) + 100.0) / 200.0;
        self.value = w * f + (1.0 - w) * s;
        self.value
    }

    /// Returns the period of this JMA.
    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`JurikMa`]: base `period` + the `phase` blend (-100..+100,
/// fast↔slow weighting) — `phase` moves off the raw `additional_params`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct JmaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    pub phase: Param<f64>,
}

impl crate::contract::Config for JmaConfig {
    fn defaults() -> Self {
        JmaConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            phase: Param::Solo(0.0),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        let mut s = Self::machine_defaults_auto();
        // phase: Class K (Jurik phase, -100..+100) — sweep_f64(-100.0,100.0,5.0).
        s.phase = Param::many(sweep_f64(-100.0, 100.0, 5.0));
        s
    }
}

impl Indicator for JurikMa {
    const ID: IndicatorId = IndicatorId::Jma;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Two EMA smoothers (fast/slow). Own state is O(1) (the phase blend); the two
    /// EMAs are edges, charged recursively by the barometer (not hand-approximated).
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
            Port::new(IndicatorId::Ema, &[IndicatorOutputId::Ema]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Jma)];
    type Config = JmaConfig;
    type Runtime = JurikMa;

    fn create(cfg: JmaConfig) -> JurikMa {
        JurikMa::new(cfg.period.resolved(), cfg.phase.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &Self::Config) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for JurikMa {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Jma, "JMA", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jma_basic_calculation() {
        let mut jma = JurikMa::new(10, 0.0);

        for i in 1..=30 {
            jma.feed(i as f64 * 10.0);
        }

        assert!(jma.is_ready());
        assert!(jma.value() > 0.0);
    }

    #[test]
    fn test_jma_phase_effect() {
        let mut jma_fast = JurikMa::new(10, 100.0);
        let mut jma_slow = JurikMa::new(10, -100.0);

        for i in 1..=30 {
            jma_fast.feed(i as f64 * 10.0);
            jma_slow.feed(i as f64 * 10.0);
        }

        // Fast phase should be closer to current price (higher in uptrend)
        assert!(jma_fast.value() > jma_slow.value());
    }

    #[test]
    fn test_jma_reset() {
        let mut jma = JurikMa::new(5, 0.0);
        for i in 1..=20 {
            jma.feed(i as f64 * 10.0);
        }
        assert!(jma.is_ready());

        jma.reset();
        assert!(!jma.is_ready());
    }

}
