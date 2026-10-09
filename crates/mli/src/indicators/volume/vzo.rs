// Volume Zone Oscillator (VZO) - optimized with O(1) running sum

#[derive(Debug, Clone)]
pub struct Vzo {
    period: usize,
    vol_pos: Vec<f64>,
    vol_neg: Vec<f64>,

    // Running sums for O(1) calculation
    sum_pos: f64,
    sum_neg: f64,

    idx: usize,
    count: usize,
    value: f64,
    prev_close: f64,
    initialized: bool,
}

impl Vzo {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.clamp(2, 1024),
            vol_pos: Vec::with_capacity(period.clamp(2, 1024)),
            vol_neg: Vec::with_capacity(period.clamp(2, 1024)),
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
        self.vol_pos.clear();
        self.vol_neg.clear();
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
    /// Feed resolved lanes `[close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let c = lanes[0];
        let v = lanes[1];
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
        }
        let up = (c >= self.prev_close) as i32 as f64;
        let down = 1.0 - up;
        let pos_v = up * v;
        let neg_v = down * v;

        if self.count < self.period {
            self.vol_pos.push(pos_v);
            self.vol_neg.push(neg_v);
            self.sum_pos += pos_v;
            self.sum_neg += neg_v;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            self.sum_pos -= self.vol_pos[self.idx];
            self.sum_neg -= self.vol_neg[self.idx];
            self.vol_pos[self.idx] = pos_v;
            self.vol_neg[self.idx] = neg_v;
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
    fn test_vzo_creation() {
        let vzo = Vzo::new(14);
        assert!(!vzo.is_ready());
        assert_eq!(vzo.value(), 0.0);
    }

    #[test]
    fn test_vzo_warmup() {
        let mut vzo = Vzo::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vzo.feed(&[price, 1000.0]);
        }
        assert!(vzo.is_ready());
    }

    #[test]
    fn test_vzo_range() {
        let mut vzo = Vzo::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vzo.feed(&[price, 1000.0]);
            assert!(value >= -100.0 && value <= 100.0, "VZO should be in [-100, 100]");
        }
    }

    #[test]
    fn test_vzo_reset() {
        let mut vzo = Vzo::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            vzo.feed(&[price, 1000.0]);
        }
        vzo.reset();
        assert!(!vzo.is_ready());
        assert_eq!(vzo.value(), 0.0);
    }
}

impl Default for Vzo {
    fn default() -> Self {
        Self::new(14)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`Vzo`] — period-only, fixed close+volume source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VzoConfig {
    pub period: Param<usize>,
}

impl Indicator for Vzo {
    const ID: IndicatorId = IndicatorId::Vzo;
    /// Volume Zone Oscillator — standalone volume breadth oscillator, not pluggable.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close (direction) + volume (magnitude) lanes.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(1) ring buffer update — two parallel vecs but updated with running sums.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Vzo)];
    type Config = VzoConfig;
    type Runtime = Vzo;

    fn create(cfg: VzoConfig) -> Vzo {
        Vzo::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for VzoConfig {
    fn defaults() -> Self {
        VzoConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Vzo {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vzo, "VZO", Color::hex(0x009688))
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
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
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Vzo(<<Vzo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // All bars up (close > prev close) → full positive volume → VZO = +100
        let mut price = 100.0;
        for _ in 0..20 {
            price += 1.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: price, volume: 1000.0 });
        }
        let v = f.read(IndicatorOutputId::Vzo);
        assert!(v > 50.0, "all-up bars should give VZO near +100, got {v}");
    }
}
