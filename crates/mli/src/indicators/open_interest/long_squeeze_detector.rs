//! LongSqueezeDetector — dual-consumer: OpenInterest + MarkPrice.
//!
//! Detects squeeze/cascade patterns from OI and price direction changes:
//! - Long squeeze:  dOI < 0 AND dPrice < 0 → Signal(+1)  (long liquidation cascade)
//! - Short squeeze: dOI < 0 AND dPrice > 0 → Signal(-1)  (short cover rally)
//! - Otherwise:                               Signal(0)
//!
//! "d" = change from previous observation in the same stream.
//!
//! Implements both `OpenInterestConsumer` and `MarkPriceConsumer`.
//! Uses inherent methods (`indicator_value`, `indicator_is_ready`, `indicator_reset`)
//! to avoid UFCS ambiguity between the two consumer traits.
//!
//! Output: `Signal(i8)`.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;
use crate::core::types::OpenInterest;

/// Detects long/short squeeze signals from OI and price co-movement.
#[derive(Debug, Clone)]
pub struct LongSqueezeDetector {
    last_oi: f64,
    last_price: f64,
    prev_oi: f64,
    prev_price: f64,
    last_signal: i8,
    oi_seen: bool,
    price_seen: bool,
}

impl LongSqueezeDetector {
    pub fn new() -> Self {
        Self {
            last_oi: 0.0,
            last_price: 0.0,
            prev_oi: 0.0,
            prev_price: 0.0,
            last_signal: 0,
            oi_seen: false,
            price_seen: false,
        }
    }

    fn recompute_signal(&mut self) {
        if !self.oi_seen || !self.price_seen {
            return;
        }
        let d_oi = self.last_oi - self.prev_oi;
        let d_price = self.last_price - self.prev_price;
        self.last_signal = if d_oi < 0.0 && d_price < 0.0 {
            1 // long liquidation cascade
        } else if d_oi < 0.0 && d_price > 0.0 {
            -1 // short cover rally
        } else {
            0
        };
    }

    /// Current signal value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_signal as f64
    }

    /// True once both OI and price have received at least 2 updates each.
    pub fn indicator_is_ready(&self) -> bool {
        self.oi_seen && self.price_seen
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_oi = 0.0;
        self.last_price = 0.0;
        self.prev_oi = 0.0;
        self.prev_price = 0.0;
        self.last_signal = 0;
        self.oi_seen = false;
        self.price_seen = false;
    }
}

impl Default for LongSqueezeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenInterestConsumer for LongSqueezeDetector {
    fn update_oi(&mut self, oi: &OpenInterest) {
        if self.oi_seen {
            self.prev_oi = self.last_oi;
        }
        self.last_oi = oi.open_interest;
        self.oi_seen = true;
        self.recompute_signal();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for LongSqueezeDetector {
    fn update_mark(&mut self, mp: &MarkPrice) {
        if self.price_seen {
            self.prev_price = self.last_price;
        }
        self.last_price = mp.mark_price;
        self.price_seen = true;
        self.recompute_signal();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`LongSqueezeDetector`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct LongSqueezeDetectorConfig;

impl Indicator for LongSqueezeDetector {
    const ID: IndicatorId = IndicatorId::LongSqueezeDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OpenInterest, StreamKind::MarkPrice];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::LongSqueezeDetector)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = LongSqueezeDetectorConfig;
    type Runtime = LongSqueezeDetector;

    fn create(_cfg: LongSqueezeDetectorConfig) -> LongSqueezeDetector {
        LongSqueezeDetector::new()
    }
}

impl crate::contract::Config for LongSqueezeDetectorConfig {
    fn defaults() -> Self {
        LongSqueezeDetectorConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
}


impl Render for LongSqueezeDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::LongSqueezeDetector, "Long/Short Squeeze Signal", Color::hex(0xF44336)))
            .bounds(-1.5, 1.5)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oi(oi: f64) -> OpenInterest {
        OpenInterest {
            open_interest: oi,
            open_interest_value: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    fn make_mark(price: f64) -> MarkPrice {
        MarkPrice {
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_until_both_streams() {
        let mut ind = LongSqueezeDetector::new();
        assert!(!ind.indicator_is_ready());
        ind.update_oi(&make_oi(1000.0));
        assert!(!ind.indicator_is_ready()); // price not seen yet
        ind.update_mark(&make_mark(50000.0));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn long_squeeze_signal_plus_one() {
        // dOI < 0, dPrice < 0 → +1
        let mut ind = LongSqueezeDetector::new();
        ind.update_oi(&make_oi(1000.0));
        ind.update_mark(&make_mark(50000.0));
        // now drop both
        ind.update_oi(&make_oi(900.0));
        ind.update_mark(&make_mark(49000.0));
        assert_eq!(ind.indicator_value() as i8, 1, "expected +1 long squeeze signal");
    }

    #[test]
    fn short_cover_signal_minus_one() {
        // dOI < 0, dPrice > 0 → -1
        let mut ind = LongSqueezeDetector::new();
        ind.update_oi(&make_oi(1000.0));
        ind.update_mark(&make_mark(50000.0));
        ind.update_oi(&make_oi(900.0));
        ind.update_mark(&make_mark(51000.0));
        assert_eq!(ind.indicator_value() as i8, -1, "expected -1 short cover signal");
    }

    #[test]
    fn neutral_when_oi_rising() {
        // dOI > 0 → 0 regardless of price
        let mut ind = LongSqueezeDetector::new();
        ind.update_oi(&make_oi(1000.0));
        ind.update_mark(&make_mark(50000.0));
        ind.update_oi(&make_oi(1100.0));
        ind.update_mark(&make_mark(48000.0));
        assert_eq!(ind.indicator_value() as i8, 0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LongSqueezeDetector::new();
        ind.update_oi(&make_oi(1000.0));
        ind.update_mark(&make_mark(50000.0));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value() as i8, 0);
    }

    #[test]
    fn factory_feeds_resolved_long_squeeze_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::LongSqueezeDetector(LongSqueezeDetectorConfig).build_solo().unwrap();
        let oi1 = make_oi(1000.0);
        let mp1 = make_mark(50000.0);
        let oi2 = make_oi(900.0);
        let mp2 = make_mark(49000.0);
        f.feed(0, MarketSample::OpenInterest(&oi1));
        f.feed(0, MarketSample::MarkPrice(&mp1));
        f.feed(0, MarketSample::OpenInterest(&oi2));
        f.feed(0, MarketSample::MarkPrice(&mp2));
        assert_eq!(f.primary() as i8, 1, "expected +1 long squeeze");
    }
}
