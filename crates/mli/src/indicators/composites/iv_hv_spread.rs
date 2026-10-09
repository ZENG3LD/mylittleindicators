//! IvHvSpread — implied volatility minus historical volatility (volatility risk premium).
//!
//! Dual consumer: `HistoricalVolatilityConsumer` + `VolatilityIndexConsumer`.
//!
//! Logic:
//! - IV  = `VolatilityIndex.value`
//! - HV  = `HistoricalVolatility.volatility`
//! - VRP = IV - HV
//!
//! Output: `Triple(iv, hv, spread)`.

use crate::engine::streams::historical_volatility_consumer::HistoricalVolatilityConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::HistoricalVolatility;
use crate::core::types::VolatilityIndex;

/// Volatility risk premium: spread between implied and historical volatility.
///
/// Implements both `HistoricalVolatilityConsumer` and `VolatilityIndexConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct IvHvSpread {
    last_iv: f64,
    last_hv: f64,
}

impl IvHvSpread {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self { last_iv: 0.0, last_hv: 0.0 }
    }

    /// Named output: brace `iv`.
    pub fn iv(&self) -> f64 { self.last_iv }
    /// Named output: brace `hv`.
    pub fn hv(&self) -> f64 { self.last_hv }
    /// Named output: brace `spread`.
    pub fn spread(&self) -> f64 { self.last_iv - self.last_hv }


    /// True when both streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_iv > 0.0 && self.last_hv > 0.0
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_iv = 0.0;
        self.last_hv = 0.0;
    }
}

impl Default for IvHvSpread {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoricalVolatilityConsumer for IvHvSpread {
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) {
        self.last_hv = hv.volatility;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl VolatilityIndexConsumer for IvHvSpread {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) {
        self.last_iv = vi.value;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`IvHvSpread`] — no parameters.
#[derive(Debug, Clone)]
pub struct IvHvSpreadConfig;

impl IvHvSpreadConfig {
    /// Config fingerprint: no params → stable constant.
    pub fn config_hash(&self) -> u64 { 0 }

    /// Paramless config — the sole cube point is `self` (mixed-radix dual of `iter().nth`).
    pub fn axes_decode(&self, _idx: u128) -> Self { self.clone() }
}

impl Indicator for IvHvSpread {
    const ID: IndicatorId = IndicatorId::IvHvSpread;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::HistoricalVolatility, StreamKind::VolatilityIndex];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::IvHvSpreadIv),
        Output::magnitude(IndicatorOutputId::IvHvSpreadHv),
        Output::centered(IndicatorOutputId::IvHvSpreadSpread),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = IvHvSpreadConfig;
    type Runtime = IvHvSpread;

    fn create(_cfg: IvHvSpreadConfig) -> IvHvSpread {
        IvHvSpread::new()
    }
}

impl crate::contract::Config for IvHvSpreadConfig {
    fn defaults() -> Self {
        IvHvSpreadConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — nothing to sweep. machine_defaults == defaults.
        IvHvSpreadConfig
    }
    fn cube_size(&self) -> u128 {
        1
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(self.clone()))
    }
}


impl Render for IvHvSpread {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::IvHvSpreadIv, "IV", Color::hex(0xFF5722), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::IvHvSpreadHv, "HV", Color::hex(0x4CAF50), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::IvHvSpreadSpread, "IV-HV Spread", Color::hex(0x2196F3), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hv(volatility: f64) -> HistoricalVolatility {
        HistoricalVolatility { volatility, timestamp: 1000 }
    }

    fn make_vi(value: f64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn spread_is_iv_minus_hv() {
        let mut ind = IvHvSpread::new();
        ind.update_volatility_index(&make_vi(0.30));
        ind.update_historical_volatility(&make_hv(0.20));
        assert!((ind.iv() - 0.30).abs() < 1e-9);
        assert!((ind.hv() - 0.20).abs() < 1e-9);
        assert!((ind.spread() - 0.10).abs() < 1e-9);
    }

    #[test]
    fn negative_spread_when_hv_exceeds_iv() {
        let mut ind = IvHvSpread::new();
        ind.update_volatility_index(&make_vi(0.20));
        ind.update_historical_volatility(&make_hv(0.35));
        assert!(ind.spread() < 0.0);
    }

    #[test]
    fn not_ready_until_both_streams_updated() {
        let mut ind = IvHvSpread::new();
        assert!(!ind.indicator_is_ready());
        ind.update_volatility_index(&make_vi(0.30));
        assert!(!ind.indicator_is_ready());
        ind.update_historical_volatility(&make_hv(0.20));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IvHvSpread::new();
        ind.update_volatility_index(&make_vi(0.30));
        ind.update_historical_volatility(&make_hv(0.20));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.iv(), 0.0);
        assert_eq!(ind.hv(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_iv_hv_spread() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::IvHvSpread(IvHvSpreadConfig).build_solo().unwrap();
        let hv = make_hv(0.20);
        let vi = make_vi(0.30);
        f.feed(0, MarketSample::HistoricalVolatility(&hv));
        f.feed(0, MarketSample::VolatilityIndex(&vi));
        let iv = f.primary();
        assert!((iv - 0.30).abs() < 1e-9);
    }
}
