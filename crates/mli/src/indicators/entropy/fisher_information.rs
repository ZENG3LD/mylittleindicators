// Rolling Fisher Information proxy for Gaussian mean: I = n / var(returns)

#[derive(Debug, Clone)]
pub struct RollingFisherInformation {
    window: usize,
    rets: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl RollingFisherInformation {
    pub fn new(window: usize) -> Self {
        let w = window.max(10);
        Self {
            window: w,
            rets: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.rets.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the close scalar (resolved by the factory; `const SOURCE = Field{Close}`).
    pub fn feed(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.rets[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                let n = self.window as f64;
                let mut mean = 0.0;
                for i in 0..self.window {
                    mean += self.rets[i];
                }
                mean /= n;
                let mut var = 0.0;
                for i in 0..self.window {
                    let d = self.rets[i] - mean;
                    var += d * d;
                }
                var /= n;
                self.value = if var <= 1e-12 { 0.0 } else { n / var };
            }
        }
        self.last_close = Some(close);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for RollingFisherInformation {
    /// Factory default (Fisher arm): window=200.
    fn default() -> Self {
        Self::new(200)
    }
}

// ── Contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, StoreKind, Store, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

use crate::contract::Param;

/// Typed config for [`RollingFisherInformation`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FisherConfig {
    pub window: Param<usize>,
}

impl Indicator for RollingFisherInformation {
    const ID: IndicatorId = IndicatorId::Fisher;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close — computes log-returns internally.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(n) per bar — variance scan over the ring.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Fisher)];
    type Config = FisherConfig;
    type Runtime = RollingFisherInformation;

    fn create(cfg: FisherConfig) -> RollingFisherInformation {
        RollingFisherInformation::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for FisherConfig {
    fn defaults() -> Self {
        FisherConfig { window: Param::Solo(200) }
    }
    fn machine_defaults() -> Self {
        // window: Class A period — auto handles range(2,4048,1); no other axes
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RollingFisherInformation {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Fisher, "Fisher Information", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn test_rolling_fisher_information_creation() {
        let fi = RollingFisherInformation::new(20);
        assert!(!fi.is_ready());
        assert_eq!(fi.value(), 0.0);
    }

    #[test]
    fn test_rolling_fisher_information_warmup() {
        let mut fi = RollingFisherInformation::new(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            fi.feed(price);
        }
        assert!(fi.is_ready());
    }

    #[test]
    fn test_rolling_fisher_information_values_finite() {
        let mut fi = RollingFisherInformation::new(15);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = fi.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_rolling_fisher_information_reset() {
        let mut fi = RollingFisherInformation::new(15);
        for i in 0..25 {
            fi.feed(100.0 + i as f64);
        }
        fi.reset();
        assert!(!fi.is_ready());
        assert_eq!(fi.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_fisher() {
        
        let mut f = IndicatorOrder::Fisher(<<RollingFisherInformation as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..220 {
            let price = 100.0 + (i as f64 * 0.12).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Fisher);
        assert!(v.is_finite() && v >= 0.0, "fisher must be finite >= 0: {v}");
    }
}
