// DPO Bands on oscillator scale: upper/lower = +/- k * std(DPO)

use crate::indicators::momentum::dpo::DetrendedPriceOscillator;

#[derive(Debug, Clone)]
pub struct DpoBands {
    dpo: DetrendedPriceOscillator,
    window: usize,
    k: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for DpoBands {
    fn default() -> Self {
        Self::new(14, 14, 2.0)
    }
}

impl DpoBands {
    pub fn new(period: usize, window: usize, k: f64) -> Self {
        Self {
            dpo: DetrendedPriceOscillator::with_period(period.max(2)),
            window: window.clamp(5, 512),
            k: if k > 0.0 { k } else { 2.0 },
            buf: Vec::with_capacity(window.clamp(5, 512)),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Create DpoBands with a specific MA type for the internal DPO.
    pub fn with_ma_type(period: usize, window: usize, k: f64, ma_type: crate::engine::contract_engine::SmootherId) -> Self {
        let win = window.clamp(5, 512);
        Self {
            dpo: DetrendedPriceOscillator::with_period_and_ma_type(period.max(2), ma_type),
            window: win,
            k: if k > 0.0 { k } else { 2.0 },
            buf: Vec::with_capacity(win),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Feed a resolved close price scalar (the outer's `Source` = Close).
    /// The inner DPO's transitional bridge is called with the scalar as the close.
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        let d = self.dpo.feed(price);
        self.push_and_compute(d);
        (self.upper, self.middle, self.lower)
    }

    /// Legacy bar-level update kept for existing tests.

    fn push_and_compute(&mut self, d: f64) {
        if self.buf.len() < self.window {
            self.buf.push(d);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = d;
        }
        self.idx = (self.idx + 1) % self.window;
        self.middle = 0.0;
        if self.is_ready() {
            let mean = self.buf.iter().sum::<f64>() / (self.window as f64);
            let var = self.buf.iter()
                .map(|&x| { let dd = x - mean; dd * dd })
                .sum::<f64>() / (self.window as f64);
            let sd = var.sqrt();
            self.upper = self.k * sd;
            self.lower = -self.k * sd;
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.dpo.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool { self.filled && self.dpo.is_ready() }


    pub fn value_tuple(&self) -> (f64, f64, f64) { (self.upper, self.middle, self.lower) }
    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }
    pub fn window(&self) -> usize { self.window }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

use crate::contract::{Param, sweep_f64};

/// Typed dual-mode config for [`DpoBands`] — DPO Bands.
///
/// `dpo_period` controls the inner DPO's period; `window` and `k` are the outer's
/// rolling-std window and band multiplier. No smoother slot on the outer — the MA
/// shape lives inside the contracted DPO (accessed via its `Port`).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DpoBandsConfig {
    pub dpo_period: Param<usize>,
    pub window: Param<usize>,
    pub k: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for DpoBands {
    const ID: IndicatorId = IndicatorId::Dpobands;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close); the inner DPO reads the same.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Dpo, &[IndicatorOutputId::Dpo])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::DpobandsUpper),
        Output::centered(IndicatorOutputId::DpobandsMiddle),
        Output::centered(IndicatorOutputId::DpobandsLower),
    ];
    type Config = DpoBandsConfig;
    type Runtime = Self;

    fn create(cfg: DpoBandsConfig) -> Self {
        DpoBands::new(cfg.dpo_period.resolved(), cfg.window.resolved(), cfg.k.resolved())
    }

    fn source_fields(cfg: &DpoBandsConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for DpoBandsConfig {
    /// dpo_period 14, window 14, k 2.0, source Close.
    fn defaults() -> Self {
        DpoBandsConfig {
            dpo_period: Param::Solo(14),
            window: Param::Solo(14),
            k: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // dpo_period/window→range(2,4048,1), source→all 8
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for DpoBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::DpobandsUpper, "Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::DpobandsMiddle, "Middle", Color::hex(0x9E9E9E))
            .line_output(IndicatorOutputId::DpobandsLower, "Lower", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_ma_type_non_default() {
        use crate::engine::contract_engine::SmootherId;
        let mut db = DpoBands::with_ma_type(14, 20, 2.0, SmootherId::Ema);
        assert!(!db.is_ready());
        for i in 0..50 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let (u, m, l) = db.feed(p);
            assert!(u.is_finite());
            assert!(m.is_finite());
            assert!(l.is_finite());
        }
        assert!(db.is_ready());
    }

    #[test]
    fn test_dpo_bands_creation() {
        let db = DpoBands::new(14, 20, 2.0);
        assert!(!db.is_ready());
        assert_eq!(db.window(), 20);
    }

    #[test]
    fn test_dpo_bands_warmup() {
        let mut db = DpoBands::new(14, 20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            db.feed(price);
        }
        assert!(db.is_ready());
    }

    #[test]
    fn test_dpo_bands_symmetric() {
        let mut db = DpoBands::new(14, 20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = db.feed(price);
            if db.is_ready() {
                assert_eq!(middle, 0.0, "Middle should be 0");
                assert!((upper + lower).abs() < 1e-9, "Bands should be symmetric around 0");
            }
        }
    }

    #[test]
    fn test_dpo_bands_reset() {
        let mut db = DpoBands::new(14, 20, 2.0);
        for i in 0..50 {
            db.feed(100.0 + i as f64);
        }
        db.reset();
        assert!(!db.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_dpobands() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DpoBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Dpobands(cfg).build_solo().unwrap();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
