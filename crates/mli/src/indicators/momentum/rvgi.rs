use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Relative Vigor Index (RVGI): numerator = SMA((close-open)), denominator = SMA((high-low))
#[derive(Debug, Clone)]
pub struct Rvgi {
    num_ma: SmootherSlot,
    den_ma: SmootherSlot,
    signal_ma: SmootherSlot,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Rvgi {
    pub fn new(period: usize, signal_period: usize) -> Self {
        Self {
            num_ma: SmootherSlot::new(SmootherId::Sma, period.max(1)),
            den_ma: SmootherSlot::new(SmootherId::Sma, period.max(1)),
            signal_ma: SmootherSlot::new(SmootherId::Sma, signal_period.max(1)),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Feed resolved `[open, high, low, close]` lanes — contract input (SOURCE = KlineSlice[O,H,L,C]).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let open = lanes[0];
        let high = lanes[1];
        let low = lanes[2];
        let close = lanes[3];
        let num = close - open;
        let den = (high - low).max(1e-12);
        self.num_ma.feed(num);
        self.den_ma.feed(den);
        let n = self.num_ma.value();
        let d = self.den_ma.value();
        let ratio = if d.abs() < 1e-12 { 0.0 } else { n / d };
        self.value = ratio;
        if self.num_ma.is_ready() && self.den_ma.is_ready() {
            self.signal_ma.feed(self.value);
            self.signal = self.signal_ma.value();
        }
        self.ready = self.num_ma.is_ready() && self.den_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }


    pub fn value_rvgi(&self) -> f64 {
        self.value
    }

    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Brace-named getter: `rvgi` output (RVGI line).
    #[inline]
    pub fn rvgi(&self) -> f64 {
        self.value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.num_ma.reset();
        self.den_ma.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rvgi_creation() {
        let rvgi = Rvgi::new(10, 4);
        assert!(!rvgi.is_ready());
        assert_eq!(rvgi.value_rvgi(), 0.0);
        assert_eq!(rvgi.value_signal(), 0.0);
    }

    #[test]
    fn test_rvgi_uptrend() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=30 {
            let open = 100.0 + (i - 1) as f64 * 2.0;
            let close = 100.0 + i as f64 * 2.0;
            rvgi.feed(&[open, close + 1.0, open - 1.0, close]);
        }
        assert!(rvgi.is_ready());
        assert!(rvgi.value_rvgi() > 0.0, "RVGI should be > 0 in uptrend, got {}", rvgi.value_rvgi());
    }

    #[test]
    fn test_rvgi_downtrend() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=30 {
            let open = 200.0 - (i - 1) as f64 * 2.0;
            let close = 200.0 - i as f64 * 2.0;
            rvgi.feed(&[open, open + 1.0, close - 1.0, close]);
        }
        assert!(rvgi.is_ready());
        assert!(rvgi.value_rvgi() < 0.0, "RVGI should be < 0 in downtrend, got {}", rvgi.value_rvgi());
    }

    #[test]
    fn test_rvgi_finite_values() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rvgi.feed(&[price - 1.0, price + 2.0, price - 2.0, price]);
            assert!(value.is_finite(), "RVGI should always be finite");
        }
    }

    #[test]
    fn test_rvgi_reset() {
        let mut rvgi = Rvgi::new(10, 4);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            rvgi.feed(&[price - 1.0, price + 1.0, price - 1.0, price]);
        }
        assert!(rvgi.is_ready());
        rvgi.reset();
        assert!(!rvgi.is_ready());
        assert_eq!(rvgi.value_rvgi(), 0.0);
        assert_eq!(rvgi.value_signal(), 0.0);
    }
}

impl Default for Rvgi {
    fn default() -> Self {
        Self::new(14, 9)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for RVGI: SMA period + signal SMA period.
///
/// Dual-mode: every field is a `Param`. No smoother slots (no `#[slot]` fields).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RvgiConfig {
    /// SMA period for numerator (close-open) and denominator (high-low) smoothing.
    pub period: Param<usize>,
    /// SMA period for the signal line.
    pub signal_period: Param<usize>,
}

impl Indicator for Rvgi {
    const ID: IndicatorId = IndicatorId::Rvgi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads Open, High, Low, Close — all four price fields, fixed.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::RvgiRvgi),
        Output::centered(IndicatorOutputId::RvgiSignal),
    ];
    /// O(1): three SMA slots with fixed-period ring buffers.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Vec), // num_ma
            Store::window(StoreKind::Vec), // den_ma
            Store::window(StoreKind::Vec), // signal_ma
        ],
    );

    type Config = RvgiConfig;
    type Runtime = Rvgi;

    fn create(cfg: RvgiConfig) -> Rvgi {
        Rvgi::new(cfg.period.resolved(), cfg.signal_period.resolved())
    }
}

impl crate::contract::Config for RvgiConfig {
    fn defaults() -> Self {
        RvgiConfig {
            period: Param::Solo(14),
            signal_period: Param::Solo(9),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period/signal_period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for Rvgi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::RvgiRvgi,
                "RVGI",
                Color::hex(0x4CAF50),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::RvgiSignal,
                "Signal",
                Color::hex(0xF44336),
                1.0,
            ))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Rvgi(<<Rvgi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let open = 100.0 + (i - 1) as f64 * 2.0;
            let close = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open,
                high: close + 1.0,
                low: open - 1.0,
                close,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
