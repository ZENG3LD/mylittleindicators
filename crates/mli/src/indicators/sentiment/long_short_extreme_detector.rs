//! LongShortExtremeDetector — flags extreme positioning as potential reversal signal.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::contract::render::Render;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, RenderSpec, SourceAxis, UpdateComplexity};
use crate::contract::sweep_f64;
use crate::core::types::LongShortRatio;
use crate::engine::stream_kind::StreamKind;

/// Emits a contrarian signal when `long_ratio` reaches an extreme.
///
/// - `long_ratio > upper` → `Signal(1)` — crowd is extremely long; bearish reversal expected.
/// - `long_ratio < lower` → `Signal(-1)` — crowd is extremely short; bullish reversal expected.
/// - Otherwise → `Signal(0)`.
///
/// Output: a signal (i8).
#[derive(Clone, Debug)]
pub struct LongShortExtremeDetector {
    upper: f64,
    lower: f64,
    last_signal: i8,
}

impl LongShortExtremeDetector {
    /// Create a new detector. Typical defaults: `upper = 0.8`, `lower = 0.2`.
    pub fn new(upper: f64, lower: f64) -> Self {
        Self {
            upper,
            lower,
            last_signal: 0,
        }
    }
}

/// Typed dual-mode config for [`LongShortExtremeDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LongShortExtremeDetectorConfig {
    pub upper: Param<f64>,
    pub lower: Param<f64>,
}

impl Indicator for LongShortExtremeDetector {
    const ID: IndicatorId = IndicatorId::LongShortExtremeDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::LongShortRatio];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::LongShortExtremeDetector)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = LongShortExtremeDetectorConfig;
    type Runtime = LongShortExtremeDetector;

    fn create(cfg: LongShortExtremeDetectorConfig) -> LongShortExtremeDetector {
        LongShortExtremeDetector::new(cfg.upper.resolved(), cfg.lower.resolved())
    }
}

impl LongShortRatioConsumer for LongShortExtremeDetector {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) {
        self.last_signal = if lsr.long_ratio > self.upper {
            1
        } else if lsr.long_ratio < self.lower {
            -1
        } else {
            0
        };
    }


    fn reset(&mut self) {
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

impl Default for LongShortExtremeDetector {
    /// Factory default: upper=0.8, lower=0.2.
    fn default() -> Self {
        Self::new(0.8, 0.2)
    }
}

impl crate::contract::Config for LongShortExtremeDetectorConfig {
    fn defaults() -> Self {
        LongShortExtremeDetectorConfig {
            upper: Param::Solo(0.8),
            lower: Param::Solo(0.2),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // Class F OB/OS sub-case — upper/lower live in [0,1] long_ratio space.
        // Proportional equivalent of the 0..100 oscillator sweep:
        //   upper: overbought boundary — sweep 0.50..=0.95 step 0.01
        //   lower: oversold boundary  — sweep 0.05..=0.50 step 0.01
        s.upper = Param::many(sweep_f64(0.50, 0.95, 0.01));
        s.lower = Param::many(sweep_f64(0.05, 0.50, 0.01));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LongShortExtremeDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::LongShortExtremeDetector,
                "L/S Extreme Signal",
                Color::hex(0xEF5350),
            )
            .bounds(-1.0, 1.0)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn extreme_long_gives_signal_one() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        ind.update_long_short_ratio(&make_lsr(0.85));
        assert_eq!(ind.value() as i8, 1);
    }

    #[test]
    fn extreme_short_gives_signal_minus_one() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        ind.update_long_short_ratio(&make_lsr(0.15));
        assert_eq!(ind.value() as i8, -1);
    }

    #[test]
    fn neutral_gives_signal_zero() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        ind.update_long_short_ratio(&make_lsr(0.5));
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn is_ready_immediately() {
        let ind = LongShortExtremeDetector::new(0.8, 0.2);
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        ind.update_long_short_ratio(&make_lsr(0.9));
        ind.reset();
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn factory_feeds_resolved_long_short_extreme_detector() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::LongShortExtremeDetector(
            <<LongShortExtremeDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // feed an extreme long ratio
        f.feed(0, MarketSample::LongShortRatio(&make_lsr(0.9)));
        assert_eq!(f.primary() as i8, 1, "expected Signal(1) for extreme long");
    }
}

impl LongShortExtremeDetector {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
