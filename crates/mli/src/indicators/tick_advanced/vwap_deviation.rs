//! VwapDeviation — rolling VWAP with deviation from current price.
//!
//! Maintains a time-windowed VWAP and reports the percent deviation of the
//! current price from that VWAP.
//!
//! Outputs: `current_price`, `vwap`, `deviation_pct`
//!   deviation_pct = (price - vwap) / vwap, e.g. 0.01 = 1% above VWAP.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Rolling VWAP deviation indicator.
///
/// Computes volume-weighted average price over a rolling `window_ms` millisecond
/// window and returns the percent deviation of the current price from that VWAP.
#[derive(Debug, Clone)]
pub struct VwapDeviation {
    window_ms: i64,
    /// (timestamp_ms, price, qty)
    events: VecDeque<(i64, f64, f64)>,
    last_price: f64,
    last_vwap: f64,
    last_deviation: f64,
}

impl VwapDeviation {
    /// Create with rolling `window_ms` millisecond window (default 60 000 ms = 1 min).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::with_capacity(512),
            last_price: 0.0,
            last_vwap: 0.0,
            last_deviation: 0.0,
        }
    }
}

impl VwapDeviation {
    /// Current tick price.
    pub fn price(&self) -> f64 {
        self.last_price
    }

    /// Rolling volume-weighted average price.
    pub fn vwap(&self) -> f64 {
        self.last_vwap
    }

    /// Percent deviation of current price from VWAP: `(price - vwap) / vwap`.
    pub fn deviation(&self) -> f64 {
        self.last_deviation
    }
}

impl TickConsumer for VwapDeviation {
    fn update_tick(&mut self, tick: &Tick) {
        self.events.push_back((tick.time, tick.price, tick.size));

        // Evict ticks outside the time window.
        while let Some(&(ts, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let total_qty: f64 = self.events.iter().map(|&(_, _, q)| q).sum();
        let vwap = if total_qty > 0.0 {
            self.events.iter().map(|&(_, p, q)| p * q).sum::<f64>() / total_qty
        } else {
            tick.price
        };

        self.last_price = tick.price;
        self.last_vwap = vwap;
        self.last_deviation = if vwap > 0.0 {
            (tick.price - vwap) / vwap
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_price = 0.0;
        self.last_vwap = 0.0;
        self.last_deviation = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_at(time_ms: i64, price: f64, size: f64) -> Tick {
        Tick::new(time_ms, price, size, true)
    }

    #[test]
    fn single_tick_deviation_is_zero() {
        let mut ind = VwapDeviation::new(60_000);
        // With only one tick, VWAP == price → deviation == 0.
        ind.update_tick(&tick_at(0, 100.0, 1.0));
        assert!((ind.price() - 100.0).abs() < 1e-9);
        assert!((ind.vwap() - 100.0).abs() < 1e-9);
        assert!(ind.deviation().abs() < 1e-9);
    }

    #[test]
    fn deviation_above_vwap() {
        // Two ticks: large volume at 100, small volume at 110.
        // VWAP ≈ (100*10 + 110*1) / 11 ≈ 101.0
        // deviation = (110 - ~101) / ~101 > 0
        let mut ind = VwapDeviation::new(60_000);
        ind.update_tick(&tick_at(0, 100.0, 10.0));
        ind.update_tick(&tick_at(1, 110.0, 1.0));
        assert!((ind.price() - 110.0).abs() < 1e-9);
        let expected_vwap = (100.0 * 10.0 + 110.0 * 1.0) / 11.0;
        assert!((ind.vwap() - expected_vwap).abs() < 1e-9);
        assert!(ind.deviation() > 0.0, "price above vwap → positive deviation");
    }

    #[test]
    fn old_ticks_evicted_by_window() {
        let mut ind = VwapDeviation::new(1_000); // 1 second window
        ind.update_tick(&tick_at(0, 100.0, 10.0));
        // New tick 2 seconds later — old tick evicted, VWAP resets to new price.
        ind.update_tick(&tick_at(2_100, 200.0, 1.0));
        assert!((ind.price() - 200.0).abs() < 1e-9);
        assert!((ind.vwap() - 200.0).abs() < 1e-9, "only one tick → vwap == price");
        assert!(ind.deviation().abs() < 1e-9);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = VwapDeviation::new(60_000);
        ind.update_tick(&tick_at(0, 100.0, 1.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.price(), 0.0);
        assert_eq!(ind.vwap(), 0.0);
        assert_eq!(ind.deviation(), 0.0);
    }
}

impl Default for VwapDeviation {
    /// Factory default: window_ms=60000.
    fn default() -> Self {
        Self::new(60_000)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`VwapDeviation`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VwapDeviationConfig {
    pub window: Param<TimeWindow>,
}

impl Indicator for VwapDeviation {
    const ID: IndicatorId = IndicatorId::VwapDeviation;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::VwapDeviationPrice),
        Output::price(IndicatorOutputId::VwapDeviationVwap),
        Output::centered(IndicatorOutputId::VwapDeviationDeviation),
    ];
    type Config = VwapDeviationConfig;
    type Runtime = VwapDeviation;

    fn create(cfg: VwapDeviationConfig) -> VwapDeviation {
        VwapDeviation::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for VwapDeviationConfig {
    fn defaults() -> Self {
        VwapDeviationConfig {
            window: Param::Solo(TimeWindow::Minutes(1)),
        }
    }
    fn machine_defaults() -> Self {
        // window (TimeWindow): Class N — 11-point curated discrete set.
        let mut s = Self::machine_defaults_auto();
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VwapDeviation {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::VwapDeviationPrice, "Price", Color::hex(0xB0BEC5), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::VwapDeviationVwap, "VWAP", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::VwapDeviationDeviation, "Deviation %", Color::hex(0x2196F3), 1.5))
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
    fn factory_feeds_resolved_vwap_deviation() {
        let mut f = IndicatorOrder::VwapDeviation(
            <<VwapDeviation as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}
