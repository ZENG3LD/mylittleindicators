//! NVI/PVI (Negative/Positive Volume Index) — Norman Fosback volume indicators.
//! NVI updates only when volume falls vs prior bar; PVI updates only when volume rises.
//! Both start at base value 1000 and compound multiplicatively on price-change-pct.

use crate::engine::contract_engine::SmootherSlot;

/// NVI/PVI indicator — dual volume-direction cumulative index.
#[derive(Debug, Clone)]
pub struct NegativePositiveVolumeIndex {
    nvi_ma: SmootherSlot,
    pvi_ma: SmootherSlot,
    prev_close: f64,
    prev_volume: f64,
    nvi_value: f64,
    pvi_value: f64,
    nvi_ma_value: f64,
    pvi_ma_value: f64,
    nvi_ma_period: usize,
    pvi_ma_period: usize,
    bars_count: usize,
    is_ready: bool,
}

impl NegativePositiveVolumeIndex {
    /// Create new NVI/PVI with default parameters (255, 255 EMA periods).
    pub fn new() -> Self {
        Self::with_params(255, 255)
    }

    pub fn with_params(nvi_ma_period: usize, pvi_ma_period: usize) -> Self {
        assert!(nvi_ma_period > 0, "NVI MA period must be greater than 0");
        assert!(pvi_ma_period > 0, "PVI MA period must be greater than 0");
        use crate::engine::contract_engine::SmootherId;
        Self {
            nvi_ma: SmootherSlot::new(SmootherId::Ema, nvi_ma_period),
            pvi_ma: SmootherSlot::new(SmootherId::Ema, pvi_ma_period),
            prev_close: 0.0,
            prev_volume: 0.0,
            nvi_value: 1000.0,
            pvi_value: 1000.0,
            nvi_ma_value: 1000.0,
            pvi_ma_value: 1000.0,
            nvi_ma_period,
            pvi_ma_period,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// lanes = [close, volume]
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let close  = lanes[0];
        let volume = lanes[1];
        self.bars_count += 1;

        if self.bars_count == 1 {
            self.prev_close = close;
            self.prev_volume = volume;
            return (self.nvi_value, self.pvi_value);
        }

        let price_change_pct = if self.prev_close.abs() > 1e-12 {
            (close - self.prev_close) / self.prev_close
        } else {
            0.0
        };

        if volume < self.prev_volume {
            self.nvi_value *= 1.0 + price_change_pct;
        }
        if volume > self.prev_volume {
            self.pvi_value *= 1.0 + price_change_pct;
        }

        self.nvi_ma_value = self.nvi_ma.feed(self.nvi_value);
        self.pvi_ma_value = self.pvi_ma.feed(self.pvi_value);

        self.prev_close = close;
        self.prev_volume = volume;

        if self.bars_count >= self.nvi_ma_period.max(self.pvi_ma_period) + 10 {
            self.is_ready = true;
        }

        (self.nvi_value, self.pvi_value)
    }

    pub fn nvi_value(&self) -> f64 { self.nvi_value }
    pub fn pvi_value(&self) -> f64 { self.pvi_value }
    pub fn nvi_ma_value(&self) -> f64 { self.nvi_ma_value }
    pub fn pvi_ma_value(&self) -> f64 { self.pvi_ma_value }
    pub fn periods(&self) -> (usize, usize) { (self.nvi_ma_period, self.pvi_ma_period) }

    /// Named output: brace `nvi`.
    pub fn nvi(&self) -> f64 { self.nvi_value }
    /// Named output: brace `pvi`.
    pub fn pvi(&self) -> f64 { self.pvi_value }


    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.nvi_ma.reset();
        self.pvi_ma.reset();
        self.prev_close = 0.0;
        self.prev_volume = 0.0;
        self.nvi_value = 1000.0;
        self.pvi_value = 1000.0;
        self.nvi_ma_value = 1000.0;
        self.pvi_ma_value = 1000.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
}

impl Default for NegativePositiveVolumeIndex {
    fn default() -> Self {
        Self::with_params(255, 255)
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Config for [`NegativePositiveVolumeIndex`] — two independent EMA periods.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct NviPviConfig {
    pub nvi_ma_period: Param<usize>,
    pub pvi_ma_period: Param<usize>,
}

impl Indicator for NegativePositiveVolumeIndex {
    const ID: IndicatorId = IndicatorId::NviPvi;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed: Close then Volume — NVI/PVI always uses these two fields.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::flow(IndicatorOutputId::NviPviNvi),
        Output::flow(IndicatorOutputId::NviPviPvi),
    ];
    type Config = NviPviConfig;
    type Runtime = NegativePositiveVolumeIndex;

    fn create(cfg: NviPviConfig) -> NegativePositiveVolumeIndex {
        NegativePositiveVolumeIndex::with_params(
            cfg.nvi_ma_period.resolved().max(1),
            cfg.pvi_ma_period.resolved().max(1),
        )
    }
}

impl crate::contract::Config for NviPviConfig {
    fn defaults() -> Self {
        NviPviConfig {
            nvi_ma_period: Param::Solo(255),
            pvi_ma_period: Param::Solo(255),
        }
    }
    fn machine_defaults() -> Self {
        // nvi_ma_period/pvi_ma_period: Class A sub-period (taxonomy: nvi_ma_period/pvi_ma_period
        // are MA smoothing periods) — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for NegativePositiveVolumeIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::NviPviNvi, "NVI", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::NviPviPvi, "PVI", Color::hex(0x4CAF50))
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
    fn test_nvi_pvi_creation() {
        let ind = NegativePositiveVolumeIndex::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.nvi_value(), 1000.0);
        assert_eq!(ind.pvi_value(), 1000.0);
    }

    #[test]
    fn test_nvi_pvi_with_params() {
        let ind = NegativePositiveVolumeIndex::with_params(20, 20);
        assert!(!ind.is_ready());
        assert_eq!(ind.periods(), (20, 20));
    }

    #[test]
    fn test_nvi_pvi_update() {
        let mut ind = NegativePositiveVolumeIndex::with_params(10, 10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let volume = 1000.0 + (i as f64 * 100.0);
            ind.feed(&[price, volume]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_nvi_pvi_values_finite() {
        let mut ind = NegativePositiveVolumeIndex::with_params(10, 10);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let volume = if i % 2 == 0 { 1000.0 } else { 2000.0 };
            let (nvi, pvi) = ind.feed(&[price, volume]);
            assert!(nvi.is_finite());
            assert!(pvi.is_finite());
        }
    }

    #[test]
    fn test_nvi_pvi_reset() {
        let mut ind = NegativePositiveVolumeIndex::with_params(10, 10);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            ind.feed(&[price, 1000.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.nvi_value(), 1000.0);
        assert_eq!(ind.pvi_value(), 1000.0);
    }

    #[test]
    fn factory_feeds_resolved_nvi_pvi() {
        let mut f = IndicatorOrder::NviPvi(
            <<NegativePositiveVolumeIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        )
        .build_solo()
        .unwrap();
        for i in 0..300 {
            let price = 100.0 + (i as f64 * 0.05).sin() * 5.0;
            let volume = if i % 2 == 0 { 1000.0 } else { 2000.0 };
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume,
            });
        }
        assert!(f.read(IndicatorOutputId::NviPviNvi).is_finite());
    }
}
