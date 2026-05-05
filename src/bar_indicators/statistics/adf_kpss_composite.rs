// Composite of ADF proxy (from EngleGrangerAdfProxy style) and KPSS proxies into simple stationarity score

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::statistics::engle_granger_adf_proxy::EngleGrangerAdfProxy;
use crate::bar_indicators::statistics::kpss_proxy::KpssProxy;

#[derive(Clone)]
pub struct AdfKpssComposite {
    adf: EngleGrangerAdfProxy,
    kpss_level: KpssProxy,
    kpss_trend:
        Option<crate::bar_indicators::statistics::kpss_trend_proxy::KpssTrendProxy>,
    pub value: f64,
}

impl AdfKpssComposite {
    pub fn new(window: usize, ma_period: usize, use_trend: bool) -> Self {
        Self {
            adf: EngleGrangerAdfProxy::new(window, ma_period),
            kpss_level: KpssProxy::new(window),
            kpss_trend: if use_trend {
                Some(crate::bar_indicators::statistics::kpss_trend_proxy::KpssTrendProxy::new(window))
            } else {
                None
            },
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.adf.reset();
        self.kpss_level.reset();
        if let Some(t) = &mut self.kpss_trend {
            t.reset();
        }
        self.value = 0.0;
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (_phi, adf_t) = self.adf.update_bar(o, h, l, c, v);
        let kpss_l = self.kpss_level.update_bar(o, h, l, c, v);
        let kpss_t = self
            .kpss_trend
            .as_mut()
            .map(|t| t.update_bar(o, h, l, c, v))
            .unwrap_or(0.0); // map to [0,1]
        let adf_score = 1.0 / (1.0 + (-adf_t).abs().exp()); // larger |t| -> more stationary
        let kpss_score = 1.0 / (1.0 + kpss_l.max(kpss_t)); // larger stat -> less stationary
        self.value = 0.5 * (adf_score + kpss_score);
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.adf.is_ready() && self.kpss_level.is_ready()
    }

    /// Returns composite stationarity score
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adf_kpss_composite_creation() {
        let akc = AdfKpssComposite::new(50, 20, false);
        assert!(!akc.is_ready());
        assert_eq!(akc.value, 0.0);
    }

    #[test]
    fn test_adf_kpss_composite_with_trend() {
        let akc = AdfKpssComposite::new(50, 20, true);
        assert!(!akc.is_ready());
    }

    #[test]
    fn test_adf_kpss_composite_warmup() {
        let mut akc = AdfKpssComposite::new(50, 20, false);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            akc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(akc.is_ready());
    }

    #[test]
    fn test_adf_kpss_composite_range() {
        let mut akc = AdfKpssComposite::new(50, 20, false);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = akc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Composite score should be in [0, 1]");
        }
    }

    #[test]
    fn test_adf_kpss_composite_reset() {
        let mut akc = AdfKpssComposite::new(50, 20, false);
        for i in 0..60 {
            akc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        akc.reset();
        assert!(!akc.is_ready());
        assert_eq!(akc.value, 0.0);
    }
}
