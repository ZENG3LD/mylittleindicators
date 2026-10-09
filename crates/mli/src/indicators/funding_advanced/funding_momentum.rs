//! Funding Momentum — EMA of funding rate with slope direction.
//!
//! Smooths funding rate updates via EMA and tracks slope direction.
//! Positive slope = funding trending up (longs paying more = bearish signal).
//! Negative slope = funding trending down.
//!
//! Output: `Double(ema_value, slope)` where slope is ema[now] - ema[prev].

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::FundingRate;
use crate::engine::stream_kind::StreamKind;

/// EMA-smoothed funding rate with momentum slope.
#[derive(Debug, Clone)]
pub struct FundingMomentum {
    /// EMA period.
    period: usize,
    /// EMA smoothing factor (2 / (period + 1)).
    alpha: f64,
    /// Current EMA value.
    ema: f64,
    /// Previous EMA value for slope calculation.
    prev_ema: f64,
    /// Number of updates seen.
    count: usize,
}

impl FundingMomentum {
    /// Create with given EMA period.
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            alpha: 2.0 / (p as f64 + 1.0),
            ema: 0.0,
            prev_ema: 0.0,
            count: 0,
        }
    }
}

impl FundingMomentum {
    /// EMA of the funding rate.
    pub fn ema(&self) -> f64 {
        self.ema
    }

    /// EMA slope: `ema[now] - ema[prev]`.
    pub fn slope(&self) -> f64 {
        self.ema - self.prev_ema
    }
}

/// Typed configuration for [`FundingMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingMomentumConfig {
    pub period: Param<usize>,
}

impl Indicator for FundingMomentum {
    const ID: IndicatorId = IndicatorId::FundingMomentum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::FundingMomentumEma),
        Output::centered(IndicatorOutputId::FundingMomentumSlope),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = FundingMomentumConfig;
    type Runtime = Self;

    fn create(cfg: FundingMomentumConfig) -> Self {
        Self::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for FundingMomentumConfig {
    fn defaults() -> Self {
        FundingMomentumConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); already set by auto.
        Self::machine_defaults_auto()
    }
}


impl Render for FundingMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingMomentumEma, "Funding EMA", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::FundingMomentumSlope, "Slope", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(6)
            .build()
    }
}

impl FundingRateConsumer for FundingMomentum {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.prev_ema = self.ema;
        if self.count == 0 {
            self.ema = fr.rate;
        } else {
            self.ema = self.ema + self.alpha * (fr.rate - self.ema);
        }
        self.count += 1;
        let _slope = self.ema - self.prev_ema;
    }


    fn reset(&mut self) {
        self.ema = 0.0;
        self.prev_ema = 0.0;
        self.count = 0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_funding_momentum() {
        let mut f = IndicatorOrder::FundingMomentum(
            <<FundingMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let fr = FundingRate { rate: 0.0001, next_funding_time: None, timestamp: 1000, ..Default::default()};
        f.feed(0, MarketSample::Funding(&fr));
        let _ = f.primary();
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn not_ready_initially() {
        let fm = FundingMomentum::new(5);
        assert!(!fm.is_ready());
    }

    #[test]
    fn ema_converges_to_constant_rate() {
        let mut fm = FundingMomentum::new(3);
        for _ in 0..30 {
            fm.update_funding(&make_fr(0.0001));
        }
        assert!(fm.is_ready());
        assert!((fm.ema() - 0.0001).abs() < 1e-10);
    }

    #[test]
    fn slope_positive_on_rising_rates() {
        let mut fm = FundingMomentum::new(2);
        for i in 0..20 {
            fm.update_funding(&make_fr(i as f64 * 0.0001));
        }
        assert!(fm.slope() > 0.0, "slope should be positive on rising rates");
    }

    #[test]
    fn reset_clears_state() {
        let mut fm = FundingMomentum::new(3);
        for _ in 0..10 {
            fm.update_funding(&make_fr(0.0001));
        }
        fm.reset();
        assert!(!fm.is_ready());
        assert_eq!(fm.ema(), 0.0);
        assert_eq!(fm.slope(), 0.0);
    }
}

impl Default for FundingMomentum {
    /// Factory default: period=14.
    fn default() -> Self {
        Self::new(14)
    }
}
