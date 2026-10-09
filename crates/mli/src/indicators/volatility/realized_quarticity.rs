// Realized Quarticity - estimator for variance of volatility (fourth power of returns)

#[derive(Debug, Clone)]
pub struct RealizedQuarticity {
    window: usize,
    r4_buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    /// Running sum of r^4 values in the ring -- O(1) eviction: `sum += new - old`.
    sum_r4: f64,
    value: f64,
}

impl RealizedQuarticity {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            r4_buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            prev_close: 0.0,
            sum_r4: 0.0,
            value: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.r4_buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.sum_r4 = 0.0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE resolved scalar (close) — close-to-close realized estimator.
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.prev_close <= 0.0 {
            self.prev_close = c.max(1e-12);
            return self.value;
        }
        let r = (c / self.prev_close).ln();
        self.prev_close = c.max(1e-12);
        let r4 = r * r * r * r;

        // O(1) running-sum eviction: subtract the slot being overwritten, add new.
        let old = self.r4_buffer[self.idx];
        self.r4_buffer[self.idx] = r4;
        self.sum_r4 += r4 - old;

        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n > 0.0 {
            // Scale by 10^6 -- fourth-power returns are vanishingly small otherwise.
            self.value = (self.sum_r4 / n) * 1_000_000.0;
        }
        self.value
    }
}

// -- contract -----------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`RealizedQuarticity`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RqConfig {
    pub period: Param<usize>,
}

/// Period-deep ring of r^4 values plus the running sum scalar.
/// The ring buffer is Vec(Window); the scalar running sum is implicit in `sum_r4`.
static RQ_STORES: &[Store] = &[Store::window(StoreKind::Vec)];

impl Indicator for RealizedQuarticity {
    const ID: IndicatorId = IndicatorId::Rq;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close-to-close returns -- reads close intrinsically, not a swept field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// O(1): running sum eviction -- `sum_r4 += r4 - old` -- no per-bar rescan.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, RQ_STORES);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Rq)];
    type Config = RqConfig;
    type Runtime = RealizedQuarticity;

    fn create(cfg: RqConfig) -> RealizedQuarticity {
        RealizedQuarticity::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for RqConfig {
    fn defaults() -> Self {
        RqConfig { period: Param::Solo(30) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RealizedQuarticity {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rq, "Realized Quart", Color::hex(0x009688))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realized_quarticity_creation() {
        let rq = RealizedQuarticity::new(20);
        assert!(!rq.is_ready());
        assert_eq!(rq.value(), 0.0);
    }

    #[test]
    fn test_realized_quarticity_warmup() {
        let mut rq = RealizedQuarticity::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rq.feed(price);
        }
        assert!(rq.is_ready());
    }

    #[test]
    fn test_realized_quarticity_non_negative() {
        let mut rq = RealizedQuarticity::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rq.feed(price);
            assert!(value >= 0.0, "Realized quarticity should be non-negative");
        }
    }

    #[test]
    fn test_realized_quarticity_reset() {
        let mut rq = RealizedQuarticity::new(20);
        for i in 0..25 {
            rq.feed(100.0 + i as f64);
        }
        rq.reset();
        assert!(!rq.is_ready());
        assert_eq!(rq.value(), 0.0);
    }
}
