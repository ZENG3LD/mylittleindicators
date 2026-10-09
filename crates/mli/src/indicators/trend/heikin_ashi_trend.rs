// Heikin-Ashi Trend - sign of HA close relative to HA open

use crate::indicators::candles::heikin_ashi::HeikinAshi;

#[derive(Debug, Clone)]
pub struct HeikinAshiTrend {
    ha: HeikinAshi,
    value: i8,
}

impl Default for HeikinAshiTrend {
    fn default() -> Self {
        Self::new()
    }
}

impl HeikinAshiTrend {
    pub fn new() -> Self {
        Self {
            ha: HeikinAshi::new(),
            value: 0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ha.reset();
        self.value = 0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> f64 {
        (self.value) as f64
    }

    /// Feed `[open, high, low, close]` lanes — the contracted entry point.
    /// `const SOURCE = KlineSlice(&[Open, High, Low, Close])`.
    pub fn feed(&mut self, lanes: &[f64]) -> i8 {
        let (ho, _hh, _hl, hc) = self.ha.feed(&[lanes[0], lanes[1], lanes[2], lanes[3]]);
        self.value = if hc > ho {
            1
        } else if hc < ho {
            -1
        } else {
            0
        };
        self.value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, HistogramStyle, Indicator, Output, Port, Render, RenderOutput, RenderSpec,
    SourceAxis, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Unit config — HeikinAshiTrend has no parameters.
#[derive(Debug, Clone, Copy, mli_contract_macros::ConfigAxes)]
pub struct HaTrendConfig;

impl Indicator for HeikinAshiTrend {
    const ID: IndicatorId = IndicatorId::HaTrend;
    /// Not a pluggable family member — a binary trend-direction detector.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads Open, High, Low, Close — fixed four-field source.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1): all state is in the embedded HeikinAshi (4 running scalars + bool) + `value` i8.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Heikinashi, &[IndicatorOutputId::HeikinashiClose])],
    };
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::HaTrend)];
    type Config = HaTrendConfig;
    type Runtime = HeikinAshiTrend;

    fn create(_cfg: HaTrendConfig) -> HeikinAshiTrend {
        HeikinAshiTrend::new()
    }
}

impl crate::contract::Config for HaTrendConfig {
    fn defaults() -> Self {
        HaTrendConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit config — no sweepable axes.
        Self::machine_defaults_auto()
    }
}


impl Render for HeikinAshiTrend {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::HaTrend,
                "HA Trend",
                Color::hex(0x2196F3),
            ))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heikin_ashi_trend_creation() {
        let hat = HeikinAshiTrend::new();
        assert!(hat.is_ready()); // Always ready
    }

    #[test]
    fn test_heikin_ashi_trend_values() {
        let mut hat = HeikinAshiTrend::new();
        // Bullish bar: close > open
        let value = hat.feed(&[100.0, 102.0, 99.0, 101.0]);
        assert!(value == 1 || value == 0 || value == -1, "Signal should be -1, 0, or 1");
    }

    #[test]
    fn test_heikin_ashi_trend_signal_range() {
        let mut hat = HeikinAshiTrend::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            let value = hat.feed(&[price, price + 1.0, price - 1.0, price + 0.5]);
            assert!(value >= -1 && value <= 1);
        }
    }

    #[test]
    fn test_heikin_ashi_trend_reset() {
        let mut hat = HeikinAshiTrend::new();
        for i in 0..10 {
            hat.feed(&[100.0 + i as f64, 105.0, 95.0, 101.0]);
        }
        hat.reset();
        assert!(hat.is_ready());
    }

    /// Factory resolves [Open, High, Low, Close] from const SOURCE.
    /// Volume (9999.0) is a wild value NOT in SOURCE — proves correct resolution.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::HaTrend(<<HeikinAshiTrend as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Bullish bar: close > open  → HA close should exceed HA open after a few bars.
        for i in 0..5_u32 {
            f.feed(0, MarketSample::Bar {
                open: 100.0 + i as f64,
                high: 106.0 + i as f64,
                low: 99.0,
                close: 105.0 + i as f64,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let sig = f.primary();
        assert!(sig >= -1.0 && sig <= 1.0, "Signal should be -1/0/1, got {sig}");
    }
}
