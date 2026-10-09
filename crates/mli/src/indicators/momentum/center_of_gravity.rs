// Center of Gravity (COG) — Ehlers oscillator.
//
// Formula from John Ehlers "Cybernetic Analysis for Stocks and Futures":
//   COG = -sum(price[i] * (i+1), i=0..N-1) / sum(price[i], i=0..N-1)  +  (N+1)/2
//
// The result is centred around 0. Negative = recent bars heavier (uptrend).
// Positive = older bars heavier (downtrend).


#[derive(Debug, Clone)]
pub struct CenterOfGravity {
    period: usize,
    buf: Vec<f64>,
    idx: usize,
    count: usize,
    value: f64,
}

impl CenterOfGravity {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.clamp(2, 512),
            buf: Vec::with_capacity(period.clamp(2, 512)),
            idx: 0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn period(&self) -> usize {
        self.period
    }

    /// Feed ONE pre-extracted scalar (close) — contract input.
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.count < self.period {
            self.buf.push(c);
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            self.buf[self.idx] = c;
            self.idx = (self.idx + 1) % self.period;
        }
        if self.is_ready() {
            let mut num = 0.0;
            let mut den = 0.0;
            let n = self.period;
            for i in 0..n {
                let price = self.buf[(self.idx + i) % n];
                let w = (i + 1) as f64;
                num += w * price;
                den += price;
            }
            self.value = if den.abs() > 1e-12 {
                -(num / den) + (n as f64 + 1.0) / 2.0
            } else {
                0.0
            };
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cog_creation() {
        let cog = CenterOfGravity::new(10);
        assert!(!cog.is_ready());
        assert_eq!(cog.value(), 0.0);
        assert_eq!(cog.period(), 10);
    }

    #[test]
    fn test_cog_min_period() {
        let cog = CenterOfGravity::new(1);
        assert_eq!(cog.period(), 2); // min period is 2
    }

    #[test]
    fn test_cog_max_period() {
        let cog = CenterOfGravity::new(1000);
        assert_eq!(cog.period(), 512); // max period is 512
    }

    #[test]
    fn test_cog_is_ready_timing() {
        let mut cog = CenterOfGravity::new(5);
        for i in 1..=10 {
            let price = 100.0 + i as f64;
            cog.feed(price);
            if i < 5 {
                assert!(!cog.is_ready(), "COG should not be ready at bar {}", i);
            } else {
                assert!(cog.is_ready(), "COG should be ready at bar {}", i);
            }
        }
    }

    #[test]
    fn test_cog_constant_price() {
        let mut cog = CenterOfGravity::new(5);
        for _ in 0..20 {
            cog.feed(100.0);
        }
        assert!(cog.is_ready());
        // For constant prices, COG = -(sum(i*p)/sum(p)) + (n+1)/2
        // = -(p*sum(i))/(p*n) + (n+1)/2 = -sum(i)/n + (n+1)/2
        // For n=5: sum(i)=1+2+3+4+5=15, -(15/5) + 3 = -3 + 3 = 0
        assert!((cog.value()).abs() < 1e-10, "COG for constant prices should be near 0");
    }

    #[test]
    fn test_cog_reset() {
        let mut cog = CenterOfGravity::new(5);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            cog.feed(price);
        }
        assert!(cog.is_ready());
        cog.reset();
        assert!(!cog.is_ready());
        assert_eq!(cog.value(), 0.0);
    }

    #[test]
    fn test_cog_finite_values() {
        let mut cog = CenterOfGravity::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = cog.feed(price);
            assert!(value.is_finite(), "COG should always be finite");
        }
    }

    #[test]
    fn test_cog_uptrend() {
        let mut cog = CenterOfGravity::new(10);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            cog.feed(price);
        }
        assert!(cog.is_ready());
        // In uptrend, recent prices are higher, COG should be negative (recent > average)
        assert!(cog.value() < 0.0, "COG should be negative in uptrend (recent prices weighted higher)");
    }

    #[test]
    fn test_cog_downtrend() {
        let mut cog = CenterOfGravity::new(10);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            cog.feed(price);
        }
        assert!(cog.is_ready());
        // In downtrend, recent prices are lower, COG should be positive
        assert!(cog.value() > 0.0, "COG should be positive in downtrend");
    }
}

impl Default for CenterOfGravity {
    fn default() -> Self {
        Self::new(10)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`CenterOfGravity`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CogConfig {
    pub period: Param<usize>,
}

impl Indicator for CenterOfGravity {
    const ID: IndicatorId = IndicatorId::Cog;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cog)];
    /// O(period): each bar sums the price-weighted window.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    type Config = CogConfig;
    type Runtime = CenterOfGravity;

    fn create(cfg: CogConfig) -> CenterOfGravity {
        CenterOfGravity::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for CogConfig {
    fn defaults() -> Self {
        CogConfig { period: Param::Solo(10) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for CenterOfGravity {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::Cog, "COG", Color::hex(0x9C27B0), 1.5))
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
    fn factory_feeds_resolved_source() {
        let mut f = IndicatorOrder::Cog(<<CenterOfGravity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=20 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: 100.0 + i as f64,
                volume: 0.0,
            });
        }
        // COG is overlay; value should be finite and near price range
        assert!(f.primary().is_finite());
    }
}
