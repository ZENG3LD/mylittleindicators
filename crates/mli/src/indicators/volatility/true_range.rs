/// True Range: max(high-low, |high-prev_close|, |low-prev_close|)
#[derive(Debug, Clone)]
pub struct TrueRange {
    prev_close: Option<f64>,
    value: f64,
}

// -- contract -----------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed config for [`TrueRange`]: stateless -- no period, no source knob.
/// Exists only to satisfy the `Indicator` associated type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrConfig;

impl TrConfig {
    /// Config fingerprint: stateless config hashes to a stable constant.
    pub fn config_hash(&self) -> u64 { 0 }

    /// Paramless config — the sole cube point is `self` (mixed-radix dual of `iter().nth`).
    pub fn axes_decode(&self, _idx: u128) -> Self { self.clone() }
}

impl Indicator for TrueRange {
    const ID: IndicatorId = IndicatorId::Tr;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// True Range reads h / l / prev-close.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// Scalar-only state, O(1) per bar -- prev_close is a single field.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Tr)];
    type Config = TrConfig;
    type Runtime = TrueRange;

    fn create(_cfg: TrConfig) -> TrueRange {
        TrueRange::new()
    }
}

impl Default for TrueRange {
    fn default() -> Self {
        Self::new()
    }
}

impl TrueRange {
    pub fn new() -> Self {
        Self {
            prev_close: None,
            value: 0.0,
        }
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). The factory
    /// extracts the fields from the bar; the core knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let range_hl = (h - l).abs();
        let tr = if let Some(pc) = self.prev_close {
            range_hl.max((h - pc).abs()).max((l - pc).abs())
        } else {
            range_hl
        };
        self.value = tr;
        self.prev_close = Some(c);
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.prev_close.is_some()
    }
    pub fn reset(&mut self) {
        self.prev_close = None;
        self.value = 0.0;
    }
}

impl crate::contract::Config for TrConfig {
    fn defaults() -> Self {
        TrConfig
    }
    fn machine_defaults() -> Self {
        // Stateless unit config — no axes, no sweep possible
        TrConfig
    }
    fn cube_size(&self) -> u128 {
        1
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(TrConfig))
    }
}


impl Render for TrueRange {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Tr, "True Range", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_range_creation() {
        let tr = TrueRange::new();
        assert!(!tr.is_ready());
        assert_eq!(tr.value(), 0.0);
    }

    #[test]
    fn test_true_range_first_bar() {
        let mut tr = TrueRange::new();
        let value = tr.feed(&[105.0, 95.0, 102.0]);
        assert_eq!(value, 10.0); // high - low = 105 - 95
        assert!(tr.is_ready());
    }

    #[test]
    fn test_true_range_gap_up() {
        let mut tr = TrueRange::new();
        tr.feed(&[105.0, 95.0, 104.0]);
        // Next bar gaps up: prev_close=104, high=115, low=110
        let value = tr.feed(&[115.0, 110.0, 113.0]);
        // TR = max(115-110, |115-104|, |110-104|) = max(5, 11, 6) = 11
        assert_eq!(value, 11.0);
    }

    #[test]
    fn test_true_range_gap_down() {
        let mut tr = TrueRange::new();
        tr.feed(&[105.0, 95.0, 96.0]);
        // Next bar gaps down: prev_close=96, high=90, low=85
        let value = tr.feed(&[90.0, 85.0, 87.0]);
        // TR = max(90-85, |90-96|, |85-96|) = max(5, 6, 11) = 11
        assert_eq!(value, 11.0);
    }

    #[test]
    fn test_true_range_reset() {
        let mut tr = TrueRange::new();
        tr.feed(&[105.0, 95.0, 102.0]);
        tr.feed(&[107.0, 97.0, 105.0]);
        tr.reset();
        assert!(!tr.is_ready());
        assert_eq!(tr.value(), 0.0);
    }
}
