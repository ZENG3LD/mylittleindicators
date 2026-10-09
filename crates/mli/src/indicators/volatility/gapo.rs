// GAPO (Gopalakrishnan Range Index)

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
#[derive(Debug, Clone)]
pub struct Gapo {
    window: usize,
    value: f64,
}

impl Gapo {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Contract feed: `lanes` carries the declared SOURCE fields `[High, Low]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let (h, l) = (lanes[0], lanes[1]);
        let range = (h - l).max(1e-9);
        self.value = (range.ln()) / ((self.window as f64).ln());
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

// -- contract -----------------------------------------------------------------

use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed config for [`Gapo`].
/// `window` is the period N in the denominator `ln(N)`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct GapoConfig {
    pub window: Param<usize>,
}

impl Indicator for Gapo {
    const ID: IndicatorId = IndicatorId::Gapo;
    /// FAMILY DECISION: `IndicatorId::Gapo` is in the Momentum enum group.
    /// The formula `ln(H-L) / ln(N)` is a bounded range-efficiency index (roughly
    /// 0..2; ~0.5 flat market, ~1.0 trending) -- oscillator behaviour, NOT a
    /// volatility level (no variance, no std dev). Declared `Oscillator`.
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads H and L only -- KlineSlice; cannot vary the field.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// Stateless single-bar: no window, no store, always ready.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Gapo)];
    type Config = GapoConfig;
    type Runtime = Gapo;

    fn create(cfg: GapoConfig) -> Gapo {
        Gapo::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for GapoConfig {
    fn defaults() -> Self {
        GapoConfig { window: Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // window: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for Gapo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Gapo, "GAPO", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gapo_creation() {
        let gapo = Gapo::new(14);
        assert!(gapo.is_ready()); // GAPO is always ready
        assert_eq!(gapo.value(), 0.0);
        assert_eq!(gapo.window(), 14);
    }

    #[test]
    fn test_gapo_min_window() {
        let gapo = Gapo::new(1);
        assert_eq!(gapo.window(), 2); // min window is 2
    }

    #[test]
    fn test_gapo_basic() {
        let mut gapo = Gapo::new(14);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            gapo.feed(&[price + 5.0, price - 5.0]);
        }
        // GAPO is ln(range) / ln(window), should be finite and positive for typical ranges
        assert!(gapo.value().is_finite());
    }

    #[test]
    fn test_gapo_reset() {
        let mut gapo = Gapo::new(14);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            gapo.feed(&[price + 5.0, price - 5.0]);
        }
        gapo.reset();
        assert_eq!(gapo.value(), 0.0);
    }

    #[test]
    fn test_gapo_finite_values() {
        let mut gapo = Gapo::new(14);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = gapo.feed(&[price + 3.0, price - 3.0]);
            assert!(value.is_finite(), "GAPO should always be finite");
        }
    }
}
