// Price Median Absolute Deviation Z-Score over rolling window

#[derive(Debug, Clone)]
pub struct PriceMadZscore {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl PriceMadZscore {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(3),
            buf: vec![0.0; window.max(3)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, close: f64) -> f64 {
        self.buf[self.idx] = close;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return self.value;
        }

        let len = self.window;
        let mut tmp = self.buf[..len].to_vec();
        tmp.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if len % 2 == 1 {
            tmp[len / 2]
        } else {
            0.5 * (tmp[len / 2 - 1] + tmp[len / 2])
        };
        let mut dev: Vec<f64> = tmp.iter().map(|v| (v - median).abs()).collect();
        dev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mad = if len % 2 == 1 {
            dev[len / 2]
        } else {
            0.5 * (dev[len / 2 - 1] + dev[len / 2])
        };
        let denom = (mad * 1.4826).max(1e-12);
        self.value = (close - median) / denom;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

impl Default for PriceMadZscore {
    /// Factory defaults: window=100 (clamped to max(100,3)=100).
    fn default() -> Self {
        Self::new(100)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`PriceMadZscore`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ZmadConfig {
    pub period: Param<usize>,
}

impl Indicator for PriceMadZscore {
    const ID: IndicatorId = IndicatorId::Zmad;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Zmad)];
    /// O(n log n): sort of window buffer on every bar.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = ZmadConfig;
    type Runtime = PriceMadZscore;

    fn create(cfg: ZmadConfig) -> PriceMadZscore {
        PriceMadZscore::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &ZmadConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for ZmadConfig {
    fn defaults() -> Self {
        ZmadConfig { period: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for PriceMadZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Zmad, "Z-Score MAD", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::indicator_id::IndicatorId;
    use crate::contract::Indicator;

    #[test]
    fn test_price_mad_zscore_creation() {
        let pmz = PriceMadZscore::new(20);
        assert!(!pmz.is_ready());
        assert_eq!(pmz.value(), 0.0);
        assert_eq!(pmz.window(), 20);
    }

    #[test]
    fn test_price_mad_zscore_warmup() {
        let mut pmz = PriceMadZscore::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pmz.feed(price);
        }
        assert!(pmz.is_ready());
    }

    #[test]
    fn test_price_mad_zscore_finite() {
        let mut pmz = PriceMadZscore::new(20);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pmz.feed(price);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_price_mad_zscore_reset() {
        let mut pmz = PriceMadZscore::new(20);
        for i in 0..30 {
            pmz.feed(100.0 + i as f64);
        }
        pmz.reset();
        assert!(!pmz.is_ready());
        assert_eq!(pmz.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_zmad() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Zmad(<<PriceMadZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..150 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::Zmad).is_finite());
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<PriceMadZscore as Indicator>::ID, IndicatorId::Zmad);
    }
}
