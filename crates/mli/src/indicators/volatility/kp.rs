// High-performance Keltner Position (KP)
// (c) 2024

use super::kc::Kc;

#[derive(Debug, Clone)]
pub struct Kp {
    kc: Kc,
    value: f64,
}

impl Kp {
    pub fn new(period: usize, k_multiplier: f64) -> Self {
        Self {
            kc: Kc::new(period, k_multiplier),
            value: 0.0,
        }
    }

    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let close = lanes[2];
        let (_, middle, _) = self.kc.feed(lanes);
        let k_width = (self.kc.upper - self.kc.lower) / 2.0;
        if k_width > 0.0 {
            self.value = (close - middle) / k_width;
        } else {
            self.value = 0.0;
        }
        self.value
    }


    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.kc.is_ready()
    }
    pub fn reset(&mut self) {
        self.kc.reset();
        self.value = 0.0;
    }
}

impl Default for Kp {
    fn default() -> Self {
        Self::new(20, 2.0)
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

/// Typed dual-mode config for [`Kp`]: period and ATR-band multiplier.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KpConfig {
    pub period: Param<usize>,
    pub k_multiplier: Param<f64>,
}

impl Indicator for Kp {
    const ID: IndicatorId = IndicatorId::Kp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::VoKc, &[
            IndicatorOutputId::VoKcUpper,
            IndicatorOutputId::VoKcMiddle,
            IndicatorOutputId::VoKcLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Kp)];
    type Config = KpConfig;
    type Runtime = Kp;

    fn create(cfg: KpConfig) -> Kp {
        Kp::new(cfg.period.resolved(), cfg.k_multiplier.resolved())
    }
}

impl crate::contract::Config for KpConfig {
    fn defaults() -> Self {
        KpConfig { period: Param::Solo(20), k_multiplier: Param::Solo(2.0) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        let mut s = Self::machine_defaults_auto();
        s.k_multiplier = Param::many(crate::contract::sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Kp {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Kp, "Kase Peak", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_kp_creation() {
        let kp = Kp::new(20, 2.0);
        assert!(!kp.is_ready());
        assert_eq!(kp.value(), 0.0);
    }

    #[test]
    fn test_kp_warmup() {
        let mut kp = Kp::new(20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kp.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(kp.is_ready());
    }

    #[test]
    fn test_kp_values() {
        let mut kp = Kp::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kp.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value.is_finite(), "KP should be finite");
        }
    }

    #[test]
    fn test_kp_reset() {
        let mut kp = Kp::new(20, 2.0);
        for i in 0..25 {
            kp.feed(&[100.0 + i as f64, 99.0, 100.0 + i as f64]);
        }
        kp.reset();
        assert!(!kp.is_ready());
        assert_eq!(kp.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_kp() {
        let cfg = <<Kp as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Kp(cfg).build_solo().unwrap();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Kp).is_finite());
    }
}
