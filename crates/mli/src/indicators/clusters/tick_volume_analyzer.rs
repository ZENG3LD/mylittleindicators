//! Tick Volume Analyzer — buy/sell volume split from trade stream.
//!
//! Primary path: `update_tick(&Tick)` — uses real `tick.is_buy` flag set by exchange.
//! Accurate buy/sell delta requires a live tick feed.
//!
//! Fallback path: `update_bar` — SYNTHETIC ESTIMATE only.
//! close > open → all volume counted as buy; close < open → all as sell.
//! This heuristic is only an approximation. Prefer `update_tick` when ticks available.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Tick Volume Analyzer.
#[derive(Debug, Clone)]
pub struct TickVolumeAnalyzer {
    period: usize,
    ticks: Vec<Tick>,

    total_volume: f64,
    buy_volume: f64,
    sell_volume: f64,

    volume_delta: f64,
    volume_ratio: f64,
    tick_count: usize,
    avg_tick_size: f64,

    avg_spread: f64,
    spread_samples: usize,
}

/// Typed dual-mode config for [`TickVolumeAnalyzer`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TickVolumeAnalyzerConfig {
    pub period: Param<usize>,
}

impl TickVolumeAnalyzer {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ticks: Vec::with_capacity(period),
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            volume_delta: 0.0,
            volume_ratio: 1.0,
            tick_count: 0,
            avg_tick_size: 0.0,
            avg_spread: 0.0,
            spread_samples: 0,
        }
    }

    /// Update with a real trade tick. Uses `tick.is_buy` directly.
    pub fn update(&mut self, tick: &Tick) {
        self.total_volume += tick.size;
        if tick.is_buy {
            self.buy_volume += tick.size;
        } else {
            self.sell_volume += tick.size;
        }

        if let Some(spread) = tick.spread() {
            self.avg_spread = (self.avg_spread * self.spread_samples as f64 + spread)
                / (self.spread_samples + 1) as f64;
            self.spread_samples += 1;
        }

        self.tick_count += 1;
        self.recalculate_derived();

        if self.ticks.len() >= self.period {
            self.ticks.remove(0);
        }
        self.ticks.push(*tick);
    }

    fn recalculate_derived(&mut self) {
        self.volume_delta = self.buy_volume - self.sell_volume;
        self.volume_ratio = if self.sell_volume > 0.0 {
            self.buy_volume / self.sell_volume
        } else {
            1.0
        };
        self.avg_tick_size = self.total_volume / self.tick_count as f64;
    }

    pub fn volume_delta(&self) -> f64 { self.volume_delta }
    pub fn volume_ratio(&self) -> f64 { self.volume_ratio }
    pub fn buy_volume(&self) -> f64 { self.buy_volume }
    pub fn sell_volume(&self) -> f64 { self.sell_volume }
    pub fn avg_spread(&self) -> f64 { self.avg_spread }
    pub fn tick_count(&self) -> usize { self.tick_count }

    #[inline]
    pub fn value(&self) -> f64 {
        self.volume_delta
    }

    pub fn is_ready(&self) -> bool { self.tick_count > 0 }

    pub fn reset(&mut self) {
        self.ticks.clear();
        self.total_volume = 0.0;
        self.buy_volume = 0.0;
        self.sell_volume = 0.0;
        self.volume_delta = 0.0;
        self.volume_ratio = 1.0;
        self.tick_count = 0;
        self.avg_tick_size = 0.0;
        self.avg_spread = 0.0;
        self.spread_samples = 0;
    }
}

impl Default for TickVolumeAnalyzer {
    /// Factory default: period=14.
    fn default() -> Self {
        Self::new(14)
    }
}

impl Indicator for TickVolumeAnalyzer {
    const ID: IndicatorId = IndicatorId::TickVolume;
    /// Tick-stream consumer — not a pluggable family member (specialized counter).
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::TickVolume)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[
        Store::window(StoreKind::Vec), // ticks rolling buffer
    ]);
    type Config = TickVolumeAnalyzerConfig;
    type Runtime = TickVolumeAnalyzer;

    fn create(cfg: TickVolumeAnalyzerConfig) -> TickVolumeAnalyzer {
        TickVolumeAnalyzer::new(cfg.period.resolved())
    }
}

impl TickConsumer for TickVolumeAnalyzer {
    fn update_tick(&mut self, tick: &Tick) {
        self.update(tick);
    }


    fn reset(&mut self) {
        self.reset();
    }

    fn is_ready(&self) -> bool {
        self.is_ready()
    }
}

impl crate::contract::Config for TickVolumeAnalyzerConfig {
    fn defaults() -> Self {
        TickVolumeAnalyzerConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A period usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for TickVolumeAnalyzer {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::TickVolume, "Tick Volume", Color::hex(0x607D8B)))
            .histogram_style(HistogramStyle::FromBottom)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;
    use crate::core::types::Tick;

    #[test]
    fn test_tick_volume_analyzer_creation() {
        let ind = TickVolumeAnalyzer::new(100);
        assert!(!ind.is_ready());
        assert_eq!(ind.volume_delta(), 0.0);
    }

    #[test]
    fn test_tick_volume_analyzer_real_tick() {
        let mut ind = TickVolumeAnalyzer::new(100);
        let tick = Tick::new(1000, 100.0, 50.0, true);
        ind.update(&tick);
        assert!(ind.is_ready());
        assert_eq!(ind.buy_volume(), 50.0);
        assert_eq!(ind.sell_volume(), 0.0);
    }

    #[test]
    fn test_tick_volume_analyzer_multiple_updates() {
        let mut ind = TickVolumeAnalyzer::new(100);
        for i in 0..10 {
            let tick = Tick::new(1000 + i as i64, 100.0, 10.0 + i as f64, true);
            ind.update(&tick);
        }
        assert_eq!(ind.tick_count(), 10);
    }

    #[test]
    fn test_tick_volume_analyzer_reset() {
        let mut ind = TickVolumeAnalyzer::new(100);
        for _ in 0..5 {
            let tick = Tick::new(1000, 100.0, 50.0, true);
            ind.update(&tick);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.volume_delta(), 0.0);
        assert_eq!(ind.tick_count(), 0);
    }

    #[test]
    fn factory_feeds_resolved_tick_volume() {
        
        let mut f = IndicatorOrder::TickVolume(
            <<TickVolumeAnalyzer as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let tick = Tick::new(1000, 100.0, 50.0, true);
        f.feed(0, MarketSample::Tick(&tick));
        assert!(f.primary().is_finite());
    }
}
