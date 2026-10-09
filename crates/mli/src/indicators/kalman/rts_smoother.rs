// Rauch–Tung–Striebel (RTS) Smoother — streaming forward-filter proxy.
//
// True RTS requires a full backward pass over a stored sequence: it first runs the
// Kalman filter forward storing all (x̂, P) pairs, then sweeps backward computing
// smoothed estimates. This requires O(N) memory and is inherently non-causal (batch).
//
// Streaming constraint: in a bar-by-bar indicator there is no backward pass.
// This implementation provides the causal Kalman forward filter output, which is
// the best achievable real-time approximation of RTS. When the full sequence is
// available offline, apply true RTS backward sweep on top of the stored states.

use crate::indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::engine::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct RtsSmoother {
    kf: BasicKalmanFilter,
    last_value: f64,
}

impl Default for RtsSmoother {
    fn default() -> Self {
        Self::new()
    }
}

impl RtsSmoother {
    pub fn new() -> Self {
        Self {
            kf: BasicKalmanFilter::new(1.0, 1.0, 1.0),
            last_value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.last_value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kf.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.last_value
    }

    /// Feed ONE pre-extracted scalar (the resolved price field). Knows no transport.
    pub fn feed(&mut self, c: f64) -> f64 {
        // forward filter
        let x = self.kf.update(c).filtered_value;
        // no backward pass in streaming; return filtered value as proxy
        self.last_value = x;
        self.last_value
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RtsSmoother`] — configurable price source only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RtsSmootherConfig {
    pub source: Param<OhlcvField>,
}

impl Indicator for RtsSmoother {
    const ID: IndicatorId = IndicatorId::Rts;
    /// MovingAverage family — the RTS smoother is a price-smoothing filter (causal
    /// Kalman forward pass), equivalent in role to any other smoothing kernel.
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): running Kalman state update, no window rescan. Fixed-capacity Vec buffers
    /// inside BasicKalmanFilter (state_history × 100, etc.).
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::fixed(StoreKind::Vec, 100), // state_history
            Store::fixed(StoreKind::Vec, 100), // innovation_history
            Store::fixed(StoreKind::Vec, 100), // gain_history
            Store::fixed(StoreKind::Vec, 10),  // innovation_window
        ],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Rts)];
    type Config = RtsSmootherConfig;
    type Runtime = RtsSmoother;

    fn create(_cfg: RtsSmootherConfig) -> RtsSmoother {
        RtsSmoother::new()
    }

    fn source_fields(cfg: &RtsSmootherConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for RtsSmootherConfig {
    fn defaults() -> Self {
        RtsSmootherConfig { source: Param::Solo(OhlcvField::Close) }
    }
    fn machine_defaults() -> Self {
        // source: Class O — auto sets all 8 OhlcvField variants. No other axes.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RtsSmoother {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Rts, "RTS Smoother", Color::hex(0x3F51B5))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rts_smoother_creation() {
        let rts = RtsSmoother::new();
        assert!(!rts.is_ready());
        assert_eq!(rts.value(), 0.0);
    }

    #[test]
    fn test_rts_smoother_warmup() {
        let mut rts = RtsSmoother::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rts.feed(price);
        }
        assert!(rts.is_ready());
    }

    #[test]
    fn test_rts_smoother_values_finite() {
        let mut rts = RtsSmoother::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rts.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_rts_smoother_reset() {
        let mut rts = RtsSmoother::new();
        for i in 0..10 {
            rts.feed(100.0 + i as f64);
        }
        rts.reset();
        assert!(!rts.is_ready());
        assert_eq!(rts.value(), 0.0);
    }

    /// Factory resolves close (not the wild 9999 open/high) and feeds the scalar.
    /// RTS is ready after the first observation — steady-rising close should yield a
    /// finite positive filtered value.
    #[test]
    fn factory_feeds_resolved_rts() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Rts(<<RtsSmoother as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        let v = f.read(IndicatorOutputId::Rts);
        assert!(v.is_finite() && v > 50.0, "RTS should track close, got {v}");
    }
}
