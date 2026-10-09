//! Augmented Dickey-Fuller unit-root test over a rolling price-LEVEL window.
//!
//! REAL implementation. The prior version fit an AR(1) on LOG-RETURNS (doubly
//! wrong: ADF tests a unit root in the LEVEL series, and must augment with
//! lagged differences to whiten residuals). Now delegates to the shared
//! `timeseries::adf_regression` (real augmented DF, proper t-stat via
//! (XᵀX)⁻¹, Schwert lag selection, constant deterministic term).
//!
//! Emits the raw ADF t-statistic as the main value (regime filter thresholds
//! it: t < MacKinnon 5% crit ≈ −2.86 ⇒ reject unit root, level is stationary).
//! `phi` exposed as ρ = 1 + γ for backward compatibility.

use crate::indicators::utils::math::timeseries::{adf_regression, AdfTrend};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct AdfProxy {
    window: usize,
    prices: VecDeque<f64>,
    /// ρ = 1 + γ (AR(1)-style persistence). Kept for backward-compat callers.
    pub phi: f64,
    /// The ADF t-statistic on the lagged-level coefficient.
    pub t_stat: f64,
}

impl AdfProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            prices: VecDeque::with_capacity(w + 1),
            phi: 0.0,
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prices.clear();
        self.phi = 0.0;
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.prices.len() >= self.window
    }

    /// Returns the ADF t-statistic as the main value.
    pub fn value(&self) -> f64 {
        self.t_stat
    }

    /// Feed one pre-extracted close price (resolved by the factory from `const SOURCE`).
    pub fn feed(&mut self, c: f64) -> f64 {
        self.prices.push_back(c);
        while self.prices.len() > self.window {
            self.prices.pop_front();
        }
        if self.is_ready() {
            let y: Vec<f64> = self.prices.iter().copied().collect();
            // Constant term (the standard ADF spec for a price level with no
            // assumed deterministic trend); auto lag selection.
            if let Some(res) = adf_regression(&y, AdfTrend::Constant, None) {
                self.t_stat = res.t_stat;
                self.phi = 1.0 + res.gamma; // ρ = 1 + γ
            }
        }
        self.t_stat
    }
}

impl Default for AdfProxy {
    /// Factory defaults: window=200 (clamped to max(200,20)=200).
    fn default() -> Self {
        Self::new(200)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, StoreKind, Store, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`AdfProxy`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AdfConfig {
    pub period: Param<usize>,
}

impl Indicator for AdfProxy {
    const ID: IndicatorId = IndicatorId::Adf;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Adf)];
    /// O(n) per bar: ADF regression rescans the window.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);

    type Config = AdfConfig;
    type Runtime = AdfProxy;

    fn create(cfg: AdfConfig) -> AdfProxy {
        AdfProxy::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &AdfConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for AdfConfig {
    fn defaults() -> Self {
        AdfConfig { period: Param::Solo(200) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AdfProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Adf, "ADF T-Statistic", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::indicator_id::IndicatorId;
    use crate::contract::Indicator;

    fn lcg_noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u = ((s >> 33) as f64) / (1u64 << 31) as f64;
            out.push(u - 1.0);
        }
        out
    }

    #[test]
    fn creation_and_warmup() {
        let mut adf = AdfProxy::new(60);
        assert!(!adf.is_ready());
        for &e in &lcg_noise(70, 1) {
            adf.feed(100.0 + e);
        }
        assert!(adf.is_ready());
        assert!(adf.t_stat.is_finite() && adf.phi.is_finite());
    }

    #[test]
    fn stationary_level_rejects() {
        // Mean-reverting level around 100 → ADF should reject the unit root.
        let mut adf = AdfProxy::new(120);
        let noise = lcg_noise(160, 42);
        let mut level = 0.0;
        for &e in &noise {
            level = 0.2 * level + e;
            adf.feed(100.0 + level);
        }
        // 5% MacKinnon crit (constant) ≈ −2.86.
        assert!(
            adf.t_stat < -2.86,
            "stationary level should reject @5%, got {}",
            adf.t_stat
        );
    }

    #[test]
    fn random_walk_does_not_reject() {
        let mut adf = AdfProxy::new(120);
        let noise = lcg_noise(160, 7);
        let mut level = 100.0;
        for &e in &noise {
            level += e;
            adf.feed(level);
        }
        assert!(
            adf.t_stat > -2.86,
            "random walk should not reject @5%, got {}",
            adf.t_stat
        );
    }

    #[test]
    fn reset_clears() {
        let mut adf = AdfProxy::new(50);
        for &e in &lcg_noise(60, 3) {
            adf.feed(100.0 + e);
        }
        adf.reset();
        assert!(!adf.is_ready());
        assert_eq!(adf.phi, 0.0);
        assert_eq!(adf.t_stat, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_adf() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Adf(<<AdfProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..250 {
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: 100.0, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::Adf).is_finite());
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<AdfProxy as Indicator>::ID, IndicatorId::Adf);
    }
}
