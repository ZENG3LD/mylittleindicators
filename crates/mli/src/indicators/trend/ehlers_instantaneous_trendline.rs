//! Ehlers Instantaneous Trendline — adaptive trendline via Hilbert Transform approximation.
//!
//! Tracks price direction by splitting the signal into a trend component (ITrend)
//! and a cycle component (price − ITrend). The three internal MAs (smoothing EMA(5),
//! trend EMA(10), noise SMA(20)) are implementation details of the Hilbert
//! approximation and are NOT user-configurable. Only `alpha` (the ITrend smoothing
//! coefficient, default 0.07) and the price `source` are exposed in the contract config.
//!
//! Reference: John Ehlers "Rocket Science for Traders".

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;

/// Ehlers Instantaneous Trendline indicator.
#[derive(Clone)]
pub struct EhlersInstantaneousTrendline {
    // Three hardcoded internal stages for the Hilbert approximation.
    smoothing_ma: SmootherSlot,   // EMA(5)  — price pre-smoothing
    trend_ma:     SmootherSlot,   // EMA(10) — delayed-price smoothing for Q component
    noise_ma:     SmootherSlot,   // SMA(20) — noise-level estimator

    prices:      Vec<f64>,  // rolling price buffer (capped at 64)
    i_components: Vec<f64>, // In-Phase components  (capped at 32)
    q_components: Vec<f64>, // Quadrature components (capped at 32)
    trendlines:   Vec<f64>, // ITrend history        (capped at 32)

    alpha: f64,         // ITrend smoothing coefficient (0.0 < alpha ≤ 1.0)
    trendline: f64,     // current ITrend value (the primary output)
    current_phase: f64, // last computed phase

    is_ready:     bool,
    update_count: usize,
}

impl EhlersInstantaneousTrendline {
    /// Default ctor — `alpha = 0.07`, source = close.
    pub fn new() -> Self {
        Self::with_alpha(0.07)
    }

    /// Build with a custom alpha coefficient.
    pub fn with_alpha(alpha: f64) -> Self {
        assert!(alpha > 0.0 && alpha <= 1.0, "Alpha must be in (0.0, 1.0]");
        Self {
            smoothing_ma: SmootherSlot::new(SmootherId::Ema, 5),
            trend_ma:     SmootherSlot::new(SmootherId::Ema, 10),
            noise_ma:     SmootherSlot::new(SmootherId::Sma, 20),
            prices:       Vec::with_capacity(64),
            i_components: Vec::with_capacity(32),
            q_components: Vec::with_capacity(32),
            trendlines:   Vec::with_capacity(32),
            alpha,
            trendline:    0.0,
            current_phase: 0.0,
            is_ready:     false,
            update_count: 0,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, price: f64) -> f64 {
        // Rolling price buffer
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.prices.len() >= 7 {
            let smoothed_price = self.smoothing_ma.feed(price);
            self.update_hilbert_components(smoothed_price);
            self.update_instantaneous_trendline();
            self.is_ready = true;
        }

        self.update_count += 1;
        self.trendline
    }

    fn update_hilbert_components(&mut self, smoothed_price: f64) {
        let len = self.prices.len();

        let i_component = smoothed_price;
        let q_component = if len >= 4 {
            let delayed_price = self.prices[len - 4];
            let smoothed_delayed = self.trend_ma.feed(delayed_price);
            (smoothed_price - smoothed_delayed) * 0.707 // sin(45°) ≈ 0.707
        } else {
            0.0
        };

        if self.i_components.len() >= 32 { self.i_components.remove(0); }
        self.i_components.push(i_component);

        if self.q_components.len() >= 32 { self.q_components.remove(0); }
        self.q_components.push(q_component);

        if i_component != 0.0 {
            self.current_phase = (q_component / i_component).atan();
        }
    }

    fn update_instantaneous_trendline(&mut self) {
        if self.i_components.len() < 2 || self.q_components.len() < 2 {
            return;
        }

        let a  = self.alpha;
        let a2 = a * a;

        let len = self.prices.len();
        let current_price = self.prices[len - 1];
        let prev_price    = if len >= 2 { self.prices[len - 2] } else { current_price };

        let prev_trendline = if self.trendlines.is_empty() {
            current_price
        } else {
            self.trendlines[self.trendlines.len() - 1]
        };

        // Ehlers ITrend formula:
        // ITrend = (a - a²/4)*Price + (a²/2)*Price[1] - (a - 3a²/4)*ITrend[1]
        let trendline = (a - a2 / 4.0) * current_price
                      + (a2 / 2.0) * prev_price
                      - (a - 3.0 * a2 / 4.0) * prev_trendline;

        if self.trendlines.len() >= 32 { self.trendlines.remove(0); }
        self.trendlines.push(trendline);

        self.trendline = trendline;

        // Feed noise MA (side effect for noise-level tracking).
        let noise = (current_price - trendline).abs();
        self.noise_ma.feed(noise);
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.trendline
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn reset(&mut self) {
        self.smoothing_ma.reset();
        self.trend_ma.reset();
        self.noise_ma.reset();
        self.prices.clear();
        self.i_components.clear();
        self.q_components.clear();
        self.trendlines.clear();
        self.trendline    = 0.0;
        self.current_phase = 0.0;
        self.is_ready     = false;
        self.update_count = 0;
    }
}

impl std::fmt::Debug for EhlersInstantaneousTrendline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EhlersInstantaneousTrendline")
            .field("alpha", &self.alpha)
            .field("is_ready", &self.is_ready)
            .field("trendline", &self.trendline)
            .finish()
    }
}

impl Default for EhlersInstantaneousTrendline {
    fn default() -> Self {
        Self::new()
    }
}

// ─── contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`EhlersInstantaneousTrendline`].
///
/// `alpha` — ITrend smoothing coefficient (0.0 < alpha ≤ 1.0; default 0.07).
/// The three internal MAs (smoothing EMA(5), trend EMA(10), noise SMA(20)) are
/// fixed implementation details and are NOT exposed here.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EitConfig {
    pub source: Param<OhlcvField>,
    /// ITrend smoothing coefficient (Ehlers default 0.07 ≈ period 14).
    pub alpha: Param<f64>,
}

impl Indicator for EhlersInstantaneousTrendline {
    const ID: IndicatorId = IndicatorId::Eit;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) per bar: three recursive smoothers + bounded window buffers.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[
        Store::fixed(StoreKind::Vec, 64),  // prices buffer
        Store::fixed(StoreKind::Vec, 32),  // i_components
        Store::fixed(StoreKind::Vec, 32),  // q_components
        Store::fixed(StoreKind::Vec, 32),  // trendlines
    ]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Eit)];
    type Config = EitConfig;
    type Runtime = EhlersInstantaneousTrendline;

    fn create(cfg: EitConfig) -> EhlersInstantaneousTrendline {
        let alpha = cfg.alpha.resolved();
        assert!(alpha > 0.0 && alpha <= 1.0, "EitConfig: alpha must be in (0.0, 1.0]");
        EhlersInstantaneousTrendline::with_alpha(alpha)
        // cfg.source is resolved by the factory via source_fields; the runtime
        // receives the scalar directly through feed(f64).
    }

    fn source_fields(cfg: &EitConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for EitConfig {
    fn defaults() -> Self {
        EitConfig {
            source: Param::Solo(OhlcvField::Close),
            alpha:  Param::Solo(0.07),
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
        // source: Class O — auto all-8.
        let mut s = Self::machine_defaults_auto();
        // alpha: Class E (EMA-style decay coefficient, 0 < alpha ≤ 1) — sweep_f64(0.01,0.99,0.01).
        s.alpha = Param::many(sweep_f64(0.01, 0.99, 0.01));
        s
    }
}


impl Render for EhlersInstantaneousTrendline {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Eit, "EIT", Color::hex(0x4CAF50))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_ehlers_instantaneous_trendline_creation() {
        let itl = EhlersInstantaneousTrendline::new();
        assert!(!itl.is_ready());
        assert_eq!(itl.alpha(), 0.07);
    }

    #[test]
    fn test_ehlers_with_alpha() {
        let itl = EhlersInstantaneousTrendline::with_alpha(0.1);
        assert_eq!(itl.alpha(), 0.1);
    }

    #[test]
    fn test_ehlers_update() {
        let mut itl = EhlersInstantaneousTrendline::new();

        for i in 0..20 {
            let price = 100.0 + i as f64 * 0.5;
            itl.feed(price);

            if i > 10 {
                assert!(itl.is_ready());
                assert!(itl.value() > 0.0);
            }
        }

        assert!(itl.is_ready());
    }

    #[test]
    fn test_ehlers_reset() {
        let mut itl = EhlersInstantaneousTrendline::new();
        for i in 0..20 {
            itl.feed(100.0 + i as f64);
        }
        itl.reset();
        assert!(!itl.is_ready());
        assert_eq!(itl.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_eit() {
        let mut f = IndicatorOrder::Eit(<<EhlersInstantaneousTrendline as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            // WILD values in open/high/low/volume — only close is resolved by SOURCE.
            f.feed(0, MarketSample::Bar {
                open:   9999.0,
                high:   9999.0,
                low:    9999.0,
                close:  price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
        assert!(f.primary() > 0.0);
    }
}
