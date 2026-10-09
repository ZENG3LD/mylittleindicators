use crate::engine::contract_engine::SmootherId;

use crate::indicators::volatility::atr::Atr;

/// Random Walk Index (simplified):
/// rwi_up = (High - prev_low) / (ATR(period) * sqrt(period))
/// rwi_down = (prev_high - Low) / (ATR(period) * sqrt(period))
#[derive(Debug, Clone)]
pub struct Rwi {
    period: usize,
    atr: Atr,
    prev_high: Option<f64>,
    prev_low: Option<f64>,
    up: f64,
    down: f64,
}

impl Rwi {
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Create RWI with configurable ATR smoothing.
    ///
    /// # Arguments
    /// * `period`   - Lookback period (minimum 2)
    /// * `smoother` - Smoother kind for internal ATR (default RMA)
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            period: period.max(2),
            atr: Atr::from_smoother(period.max(2), smoother),
            prev_high: None,
            prev_low: None,
            up: 0.0,
            down: 0.0,
        }
    }

    /// Feed the resolved input lanes — `[high, low, close]` (const SOURCE = KlineSlice[H,L,C]).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let _ = self.atr.feed(&[h, l, c]);
        let atrv = self.atr.value();
        let denom = (atrv * (self.period as f64).sqrt()).max(1e-12);
        if let (Some(ph), Some(pl)) = (self.prev_high, self.prev_low) {
            self.up = (h - pl).max(0.0) / denom;
            self.down = (ph - l).max(0.0) / denom;
        }
        self.prev_high = Some(h);
        self.prev_low = Some(l);
        (self.up, self.down)
    }

    pub fn is_ready(&self) -> bool {
        self.atr.is_ready() && self.prev_high.is_some() && self.prev_low.is_some()
    }
    pub fn reset(&mut self) {
        self.atr.reset();
        self.prev_high = None;
        self.prev_low = None;
        self.up = 0.0;
        self.down = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }

    #[inline]
    pub fn high(&self) -> f64 {
        self.up
    }

    #[inline]
    pub fn low(&self) -> f64 {
        self.down
    }
}

impl Default for Rwi {
    /// Factory default: period = 14, ATR smoothed with RMA.
    fn default() -> Self {
        Self::from_smoother(14, SmootherId::Rma)
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::contract::{Color, ReferenceLine, Render, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`Rwi`] — Random Walk Index.
/// Fixed H/L/C source; the embedded ATR uses its legacy default RMA.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RwiConfig {
    pub period: Param<usize>,
}

impl Indicator for Rwi {
    const ID: IndicatorId = IndicatorId::Rwi;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C slices — the inner ATR is also H/L/C.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// RWI is O(1) per bar. Inner Atr edge carries its full cost.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::RwiHigh),
        Output::magnitude(IndicatorOutputId::RwiLow),
    ];
    type Config = RwiConfig;
    type Runtime = Rwi;

    fn create(cfg: RwiConfig) -> Rwi {
        let p = cfg.period.resolved().max(2);
        Rwi {
            period: p,
            atr: Atr::from_smoother(p, SmootherId::Rma),
            prev_high: None,
            prev_low: None,
            up: 0.0,
            down: 0.0,
        }
    }
}

impl crate::contract::Config for RwiConfig {
    fn defaults() -> Self {
        RwiConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1) covers it.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Rwi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::RwiHigh, "RWI High", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::RwiLow, "RWI Low", Color::hex(0xF44336), 2.0))
            .reference_line(ReferenceLine::new(1.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn test_rwi_creation() {
        let rwi = Rwi::new(14);
        assert!(!rwi.is_ready());
        assert_eq!((rwi.high(), rwi.low()), (0.0, 0.0));
        assert_eq!(rwi.period(), 14);
    }

    #[test]
    fn test_rwi_from_smoother() {
        let mut rwi = Rwi::from_smoother(14, SmootherId::Ema);
        for i in 1..=30 {
            let p = 100.0 + i as f64;
            let (up, down) = rwi.feed(&[p + 1.0, p - 1.0, p]);
            assert!(up.is_finite() && down.is_finite());
        }
        assert!(rwi.is_ready());
    }

    #[test]
    fn test_rwi_uptrend() {
        let mut rwi = Rwi::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            rwi.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(rwi.is_ready());
        let (up, down) = (rwi.high(), rwi.low());
        assert!(up > down, "RWI up should > down in uptrend, got up={}, down={}", up, down);
    }

    #[test]
    fn test_rwi_downtrend() {
        let mut rwi = Rwi::new(14);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            rwi.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(rwi.is_ready());
        let (up, down) = (rwi.high(), rwi.low());
        assert!(down > up, "RWI down should > up in downtrend, got up={}, down={}", up, down);
    }

    #[test]
    fn test_rwi_finite_values() {
        let mut rwi = Rwi::new(14);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (up, down) = rwi.feed(&[price + 2.0, price - 2.0, price]);
            assert!(up.is_finite() && down.is_finite(), "RWI values should always be finite");
            assert!(up >= 0.0 && down >= 0.0, "RWI values should be non-negative");
        }
    }

    #[test]
    fn test_rwi_reset() {
        let mut rwi = Rwi::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            rwi.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(rwi.is_ready());
        rwi.reset();
        assert!(!rwi.is_ready());
        assert_eq!((rwi.high(), rwi.low()), (0.0, 0.0));
    }

    #[test]
    fn factory_feeds_resolved_rwi() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Rwi(<<Rwi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // In an uptrend rwi_up should be positive
        assert!(f.primary() > 0.0);
    }
}
