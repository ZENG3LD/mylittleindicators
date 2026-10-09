//! RiskOffDetector — risk-off signal when 2+ stress components exceed threshold.
//!
//! Quad consumer: `VolatilityIndexConsumer` + `LiquidationConsumer` +
//! `FundingRateConsumer` + `InsuranceFundConsumer`.
//!
//! Logic: if 2+ components exceed threshold → Signal = +1 (risk-off), else 0.
//! Components:
//! 1. vol_idx > threshold
//! 2. liq_count_in_window >= threshold_count
//! 3. |funding_rate| × 100 > threshold
//! 4. fund_depletion active (recent slope < 0)
//!
//! Output: `Signal(i8)` — +1 risk-off, 0 normal.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::contract::sweep_f64;
use crate::core::types::{FundingRate, InsuranceFund, Liquidation, VolatilityIndex};
use crate::engine::stream_kind::StreamKind;

/// Risk-off detector composite.
///
/// Implements `VolatilityIndexConsumer`, `LiquidationConsumer`,
/// `FundingRateConsumer`, and `InsuranceFundConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct RiskOffDetector {
    window_ms: i64,
    /// Threshold for vol index to be considered elevated.
    vol_threshold: f64,
    /// Minimum liq events in window to count as spike.
    liq_threshold: usize,
    /// |funding| × 100 threshold.
    funding_threshold: f64,

    current_vol: f64,
    liq_events: VecDeque<i64>,
    last_funding_abs: f64,
    fund_history: VecDeque<(i64, f64)>,

    last_signal: i8,
}

impl RiskOffDetector {
    /// Create a new indicator.
    ///
    /// - `window_ms`          — rolling window for liq events (default 60_000)
    /// - `vol_threshold`      — vol index level considered elevated (default 0.5)
    /// - `liq_threshold`      — min liquidation count for spike (default 3)
    /// - `funding_threshold`  — |funding| × 100 threshold (default 0.05)
    pub fn new(
        window_ms: i64,
        vol_threshold: f64,
        liq_threshold: usize,
        funding_threshold: f64,
    ) -> Self {
        Self {
            window_ms,
            vol_threshold,
            liq_threshold,
            funding_threshold,
            current_vol: 0.0,
            liq_events: VecDeque::new(),
            last_funding_abs: 0.0,
            fund_history: VecDeque::with_capacity(10),
            last_signal: 0,
        }
    }

    fn fund_depleting(&self) -> bool {
        if self.fund_history.len() < 2 {
            return false;
        }
        let first = self.fund_history.front().copied().unwrap_or((0, 0.0));
        let last = self.fund_history.back().copied().unwrap_or((0, 0.0));
        last.1 < first.1
    }

    fn recompute(&mut self, now: i64) {
        // Evict stale liq events
        let cutoff = now - self.window_ms;
        while self.liq_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_events.pop_front();
        }

        let components_active: usize = [
            self.current_vol > self.vol_threshold,
            self.liq_events.len() >= self.liq_threshold,
            self.last_funding_abs * 100.0 > self.funding_threshold,
            self.fund_depleting(),
        ]
        .iter()
        .filter(|&&b| b)
        .count();

        self.last_signal = if components_active >= 2 { 1 } else { 0 };
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_signal as f64
    }

    /// True when at least one vol update has arrived.
    pub fn indicator_is_ready(&self) -> bool {
        self.current_vol > 0.0 || !self.liq_events.is_empty()
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.current_vol = 0.0;
        self.liq_events.clear();
        self.last_funding_abs = 0.0;
        self.fund_history.clear();
        self.last_signal = 0;
    }
}

impl Default for RiskOffDetector {
    fn default() -> Self {
        Self::new(60_000, 0.5, 3, 0.05)
    }
}

/// Typed configuration for [`RiskOffDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RiskOffDetectorConfig {
    /// Rolling window for liquidation events (milliseconds).
    pub window_ms: Param<i64>,
    /// Vol index level considered elevated.
    pub vol_threshold: Param<f64>,
    /// Minimum liquidation count in window to count as spike.
    pub liq_threshold: Param<usize>,
    /// |funding| × 100 threshold.
    pub funding_threshold: Param<f64>,
}

impl Indicator for RiskOffDetector {
    const ID: IndicatorId = IndicatorId::RiskOffDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[
        StreamKind::VolatilityIndex,
        StreamKind::Liquidation,
        StreamKind::Funding,
        StreamKind::InsuranceFund,
    ];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::RiskOffDetector)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Deque),
            Store::window(StoreKind::Deque),
        ],
    );
    type Config = RiskOffDetectorConfig;
    type Runtime = RiskOffDetector;

    fn create(cfg: RiskOffDetectorConfig) -> RiskOffDetector {
        RiskOffDetector::new(
            cfg.window_ms.resolved(),
            cfg.vol_threshold.resolved(),
            cfg.liq_threshold.resolved(),
            cfg.funding_threshold.resolved(),
        )
    }
}

impl crate::contract::Config for RiskOffDetectorConfig {
    fn defaults() -> Self {
        RiskOffDetectorConfig {
            window_ms: Param::Solo(60_000),
            vol_threshold: Param::Solo(0.5),
            liq_threshold: Param::Solo(3),
            funding_threshold: Param::Solo(0.05),
        }
    }
    fn machine_defaults() -> Self {
        // window_ms (i64): Class M raw-ms — canonical named durations (§3.8)
        // vol_threshold (f64): Class F — vol index level considered elevated; sweep_f64(0.1,5.0,0.1)
        // liq_threshold (usize): Class B count — min liq events for spike; auto gives 2..4048 too
        //   wide; corrected to range(1,20,1)
        // funding_threshold (f64): Class F — |funding|×100 threshold; sweep_f64(0.1,5.0,0.1)
        let mut s = Self::machine_defaults_auto();
        s.window_ms = Param::many(vec![1000i64, 5000, 10000, 30000, 60000, 300000, 3600000]);
        s.vol_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s.liq_threshold = Param::range(1, 20, 1);
        s.funding_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RiskOffDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::RiskOffDetector,
                "Risk-Off Signal",
                Color::hex(0xF44336),
            ))
            .bounds(-0.5, 1.5)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

impl VolatilityIndexConsumer for RiskOffDetector {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) {
        self.current_vol = vi.value;
        self.recompute(vi.timestamp);
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl LiquidationConsumer for RiskOffDetector {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        self.liq_events.push_back(liq.timestamp);
        self.recompute(liq.timestamp);
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingRateConsumer for RiskOffDetector {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.last_funding_abs = fr.rate.abs();
        self.recompute(fr.timestamp);
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl InsuranceFundConsumer for RiskOffDetector {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) {
        if self.fund_history.len() >= 10 {
            self.fund_history.pop_front();
        }
        self.fund_history.push_back((ins.timestamp, ins.balance));
        self.recompute(ins.timestamp);
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn make_vi(value: f64, ts: i64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: ts, ..Default::default()}
    }

    fn make_liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None, ..Default::default()}
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn two_components_trigger_risk_off() {
        // vol high + funding high → 2 components
        let mut ind = RiskOffDetector::new(60_000, 0.5, 3, 0.05);
        ind.update_volatility_index(&make_vi(1.0, 1000)); // vol above 0.5
        ind.update_funding(&make_fr(0.001)); // |0.001| × 100 = 0.1 > 0.05
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 1, "expected risk-off");
    }

    #[test]
    fn single_component_no_risk_off() {
        let mut ind = RiskOffDetector::new(60_000, 0.5, 3, 0.05);
        ind.update_volatility_index(&make_vi(1.0, 1000)); // only 1 component
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 0, "only 1 component, should be 0");
    }

    #[test]
    fn liq_spike_plus_vol_triggers() {
        let mut ind = RiskOffDetector::new(60_000, 0.5, 2, 0.05);
        ind.update_volatility_index(&make_vi(1.0, 1000));
        for i in 0..2i64 {
            ind.update_liquidation(&make_liq(1000 + i * 100));
        }
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 1, "vol+liq should trigger risk-off");
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = RiskOffDetector::default();
        ind.update_volatility_index(&make_vi(1.0, 1000));
        ind.update_funding(&make_fr(0.001));
        ind.indicator_reset();
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 0);
    }

    #[test]
    fn factory_feeds_risk_off_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::core::types::InsuranceFund;
        let mut f = IndicatorOrder::RiskOffDetector(RiskOffDetectorConfig {
            window_ms: Param::Solo(60_000),
            vol_threshold: Param::Solo(0.5),
            liq_threshold: Param::Solo(3),
            funding_threshold: Param::Solo(0.05),
        })
        .build_solo()
        .unwrap();
        // Feed vol above threshold + funding above threshold → 2 components → risk-off
        let vi = make_vi(1.0, 1000);
        f.feed(0, MarketSample::VolatilityIndex(&vi));
        let fr = make_fr(0.001);
        f.feed(0, MarketSample::Funding(&fr));
        let s = f.primary().round() as i8;
        assert_eq!(s, 1, "expected risk-off signal from factory");
        // Feed insurance fund data (should not alter signal — only 2 components active)
        let ins = InsuranceFund { balance: 1_000_000.0, timestamp: 2000 };
        f.feed(0, MarketSample::InsuranceFund(&ins));
        let s = f.primary().round() as i8;
        assert_eq!(s, 1, "signal should remain risk-off");
    }
}
