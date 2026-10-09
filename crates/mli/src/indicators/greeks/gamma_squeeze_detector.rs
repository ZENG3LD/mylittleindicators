//! GammaSqueezeDetector — detects potential gamma squeeze conditions.

use crate::engine::streams::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Detects potential gamma squeeze: high gamma + significant price movement.
///
/// Fires +1 when:
/// - `gamma > gamma_threshold` AND
/// - `|last_price - prev_price| > price_move_threshold`
///
/// Price movement is tracked via the inherent `update_price` helper (seeded
/// from bar-close values). The `OptionGreeksConsumer` data path evaluates the
/// signal on every greeks update.
///
/// Output: `Signal(i8)`.
#[derive(Debug, Clone)]
pub struct GammaSqueezeDetector {
    gamma_threshold: f64,
    price_move_threshold: f64,
    last_gamma: f64,
    prev_price: f64,
    last_price: f64,
    last_signal: i8,
}

impl GammaSqueezeDetector {
    /// Create a new indicator.
    /// - `gamma_threshold`: minimum gamma value to consider (default 0.01)
    /// - `price_move_threshold`: minimum absolute price move to trigger (default 1.0)
    pub fn new(gamma_threshold: f64, price_move_threshold: f64) -> Self {
        Self {
            gamma_threshold,
            price_move_threshold,
            last_gamma: 0.0,
            prev_price: f64::NAN,
            last_price: f64::NAN,
            last_signal: 0,
        }
    }

    /// Seed the price-move tracker from a bar close. Records close price and
    /// re-evaluates the squeeze condition. Not a contracted bar-indicator path —
    /// call this alongside the greeks stream when bar data is also available.
    pub fn feed(&mut self, c: f64) {
        self.prev_price = self.last_price;
        self.last_price = c;
        self.last_signal = self.evaluate_signal();
    }

    fn evaluate_signal(&self) -> i8 {
        let price_moved = if self.prev_price.is_finite() && self.last_price.is_finite() {
            (self.last_price - self.prev_price).abs() > self.price_move_threshold
        } else {
            false
        };
        if self.last_gamma > self.gamma_threshold && price_moved {
            1
        } else {
            0
        }
    }
}

impl Default for GammaSqueezeDetector {
    fn default() -> Self {
        Self::new(0.01, 1.0)
    }
}

impl OptionGreeksConsumer for GammaSqueezeDetector {
    fn update_option_greeks(&mut self, g: &OptionGreeks) {
        self.last_gamma = g.gamma;
        self.last_signal = self.evaluate_signal();
    }


    fn reset(&mut self) {
        self.last_gamma = 0.0;
        self.prev_price = f64::NAN;
        self.last_price = f64::NAN;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.last_gamma > 0.0 && self.last_price.is_finite()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`GammaSqueezeDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct GammaSqueezeDetectorConfig {
    /// Minimum gamma to consider (absolute value). Default 0.01.
    pub gamma_threshold: Param<f64>,
    /// Minimum absolute price move to confirm a squeeze. Default 1.0.
    pub price_move_threshold: Param<f64>,
}

impl Indicator for GammaSqueezeDetector {
    const ID: IndicatorId = IndicatorId::GammaSqueezeDetector;
    /// Not a pluggable family — a squeeze-condition detector, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OptionGreeks];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    /// O(1) per update: scalar comparisons, no window buffers.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::GammaSqueezeDetector)];
    type Config = GammaSqueezeDetectorConfig;
    type Runtime = GammaSqueezeDetector;

    fn create(cfg: GammaSqueezeDetectorConfig) -> GammaSqueezeDetector {
        GammaSqueezeDetector::new(cfg.gamma_threshold.resolved(), cfg.price_move_threshold.resolved())
    }
}

impl crate::contract::Config for GammaSqueezeDetectorConfig {
    fn defaults() -> Self {
        GammaSqueezeDetectorConfig {
            gamma_threshold: Param::Solo(0.01),
            price_move_threshold: Param::Solo(1.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // gamma_threshold: F — threshold on raw gamma value (sigma-class)
        s.gamma_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        // price_move_threshold: F — minimum absolute price move (sigma-class threshold)
        s.price_move_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for GammaSqueezeDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::GammaSqueezeDetector,
                "Gamma Squeeze",
                Color::hex(0xFF5722),
            )
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(gamma: f64) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn squeeze_detected_with_high_gamma_and_price_move() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        // Set prev price and last price with significant move
        ind.feed(100.0); // last_price = 100, prev = NAN
        ind.feed(103.0); // price_move = 3 > 1
        // Set high gamma
        ind.update_option_greeks(&make_greeks(0.05));
        assert_eq!(ind.value() as i8, 1, "should detect squeeze");
    }

    #[test]
    fn no_squeeze_with_low_gamma() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        ind.feed(100.0);
        ind.feed(103.0);
        ind.update_option_greeks(&make_greeks(0.001)); // below threshold
        assert_eq!(ind.value() as i8, 0, "should not detect squeeze with low gamma");
    }

    #[test]
    fn no_squeeze_with_small_price_move() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        ind.feed(100.0);
        ind.feed(100.5); // move = 0.5 < 1.0
        ind.update_option_greeks(&make_greeks(0.05));
        assert_eq!(ind.value() as i8, 0, "should not detect squeeze with small price move");
    }

    #[test]
    fn reset_clears() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        ind.feed(100.0);
        ind.feed(105.0);
        ind.update_option_greeks(&make_greeks(0.05));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value() as i8, 0);
    }

    /// Factory smoke test: build the factory and feed a greeks sample.
    /// The price-move tracker (feed) is not driven by the factory — so
    /// prev_price/last_price stay NAN and the signal stays 0. Correct for pure-greeks feed.
    #[test]
    fn factory_feeds_resolved_greeks() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::GammaSqueezeDetector(
            <<GammaSqueezeDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let g = make_greeks(0.05);
        f.feed(0, MarketSample::OptionGreeks(&g));
        // Without price seeding, signal stays 0 — factory uses greeks path only.
        assert_eq!(f.primary() as i8, 0, "no price data fed — signal must be 0");
    }
}

impl GammaSqueezeDetector {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
