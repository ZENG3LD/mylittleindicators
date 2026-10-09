// Ehlers Cyber Cycle - Isolates market cycle component from trend
//
// Formula:
// smooth = (price + 2*price[1] + 2*price[2] + price[3]) / 6
// cycle = (1 - 0.5*alpha)^2 * (smooth - 2*smooth[1] + smooth[2])
//         + 2*(1 - alpha) * cycle[1]
//         - (1 - alpha)^2 * cycle[2]



#[derive(Debug, Clone)]
pub struct EhlersCyberCycle {
    alpha: f64,
    coeff1: f64,  // (1 - 0.5*alpha)^2
    coeff2: f64,  // 2*(1 - alpha)
    coeff3: f64,  // (1 - alpha)^2

    smooth_history: Vec<f64>,  // smooth[0..2]
    cycle_history: Vec<f64>,   // cycle[0..1]
    price_history: Vec<f64>,   // price[0..3]

    value: f64,
}

impl EhlersCyberCycle {
    pub fn new(alpha: f64) -> Self {
        let alpha = alpha.clamp(0.01, 0.99);
        let one_minus_alpha = 1.0 - alpha;
        let one_minus_half_alpha = 1.0 - 0.5 * alpha;

        let coeff1 = one_minus_half_alpha * one_minus_half_alpha;
        let coeff2 = 2.0 * one_minus_alpha;
        let coeff3 = one_minus_alpha * one_minus_alpha;

        Self {
            alpha,
            coeff1,
            coeff2,
            coeff3,
            smooth_history: Vec::with_capacity(3),
            cycle_history: Vec::with_capacity(2),
            price_history: Vec::with_capacity(4),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.smooth_history.clear();
        self.cycle_history.clear();
        self.price_history.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.price_history.len() >= 4
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed resolved `[high, low]` lanes — contract input (SOURCE = KlineSlice[H, L]).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        // Use HL2 (high-low average) as price source
        let price = (h + l) / 2.0;

        if self.price_history.len() >= 4 {
            self.price_history.remove(0);
        }
        self.price_history.push(price);

        if self.price_history.len() < 4 {
            return self.value;
        }

        // smooth = (price + 2*price[1] + 2*price[2] + price[3]) / 6
        let smooth = (self.price_history[3]
                    + 2.0 * self.price_history[2]
                    + 2.0 * self.price_history[1]
                    + self.price_history[0]) / 6.0;

        if self.smooth_history.len() >= 3 {
            self.smooth_history.remove(0);
        }
        self.smooth_history.push(smooth);

        if self.smooth_history.len() < 3 {
            return self.value;
        }

        let smooth_diff = self.smooth_history[2] - 2.0 * self.smooth_history[1] + self.smooth_history[0];
        let mut cycle = self.coeff1 * smooth_diff;

        if !self.cycle_history.is_empty() {
            cycle += self.coeff2 * self.cycle_history[self.cycle_history.len() - 1];
        }
        if self.cycle_history.len() >= 2 {
            cycle -= self.coeff3 * self.cycle_history[self.cycle_history.len() - 2];
        }

        if self.cycle_history.len() >= 2 {
            self.cycle_history.remove(0);
        }
        self.cycle_history.push(cycle);

        self.value = cycle;
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyber_cycle_creation() {
        let cc = EhlersCyberCycle::new(0.07);
        assert!(!cc.is_ready());
        assert_eq!(cc.value(), 0.0);
        assert!((cc.alpha() - 0.07).abs() < 1e-10);
    }

    #[test]
    fn test_cyber_cycle_alpha_clamped() {
        let cc1 = EhlersCyberCycle::new(0.0);
        assert!((cc1.alpha() - 0.01).abs() < 1e-10); // clamped to 0.01

        let cc2 = EhlersCyberCycle::new(1.5);
        assert!((cc2.alpha() - 0.99).abs() < 1e-10); // clamped to 0.99
    }

    #[test]
    fn test_cyber_cycle_basic() {
        let mut cc = EhlersCyberCycle::new(0.07);
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            cc.feed(&[price + 2.0, price - 2.0]);
        }
        assert!(cc.is_ready());
        assert!(cc.value().is_finite());
    }

    #[test]
    fn test_cyber_cycle_reset() {
        let mut cc = EhlersCyberCycle::new(0.07);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            cc.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(cc.is_ready());
        cc.reset();
        assert!(!cc.is_ready());
        assert_eq!(cc.value(), 0.0);
    }

    #[test]
    fn test_cyber_cycle_finite_values() {
        let mut cc = EhlersCyberCycle::new(0.07);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = cc.feed(&[price + 2.0, price - 2.0]);
            assert!(value.is_finite(), "Cyber Cycle should always be finite");
        }
    }
}

impl Default for EhlersCyberCycle {
    fn default() -> Self {
        Self::new(0.07)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for Ehlers Cyber Cycle: alpha parameter (0.01..0.99).
///
/// `alpha` is the sole axis; `Solo(0.07)` is the default (≈ 14-period equivalent).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EhlersCyberCycleConfig {
    /// Smoothing coefficient — controls cycle period sensitivity.
    pub alpha: Param<f64>,
}

impl Indicator for EhlersCyberCycle {
    const ID: IndicatorId = IndicatorId::EhlersCc;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Uses HL2 (high/low average) as price input — fixed, not configurable.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::EhlersCc)];
    /// O(1): fixed-depth history buffers (price×4, smooth×3, cycle×2).
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::fixed(StoreKind::Vec, 4),
            Store::fixed(StoreKind::Vec, 3),
            Store::fixed(StoreKind::Vec, 2),
        ],
    );

    type Config = EhlersCyberCycleConfig;
    type Runtime = EhlersCyberCycle;

    fn create(cfg: EhlersCyberCycleConfig) -> EhlersCyberCycle {
        EhlersCyberCycle::new(cfg.alpha.resolved())
    }
}

impl crate::contract::Config for EhlersCyberCycleConfig {
    fn defaults() -> Self {
        EhlersCyberCycleConfig { alpha: Param::Solo(0.07) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // alpha: Class E (Ehlers EMA-style decay, 0.01..0.99) — sweep_f64(0.01,0.99,0.01).
        let mut s = Self::machine_defaults_auto();
        s.alpha = Param::many(sweep_f64(0.01, 0.99, 0.01));
        s
    }
}


impl Render for EhlersCyberCycle {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::EhlersCc,
                "Cyber Cycle",
                Color::hex(0x00BCD4),
                1.5,
            ))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::EhlersCc(<<EhlersCyberCycle as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 2.0,
                low: price - 2.0,
                close: 9999.0,
                volume: 0.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
