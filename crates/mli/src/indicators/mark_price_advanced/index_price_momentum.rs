//! IndexPriceMomentum — EMA-based momentum (slope) of the index/spot price
//! extracted from mark price feed.
//!
//! Uses the `index_price` field from `MarkPrice` when available; falls back to
//! `mark_price` if `index_price` is `None`.
//!
//! Output: `Double(ema, slope)`
//!   - ema:   exponential moving average of the index price
//!   - slope: ema[now] - ema[prev] (positive = trending up)

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;

/// EMA-smoothed momentum of the index/spot price from the mark price feed.
#[derive(Debug, Clone)]
pub struct IndexPriceMomentum {
    period: usize,
    alpha: f64,
    ema: f64,
    prev_ema: f64,
    count: usize,
}

impl IndexPriceMomentum {
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

impl IndexPriceMomentum {
    /// EMA of the index price.
    pub fn ema(&self) -> f64 {
        self.ema
    }

    /// EMA slope: `ema[now] - ema[prev]`.
    pub fn slope(&self) -> f64 {
        self.ema - self.prev_ema
    }
}

impl MarkPriceConsumer for IndexPriceMomentum {
    fn update_mark(&mut self, mp: &MarkPrice) {
        let price = mp.index_price.unwrap_or(mp.mark_price);
        self.prev_ema = self.ema;
        if self.count == 0 {
            self.ema = price;
        } else {
            self.ema = self.ema + self.alpha * (price - self.ema);
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

impl Default for IndexPriceMomentum {
    /// Factory default: period=14.
    fn default() -> Self {
        Self::new(14)
    }
}

/// Typed configuration for [`IndexPriceMomentum`]. Consumes `MarkPrice` stream.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IndexPriceMomentumConfig {
    pub period: crate::contract::Param<usize>,
}

impl Indicator for IndexPriceMomentum {
    const ID: IndicatorId = IndicatorId::IndexPriceMomentum;
    /// Mark-price indicator — not a pluggable family.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarkPrice];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::IndexPriceMomentumEma),
        Output::centered(IndicatorOutputId::IndexPriceMomentumSlope),
    ];
    type Config = IndexPriceMomentumConfig;
    type Runtime = IndexPriceMomentum;

    fn create(cfg: IndexPriceMomentumConfig) -> IndexPriceMomentum {
        IndexPriceMomentum::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for IndexPriceMomentumConfig {
    fn defaults() -> Self {
        IndexPriceMomentumConfig { period: crate::contract::Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (EMA lookback period) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IndexPriceMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::IndexPriceMomentumEma, "Index EMA", Color::hex(0x42A5F5))
            .line_output(IndicatorOutputId::IndexPriceMomentumSlope, "Slope", Color::hex(0xFF7043))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp_with_index(mark: f64, index: f64) -> MarkPrice {
        MarkPrice {
            mark_price: mark,
            index_price: Some(index),
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    fn mp_no_index(mark: f64) -> MarkPrice {
        MarkPrice {
            mark_price: mark,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_before_period() {
        let ind = IndexPriceMomentum::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn ready_after_period_updates() {
        let mut ind = IndexPriceMomentum::new(3);
        for i in 0..3 {
            ind.update_mark(&mp_no_index(50_000.0 + i as f64));
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn uses_index_price_when_available() {
        let mut ind = IndexPriceMomentum::new(1);
        // index=50_000, mark=51_000 → EMA should track index
        ind.update_mark(&mp_with_index(51_000.0, 50_000.0));
        let ema = ind.ema();
        assert!((ema - 50_000.0).abs() < 1e-6, "should use index price");
    }

    #[test]
    fn falls_back_to_mark_price() {
        let mut ind = IndexPriceMomentum::new(1);
        ind.update_mark(&mp_no_index(50_000.0));
        let ema = ind.ema();
        assert!((ema - 50_000.0).abs() < 1e-6, "fallback to mark_price");
    }

    #[test]
    fn slope_positive_on_rising_index() {
        let mut ind = IndexPriceMomentum::new(2);
        for i in 0..20 {
            ind.update_mark(&mp_with_index(0.0, 50_000.0 + i as f64 * 10.0));
        }
        let slope = ind.slope();
        assert!(slope > 0.0, "slope should be positive on rising prices");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IndexPriceMomentum::new(3);
        for _ in 0..5 {
            ind.update_mark(&mp_no_index(50_000.0));
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.ema(), 0.0);
        assert_eq!(ind.slope(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_mark_price() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::IndexPriceMomentum(<<IndexPriceMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let mp = MarkPrice {
            mark_price: 51_000.0,
            index_price: Some(50_000.0),
            funding_rate: None,
            timestamp: 1,
            ..Default::default()
        };
        // Feed several samples; f.value() returns first output (ema)
        for _ in 0..3 {
            f.feed(0, MarketSample::MarkPrice(&mp));
        }
        let ema = f.primary();
        assert!(ema > 0.0, "EMA should be positive, got {ema}");
    }
}
