// Percentile of Volatility-of-Volatility over rolling window

use crate::indicators::volatility::vol_of_vol::{VoVSource, VolOfVol};

#[derive(Debug, Clone)]
pub struct VolOfVolPercentile {
    vov: VolOfVol,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl VolOfVolPercentile {
    /// AbsReturn VoV + percentile window.
    pub fn with_period(vov_window: usize, percentile_window: usize) -> Self {
        let w = percentile_window.max(1);
        Self {
            vov: VolOfVol::new(VoVSource::AbsReturn, vov_window),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.vov.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let cur = self.vov.feed(lanes).abs();
        let _old = self.buf[self.idx];
        self.buf[self.idx] = cur;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut le = 0usize;
            for i in 0..len {
                if self.buf[i] <= cur {
                    le += 1;
                }
            }
            self.value = le as f64 / len as f64;
        }
        self.value
    }

    #[inline]
    pub fn value_f64(&self) -> f64 {
        self.value
    }

}

impl Default for VolOfVolPercentile {
    fn default() -> Self {
        Self::with_period(20, 100)
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`VolOfVolPercentile`] — VoV window + percentile window.
/// The contracted path always uses the AbsReturn VoV (close-only) — no embedded ATR.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VovpConfig {
    pub vov_period: Param<usize>,
    pub percentile_window: Param<usize>,
}

impl VolOfVolPercentile {
    /// The typed `value()` the factory reads.
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Indicator for VolOfVolPercentile {
    const ID: IndicatorId = IndicatorId::Vovp;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bar-fed: forwards high/low/close to the embedded VoV.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Vov, &[IndicatorOutputId::Vov])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Vovp)];
    type Config = VovpConfig;
    type Runtime = VolOfVolPercentile;

    fn create(cfg: VovpConfig) -> VolOfVolPercentile {
        VolOfVolPercentile::with_period(cfg.vov_period.resolved(), cfg.percentile_window.resolved())
    }
}

impl crate::contract::Config for VovpConfig {
    fn defaults() -> Self {
        VovpConfig { vov_period: Param::Solo(20), percentile_window: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // vov_period, percentile_window: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolOfVolPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vovp, "VoV Percentile", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_of_vol_percentile_creation() {
        let vovp = VolOfVolPercentile::with_period(20, 50);
        assert!(!vovp.is_ready());
        assert_eq!(vovp.value_f64(), 0.0);
    }

    #[test]
    fn test_vol_of_vol_percentile_warmup() {
        let mut vovp = VolOfVolPercentile::with_period(20, 50);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vovp.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(vovp.is_ready());
    }

    #[test]
    fn test_vol_of_vol_percentile_range() {
        let mut vovp = VolOfVolPercentile::with_period(20, 50);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vovp.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= 0.0 && value <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_vol_of_vol_percentile_reset() {
        let mut vovp = VolOfVolPercentile::with_period(20, 50);
        for i in 0..70 {
            let p = 100.0 + i as f64;
            vovp.feed(&[p + 1.0, p - 1.0, p]);
        }
        vovp.reset();
        assert!(!vovp.is_ready());
        assert_eq!(vovp.value_f64(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vovp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Vovp(<<VolOfVolPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.read(IndicatorOutputId::Vovp) >= 0.0 && f.read(IndicatorOutputId::Vovp) <= 1.0);
    }
}
