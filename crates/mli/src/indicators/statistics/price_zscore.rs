use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Rolling Z-Score of Close: (close - MA(close,n)) / StdDev(close,n)
///
/// Uses two independent smoother slots: one smooths the value (mean), the other
/// smooths the squared deviation (variance). Both slots are driven by the same
/// smoother shape and period — user picks period via the `ma` slot.
#[derive(Debug, Clone)]
pub struct PriceZScore {
    mean_ma: SmootherSlot,
    var_ma: SmootherSlot,
    value: f64,
}

impl PriceZScore {
    pub fn new(period: usize) -> Self {
        Self::from_smoothers(SmootherId::Sma, period)
    }

    pub fn from_smoothers(id: SmootherId, period: usize) -> Self {
        let n = period.max(2);
        Self {
            mean_ma: SmootherSlot::new(id, n),
            var_ma: SmootherSlot::new(id, n),
            value: 0.0,
        }
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        let mean = self.mean_ma.feed(c);
        let diff = c - mean;
        let var = self.var_ma.feed(diff * diff);
        let std = var.max(0.0).sqrt();
        self.value = if std > 0.0 { diff / std } else { 0.0 };
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.mean_ma.is_ready() && self.var_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.mean_ma.reset();
        self.var_ma.reset();
        self.value = 0.0;
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Dual-mode config for [`PriceZScore`].
/// `period` is the host window; the `ma` slot controls both mean and variance smoothers.
/// With the default `follow(Sma)`, both smoothers run at `period`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct PriceZScoreConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for PriceZScore {
    const ID: IndicatorId = IndicatorId::PriceZscore;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = PriceZScoreConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::PriceZscore)];
    type Config = PriceZScoreConfig;
    type Runtime = PriceZScore;

    fn create(cfg: PriceZScoreConfig) -> PriceZScore {
        let p = cfg.period.resolved();
        let choice = cfg.ma.resolved();
        let ma_p = choice.period.resolve(p);
        PriceZScore::from_smoothers(choice.kind, ma_p)
    }

    fn source_fields(cfg: &PriceZScoreConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &PriceZScoreConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for PriceZScoreConfig {
    fn defaults() -> Self {
        PriceZScoreConfig {
            period: Param::Solo(100),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // source: Class O → auto (all 8 fields).
        // ma: #[slot] SmootherChoice → leave Solo (deferred wave).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for PriceZScore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::PriceZscore, "Price Z-Score", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_price_zscore_creation() {
        let pz = PriceZScore::new(20);
        assert!(!pz.is_ready());
        assert_eq!(pz.value(), 0.0);
    }

    #[test]
    fn test_price_zscore_warmup() {
        let mut pz = PriceZScore::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pz.feed(price);
        }
        assert!(pz.is_ready());
    }

    #[test]
    fn test_price_zscore_values() {
        let mut pz = PriceZScore::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pz.feed(price);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_price_zscore_reset() {
        let mut pz = PriceZScore::new(20);
        for i in 0..25 {
            pz.feed(100.0 + i as f64);
        }
        pz.reset();
        assert!(!pz.is_ready());
        assert_eq!(pz.value(), 0.0);
    }

    #[test]
    fn test_price_zscore_with_ema() {
        let mut pz = PriceZScore::from_smoothers(SmootherId::Ema, 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let z = pz.feed(price);
            assert!(z.is_finite());
        }
        assert!(pz.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_price_zscore() {
        let mut f = IndicatorOrder::PriceZscore(<<PriceZScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::PriceZscore).is_finite());
    }
}
