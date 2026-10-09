// Price-Volume Coherence proxy via rolling absolute correlation of returns and volume changes

#[derive(Debug, Clone)]
pub struct PriceVolumeCoherenceProxy {
    window: usize,
    buf_price: Vec<f64>,
    buf_volume: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_price: Option<f64>,
    prev_volume: Option<f64>,
    coherence: f64,
}

impl PriceVolumeCoherenceProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(16);
        Self {
            window: w,
            buf_price: vec![0.0; w],
            buf_volume: vec![0.0; w],
            idx: 0,
            filled: false,
            prev_price: None,
            prev_volume: None,
            coherence: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf_price.fill(0.0);
        self.buf_volume.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_price = None;
        self.prev_volume = None;
        self.coherence = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the resolved input lanes `[close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let c = lanes[0];
        let v = lanes[1];
        let pr = if let Some(p) = self.prev_price {
            (c / p).ln()
        } else {
            0.0
        };
        let vr = if let Some(u) = self.prev_volume {
            if u > 0.0 { (v / u).ln() } else { 0.0 }
        } else {
            0.0
        };
        self.prev_price = Some(c.max(1e-12));
        self.prev_volume = Some(v.max(1e-9));
        self.buf_price[self.idx] = pr;
        self.buf_volume[self.idx] = vr;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let n = if self.filled { self.window } else { self.idx };
        if n >= 2 {
            let mut sx = 0.0;
            let mut sy = 0.0;
            let mut sxx = 0.0;
            let mut syy = 0.0;
            let mut sxy = 0.0;
            for i in 0..n {
                let x = self.buf_price[i];
                let y = self.buf_volume[i];
                sx += x;
                sy += y;
                sxx += x * x;
                syy += y * y;
                sxy += x * y;
            }
            let nn = n as f64;
            let num = nn * sxy - sx * sy;
            let den = ((nn * sxx - sx * sx) * (nn * syy - sy * sy))
                .max(1e-24)
                .sqrt();
            let r = if den > 0.0 { (num / den).abs().min(1.0) } else { 0.0 };
            self.coherence = r;
        }
        self.coherence
    }

    pub fn value(&self) -> f64 {
        self.coherence
    }
}

impl Default for PriceVolumeCoherenceProxy {
    /// Factory defaults: window=50 (clamped to max(50,16)=50).
    fn default() -> Self {
        Self::new(50)
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

/// Own config for [`PriceVolumeCoherenceProxy`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PvCoherenceConfig {
    pub period: Param<usize>,
}

impl Indicator for PriceVolumeCoherenceProxy {
    const ID: IndicatorId = IndicatorId::PvCoherence;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: close (price return) + volume (volume return) — both required by the formula.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::PvCoherence)];
    /// O(n): single-pass correlation accumulation over window.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = PvCoherenceConfig;
    type Runtime = PriceVolumeCoherenceProxy;

    fn create(cfg: PvCoherenceConfig) -> PriceVolumeCoherenceProxy {
        PriceVolumeCoherenceProxy::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for PvCoherenceConfig {
    fn defaults() -> Self {
        PvCoherenceConfig { period: Param::Solo(50) }
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


impl Render for PriceVolumeCoherenceProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::PvCoherence, "PV Coherence", Color::hex(0x009688))
            .bounds(0.0, 1.0)
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
    fn test_price_volume_coherence_proxy_creation() {
        let pvc = PriceVolumeCoherenceProxy::new(50);
        assert!(!pvc.is_ready());
        assert_eq!(pvc.coherence, 0.0);
    }

    #[test]
    fn test_price_volume_coherence_proxy_warmup() {
        let mut pvc = PriceVolumeCoherenceProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let vol = 1000.0 + (i as f64 * 0.2).sin() * 200.0;
            pvc.feed(&[price, vol]);
        }
        assert!(pvc.is_ready());
    }

    #[test]
    fn test_price_volume_coherence_proxy_range() {
        let mut pvc = PriceVolumeCoherenceProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let vol = 1000.0 + (i as f64 * 0.3).sin() * 300.0;
            let value = pvc.feed(&[price, vol]);
            assert!(value >= 0.0 && value <= 1.0, "Coherence should be in [0, 1]");
        }
    }

    #[test]
    fn test_price_volume_coherence_proxy_reset() {
        let mut pvc = PriceVolumeCoherenceProxy::new(50);
        for i in 0..60 {
            pvc.feed(&[100.0 + i as f64, 1000.0]);
        }
        pvc.reset();
        assert!(!pvc.is_ready());
        assert_eq!(pvc.coherence, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_pv_coherence() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::PvCoherence(<<PriceVolumeCoherenceProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..80 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            // volume != 9999 so coherence computes real data; open/high/low = 9999
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1000.0 + i as f64 });
        }
        let v = f.read(IndicatorOutputId::PvCoherence);
        assert!(v >= 0.0 && v <= 1.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<PriceVolumeCoherenceProxy as Indicator>::ID, IndicatorId::PvCoherence);
    }
}
