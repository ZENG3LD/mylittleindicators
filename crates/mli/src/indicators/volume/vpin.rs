//! VPIN — Volume-Synchronized Probability of Informed Trading
//!
//! Real implementation: Easley, López de Prado, O'Hara (2012).
//! Primary path: `TickConsumer::update_tick` — Bulk Volume Classification
//!   on live tick stream. Each `bucket_size` volume accumulates a bucket;
//!   VPIN = rolling mean of |buy_vol - sell_vol| / bucket_size over last N buckets.
//!
//! Fallback path: `feed(...)` — SYNTHETIC ESTIMATE only.
//!   A single synthetic tick per bar is injected (close ≥ open → buy side).
//!   Precision limited to bar granularity. Prefer tick stream when available.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{
    Cost, Family, Indicator, Output, Render, SourceAxis, Store, StoreKind,
    UpdateComplexity, Color, RenderSpec,
};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// VPIN — Volume-Synchronized Probability of Informed Trading.
///
/// Range: [0.0, 1.0]. Higher values indicate elevated order-flow toxicity.
#[derive(Debug, Clone)]
pub struct Vpin {
    /// Target volume per bucket (e.g. 50.0 contracts/coins).
    bucket_size: f64,
    /// Number of completed buckets to average for final VPIN.
    smoothing_window: usize,

    // Current in-flight bucket accumulators
    curr_buy: f64,
    curr_sell: f64,
    curr_volume: f64,

    /// VPIN values of completed buckets (rolling window).
    completed_vpins: VecDeque<f64>,
    last_vpin: f64,
}

impl Vpin {
    /// Create a new VPIN indicator.
    ///
    /// * `bucket_size` — target volume per bucket (clamped to 1e-9 minimum).
    /// * `smoothing_window` — number of buckets to average (clamped to 1 minimum).
    pub fn new(bucket_size: f64, smoothing_window: usize) -> Self {
        Self {
            bucket_size: bucket_size.max(1e-9),
            smoothing_window: smoothing_window.max(1),
            curr_buy: 0.0,
            curr_sell: 0.0,
            curr_volume: 0.0,
            completed_vpins: VecDeque::with_capacity(smoothing_window.max(1)),
            last_vpin: 0.0,
        }
    }


}

impl TickConsumer for Vpin {
    fn update_tick(&mut self, tick: &Tick) {
        // BVC: classify volume by trade direction (is_buy flag from exchange)
        if tick.is_buy {
            self.curr_buy += tick.size;
        } else {
            self.curr_sell += tick.size;
        }
        self.curr_volume += tick.size;

        // Finalise buckets until remaining curr_volume < bucket_size
        while self.curr_volume >= self.bucket_size {
            let bucket_vpin = (self.curr_buy - self.curr_sell).abs() / self.bucket_size;

            self.completed_vpins.push_back(bucket_vpin);
            if self.completed_vpins.len() > self.smoothing_window {
                self.completed_vpins.pop_front();
            }

            // Simple approach: reset bucket (ignore overflow carry-over for robustness)
            self.curr_buy = 0.0;
            self.curr_sell = 0.0;
            self.curr_volume = 0.0;
        }

        if !self.completed_vpins.is_empty() {
            let sum: f64 = self.completed_vpins.iter().sum();
            self.last_vpin = sum / self.completed_vpins.len() as f64;
        }

    }


    fn reset(&mut self) {
        self.curr_buy = 0.0;
        self.curr_sell = 0.0;
        self.curr_volume = 0.0;
        self.completed_vpins.clear();
        self.last_vpin = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.completed_vpins.len() >= self.smoothing_window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn buy_tick(size: f64) -> Tick {
        Tick::new(0, 100.0, size, true)
    }

    fn sell_tick(size: f64) -> Tick {
        Tick::new(0, 100.0, size, false)
    }

    #[test]
    fn test_balanced_buckets_zero_vpin() {
        // Interleaved buy+sell of equal size → each completed bucket is 50/50 → VPIN=0
        // bucket_size=20, smoothing_window=2 → need 40 interleaved ticks (2 buckets of 20)
        let mut vpin = Vpin::new(20.0, 2);
        for _ in 0..20 {
            vpin.update_tick(&buy_tick(1.0));
            vpin.update_tick(&sell_tick(1.0));
        }
        assert!(vpin.is_ready());
        assert!((vpin.last_vpin - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_buy_full_bucket_vpin_one() {
        // 10 buy size=10 → 1 bucket fully buy → VPIN ≈ 1.0
        let mut vpin = Vpin::new(100.0, 1);
        for _ in 0..10 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(vpin.is_ready());
        assert!((vpin.last_vpin - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_sell_full_bucket_vpin_one() {
        // 10 sell size=10 → 1 fully-sell bucket → |0 - 100| / 100 = 1.0
        let mut vpin = Vpin::new(100.0, 1);
        for _ in 0..10 {
            vpin.update_tick(&sell_tick(10.0));
        }
        assert!(vpin.is_ready());
        assert!((vpin.last_vpin - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_not_ready_before_window_filled() {
        let mut vpin = Vpin::new(100.0, 3);
        // Only 2 buckets completed
        for _ in 0..20 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(!vpin.is_ready()); // needs 3 buckets
    }

    #[test]
    fn test_reset() {
        let mut vpin = Vpin::new(100.0, 1);
        for _ in 0..10 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(vpin.is_ready());
        vpin.reset();
        assert!(!vpin.is_ready());
        assert_eq!(vpin.value(), 0.0);
        assert_eq!(vpin.curr_volume, 0.0);
    }

    #[test]
    fn test_different_bucket_sizes() {
        // Small bucket: every single tick (size=10) completes bucket_size=10
        let mut vpin = Vpin::new(10.0, 5);
        for _ in 0..5 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(vpin.is_ready());
        // Each bucket is pure buy → vpin = 1.0
        assert!((vpin.last_vpin - 1.0).abs() < 1e-9);
    }

}

impl Default for Vpin {
    fn default() -> Self {
        Self::new(50.0, 50)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`Vpin`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VpinConfig {
    /// Target volume per bucket.
    pub bucket_size: Param<f64>,
    /// Number of completed buckets to average.
    pub smoothing_window: Param<usize>,
}

impl Indicator for Vpin {
    const ID: IndicatorId = IndicatorId::Vpin;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::Vpin),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );

    type Config = VpinConfig;
    type Runtime = Vpin;

    fn create(cfg: VpinConfig) -> Vpin {
        Vpin::new(cfg.bucket_size.resolved(), cfg.smoothing_window.resolved().max(1))
    }
}

impl crate::contract::Config for VpinConfig {
    fn defaults() -> Self {
        VpinConfig {
            bucket_size: Param::Solo(50.0),
            smoothing_window: Param::Solo(50),
        }
    }
    fn machine_defaults() -> Self {
        // smoothing_window: Class A period/window — auto gives range(2,4048,1)
        // bucket_size (f64): PIN — instrument-relative volume bucket size, leave Solo (auto leaves f64 Solo)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Vpin {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vpin, "VPIN", Color::hex(0xF44336))
            .bounds(0.0, 1.0)
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
    fn factory_feeds_resolved_vpin() {
        let mut f = IndicatorOrder::Vpin(
            <<Vpin as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = Tick::new(0, 100.0, 10.0, true);
        f.feed(0, MarketSample::Tick(&t));
        // last_vpin starts at 0.0 until a full bucket — finite regardless
        assert!(f.read(IndicatorOutputId::Vpin).is_finite());
    }
}

impl Vpin {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_vpin
    }
}
