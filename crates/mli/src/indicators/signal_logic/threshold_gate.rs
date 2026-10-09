// Threshold Gate: converts a scalar to {-1,0,1} by asymmetric thresholds
//
// Self-contained version: uses internal RSI to generate input signal
// Returns +1 when RSI >= upper (overbought), -1 when RSI <= lower (oversold), 0 otherwise

use crate::indicators::momentum::rsi::Rsi;

#[derive(Debug, Clone)]
pub struct ThresholdGate {
    upper: f64,
    lower: f64,
    rsi: Rsi,
    signal: i8,
}

impl ThresholdGate {
    /// Creates a new ThresholdGate with RSI thresholds.
    /// Default: lower=30 (oversold), upper=70 (overbought), rsi_period=14.
    pub fn new(lower: f64, upper: f64) -> Self {
        Self::with_rsi_period(lower, upper, 14)
    }

    /// Creates a ThresholdGate with configurable RSI period.
    /// `rsi_period` replaces the baked-in default of 14.
    pub fn with_rsi_period(lower: f64, upper: f64, rsi_period: usize) -> Self {
        Self {
            upper: upper.clamp(50.0, 100.0),
            lower: lower.clamp(0.0, 50.0),
            rsi: Rsi::new(rsi_period),
            signal: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.signal = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }

    /// Feed ONE resolved scalar (the configured source, default close). The factory
    /// extracts the source field; the internal RSI drives the threshold logic.
    pub fn feed(&mut self, value: f64) -> i8 {
        // Update internal RSI
        self.rsi.feed(value);

        if self.rsi.is_ready() {
            let rsi_value = self.rsi.value();
            self.signal = if rsi_value >= self.upper {
                1  // Overbought
            } else if rsi_value <= self.lower {
                -1 // Oversold
            } else {
                0  // Neutral
            };
        }
        self.signal
    }

    #[inline]
    pub fn value(&self) -> f64 {
        (self.signal) as f64
    }

    pub fn thresholds(&self) -> (f64, f64) {
        (self.lower, self.upper)
    }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`ThresholdGate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ThresholdGateConfig {
    /// RSI oversold threshold (default 30).
    pub lower: Param<f64>,
    /// RSI overbought threshold (default 70).
    pub upper: Param<f64>,
    /// RSI period (default 14).
    pub rsi_period: Param<usize>,
}

impl Indicator for ThresholdGate {
    const ID: IndicatorId = IndicatorId::Thresh;
    /// Signal logic detector — not a pluggable oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Outer is O(1) scalar logic; the embedded RSI is charged recursively via `inner`.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::Thresh)];

    type Config = ThresholdGateConfig;
    type Runtime = ThresholdGate;

    fn create(cfg: ThresholdGateConfig) -> ThresholdGate {
        ThresholdGate::with_rsi_period(cfg.lower.resolved(), cfg.upper.resolved(), cfg.rsi_period.resolved())
    }
}

impl crate::contract::Config for ThresholdGateConfig {
    fn defaults() -> Self {
        ThresholdGateConfig {
            lower: Param::Solo(30.0),
            upper: Param::Solo(70.0),
            rsi_period: Param::Solo(14),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::{Param, sweep_f64};
        let mut s = Self::machine_defaults_auto();
        // lower/upper: RSI 0-100 space — Class F sub-case OB/OS.
        s.lower = Param::many(sweep_f64(5.0, 50.0, 1.0));
        s.upper = Param::many(sweep_f64(50.0, 95.0, 1.0));
        // rsi_period: Class A — auto range(2,4048,1) is correct.
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ThresholdGate {
    fn rendering() -> RenderSpec {
        // Bounds corrected to -1..1 with zero_baseline (output is Signal: -1, 0, or 1).
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Thresh, "Threshold Gate", Color::hex(0xF44336))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_gate_creation() {
        let tg = ThresholdGate::new(30.0, 70.0);
        assert!(!tg.is_ready()); // Not ready until RSI warmup
        assert_eq!(tg.value(), 0.0);
        assert_eq!(tg.thresholds(), (30.0, 70.0));
    }

    #[test]
    fn test_threshold_gate_with_uptrend() {
        let mut tg = ThresholdGate::new(30.0, 70.0);

        // Strong uptrend should push RSI high -> overbought signal
        let mut price = 100.0;
        for _ in 0..30 {
            price += 2.0; // Consistent gains
            tg.feed(price);
        }

        assert!(tg.is_ready());
        // Strong uptrend should give overbought signal (1)
        let signal = tg.value() as i8;
        assert!(signal >= 0, "Strong uptrend should not be oversold");
    }

    #[test]
    fn test_threshold_gate_with_downtrend() {
        let mut tg = ThresholdGate::new(30.0, 70.0);

        // Strong downtrend should push RSI low -> oversold signal
        let mut price = 200.0;
        for _ in 0..30 {
            price -= 2.0; // Consistent losses
            tg.feed(price);
        }

        assert!(tg.is_ready());
        // Strong downtrend should give oversold signal (-1)
        let signal = tg.value() as i8;
        assert!(signal <= 0, "Strong downtrend should not be overbought");
    }

    #[test]
    fn test_threshold_gate_with_rsi_period() {
        let mut tg = ThresholdGate::with_rsi_period(30.0, 70.0, 7);
        assert!(!tg.is_ready());
        let mut price = 100.0;
        for _ in 0..20 {
            price += 2.0;
            tg.feed(price);
        }
        assert!(tg.is_ready());
        let sig = tg.value() as i8;
        assert!(sig >= -1 && sig <= 1);
    }

    #[test]
    fn test_threshold_gate_reset() {
        let mut tg = ThresholdGate::new(30.0, 70.0);

        // Warm up
        let mut price = 100.0;
        for _ in 0..20 {
            price += 1.0;
            tg.feed(price);
        }

        tg.reset();
        assert!(!tg.is_ready());
        assert_eq!(tg.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_thresh() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<ThresholdGate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Thresh(cfg).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..30 {
            price += 2.0;
            f.feed(0, MarketSample::Bar {
                open: price - 1.0,
                high: price + 0.5,
                low: price - 1.5,
                close: price,
                volume: 1000.0,
            });
        }
        // After a strong uptrend the signal should be >= 0
        let sig = f.primary() as i8;
        assert!(sig >= 0, "strong uptrend: expected non-oversold signal, got {}", sig);
    }
}
