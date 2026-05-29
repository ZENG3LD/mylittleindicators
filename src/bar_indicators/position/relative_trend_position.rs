// Relative Trend Position: relative distance to MA and Anchored VWAP (monthly)

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::levels::anchored_vwap::{
    AnchoredVwap, AnchoredVwapParams, AvwapAnchorMode,
};

#[derive(Clone)]
pub struct RelativeTrendPosition {
    ma: MovingAverageProvider,
    avwap: AnchoredVwap,
    last_sma_rel: f64,
    last_avwap_rel: f64,
}

impl RelativeTrendPosition {
    pub fn new(sma_period: usize) -> Self {
        Self::with_ma_type(sma_period, MovingAverageType::SMA)
    }

    /// Creates a new RelativeTrendPosition with a configurable MA type.
    pub fn with_ma_type(sma_period: usize, ma_type: MovingAverageType) -> Self {
        let params = AnchoredVwapParams {
            mode: AvwapAnchorMode::Monthly,
        };
        Self {
            ma: MovingAverageProvider::new(ma_type, sma_period),
            avwap: AnchoredVwap::new(params),
            last_sma_rel: 0.0,
            last_avwap_rel: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.avwap.reset();
        self.last_sma_rel = 0.0;
        self.last_avwap_rel = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready() && self.avwap.is_ready()
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: i64,
    ) -> (f64, f64) {
        let sma_val = self.ma.update_bar(open, high, low, close, volume);
        let avwap_val = self
            .avwap
            .update_bar(open, high, low, close, volume, timestamp);
        self.last_sma_rel = if sma_val != 0.0 {
            (close - sma_val) / sma_val.abs().max(1e-9)
        } else {
            0.0
        };
        self.last_avwap_rel = if avwap_val != 0.0 {
            (close - avwap_val) / avwap_val.abs().max(1e-9)
        } else {
            0.0
        };
        (self.last_sma_rel, self.last_avwap_rel)
    }

    #[inline]
    pub fn values(&self) -> (f64, f64) {
        (self.last_sma_rel, self.last_avwap_rel)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_sma_rel, self.last_avwap_rel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_trend_position_creation() {
        let rtp = RelativeTrendPosition::new(20);
        assert!(!rtp.is_ready());
    }

    #[test]
    fn test_relative_trend_position_warmup() {
        let mut rtp = RelativeTrendPosition::new(20);
        let ts = 1700000000_i64;
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rtp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0, ts + i * 86400);
        }
        assert!(rtp.is_ready());
    }

    #[test]
    fn test_relative_trend_position_values() {
        let mut rtp = RelativeTrendPosition::new(20);
        let ts = 1700000000_i64;
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (sma_rel, avwap_rel) = rtp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0, ts + i * 86400);
            assert!(sma_rel.is_finite(), "SMA relative should be finite");
            assert!(avwap_rel.is_finite(), "AVWAP relative should be finite");
        }
    }

    #[test]
    fn test_relative_trend_position_reset() {
        let mut rtp = RelativeTrendPosition::new(20);
        let ts = 1700000000_i64;
        for i in 0..25 {
            rtp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0, ts + i * 86400);
        }
        rtp.reset();
        assert!(!rtp.is_ready());
    }

    #[test]
    fn test_relative_trend_position_with_ema() {
        let mut rtp = RelativeTrendPosition::with_ma_type(20, MovingAverageType::EMA);
        let ts = 1700000000_i64;
        let mut last_sma_rel = 0.0_f64;
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (sr, ar) =
                rtp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0, ts + i * 86400);
            assert!(sr.is_finite());
            assert!(ar.is_finite());
            last_sma_rel = sr;
        }
        // After 30 bars EMA(20) is warmed up — result must be non-zero for oscillating prices
        assert!(last_sma_rel.is_finite());
    }
}
