//! FundingTimeDecay — pressure score approaching the next funding settlement.
//!
//! Contract-driven via the `PredictedFunding` stream: each `update_predicted_funding`
//! event carries `next_funding_time` + `timestamp`, from which the decay pressure is
//! computed. INPUT = PredictedFunding; time travels INSIDE the event (no separate ts axis).
//!
//! Output: `Single(decay_pressure)` ∈ [0, 1].
//! - `0.0` = far from funding (or no funding event seen)
//! - `1.0` = at or past funding time

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::predicted_funding_consumer::PredictedFundingConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::PredictedFunding;
use crate::engine::stream_kind::StreamKind;

/// Funding time decay pressure score.
///
/// Pressure grows from 0 to 1 as the next funding settlement approaches, computed from the
/// `PredictedFunding` event's `next_funding_time` vs its `timestamp`.
#[derive(Debug, Clone)]
pub struct FundingTimeDecay {
    /// Lookahead window (ms): pressure is 0 when `countdown >= max_window_ms`, 1 at countdown 0.
    max_window_ms: i64,
    last_pressure: f64,
}

impl FundingTimeDecay {
    /// Create a new indicator. `max_window_ms`: pressure is 0 when `countdown >= max_window_ms`.
    pub fn new(max_window_ms: i64) -> Self {
        Self { max_window_ms: max_window_ms.max(1), last_pressure: 0.0 }
    }
}

impl Default for FundingTimeDecay {
    fn default() -> Self {
        // 8 hours default (Binance/Bybit cycle)
        Self::new(8 * 3600 * 1000)
    }
}

/// Typed configuration for [`FundingTimeDecay`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingTimeDecayConfig {
    pub max_window_ms: Param<i64>,
}

impl Indicator for FundingTimeDecay {
    const ID: IndicatorId = IndicatorId::FundingTimeDecay;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::PredictedFunding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::FundingTimeDecay)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = FundingTimeDecayConfig;
    type Runtime = Self;

    fn create(cfg: FundingTimeDecayConfig) -> Self {
        Self::new(cfg.max_window_ms.resolved())
    }
}

impl crate::contract::Config for FundingTimeDecayConfig {
    fn defaults() -> Self {
        FundingTimeDecayConfig { max_window_ms: Param::Solo(8 * 3600 * 1000) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // max_window_ms: Class M (settlement window horizon, i64 ms).
        // Explicit 5-value set: 1h / 2h / 4h / 8h (Binance default) / 24h.
        s.max_window_ms = Param::many(vec![
            3_600_000i64,   // 1h
            7_200_000,      // 2h
            14_400_000,     // 4h
            28_800_000,     // 8h
            86_400_000,     // 24h
        ]);
        s
    }
}


impl Render for FundingTimeDecay {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingTimeDecay, "Funding Time Decay", Color::hex(0x03A9F4))
            .precision(3)
            .build()
    }
}

impl PredictedFundingConsumer for FundingTimeDecay {
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) {
        // Time travels inside the event: countdown = next_funding_time − event timestamp.
        if pf.timestamp > 0 {
            let countdown = (pf.next_funding_time - pf.timestamp).max(0);
            let normalized = countdown as f64 / self.max_window_ms as f64;
            self.last_pressure = 1.0 - normalized.clamp(0.0, 1.0);
        }
    }


    fn reset(&mut self) {
        self.last_pressure = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_predicted(next_funding_time: i64, ts: i64) -> PredictedFunding {
        PredictedFunding { predicted_rate: 0.0001, next_funding_time, timestamp: ts }
    }

    #[test]
    fn factory_feeds_resolved_funding_time_decay() {
        let mut f = IndicatorOrder::FundingTimeDecay(<<FundingTimeDecay as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let pf = make_predicted(8 * 3600 * 1000, 0);
        f.feed(0, MarketSample::PredictedFunding(&pf));
        let _ = f.primary();
    }

    #[test]
    fn far_from_funding_low_pressure() {
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000);
        // event ts a full window before next funding → countdown ≈ window → pressure ≈ 0
        ind.update_predicted_funding(&make_predicted(8 * 3600 * 1000, 1));
        let p = ind.value();
        assert!(p < 0.1, "pressure should be low far from funding, got {p}");
    }

    #[test]
    fn near_funding_high_pressure() {
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000);
        // event ts just before next funding → countdown tiny → pressure ≈ 1
        ind.update_predicted_funding(&make_predicted(1_000, 990));
        let p = ind.value();
        assert!(p > 0.9, "pressure should be high near funding, got {p}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000);
        ind.update_predicted_funding(&make_predicted(1_000_000, 1));
        ind.reset();
        assert_eq!(ind.value(), 0.0);
    }
}

impl FundingTimeDecay {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_pressure
    }
}
