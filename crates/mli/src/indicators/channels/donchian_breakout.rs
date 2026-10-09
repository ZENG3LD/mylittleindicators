// Donchian Breakout: breakout signals relative to Donchian Channel

use crate::indicators::channels::donchian_channel::DonchianChannel;

#[derive(Debug, Clone)]
pub struct DonchianBreakout {
    dc: DonchianChannel,
    breakout: i8,
}

impl Default for DonchianBreakout {
    fn default() -> Self {
        Self::new(20)
    }
}

impl DonchianBreakout {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period.max(2)),
            breakout: 0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.dc.reset();
        self.breakout = 0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        (self.breakout) as f64
    }
    /// Feed resolved [High, Low, Close] lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> i8 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let (upper, lower, _mid) = self.dc.feed(&[high, low]);
        self.breakout = if close > upper {
            1
        } else if close < lower {
            -1
        } else {
            0
        };
        self.breakout
    }
}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`DonchianBreakout`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DonchianBreakoutConfig {
    pub period: Param<usize>,
}

impl Indicator for DonchianBreakout {
    const ID: IndicatorId = IndicatorId::Donbo;
    /// Not a pluggable family — a breakout signal detector, not a channel member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C — DC needs H/L for the channel, C for the breakout comparison.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// The outer update is O(1); the inner DC charges its O(period) window cost via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Dc, &[
            IndicatorOutputId::DcUpper,
            IndicatorOutputId::DcLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Donbo)];
    type Config = DonchianBreakoutConfig;
    type Runtime = DonchianBreakout;

    fn create(cfg: DonchianBreakoutConfig) -> DonchianBreakout {
        DonchianBreakout::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DonchianBreakoutConfig {
    fn defaults() -> Self {
        DonchianBreakoutConfig { period: Param::Solo(20) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1) — only axis
    }
}


impl Render for DonchianBreakout {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::Donbo, "Donchian Breakout", Color::hex(0x2196F3), 2.0))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_donbo() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DonchianBreakout as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Donbo(cfg).build_solo().unwrap();
        // Feed a steady uptrend: close always near high, should eventually break above
        for i in 1..=30usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // wild — not in SOURCE
                high: price + 1.0,
                low: price - 10.0,
                close: price,
                volume: 1000.0,
            });
        }
        let sig = f.primary();
        assert!(sig.is_finite(), "donbo signal should be finite");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_breakout_creation() {
        let ind = DonchianBreakout::new(20);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_donchian_breakout_warmup() {
        let mut ind = DonchianBreakout::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_donchian_breakout_signals() {
        let mut ind = DonchianBreakout::new(5);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let signal = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(signal >= -1 && signal <= 1);
        }
    }

    #[test]
    fn test_donchian_breakout_reset() {
        let mut ind = DonchianBreakout::new(10);
        for i in 0..15 {
            ind.feed(&[105.0, 95.0, 101.0 + i as f64 * 0.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}
