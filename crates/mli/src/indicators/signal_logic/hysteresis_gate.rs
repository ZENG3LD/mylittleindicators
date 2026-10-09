// Hysteresis Gate: sticky {-1,0,1} with lower/upper thresholds and hold logic
//
// Self-contained version: uses internal RSI to generate input signal
// Holds state until RSI crosses the opposite threshold (reduces whipsaws)

use crate::indicators::momentum::rsi::Rsi;

#[derive(Debug, Clone)]
pub struct HysteresisGate {
    lower: f64,
    upper: f64,
    state: i8,
    rsi: Rsi,
}

impl HysteresisGate {
    /// Creates a new HysteresisGate with RSI thresholds.
    /// Default: lower=30 (oversold), upper=70 (overbought), rsi_period=14.
    pub fn new(lower: f64, upper: f64) -> Self {
        Self::with_rsi_period(lower, upper, 14)
    }

    /// Creates a HysteresisGate with configurable RSI period.
    /// `rsi_period` replaces the baked-in default of 14.
    pub fn with_rsi_period(lower: f64, upper: f64, rsi_period: usize) -> Self {
        Self {
            lower: lower.clamp(0.0, 50.0),
            upper: upper.clamp(50.0, 100.0),
            state: 0,
            rsi: Rsi::new(rsi_period),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.state = 0;
        self.rsi.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }

    // Rules: if state<=0 and RSI>=upper => state=+1; if state>=0 and RSI<=lower => state=-1; else keep state
    /// Feed ONE resolved scalar (the configured source, default close). The factory
    /// extracts the source field; the internal RSI drives the sticky gate logic.
    pub fn feed(&mut self, value: f64) -> i8 {
        // Update internal RSI
        self.rsi.feed(value);

        if self.rsi.is_ready() {
            let rsi_value = self.rsi.value();
            if self.state <= 0 && rsi_value >= self.upper {
                self.state = 1;  // Flip to overbought
            } else if self.state >= 0 && rsi_value <= self.lower {
                self.state = -1; // Flip to oversold
            }
            // Otherwise hold current state (hysteresis)
        }
        self.state
    }

    #[inline]
    pub fn value(&self) -> f64 {
        (self.state) as f64
    }

    pub fn lower(&self) -> f64 {
        self.lower
    }

    pub fn upper(&self) -> f64 {
        self.upper
    }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`HysteresisGate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HysteresisGateConfig {
    /// RSI oversold threshold (default 30).
    pub lower: Param<f64>,
    /// RSI overbought threshold (default 70).
    pub upper: Param<f64>,
    /// RSI period (default 14).
    pub rsi_period: Param<usize>,
}

impl Indicator for HysteresisGate {
    const ID: IndicatorId = IndicatorId::Hyst;
    /// Signal logic gate — not a pluggable oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Outer gate is O(1) sticky logic; inner RSI charged via `inner` port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::Hyst)];

    type Config = HysteresisGateConfig;
    type Runtime = HysteresisGate;

    fn create(cfg: HysteresisGateConfig) -> HysteresisGate {
        HysteresisGate::with_rsi_period(cfg.lower.resolved(), cfg.upper.resolved(), cfg.rsi_period.resolved())
    }
}

impl crate::contract::Config for HysteresisGateConfig {
    fn defaults() -> Self {
        HysteresisGateConfig {
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


impl Render for HysteresisGate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Hyst, "Hysteresis", Color::hex(0xFF9800))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hysteresis_gate_creation() {
        let hg = HysteresisGate::new(30.0, 70.0);
        assert!(!hg.is_ready()); // Not ready until RSI warmup
        assert_eq!(hg.value(), 0.0);
        assert!((hg.lower() - 30.0).abs() < 1e-9);
        assert!((hg.upper() - 70.0).abs() < 1e-9);
    }

    #[test]
    fn test_hysteresis_gate_with_trend() {
        let mut hg = HysteresisGate::new(30.0, 70.0);

        // Strong uptrend should eventually trigger overbought state
        let mut price = 100.0;
        for _ in 0..30 {
            price += 2.0;
            hg.feed(price);
        }

        assert!(hg.is_ready());
        // State should be either 0 or 1 after uptrend
        let state = hg.value() as i8;
        assert!(state >= 0, "Uptrend should not give oversold signal");
    }

    #[test]
    fn test_hysteresis_gate_with_rsi_period() {
        let mut hg = HysteresisGate::with_rsi_period(30.0, 70.0, 7);
        assert!(!hg.is_ready());
        let mut price = 100.0;
        for _ in 0..20 {
            price += 2.0;
            hg.feed(price);
        }
        assert!(hg.is_ready());
        assert!((hg.value() as i8).abs() <= 1);
    }

    #[test]
    fn test_hysteresis_gate_reset() {
        let mut hg = HysteresisGate::new(30.0, 70.0);

        let mut price = 100.0;
        for _ in 0..20 {
            price += 1.0;
            hg.feed(price);
        }

        hg.reset();
        assert!(!hg.is_ready());
        assert_eq!(hg.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_hyst() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<HysteresisGate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Hyst(cfg).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..30 {
            price += 2.0;
            f.feed(0, MarketSample::Bar {
                open: price - 1.0,
                high: price + 0.5,
                low: price - 1.5,
                close: price,
                volume: 9999.0, // volume not used by gate logic
            });
        }
        // Strong uptrend: state should not be oversold
        let sig = f.primary() as i8;
        assert!(sig >= 0, "strong uptrend: expected non-oversold state, got {}", sig);
    }
}
