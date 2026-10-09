//! Funding Z-Score — rolling Z-score of funding rate vs window mean and std.
//!
//! Measures how extreme the current funding rate is relative to recent history.
//! High positive = unusually high funding (market leaning long).
//! High negative = unusually low/negative funding (market leaning short).
//!
//! Output: `Single(z_score)`.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::FundingRate;
use crate::engine::stream_kind::StreamKind;

/// Rolling Z-score of funding rate.
#[derive(Debug, Clone)]
pub struct FundingZScore {
    window: usize,
    rates: VecDeque<f64>,
    last_zscore: f64,
}

impl FundingZScore {
    /// Create with given lookback window.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            rates: VecDeque::new(),
            last_zscore: 0.0,
        }
    }

    fn compute_zscore(&self, current: f64) -> f64 {
        let n = self.rates.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.rates.iter().sum::<f64>() / n as f64;
        let variance = self.rates.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();
        if std < 1e-15 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

/// Typed configuration for [`FundingZScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingZScoreConfig {
    pub window: Param<usize>,
}

impl Indicator for FundingZScore {
    const ID: IndicatorId = IndicatorId::FundingZScore;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::FundingZScore)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = FundingZScoreConfig;
    type Runtime = Self;

    fn create(cfg: FundingZScoreConfig) -> Self {
        Self::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for FundingZScoreConfig {
    fn defaults() -> Self {
        FundingZScoreConfig { window: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // window: Class A → auto range(2,4048,1); already set by auto.
        Self::machine_defaults_auto()
    }
}


impl Render for FundingZScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingZScore, "Funding Z-Score", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl FundingRateConsumer for FundingZScore {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.rates.push_back(fr.rate);
        if self.rates.len() > self.window {
            self.rates.pop_front();
        }
        self.last_zscore = self.compute_zscore(fr.rate);
    }


    fn reset(&mut self) {
        self.rates.clear();
        self.last_zscore = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.rates.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_funding_z_score() {
        let mut f = IndicatorOrder::FundingZScore(
            <<FundingZScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let fr = FundingRate { rate: 0.0001, next_funding_time: None, timestamp: 1000, ..Default::default()};
        f.feed(0, MarketSample::Funding(&fr));
        let _ = f.primary();
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut fz = FundingZScore::new(5);
        for i in 0..4 {
            fz.update_funding(&make_fr(i as f64 * 0.0001));
        }
        assert!(!fz.is_ready());
        fz.update_funding(&make_fr(0.0005));
        assert!(fz.is_ready());
    }

    #[test]
    fn zscore_near_zero_for_mean_value() {
        let mut fz = FundingZScore::new(10);
        // Fill with uniform rates so std is small and current = mean
        for _ in 0..10 {
            fz.update_funding(&make_fr(0.0001));
        }
        // Z-score of constant series is 0
        assert!((fz.value()).abs() < 1e-9);
    }

    #[test]
    fn zscore_positive_for_high_rate() {
        let mut fz = FundingZScore::new(10);
        for _ in 0..9 {
            fz.update_funding(&make_fr(0.0001));
        }
        // Now push a very high rate
        fz.update_funding(&make_fr(0.01));
        assert!(fz.is_ready());
        assert!(fz.value() > 1.0, "high rate should give positive z-score");
    }

    #[test]
    fn reset_clears_state() {
        let mut fz = FundingZScore::new(5);
        for _ in 0..10 {
            fz.update_funding(&make_fr(0.0001));
        }
        fz.reset();
        assert!(!fz.is_ready());
        assert_eq!(fz.value(), 0.0);
    }
}

impl Default for FundingZScore {
    /// Factory default: window=14.
    fn default() -> Self {
        Self::new(14)
    }
}

impl FundingZScore {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_zscore
    }
}
