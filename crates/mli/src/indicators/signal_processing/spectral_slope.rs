// Spectral Slope: linear regression slope of log power vs log frequency

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralSlope {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub slope: f64,
}

impl SpectralSlope {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(32, 512);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            slope: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.slope = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // demean and feed FFT
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            for i in 0..n {
                let x = self.buf[(self.idx + i) % n] - mean;
                self.fft.update(x);
            }
            let fd = self.fft.frequency_domain();
            // linear regression of ln(P) ~ a + b ln(f) over f>0
            let mut sx = 0.0;
            let mut sy = 0.0;
            let mut sxx = 0.0;
            let mut sxy = 0.0;
            let mut cnt = 0.0;
            for i in 0..fd.power_spectrum.len() {
                if i >= fd.frequencies.len() {
                    break;
                }
                let f = fd.frequencies[i];
                if f <= 0.0 {
                    continue;
                }
                let p = fd.power_spectrum[i].max(1e-18);
                let x = f.ln();
                let y = p.ln();
                sx += x;
                sy += y;
                sxx += x * x;
                sxy += x * y;
                cnt += 1.0;
            }
            if cnt >= 2.0 {
                let denom = sxx * cnt - sx * sx;
                self.slope = if denom.abs() > 1e-12 {
                    (sxy * cnt - sx * sy) / denom
                } else {
                    0.0
                };
            }
        }
        self.slope
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.slope
    }

    pub fn window(&self) -> usize {
        self.window
    }

}

impl Default for SpectralSlope {
    /// Factory default: window=256.
    fn default() -> Self {
        Self::new(256)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`SpectralSlope`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SslopeConfig {
    pub period: Param<usize>,
}

impl Indicator for SpectralSlope {
    const ID: IndicatorId = IndicatorId::Sslope;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Sslope)];
    type Config = SslopeConfig;
    type Runtime = SpectralSlope;

    fn create(cfg: SslopeConfig) -> SpectralSlope {
        SpectralSlope::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &SslopeConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SslopeConfig {
    fn defaults() -> Self {
        SslopeConfig { period: Param::Solo(256) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for SpectralSlope {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sslope, "Spectral Slope", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to the
// contracted feed (default source = close). Removed when each caller is Port-wired.
impl SpectralSlope {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_slope_creation() {
        let ss = SpectralSlope::new(64);
        assert!(!ss.is_ready());
        assert_eq!(ss.value(), 0.0);
        assert_eq!(ss.window(), 64);
    }

    #[test]
    fn test_spectral_slope_warmup() {
        let mut ss = SpectralSlope::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ss.feed(price);
        }
        assert!(ss.is_ready());
    }

    #[test]
    fn test_spectral_slope_finite() {
        let mut ss = SpectralSlope::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ss.feed(price);
            assert!(value.is_finite(), "Slope should be finite");
        }
    }

    #[test]
    fn test_spectral_slope_reset() {
        let mut ss = SpectralSlope::new(64);
        for i in 0..70 {
            ss.feed(100.0 + i as f64);
        }
        ss.reset();
        assert!(!ss.is_ready());
        assert_eq!(ss.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_sslope() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralSlope as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sslope(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
