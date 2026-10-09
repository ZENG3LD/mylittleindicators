// Chande Kroll Stop — ATR-based stop levels with separate H/H and L/L lookback periods.

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::SmootherId;

/// Chande Kroll Stop — ATR-multiplied stop from highest-high / lowest-low windows.
///
/// long_stop  = HH(hh_period) - k × ATR(atr_period)
/// short_stop = LL(ll_period) + k × ATR(atr_period)
#[derive(Debug, Clone)]
pub struct ChandeKrollStop {
    k: f64,
    hh_period: usize,
    ll_period: usize,
    atr: Atr,
    highs: Vec<f64>,
    lows: Vec<f64>,
    long_stop: f64,
    short_stop: f64,
    ready: bool,
}

impl ChandeKrollStop {
    /// Default parameters: atr_period=14, k=1.5, hh_period=22, ll_period=22 (RMA ATR).
    pub fn new(atr_period: usize, k: f64, hh_period: usize, ll_period: usize) -> Self {
        Self {
            k: if k > 0.0 { k } else { 1.5 },
            hh_period: hh_period.max(1),
            ll_period: ll_period.max(1),
            atr: Atr::from_smoother(atr_period.max(1), SmootherId::Rma),
            highs: Vec::with_capacity(512),
            lows: Vec::with_capacity(512),
            long_stop: 0.0,
            short_stop: 0.0,
            ready: false,
        }
    }

    /// Build from a pre-built `Atr` instance (contract creation path).
    fn from_atr(k: f64, hh_period: usize, ll_period: usize, atr: Atr) -> Self {
        Self {
            k,
            hh_period,
            ll_period,
            atr,
            highs: Vec::with_capacity(512),
            lows: Vec::with_capacity(512),
            long_stop: 0.0,
            short_stop: 0.0,
            ready: false,
        }
    }

    /// Feed one bar's `[high, low, close]` lanes.
    /// Returns `(long_stop, short_stop)`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        // Drive the inner ATR — open/volume are ignored.
        let atr = self.atr.feed(&[high, low, close]);

        if self.highs.len() >= self.hh_period {
            self.highs.remove(0);
        }
        if self.lows.len() >= self.ll_period {
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);

        let hh = self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ll = self.lows.iter().cloned().fold(f64::INFINITY, f64::min);

        self.long_stop = hh - self.k * atr;
        self.short_stop = ll + self.k * atr;

        self.ready = self.atr.is_ready()
            && self.highs.len() >= self.hh_period
            && self.lows.len() >= self.ll_period;

        (self.long_stop, self.short_stop)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.long_stop
    }

    #[inline]
    pub fn levels(&self) -> (f64, f64) {
        (self.long_stop, self.short_stop)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.highs.clear();
        self.lows.clear();
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.ready = false;
    }
}

impl Default for ChandeKrollStop {
    /// Factory default: atr_period=14, k=1.5, hh_period=22, ll_period=22 (RMA ATR).
    fn default() -> Self {
        Self::new(14, 1.5, 22, 22)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::SmootherSlotOrder;
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
    sweep_f64,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`ChandeKrollStop`].
/// `atr_period` = ATR period; `k` = ATR multiplier;
/// `hh_period` / `ll_period` = extreme-window periods;
/// `atr_smoother` = inner ATR smoother order (member + its own period).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct CksConfig {
    pub atr_period: Param<usize>,
    pub k: Param<f64>,
    pub hh_period: Param<usize>,
    pub ll_period: Param<usize>,
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for ChandeKrollStop {
    const ID: IndicatorId = IndicatorId::Cks;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// H/L/C fixed lanes — outer uses H/L for HH/LL windows; inner ATR uses H/L/C.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(max(hh_period, ll_period)) — rescans both extrema windows.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Cks)];

    type Config = CksConfig;
    type Runtime = ChandeKrollStop;

    fn create(cfg: CksConfig) -> ChandeKrollStop {
        let atr_order = cfg.atr_smoother.resolved();
        let atr = Atr::from_smoother(atr_order.period(), atr_order.id());
        ChandeKrollStop::from_atr(
            cfg.k.resolved(),
            cfg.hh_period.resolved(),
            cfg.ll_period.resolved(),
            atr,
        )
    }
}

impl crate::contract::Config for CksConfig {
    fn defaults() -> Self {
        CksConfig {
            atr_period: Param::Solo(14),
            k: Param::Solo(1.5),
            hh_period: Param::Solo(22),
            ll_period: Param::Solo(22),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 14 })),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // atr_period/hh_period/ll_period→range(2,4048,1)
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier (k_multiplier pattern)
        s
    }
}


impl Render for ChandeKrollStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Cks, "CKS Stop", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Param;

    #[test]
    fn test_chande_kroll_stop_creation() {
        let ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_chande_kroll_stop_warmup() {
        let mut ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_chande_kroll_stop_values_finite() {
        let mut ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_chande_kroll_stop_reset() {
        let mut ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        for i in 0..20 {
            ind.feed(&[105.0 + i as f64, 95.0, 101.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    /// Factory resolves the fixed H/L/C lanes from `const SOURCE`.
    /// Open and Volume are wild (not used by outer or inner ATR).
    #[test]
    fn factory_feeds_resolved_cks() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Cks(<<ChandeKrollStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();

        for i in 1..=30usize {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 102.0 + i as f64,
                low: 98.0,
                close: 100.0 + i as f64,
                volume: -1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }

    #[test]
    fn test_cks_dual_mode_own_period() {
        let cfg = CksConfig {
            atr_period: Param::Solo(14),
            k: Param::Solo(1.5),
            hh_period: Param::Solo(22),
            ll_period: Param::Solo(22),
            atr_smoother: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 10 })),
        };
        let mut ind = <ChandeKrollStop as Indicator>::create(cfg);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }
}
