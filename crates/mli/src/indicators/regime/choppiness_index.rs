// Choppiness Index
// CHOP = 100 * log10(Σ ATR(1) / (Highest High − Lowest Low)) / log10(n)
// Values near 100 = choppy/sideways; near 0 = trending.

use crate::indicators::volatility::atr::Atr;

#[derive(Debug, Clone)]
pub struct ChoppinessIndex {
    period: usize,

    high_prices: Vec<f64>,
    low_prices: Vec<f64>,
    atr_values: Vec<f64>,

    atr: Atr,

    choppiness_value: f64,

    bars_count: usize,
    is_ready: bool,
}

impl ChoppinessIndex {
    pub fn new() -> Self {
        Self::with_period(14)
    }

    pub fn with_period(period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");

        Self {
            period,
            high_prices: Vec::with_capacity(period),
            low_prices: Vec::with_capacity(period),
            atr_values: Vec::with_capacity(period),
            atr: Atr::new_wilder(1),
            choppiness_value: 50.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed the resolved input lanes — `[high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        if self.high_prices.len() >= 512 {
            self.high_prices.remove(0);
        }
        if self.low_prices.len() >= 512 {
            self.low_prices.remove(0);
        }

        self.high_prices.push(high);
        self.low_prices.push(low);

        let atr_value = self.atr.feed(&[high, low, close]);

        if self.atr_values.len() >= 512 {
            self.atr_values.remove(0);
        }
        self.atr_values.push(atr_value);

        if self.high_prices.len() >= self.period
            && self.low_prices.len() >= self.period
            && self.atr_values.len() >= self.period
        {
            let start_idx = self.high_prices.len() - self.period;
            let highest_high = self.high_prices[start_idx..]
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let lowest_low = self.low_prices[start_idx..]
                .iter()
                .fold(f64::INFINITY, |a, &b| a.min(b));

            let atr_start_idx = self.atr_values.len() - self.period;
            let atr_sum: f64 = self.atr_values[atr_start_idx..].iter().sum();

            let range = highest_high - lowest_low;
            if range > 1e-12 && atr_sum > 1e-12 {
                let ratio = atr_sum / range;
                let log_ratio = ratio.log10();
                let log_period = (self.period as f64).log10();

                if log_period.abs() > 1e-12 {
                    self.choppiness_value = (100.0 * log_ratio / log_period).clamp(0.0, 100.0);
                }
            }
        }

        if self.bars_count >= self.period + 5 {
            self.is_ready = true;
        }

        self.choppiness_value
    }

    pub fn value(&self) -> f64 {
        self.choppiness_value
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn reset(&mut self) {
        self.high_prices.clear();
        self.low_prices.clear();
        self.atr_values.clear();
        self.atr.reset();
        self.choppiness_value = 50.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
}

impl Default for ChoppinessIndex {
    fn default() -> Self {
        Self::with_period(14)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`ChoppinessIndex`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ChopConfig {
    pub period: Param<usize>,
}

impl Indicator for ChoppinessIndex {
    const ID: IndicatorId = IndicatorId::Chop;
    /// Volatility / regime classifier — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C triple for highest-high, lowest-low, and driving inner ATR.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// Outer is O(period) per bar (scans H/L/ATR window arrays). Three Vec buffers.
    /// Inner ATR cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Chop)];

    type Config = ChopConfig;
    type Runtime = ChoppinessIndex;

    fn create(cfg: ChopConfig) -> ChoppinessIndex {
        ChoppinessIndex::with_period(cfg.period.resolved())
    }
}

impl crate::contract::Config for ChopConfig {
    fn defaults() -> Self {
        ChopConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1) covers it.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ChoppinessIndex {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Chop, "Choppiness", Color::hex(0x009688))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(61.8, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(38.2, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choppiness_index_creation() {
        let ci = ChoppinessIndex::new();
        assert!(!ci.is_ready());
        assert_eq!(ci.value(), 50.0);
    }

    #[test]
    fn test_choppiness_index_warmup() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ci.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ci.is_ready());
    }

    #[test]
    fn test_choppiness_index_range() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = ci.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value >= 0.0 && value <= 100.0, "Choppiness should be in [0, 100]");
        }
    }

    #[test]
    fn test_choppiness_index_reset() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..25 {
            ci.feed(&[101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64]);
        }
        ci.reset();
        assert!(!ci.is_ready());
        assert_eq!(ci.value(), 50.0);
    }

    #[test]
    fn factory_feeds_resolved_chop() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Chop(ChopConfig { period: Param::Solo(14) }).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            // SOURCE = KlineSlice([High, Low, Close]); open/volume not used
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 100.0);
    }
}
