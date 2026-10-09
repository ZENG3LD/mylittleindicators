// Volatility Percentile Rank Bands: middle=price, bands by ATR percentile rank

use crate::indicators::volatility::atr::Atr;
use crate::indicators::utils::math::percentile::quickselect_nth;

#[derive(Debug, Clone)]
pub struct VolatilityPercentileRankBands {
    atr: Atr,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for VolatilityPercentileRankBands {
    fn default() -> Self {
        Self::new(14, 100)
    }
}

impl VolatilityPercentileRankBands {
    /// Default ctor — ATR smoothed with EMA (original default).
    pub fn new(atr_period: usize, rank_window: usize) -> Self {
        use crate::engine::contract_engine::SmootherId;
        Self {
            atr: Atr::from_smoother(atr_period.max(1), SmootherId::Ema),
            window: rank_window.clamp(5, 10000),
            buf: Vec::with_capacity(rank_window.clamp(5, 10000)),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.atr.is_ready()
    }
    #[inline]
    pub fn upper(&self) -> f64 {
        self.upper
    }

    #[inline]
    pub fn middle(&self) -> f64 {
        self.middle
    }

    #[inline]
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// Feed resolved [High, Low, Close] lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let atr = self.atr.feed(&[high, low, close]);
        let val = atr.max(1e-12);
        if self.buf.len() < self.window {
            self.buf.push(val);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = val;
        }
        self.idx = (self.idx + 1) % self.window;
        self.middle = close;
        if self.is_ready() {
            let mut temp: Vec<f64> = self.buf.iter().copied().collect();
            let len = temp.len();
            let p20 = quickselect_nth(&mut temp[..], (len * 20) / 100);
            let p80 = quickselect_nth(&mut temp[..], (len * 80) / 100);
            self.upper = close + p80;
            self.lower = close - p20;
        }
        (self.upper, self.middle, self.lower)
    }

}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`VolatilityPercentileRankBands`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VprbConfig {
    pub atr_period: Param<usize>,
    pub rank_window: Param<usize>,
}

impl Indicator for VolatilityPercentileRankBands {
    const ID: IndicatorId = IndicatorId::Vprb;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C — ATR needs H/L/C; middle = close.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::VprbUpper),
        Output::price(IndicatorOutputId::VprbMiddle),
        Output::price(IndicatorOutputId::VprbLower),
    ];
    type Config = VprbConfig;
    type Runtime = VolatilityPercentileRankBands;

    fn create(cfg: VprbConfig) -> VolatilityPercentileRankBands {
        VolatilityPercentileRankBands::new(cfg.atr_period.resolved(), cfg.rank_window.resolved())
    }
}

impl crate::contract::Config for VprbConfig {
    fn defaults() -> Self {
        VprbConfig {
            atr_period: Param::Solo(14),
            rank_window: Param::Solo(100),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // atr_period/rank_window→range(2,4048,1); rank_window clamped 5..10000 in new()
    }
}


impl Render for VolatilityPercentileRankBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::VprbUpper, "Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::VprbMiddle, "Vol", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::VprbLower, "Lower", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_vprb() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<VolatilityPercentileRankBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Vprb(cfg).build_solo().unwrap();
        for i in 1..=120usize {
            let price = 100.0 + i as f64 * 0.5;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,          // not used
                high: price + 1.5,
                low: price - 1.5,
                close: price,
                volume: 9999.0,       // not used
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "vprb upper should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_percentile_rank_bands_creation() {
        let vprb = VolatilityPercentileRankBands::new(14, 50);
        assert!(!vprb.is_ready());
        assert_eq!(vprb.upper(), 0.0);
        assert_eq!(vprb.middle(), 0.0);
        assert_eq!(vprb.lower(), 0.0);
    }

    #[test]
    fn test_volatility_percentile_rank_bands_warmup() {
        let mut vprb = VolatilityPercentileRankBands::new(14, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vprb.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(vprb.is_ready());
    }

    #[test]
    fn test_volatility_percentile_rank_bands_ordering() {
        let mut vprb = VolatilityPercentileRankBands::new(14, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = vprb.feed(&[price + 1.0, price - 1.0, price]);
            if vprb.is_ready() {
                assert!(upper >= middle, "Upper should be >= middle");
                assert!(middle >= lower, "Middle should be >= lower");
            }
        }
    }

    #[test]
    fn test_volatility_percentile_rank_bands_reset() {
        let mut vprb = VolatilityPercentileRankBands::new(14, 50);
        for i in 0..60 {
            vprb.feed(&[101.0 + i as f64 * 0.0, 99.0, 100.0 + i as f64]);
        }
        vprb.reset();
        assert!(!vprb.is_ready());
        assert_eq!(vprb.upper(), 0.0);
        assert_eq!(vprb.middle(), 0.0);
        assert_eq!(vprb.lower(), 0.0);
    }
}
