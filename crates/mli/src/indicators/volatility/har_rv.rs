// HAR-RV: Heterogeneous AutoRegressive model of Realized Volatility proxy
// Minimal online proxy: combine short/medium/long horizon RVs

use crate::indicators::volatility::realized_vol::RealizedVol;

#[derive(Debug, Clone)]
pub struct HarRv {
    d: RealizedVol,
    w: RealizedVol,
    m: RealizedVol,
    value: f64,
}

impl HarRv {
    pub fn new(day_win: usize, week_win: usize, month_win: usize, annualize_factor: f64) -> Self {
        Self {
            d: RealizedVol::new(day_win.max(1), annualize_factor),
            w: RealizedVol::new(week_win.max(1), annualize_factor),
            m: RealizedVol::new(month_win.max(1), annualize_factor),
            value: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.d.reset();
        self.w.reset();
        self.m.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.d.is_ready() && self.w.is_ready() && self.m.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn feed(&mut self, close: f64) -> f64 {
        let rd = self.d.feed(close);
        let rw = self.w.feed(close);
        let rm = self.m.feed(close);
        // Simple convex combo; coefficients can be tuned offline
        self.value = 0.6 * rd + 0.3 * rw + 0.1 * rm;
        self.value
    }

}

impl Default for HarRv {
    fn default() -> Self {
        Self::new(5, 22, 66, 252.0_f64.sqrt())
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

/// Typed dual-mode config for [`HarRv`] — three horizon windows + annualization factor.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HarConfig {
    pub day_win: Param<usize>,
    pub week_win: Param<usize>,
    pub month_win: Param<usize>,
    /// Annualization factor — `252.0_f64.sqrt()` for daily bars; 0.0 = raw.
    pub annualize_factor: Param<f64>,
}

impl Indicator for HarRv {
    const ID: IndicatorId = IndicatorId::Har;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close-to-close returns — reads close only, same as inner Rv. Configurable field
    /// (default Close) since HAR is a pure log-return indicator — the field is the price
    /// from which log returns are computed.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[
            Port::new(IndicatorId::Rv, &[IndicatorOutputId::Rv]),
            Port::new(IndicatorId::Rv, &[IndicatorOutputId::Rv]),
            Port::new(IndicatorId::Rv, &[IndicatorOutputId::Rv]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Har)];
    type Config = HarConfig;
    type Runtime = HarRv;

    fn create(cfg: HarConfig) -> HarRv {
        HarRv::new(
            cfg.day_win.resolved(),
            cfg.week_win.resolved(),
            cfg.month_win.resolved(),
            cfg.annualize_factor.resolved(),
        )
    }
}

impl crate::contract::Config for HarConfig {
    fn defaults() -> Self {
        HarConfig {
            day_win: Param::Solo(5),
            week_win: Param::Solo(22),
            month_win: Param::Solo(66),
            annualize_factor: Param::Solo(252.0_f64.sqrt()),
        }
    }
    fn machine_defaults() -> Self {
        // day_win, week_win, month_win: Class A usize — auto range(2,4048,1)
        // annualize_factor: Class J PIN (discrete {252,365,8760} instrument/timeframe constant)
        //   — auto leaves f64 Solo, confirmed correct, do NOT sweep
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for HarRv {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Har, "HAR", Color::hex(0x2196F3))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_har_rv_creation() {
        let har = HarRv::new(5, 20, 60, 252.0);
        assert!(!har.is_ready());
        assert_eq!(har.value(), 0.0);
    }

    #[test]
    fn test_har_rv_warmup() {
        let mut har = HarRv::new(5, 20, 60, 252.0);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            har.feed(price);
        }
        assert!(har.is_ready());
    }

    #[test]
    fn test_har_rv_values() {
        let mut har = HarRv::new(5, 20, 60, 252.0);
        for i in 0..70 {
            let price = 100.0 + i as f64;
            let value = har.feed(price);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_har_rv_reset() {
        let mut har = HarRv::new(5, 20, 60, 252.0);
        for i in 0..70 {
            har.feed(100.0 + i as f64);
        }
        har.reset();
        assert!(!har.is_ready());
        assert_eq!(har.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_har() {
        let cfg = <<HarRv as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Har(cfg).build_solo().unwrap();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Har) >= 0.0);
    }
}
