// Rolling Cross Mutual Information over multiple lags

use crate::indicators::entropy::mutual_information::MutualInformation;

#[derive(Debug, Clone)]
pub struct CrossMutualInformationLags {
    indicators: Vec<MutualInformation>,
    pub values: Vec<f64>,
}

impl CrossMutualInformationLags {
    pub fn new(window: usize, lags: &[usize], bins: usize, clip_abs: f64) -> Self {
        let mut indicators = Vec::new();
        for &lag in lags {
            indicators.push(MutualInformation::new(window, lag, bins, clip_abs));
        }
        Self {
            indicators,
            values: vec![0.0; lags.len()],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        for mi in &mut self.indicators {
            mi.reset();
        }
        self.values.fill(0.0);
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.indicators.iter().all(|mi| mi.is_ready())
    }

    /// Feed the resolved close scalar.
    pub fn feed(&mut self, close: f64) -> &[f64] {
        for (i, mi) in self.indicators.iter_mut().enumerate() {
            self.values[i] = mi.feed(close);
        }
        &self.values
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.values.first().copied().unwrap_or(0.0)
    }
}

impl Default for CrossMutualInformationLags {
    /// Factory default (Xmil arm): window=14, lags=[1,2,3,5,10], bins=10, clip_abs=3.0.
    fn default() -> Self {
        Self::new(14, &[1, 2, 3, 5, 10], 10, 3.0)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Render, RenderSpec, SourceAxis, Store, StoreKind,
    UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

use crate::contract::Param;
use crate::contract::axis::sweep_f64;

/// Typed config for [`CrossMutualInformationLags`].
///
/// The default lag set `[1, 2, 3, 5, 10]` is baked in at construction; only the shared
/// MI parameters are configurable here. Use `CrossMutualInformationLags::new` directly
/// for a custom lag set.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct XmilConfig {
    /// Rolling window length fed to each lag-specific MI estimator.
    pub window: Param<usize>,
    /// Number of histogram bins for the joint distribution.
    pub bins: Param<usize>,
    /// Return clipping bound (±clip_abs).
    pub clip_abs: Param<f64>,
}

impl Indicator for CrossMutualInformationLags {
    const ID: IndicatorId = IndicatorId::Xmil;
    /// Not a pluggable family member — a lag-sweep information-theory detector.
    /// Consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single field (close by default).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Reports the lag-1 MI as the primary scalar; the rest of the lag values
    /// are internal state only accessible via `values`.
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Xmil)];
    /// O(window × n_lags) per bar (each MI estimator rescans its histogram).
    /// Multiple period-deep Vec windows inside the child MutualInformation instances.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    type Config = XmilConfig;
    type Runtime = CrossMutualInformationLags;

    fn create(cfg: XmilConfig) -> CrossMutualInformationLags {
        CrossMutualInformationLags::new(
            cfg.window.resolved(),
            &[1, 2, 3, 5, 10],
            cfg.bins.resolved(),
            cfg.clip_abs.resolved(),
        )
    }

    fn source_fields(cfg: &XmilConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for XmilConfig {
    fn defaults() -> Self {
        XmilConfig {
            window: Param::Solo(14),
            bins: Param::Solo(10),
            clip_abs: Param::Solo(3.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // bins: Class B (entropy histogram) — 4..=64 step 2, corrected from auto 2..=4048
        s.bins = Param::many((4usize..=64).step_by(2).collect());
        // clip_abs: Class F threshold — default 3.0 indicates whole-return scale (not fractional);
        // sweep_f64(0.1, 5.0, 0.1) covers the relevant threshold range including the 3.0 default
        s.clip_abs = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for CrossMutualInformationLags {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::Xmil,
                "Cross MI",
                Color::hex(0x9C27B0),
            )
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_mutual_information_lags_creation() {
        let cmi = CrossMutualInformationLags::new(30, &[1, 2, 3], 8, 0.05);
        assert!(!cmi.is_ready());
        assert_eq!(cmi.values.len(), 3);
    }

    #[test]
    fn test_cross_mutual_information_lags_warmup() {
        let mut cmi = CrossMutualInformationLags::new(20, &[1, 2], 8, 0.05);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cmi.feed(price);
        }
        assert!(cmi.is_ready());
    }

    #[test]
    fn test_cross_mutual_information_lags_values_finite() {
        let mut cmi = CrossMutualInformationLags::new(20, &[1, 2, 3], 8, 0.05);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let values = cmi.feed(price);
            for v in values {
                assert!(v.is_finite());
            }
        }
    }

    #[test]
    fn test_cross_mutual_information_lags_reset() {
        let mut cmi = CrossMutualInformationLags::new(20, &[1, 2], 8, 0.05);
        for i in 0..50 {
            cmi.feed(100.0 + i as f64);
        }
        cmi.reset();
        assert!(!cmi.is_ready());
    }

    /// The factory resolves close from the bar; open/high/low/volume are wildcards to
    /// prove only close is consumed. Primary output is the lag-1 MI value.
    #[test]
    fn factory_feeds_resolved_xmil() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Xmil(<<CrossMutualInformationLags as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Xmil).is_finite(), "Cross MI lag-1 should be finite");
    }
}
