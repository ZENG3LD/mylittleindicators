// MACD Histogram Z-Score over rolling window

use crate::indicators::momentum::macd::Macd;

/// MACD Histogram Z-Score.
///
/// Normalises the MACD histogram over a rolling window:
/// z = (hist - mean(hist, window)) / std(hist, window)
///
/// Uses the contracted `Macd` core (close fed on both lanes), then tracks a
/// rolling ring-buffer of histogram values to compute the running z-score.
#[derive(Debug, Clone)]
pub struct MacdHistZscore {
    macd: Macd,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl MacdHistZscore {
    pub fn new(
        fast: usize,
        slow: usize,
        signal: usize,
        window: usize,
    ) -> Self {
        let macd = Macd::new_with_signal(fast, slow, signal);
        Self {
            macd,
            window: window.max(2),
            buf: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.macd.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.macd.is_ready()
    }

    /// Feed ONE pre-extracted scalar (close).
    pub fn feed(&mut self, close: f64) -> f64 {
        // MACD is a pure two-lane core; this composite is close-based.
        let _macd_v = self.macd.feed(&[close, close]);
        let hist = self.macd.value_histogram();
        let old = self.buf[self.idx];
        self.buf[self.idx] = hist;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        self.sum += hist - old;
        self.sumsq += hist * hist - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.value = if std > 1e-12 {
                (hist - mean) / std
            } else {
                0.0
            };
        } else {
            self.value = 0.0;
        }
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

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`MacdHistZscore`].
///
/// Absorbs the MACD knobs (fast/slow/signal) as own `Param<usize>` fields.
/// Dual-mode: every field is a `Param`. No smoother slots (MACD internals are
/// classic EMA, not user-configurable here).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MacdHistZscoreConfig {
    pub fast: Param<usize>,
    pub slow: Param<usize>,
    pub signal: Param<usize>,
    pub window: Param<usize>,
}

impl Indicator for MacdHistZscore {
    const ID: IndicatorId = IndicatorId::MacdHistZ;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) own update; cost of embedded MACD is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Macd, &[
            IndicatorOutputId::MacdLine,
            IndicatorOutputId::MacdSignal,
            IndicatorOutputId::MacdHistogram,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::MacdHistZ)];
    type Config = MacdHistZscoreConfig;
    type Runtime = MacdHistZscore;

    fn create(cfg: MacdHistZscoreConfig) -> MacdHistZscore {
        MacdHistZscore::new(
            cfg.fast.resolved(),
            cfg.slow.resolved(),
            cfg.signal.resolved(),
            cfg.window.resolved(),
        )
    }

    fn source_fields(cfg: &MacdHistZscoreConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        // Close only — both MACD lanes are close.
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for MacdHistZscoreConfig {
    fn defaults() -> Self {
        MacdHistZscoreConfig {
            fast: Param::Solo(12),
            slow: Param::Solo(26),
            signal: Param::Solo(9),
            window: Param::Solo(100),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast.resolved();
        let slow = self.slow.resolved();
        if fast >= slow {
            return Err(format!("fast({fast}) >= slow({slow})"));
        }
        Ok(())
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fast/slow/signal/window: Class A → auto range(2,4048,1) EACH, but fast and slow then
        // both resolve to the same min (2), failing this config's OWN `valid_params`
        // (fast < slow) at the min corner (2026-07-03 fix). Split into disjoint ranges so
        // `resolved()` stays ordered; signal/window (independent lanes) keep the full auto range.
        let mut s = Self::machine_defaults_auto();
        s.fast = Param::range(1, 100, 1);
        s.slow = Param::range(101, 10000, 1);
        s
    }
}


impl Render for MacdHistZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MacdHistZ, "MACD Hist Z", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl Default for MacdHistZscore {
    fn default() -> Self {
        Self::new(12, 26, 9, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macd_hist_zscore_creation() {
        let mhz = MacdHistZscore::new(12, 26, 9, 20);
        assert!(!mhz.is_ready());
        assert_eq!(mhz.value(), 0.0);
        assert_eq!(mhz.window(), 20);
    }

    #[test]
    fn test_macd_hist_zscore_basic() {
        let mut mhz = MacdHistZscore::new(12, 26, 9, 20);
        for i in 1..=60 {
            let price = 100.0 + i as f64 * 2.0;
            mhz.feed(price);
        }
        assert!(mhz.is_ready());
        assert!(mhz.value().is_finite());
    }

    #[test]
    fn test_macd_hist_zscore_reset() {
        let mut mhz = MacdHistZscore::new(12, 26, 9, 20);
        for i in 1..=60 {
            mhz.feed(100.0 + i as f64);
        }
        assert!(mhz.is_ready());
        mhz.reset();
        assert!(!mhz.is_ready());
        assert_eq!(mhz.value(), 0.0);
    }

    #[test]
    fn test_macd_hist_zscore_finite_values() {
        let mut mhz = MacdHistZscore::new(12, 26, 9, 20);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = mhz.feed(price);
            assert!(value.is_finite(), "MACD Hist Zscore should always be finite");
        }
    }

    #[test]
    fn factory_feeds_resolved_macd_hist_z() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<MacdHistZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::MacdHistZ(cfg).build_solo().unwrap();
        for i in 1..=100 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
