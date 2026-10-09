// Price Zone Oscillator (PZO) - optimized with O(1) running sum


#[derive(Debug, Clone)]
pub struct Pzo {
    period: usize,
    pos: Vec<f64>,
    neg: Vec<f64>,

    // Running sums for O(1) calculation
    sum_pos: f64,
    sum_neg: f64,

    idx: usize,
    count: usize,
    value: f64,
    prev_close: f64,
    initialized: bool,
}

impl Pzo {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.clamp(2, 1024),
            pos: Vec::with_capacity(period.clamp(2, 1024)),
            neg: Vec::with_capacity(period.clamp(2, 1024)),
            sum_pos: 0.0,
            sum_neg: 0.0,
            idx: 0,
            count: 0,
            value: 0.0,
            prev_close: 0.0,
            initialized: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.pos.clear();
        self.neg.clear();
        self.sum_pos = 0.0;
        self.sum_neg = 0.0;
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
        self.prev_close = 0.0;
        self.initialized = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar (close) — contract input (SOURCE = Field{Close}).
    pub fn feed(&mut self, c: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
        }
        let diff = c - self.prev_close;
        let pos_v = diff.max(0.0);
        let neg_v = (-diff).max(0.0);

        if self.count < self.period {
            self.pos.push(pos_v);
            self.neg.push(neg_v);
            self.sum_pos += pos_v;
            self.sum_neg += neg_v;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            self.sum_pos -= self.pos[self.idx];
            self.sum_neg -= self.neg[self.idx];
            self.pos[self.idx] = pos_v;
            self.neg[self.idx] = neg_v;
            self.sum_pos += pos_v;
            self.sum_neg += neg_v;
            self.idx = (self.idx + 1) % self.period;
        }
        self.prev_close = c;

        let denom = (self.sum_pos + self.sum_neg).abs().max(1e-9);
        self.value = 100.0 * (self.sum_pos - self.sum_neg) / denom;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pzo_creation() {
        let pzo = Pzo::new(14);
        assert!(!pzo.is_ready());
        assert_eq!(pzo.value(), 0.0);
    }

    #[test]
    fn test_pzo_warmup() {
        let mut pzo = Pzo::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pzo.feed(price);
        }
        assert!(pzo.is_ready());
    }

    #[test]
    fn test_pzo_values_finite() {
        let mut pzo = Pzo::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pzo.feed(price);
            assert!(value.is_finite(), "PZO should be finite");
        }
    }

    #[test]
    fn test_pzo_reset() {
        let mut pzo = Pzo::new(14);
        for i in 0..20 {
            pzo.feed(100.0 + i as f64);
        }
        pzo.reset();
        assert!(!pzo.is_ready());
        assert_eq!(pzo.value(), 0.0);
    }
}

impl Default for Pzo {
    fn default() -> Self {
        Self::new(14)
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

/// Own config for [`Pzo`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PzoConfig {
    pub period: Param<usize>,
}

impl Indicator for Pzo {
    const ID: IndicatorId = IndicatorId::Pzo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Pzo)];
    /// O(1): period-bounded ring buffer with running sums.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Vec), // pos ring
            Store::window(StoreKind::Vec), // neg ring
        ],
    );

    type Config = PzoConfig;
    type Runtime = Pzo;

    fn create(cfg: PzoConfig) -> Pzo {
        Pzo::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for PzoConfig {
    fn defaults() -> Self {
        PzoConfig { period: Param::Solo(14) }
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


impl Render for Pzo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::Pzo,
                "PZO",
                Color::hex(0x2196F3),
                1.5,
            ))
            .bounds(-100.0, 100.0)
            .zero_baseline()
            .precision(2)
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
        let mut f = IndicatorOrder::Pzo(<<Pzo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..20 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: 100.0 + i as f64,
                volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= -100.0 && v <= 100.0, "PZO must be in [-100,100], got {v}");
    }
}
