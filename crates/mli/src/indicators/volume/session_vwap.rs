//! Session VWAP — cumulative VWAP with explicit session reset.
//!
//! Accumulates `(typical_price × volume)` and `volume` since the last
//! `reset_session()` call. The caller is responsible for resetting at
//! session boundaries (e.g. at market open each day). This is the
//! production pattern because bar streams rarely carry reliable session
//! boundary markers in their timestamps.
//!
//! Formula: `VWAP = Σ(typical_price × volume) / Σ(volume)`
//! where `typical_price = (high + low + close) / 3`.

/// Cumulative VWAP that resets when the caller invokes [`SessionVwap::reset_session`].
#[derive(Debug, Clone)]
pub struct SessionVwap {
    cumulative_pv: f64,
    cumulative_v: f64,
    last_value: f64,
}

impl SessionVwap {
    /// Create a new `SessionVwap`. No parameters are required — this
    /// indicator accumulates from session start to the last bar fed.
    pub fn new() -> Self {
        Self {
            cumulative_pv: 0.0,
            cumulative_v: 0.0,
            last_value: 0.0,
        }
    }

    /// Reset accumulated state for a new session.
    ///
    /// Call this at the start of each trading session before feeding the
    /// first bar of that session.
    pub fn reset_session(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_v = 0.0;
    }

    /// Feed resolved lanes `[high, low, close, volume]` and return the current session VWAP.
    pub fn feed(&mut self, lanes: &[f64]) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        if volume > 0.0 {
            let typical = (high + low + close) / 3.0;
            self.cumulative_pv += typical * volume;
            self.cumulative_v += volume;
        }

        self.last_value = if self.cumulative_v > 1e-9 {
            self.cumulative_pv / self.cumulative_v
        } else {
            close
        };


    }

    /// Returns the last computed VWAP without advancing state.
    pub fn value(&self) -> f64 {
        self.last_value
    }

    /// Returns `true` after the first bar with positive volume has been fed.
    pub fn is_ready(&self) -> bool {
        self.cumulative_v > 1e-9
    }

    /// Full reset (use [`reset_session`] for intraday resets).
    pub fn reset(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_v = 0.0;
        self.last_value = 0.0;
    }
}

impl Default for SessionVwap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_bar_vwap_equals_typical_price() {
        let mut v = SessionVwap::new();
        v.feed(&[102.0, 98.0, 101.0, 1000.0]);
        // typical = (102 + 98 + 101) / 3 = 100.333...
        let expected = (102.0 + 98.0 + 101.0) / 3.0;
        let val = v.value();
        assert!((val - expected).abs() < 1e-9, "expected {expected}, got {val}");
    }

    #[test]
    fn two_equal_bars_vwap_equals_typical() {
        let mut v = SessionVwap::new();
        v.feed(&[102.0, 98.0, 100.0, 500.0]);
        v.feed(&[102.0, 98.0, 100.0, 500.0]);
        let typical = (102.0 + 98.0 + 100.0) / 3.0;
        let val = v.value();
        assert!((val - typical).abs() < 1e-9);
    }

    #[test]
    fn reset_session_clears_accumulation() {
        let mut v = SessionVwap::new();
        v.feed(&[110.0, 90.0, 100.0, 1000.0]);
        v.reset_session();
        // After reset, next bar starts fresh.
        v.feed(&[202.0, 198.0, 200.0, 500.0]);
        let typical = (202.0 + 198.0 + 200.0) / 3.0;
        let val = v.value();
        assert!((val - typical).abs() < 1e-9, "expected {typical}, got {val}");
    }

    #[test]
    fn zero_volume_bar_falls_back_to_close() {
        let mut v = SessionVwap::new();
        v.feed(&[55.0, 45.0, 50.0, 0.0]);
        let val = v.value();
        assert!((val - 50.0).abs() < 1e-9);
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Unit config — SessionVwap has no parameters (cumulative session-scoped VWAP).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SessionVwapConfig;

impl Indicator for SessionVwap {
    const ID: IndicatorId = IndicatorId::SessionVwap;
    /// Standalone session VWAP — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C + volume lanes — typical-price VWAP calculation.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(1) — two running sums, no window buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::SessionVwap)];
    type Config = SessionVwapConfig;
    type Runtime = SessionVwap;

    fn create(_cfg: SessionVwapConfig) -> SessionVwap {
        SessionVwap::new()
    }
}

impl crate::contract::Config for SessionVwapConfig {
    fn defaults() -> Self {
        SessionVwapConfig
    }
    fn machine_defaults() -> Self {
        // No fields — unit config, nothing to sweep
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for SessionVwap {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::SessionVwap, "Session VWAP", Color::hex(0xFF9800))
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
        let mut f = IndicatorOrder::SessionVwap(<<SessionVwap as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Single bar: typical = (102+98+101)/3
        f.feed(0, MarketSample::Bar { open: 9999.0, high: 102.0, low: 98.0, close: 101.0, volume: 1000.0 });
        let expected = (102.0 + 98.0 + 101.0) / 3.0;
        let v = f.read(IndicatorOutputId::SessionVwap);
        assert!((v - expected).abs() < 1e-9, "expected session VWAP = {expected}, got {v}");
    }
}
