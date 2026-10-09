//! Liquidation Volume Imbalance — rolling long vs short liquidation volume ratio.
//!
//! Measures the balance of forced-close volume between long and short positions
//! over a rolling window.
//!
//! # Output
//! `Triple(imbalance, long_vol, short_vol)`
//!
//! - `imbalance ∈ [-1, 1]`:
//!   - `+1.0` — all volume is short liquidations (shorts forced-buy → bullish pressure).
//!   - `-1.0` — all volume is long liquidations (longs forced-sell → bearish pressure).
//!   - `0.0`  — balanced.
//! - `long_vol`  — cumulative quote volume of long liquidations in window.
//! - `short_vol` — cumulative quote volume of short liquidations in window.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::{Liquidation, TradeSide};

/// Rolling liquidation volume imbalance.
#[derive(Clone, Debug)]
pub struct LiquidationVolumeImbalance {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Buffered events: (timestamp, quote_value, side).
    events: VecDeque<(i64, f64, TradeSide)>,
    /// Cached imbalance.
    last_imbalance: f64,
    /// Cached long volume.
    last_long_vol: f64,
    /// Cached short volume.
    last_short_vol: f64,
}

impl Default for LiquidationVolumeImbalance {
    fn default() -> Self {
        Self::new(60_000)
    }
}

impl LiquidationVolumeImbalance {
    /// Create with the given rolling window.
    ///
    /// `window_ms` — window size in milliseconds.
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_imbalance: 0.0,
            last_long_vol: 0.0,
            last_short_vol: 0.0,
        }
    }

    /// Rolling long/short volume imbalance in `[-1, 1]`.
    pub fn imbalance(&self) -> f64 {
        self.last_imbalance
    }

    /// Cumulative quote volume of long liquidations in the window.
    pub fn long_vol(&self) -> f64 {
        self.last_long_vol
    }

    /// Cumulative quote volume of short liquidations in the window.
    pub fn short_vol(&self) -> f64 {
        self.last_short_vol
    }

    fn evict(&mut self, now: i64) {
        while let Some(&(ts, _, _)) = self.events.front() {
            if now - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn recompute(&mut self) {
        let mut long_vol = 0.0_f64;
        let mut short_vol = 0.0_f64;
        for &(_, val, side) in &self.events {
            // TradeSide::Buy = long was liquidated (forced sell)
            // TradeSide::Sell = short was liquidated (forced buy)
            match side {
                TradeSide::Buy => long_vol += val,
                TradeSide::Sell => short_vol += val,
            }
        }
        let total = long_vol + short_vol;
        self.last_long_vol = long_vol;
        self.last_short_vol = short_vol;
        // positive = more short liquidations = bullish pressure (shorts forced to buy)
        self.last_imbalance = if total > 0.0 {
            (short_vol - long_vol) / total
        } else {
            0.0
        };
    }
}

impl LiquidationConsumer for LiquidationVolumeImbalance {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        self.events.push_back((liq.timestamp, liq.quote_value(), liq.side));
        self.evict(liq.timestamp);
        self.recompute();
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_imbalance = 0.0;
        self.last_long_vol = 0.0;
        self.last_short_vol = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

use crate::contract::Param;

/// Typed dual-mode configuration for [`LiquidationVolumeImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LiquidationVolumeImbalanceConfig {
    /// Rolling window as a typed [`TimeWindow`].
    pub window: Param<TimeWindow>,
}

impl Indicator for LiquidationVolumeImbalance {
    const ID: IndicatorId = IndicatorId::LiquidationVolumeImbalance;
    const FAMILY: &'static [Family] = &[Family::Liquidations];
    const INPUT: &'static [StreamKind] = &[StreamKind::Liquidation];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::LiquidationVolumeImbalanceImbalance),
        Output::count(IndicatorOutputId::LiquidationVolumeImbalanceLongVol),
        Output::count(IndicatorOutputId::LiquidationVolumeImbalanceShortVol),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = LiquidationVolumeImbalanceConfig;
    type Runtime = LiquidationVolumeImbalance;

    fn create(cfg: LiquidationVolumeImbalanceConfig) -> LiquidationVolumeImbalance {
        LiquidationVolumeImbalance::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for LiquidationVolumeImbalanceConfig {
    fn defaults() -> Self {
        LiquidationVolumeImbalanceConfig {
            window: Param::Solo(TimeWindow::Minutes(1)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: Class N (TimeWindow enum) — 11-point curated discrete set.
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s
    }
}


impl Render for LiquidationVolumeImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LiquidationVolumeImbalanceImbalance, "Imbalance", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::LiquidationVolumeImbalanceLongVol, "Long Vol", Color::hex(0x4CAF50))
            .line_output(IndicatorOutputId::LiquidationVolumeImbalanceShortVol, "Short Vol", Color::hex(0x2196F3))
            .precision(4)
            .zero_baseline()
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liq(ts: i64, side: TradeSide, price: f64, qty: f64) -> Liquidation {
        Liquidation { symbol: String::new(), side, price, quantity: qty, timestamp: ts, value: None, ..Default::default()}
    }

    #[test]
    fn zero_initially() {
        let lvi = LiquidationVolumeImbalance::new(60_000);
        assert_eq!(lvi.imbalance(), 0.0);
        assert_eq!(lvi.long_vol(), 0.0);
        assert_eq!(lvi.short_vol(), 0.0);
        assert!(!lvi.is_ready());
    }

    #[test]
    fn pure_long_liquidations_give_neg_one() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        lvi.update_liquidation(&liq(1_000, TradeSide::Buy, 30_000.0, 1.0));
        let imb = lvi.imbalance();
        assert!((imb - (-1.0)).abs() < 1e-9, "imb={imb}");
    }

    #[test]
    fn pure_short_liquidations_give_pos_one() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Sell, 30_000.0, 1.0));
        lvi.update_liquidation(&liq(1_000, TradeSide::Sell, 30_000.0, 1.0));
        let imb = lvi.imbalance();
        assert!((imb - 1.0).abs() < 1e-9, "imb={imb}");
    }

    #[test]
    fn equal_volumes_give_zero_imbalance() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        lvi.update_liquidation(&liq(1_000, TradeSide::Sell, 30_000.0, 1.0));
        let imb = lvi.imbalance();
        let lv = lvi.long_vol();
        let sv = lvi.short_vol();
        assert!((imb).abs() < 1e-9, "imb={imb}");
        assert!((lv - 30_000.0).abs() < 1e-6);
        assert!((sv - 30_000.0).abs() < 1e-6);
    }

    #[test]
    fn old_events_evicted() {
        let mut lvi = LiquidationVolumeImbalance::new(10_000);
        // long liq at t=0 (will be evicted)
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        // short liq at t=15_000 (outside window for t=0)
        lvi.update_liquidation(&liq(15_000, TradeSide::Sell, 30_000.0, 1.0));
        // only short remains → imbalance = +1
        let imb = lvi.imbalance();
        assert!((imb - 1.0).abs() < 1e-9, "imb={imb}");
    }

    #[test]
    fn reset_clears_state() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        lvi.reset();
        assert!(!lvi.is_ready());
        assert_eq!(lvi.imbalance(), 0.0);
        assert_eq!(lvi.long_vol(), 0.0);
        assert_eq!(lvi.short_vol(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_liquidation_volume_imbalance() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        let mut f = IndicatorOrder::LiquidationVolumeImbalance(
            <<LiquidationVolumeImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(
            0,
            TradeSide::Sell,
            30_000.0,
            2.0,
        )));
        // Only short liquidations → imbalance = +1; f.value() = first output (imbalance)
        let imb = f.primary();
        assert!((imb - 1.0).abs() < 1e-9, "imb={imb}");
    }
}
