//! Uptick/Downtick Volume — rolling buy-side and sell-side volume from tick stream.
//!
//! Tracks separate rolling sums for buy (uptick) and sell (downtick) volume
//! over the last N ticks.
//!
//! Outputs: `uptick_volume`, `downtick_volume`
//!   where uptick = total buy-side volume, downtick = total sell-side volume.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{
    Cost, Family, Indicator, Output, Render, SourceAxis, Store, StoreKind,
    UpdateComplexity, Color, RenderOutput, RenderSpec,
};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Uptick/Downtick Volume — separate rolling buy/sell volume totals.
#[derive(Debug, Clone)]
pub struct UptickDowntickVolume {
    rolling_window_ticks: usize,
    /// Ring buffer of (size, is_buy) per tick.
    tick_history: VecDeque<(f64, bool)>,
    last_uptick: f64,
    last_downtick: f64,
}

impl UptickDowntickVolume {
    /// Create with `window` ticks lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(1);
        Self {
            rolling_window_ticks: cap,
            tick_history: VecDeque::with_capacity(cap),
            last_uptick: 0.0,
            last_downtick: 0.0,
        }
    }
}

impl TickConsumer for UptickDowntickVolume {
    fn update_tick(&mut self, tick: &Tick) {
        self.tick_history.push_back((tick.size, tick.is_buy));
        if self.tick_history.len() > self.rolling_window_ticks {
            self.tick_history.pop_front();
        }

        let (mut up, mut down) = (0.0_f64, 0.0_f64);
        for &(sz, is_buy) in &self.tick_history {
            if is_buy {
                up += sz;
            } else {
                down += sz;
            }
        }

        self.last_uptick = up;
        self.last_downtick = down;

    }


    fn reset(&mut self) {
        self.tick_history.clear();
        self.last_uptick = 0.0;
        self.last_downtick = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.tick_history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(size: f64, is_buy: bool) -> Tick {
        Tick::new(0, 100.0, size, is_buy)
    }

    #[test]
    fn test_buy_sell_split() {
        // 3 buy size=10, 2 sell size=20 → uptick=30, downtick=40
        let mut udv = UptickDowntickVolume::new(10);
        for _ in 0..3 {
            udv.update_tick(&tick(10.0, true));
        }
        for _ in 0..2 {
            udv.update_tick(&tick(20.0, false));
        }
        assert!((udv.last_uptick - 30.0).abs() < 1e-9);
        assert!((udv.last_downtick - 40.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_returns_zeros() {
        let udv = UptickDowntickVolume::new(10);
        assert!(!udv.is_ready());
        assert_eq!(udv.uptick(), 0.0);
        assert_eq!(udv.downtick(), 0.0);
    }

    #[test]
    fn test_reset() {
        let mut udv = UptickDowntickVolume::new(5);
        udv.update_tick(&tick(10.0, true));
        udv.reset();
        assert!(!udv.is_ready());
        assert_eq!(udv.uptick(), 0.0);
        assert_eq!(udv.downtick(), 0.0);
    }

    #[test]
    fn test_rolling_window_evicts_buys() {
        // window=3: push 3 buys then 3 sells → only sells remain
        let mut udv = UptickDowntickVolume::new(3);
        for _ in 0..3 {
            udv.update_tick(&tick(10.0, true));
        }
        for _ in 0..3 {
            udv.update_tick(&tick(5.0, false));
        }
        // window = [sell, sell, sell]
        assert!((udv.last_uptick - 0.0).abs() < 1e-9);
        assert!((udv.last_downtick - 15.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_buy() {
        let mut udv = UptickDowntickVolume::new(5);
        for _ in 0..5 {
            udv.update_tick(&tick(10.0, true));
        }
        assert!((udv.last_uptick - 50.0).abs() < 1e-9);
        assert!((udv.last_downtick - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_sell() {
        let mut udv = UptickDowntickVolume::new(5);
        for _ in 0..5 {
            udv.update_tick(&tick(7.0, false));
        }
        assert!((udv.last_uptick - 0.0).abs() < 1e-9);
        assert!((udv.last_downtick - 35.0).abs() < 1e-9);
    }
}

impl UptickDowntickVolume {
    #[inline]
    pub fn uptick(&self) -> f64 {
        self.last_uptick
    }

    #[inline]
    pub fn downtick(&self) -> f64 {
        self.last_downtick
    }
}

impl Default for UptickDowntickVolume {
    fn default() -> Self {
        Self::new(100)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`UptickDowntickVolume`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct UptickDowntickVolumeConfig {
    /// Rolling window size in ticks.
    pub window: Param<usize>,
}

impl Indicator for UptickDowntickVolume {
    const ID: IndicatorId = IndicatorId::UptickDowntickVolume;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::count(IndicatorOutputId::UptickDowntickVolumeUptick),
        Output::count(IndicatorOutputId::UptickDowntickVolumeDowntick),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );

    type Config = UptickDowntickVolumeConfig;
    type Runtime = UptickDowntickVolume;

    fn create(cfg: UptickDowntickVolumeConfig) -> UptickDowntickVolume {
        UptickDowntickVolume::new(cfg.window.resolved().max(1))
    }
}

impl crate::contract::Config for UptickDowntickVolumeConfig {
    fn defaults() -> Self {
        UptickDowntickVolumeConfig { window: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // window: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for UptickDowntickVolume {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::UptickDowntickVolumeUptick,
                "Uptick Vol",
                Color::hex(0x4CAF50),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::UptickDowntickVolumeDowntick,
                "Downtick Vol",
                Color::hex(0xF44336),
                2.0,
            ))
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
    fn factory_feeds_resolved_uptick_downtick_volume() {
        let mut f = IndicatorOrder::UptickDowntickVolume(
            <<UptickDowntickVolume as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = Tick::new(0, 100.0, 5.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.read(IndicatorOutputId::UptickDowntickVolumeUptick).is_finite());
    }
}
