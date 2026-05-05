// Regime Composite: combines VHF, Fractal Dimension, Choppiness, ATR Percentile
// Output: score in [-1..+1] and flags: trend/range

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::chaos::fractal_dimension::FractalDimension;
use crate::bar_indicators::momentum::vhf::Vhf;
use crate::bar_indicators::volatility::atr_percentile::AtrPercentile;
use crate::bar_indicators::volatility::choppiness_index::ChoppinessIndex;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone, Copy, Debug)]
pub struct RegimeCompositeParams {
    pub vhf_period: usize,
    pub fd_period: usize,
    pub chop_period: usize,
    pub atrp_period: usize,
    pub atr_ma_type: MovingAverageType,
    pub atrp_window: usize,
    pub w_vhf: f64,
    pub w_fd: f64,
    pub w_chop: f64,
    pub w_atrp: f64,
}

impl Default for RegimeCompositeParams {
    fn default() -> Self {
        Self {
            vhf_period: 28,
            fd_period: 100,
            chop_period: 14,
            atrp_period: 14,
            atr_ma_type: MovingAverageType::RMA,
            atrp_window: 100,
            w_vhf: 0.35,
            w_fd: 0.25,
            w_chop: 0.20,
            w_atrp: 0.20,
        }
    }
}

#[derive(Clone)]
pub struct RegimeComposite {
    vhf: Vhf,
    fd: FractalDimension,
    chop: ChoppinessIndex,
    atrp: AtrPercentile,
    params: RegimeCompositeParams,
    score: f64,
    is_ready: bool,
}

impl RegimeComposite {
    pub fn new(params: RegimeCompositeParams) -> Self {
        Self {
            vhf: Vhf::new(params.vhf_period),
            fd: FractalDimension::new(params.fd_period, (params.fd_period / 4).max(2)),
            chop: ChoppinessIndex::with_period(params.chop_period),
            atrp: AtrPercentile::new(params.atrp_period, params.atr_ma_type, params.atrp_window),
            params,
            score: 0.0,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.vhf.reset();
        self.fd.reset();
        self.chop.reset();
        self.atrp.reset();
        self.score = 0.0;
        self.is_ready = false;
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let vhf_v = self.vhf.update_bar(open, high, low, close, volume);
        let fd_v = self.fd.update(close);
        let chop_v = self.chop.update_bar(open, high, low, close, volume);
        let atrp_v = self.atrp.update_bar(open, high, low, close, volume);

        self.is_ready = self.vhf.is_ready()
            && self.fd.is_ready()
            && self.chop.is_ready()
            && self.atrp.is_ready();
        if !self.is_ready {
            self.score = 0.0;
            return self.score;
        }

        // Normalize components roughly to [0..1]
        let vhf_n = vhf_v.clamp(0.0, 1.0);
        // Fractal dimension ~ [1..2], map to trendiness 0..1 where lower=trendier
        let fd_n = (2.0 - fd_v).clamp(0.0, 1.0);
        // Choppiness ~ [0..100], lower=trendier
        let chop_n = (1.0 - (chop_v / 100.0)).clamp(0.0, 1.0);
        // ATR percentile already [0..1], higher=high vol
        let atrp_n = atrp_v.clamp(0.0, 1.0);

        // Trend score: weighted sum, centered to [-1..1] by affine transform
        let weighted = self.params.w_vhf * vhf_n
            + self.params.w_fd * fd_n
            + self.params.w_chop * chop_n
            + self.params.w_atrp * atrp_n;
        let sum_w =
            (self.params.w_vhf + self.params.w_fd + self.params.w_chop + self.params.w_atrp)
                .max(1e-9);
        let s = (weighted / sum_w).clamp(0.0, 1.0);
        self.score = 2.0 * s - 1.0; // [-1..+1]
        self.score
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.score)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn is_trend(&self, threshold: f64) -> bool {
        self.score >= threshold
    }

    #[inline]
    pub fn is_range(&self, threshold: f64) -> bool {
        self.score <= -threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regime_composite_creation() {
        let rc = RegimeComposite::new(RegimeCompositeParams::default());
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }

    #[test]
    fn test_regime_composite_warmup() {
        let mut rc = RegimeComposite::new(RegimeCompositeParams::default());
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rc.is_ready());
    }

    #[test]
    fn test_regime_composite_range() {
        let mut rc = RegimeComposite::new(RegimeCompositeParams::default());
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = rc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if rc.is_ready() {
                assert!(value >= -1.0 && value <= 1.0, "Score should be in [-1, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_regime_composite_reset() {
        let mut rc = RegimeComposite::new(RegimeCompositeParams::default());
        for i in 0..150 {
            rc.update_bar(100.0 + i as f64, 101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0);
        }
        rc.reset();
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }
}
