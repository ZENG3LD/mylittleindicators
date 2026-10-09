// Decycler (Ehlers) — remove cyclic components by subtracting a high-pass filter.
//
// Formula from John Ehlers "Cybernetic Analysis for Stocks and Futures":
//   alpha = cos(2*pi/period) + 2 - sqrt(2*(1 + cos(2*pi/period)))
//   (1st-order HP): hp[i] = (1 - alpha/2)^2 * (price[i] - 2*price[i-1] + price[i-2])
//                           + 2*(1 - alpha) * hp[i-1] - (1 - alpha)^2 * hp[i-2]
//   decycled[i] = price[i] - hp[i]
//
// The decycler removes cycles shorter than `period` bars, leaving the trend component.

use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct Decycler {
    /// HP filter cutoff period in bars.
    period: f64,
    /// Alpha coefficient derived from period.
    alpha: f64,
    /// Coefficient (1 - alpha/2)^2
    c0: f64,
    /// Coefficient 2*(1 - alpha)
    c1: f64,
    /// Coefficient (1 - alpha)^2
    c2: f64,
    prev_price: [f64; 2],
    prev_hp: [f64; 2],
    value: f64,
    count: usize,
}

impl Decycler {
    /// Create Decycler with given HP filter cutoff period (in bars).
    /// Typical values: 40–100 bars to remove short cycles.
    pub fn new(period: f64) -> Self {
        let p = period.max(2.0);
        let cos_val = (2.0 * PI / p).cos();
        let alpha = cos_val + 2.0 - (2.0 * (1.0 + cos_val)).sqrt();
        let c0 = (1.0 - alpha / 2.0).powi(2);
        let c1 = 2.0 * (1.0 - alpha);
        let c2 = (1.0 - alpha).powi(2);
        Self {
            period: p,
            alpha,
            c0,
            c1,
            c2,
            prev_price: [0.0; 2],
            prev_hp: [0.0; 2],
            value: 0.0,
            count: 0,
        }
    }

    /// Create from a legacy alpha parameter (for backward compatibility).
    /// When called with an alpha value in [0, 1], approximates period = 2/alpha.
    pub fn from_alpha(alpha: f64) -> Self {
        let a = alpha.clamp(0.001, 1.0);
        // Back-compute period from alpha: alpha ≈ 2/period → period ≈ 2/alpha
        let period = 2.0 / a;
        Self::new(period)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prev_price = [0.0; 2];
        self.prev_hp = [0.0; 2];
        self.value = 0.0;
        self.count = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= 3
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        self.count += 1;
        if self.count < 3 {
            // Seed: accumulate prices for first 2 bars
            if self.count == 1 {
                self.prev_price[1] = c;
                self.prev_hp[1] = 0.0;
            } else {
                self.prev_price[0] = self.prev_price[1];
                self.prev_hp[0] = self.prev_hp[1];
                self.prev_price[1] = c;
                self.prev_hp[1] = 0.0;
            }
            self.value = c;
            return self.value;
        }

        // HP filter (2nd-order Butterworth high-pass)
        let hp = self.c0 * (c - 2.0 * self.prev_price[1] + self.prev_price[0])
            + self.c1 * self.prev_hp[1]
            - self.c2 * self.prev_hp[0];

        // Decycled = price - high-pass (= low-pass component = trend)
        self.value = c - hp;

        // Shift state
        self.prev_price[0] = self.prev_price[1];
        self.prev_price[1] = c;
        self.prev_hp[0] = self.prev_hp[1];
        self.prev_hp[1] = hp;

        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn period(&self) -> f64 {
        self.period
    }

}

impl Default for Decycler {
    /// Factory default: from_alpha(0.1) — equivalent to period ≈ 20 bars.
    fn default() -> Self {
        Self::from_alpha(0.1)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`Decycler`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DecycConfig {
    pub period: Param<usize>,
}

impl Indicator for Decycler {
    const ID: IndicatorId = IndicatorId::Decyc;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): 4 scalar state registers, no heap.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[crate::contract::Output::price(IndicatorOutputId::Decyc)];
    type Config = DecycConfig;
    type Runtime = Decycler;

    fn create(cfg: DecycConfig) -> Decycler {
        Decycler::new(cfg.period.resolved() as f64)
    }

    fn source_fields(cfg: &DecycConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for DecycConfig {
    fn defaults() -> Self {
        DecycConfig { period: Param::Solo(40) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for Decycler {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Decyc, "Decycler", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decycler_creation() {
        let dc = Decycler::new(40.0);
        assert!(!dc.is_ready());
        assert_eq!(dc.value(), 0.0);
        assert!(dc.period() > 0.0);
    }

    #[test]
    fn test_decycler_from_alpha() {
        // Ensure from_alpha doesn't panic
        let dc = Decycler::from_alpha(0.05);
        assert!(!dc.is_ready());
    }

    #[test]
    fn test_decycler_finite() {
        let mut dc = Decycler::new(40.0);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = dc.feed(price);
            assert!(value.is_finite(), "Decycler should always be finite at bar {}", i);
        }
    }

    #[test]
    fn test_decycler_tracks_trend() {
        // On a pure linear trend, decycler should output values close to price
        let mut dc = Decycler::new(40.0);
        let mut last_val = 0.0;
        for i in 1..=100 {
            let price = 100.0 + i as f64 * 0.5;
            last_val = dc.feed(price);
        }
        // Decycled should be near price (HP removes little trend)
        assert!((last_val - 150.0).abs() < 10.0, "Decycler should track trend, got {}", last_val);
    }

    #[test]
    fn factory_feeds_resolved_decyc() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<Decycler as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Decyc(cfg).build_solo().unwrap();
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }

    #[test]
    fn test_decycler_reset() {
        let mut dc = Decycler::new(40.0);
        for i in 1..=20 {
            dc.feed(100.0 + i as f64);
        }
        dc.reset();
        assert!(!dc.is_ready());
        assert_eq!(dc.value(), 0.0);
    }
}
