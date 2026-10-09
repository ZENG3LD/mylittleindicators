/// Williams Fractals: detect up/down fractals over a lookback of 2 on each side (5-bar pattern)
#[derive(Debug, Clone)]
pub struct WilliamsFractals {
    buf_h: [f64; 5],
    buf_l: [f64; 5],
    idx: usize,
    filled: bool,
    up: bool,
    down: bool,
}

impl Default for WilliamsFractals {
    fn default() -> Self {
        Self::new()
    }
}

impl WilliamsFractals {
    pub fn new() -> Self {
        Self {
            buf_h: [0.0; 5],
            buf_l: [0.0; 5],
            idx: 0,
            filled: false,
            up: false,
            down: false,
        }
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Returns (is_up_fractal, is_down_fractal). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (bool, bool) {
        let h = lanes[0];
        let l = lanes[1];
        self.buf_h[self.idx % 5] = h;
        self.buf_l[self.idx % 5] = l;
        self.idx += 1;
        if self.idx >= 5 {
            self.filled = true;
        }
        self.up = false;
        self.down = false;
        if self.filled {
            // center at idx-3
            let c = (self.idx + 5 - 3) % 5;
            let h0 = self.buf_h[(c + 5 - 2) % 5];
            let h1 = self.buf_h[(c + 5 - 1) % 5];
            let hc = self.buf_h[c % 5];
            let h3 = self.buf_h[(c + 1) % 5];
            let h4 = self.buf_h[(c + 2) % 5];
            let l0 = self.buf_l[(c + 5 - 2) % 5];
            let l1 = self.buf_l[(c + 5 - 1) % 5];
            let lc = self.buf_l[c % 5];
            let l3 = self.buf_l[(c + 1) % 5];
            let l4 = self.buf_l[(c + 2) % 5];
            self.up = hc > h1 && hc > h0 && hc > h3 && hc > h4;
            self.down = lc < l1 && lc < l0 && lc < l3 && lc < l4;
        }
        (self.up, self.down)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Brace-named getter for the `up` output (1.0 if up fractal, 0.0 otherwise).
    #[inline]
    pub fn up(&self) -> f64 {
        if self.up { 1.0 } else { 0.0 }
    }

    /// Brace-named getter for the `down` output (1.0 if down fractal, 0.0 otherwise).
    #[inline]
    pub fn down(&self) -> f64 {
        if self.down { 1.0 } else { 0.0 }
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, RenderOutput, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`WilliamsFractals`] — the canonical 5-bar Bill Williams
/// fractal has NO tunable parameters (fixed 2-bar half-width), so the config is empty.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct FractalsConfig;

impl Indicator for WilliamsFractals {
    const ID: IndicatorId = IndicatorId::Fractals;
    /// No family — a swing-structure DETECTOR (5-bar fractal pivots), an atomic producer
    /// consumed by name, not a pluggable oscillator member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to high / low — an up fractal is a local high, a down fractal a local low.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// Fixed 5-bar pattern — O(1) per bar over two inline `[f64; 5]` buffers, no period scan.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::fixed(StoreKind::StackArray, 5), Store::fixed(StoreKind::StackArray, 5)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::FractalsUp),
        Output::discrete(IndicatorOutputId::FractalsDown),
    ];
    type Config = FractalsConfig;
    type Runtime = WilliamsFractals;

    fn create(_cfg: FractalsConfig) -> WilliamsFractals {
        WilliamsFractals::new()
    }
}

impl crate::contract::Config for FractalsConfig {
    fn defaults() -> Self {
        FractalsConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // no tunable axes — fixed 5-bar Williams Fractal pattern
    }
}


impl Render for WilliamsFractals {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::dots(IndicatorOutputId::FractalsUp, "Fractal Up", Color::hex(0x4CAF50)))
            .output(RenderOutput::dots(IndicatorOutputId::FractalsDown, "Fractal Down", Color::hex(0xF44336)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_williams_fractals_creation() {
        let ind = WilliamsFractals::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.up(), 0.0);
        assert_eq!(ind.down(), 0.0);
    }

    #[test]
    fn test_williams_fractals_warmup() {
        let mut ind = WilliamsFractals::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_williams_fractals_detects_up_fractal() {
        let mut ind = WilliamsFractals::new();
        // A clean peak in the middle of the 5-bar window -> up fractal once centered.
        let highs = [10.0, 11.0, 15.0, 11.0, 10.0];
        let mut saw_up = false;
        for &h in &highs {
            let (up, _) = ind.feed(&[h, h - 2.0]);
            saw_up |= up;
        }
        assert!(saw_up, "a clean 5-bar peak must register an up fractal");
    }

    #[test]
    fn test_williams_fractals_reset() {
        let mut ind = WilliamsFractals::new();
        for _ in 0..10 {
            ind.feed(&[105.0, 95.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.up(), 0.0);
        assert_eq!(ind.down(), 0.0);
    }

    /// The factory resolves the fixed High/Low lanes from `const SOURCE` (not the wild close)
    /// and feeds the pair; a clean centered peak fires an up fractal end-to-end.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Fractals(<<WilliamsFractals as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let highs = [10.0, 11.0, 15.0, 11.0, 10.0];
        let mut saw_up = false;
        for &h in &highs {
            f.feed(0, MarketSample::Bar {
                open: -1.0, high: h, low: h - 2.0, close: 9999.0, volume: -1.0,
            });
            saw_up |= f.primary() > 0.0;
        }
        assert!(saw_up, "factory-fed clean peak must fire an up fractal");
    }
}
