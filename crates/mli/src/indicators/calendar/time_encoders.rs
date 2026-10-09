// Cyclic time encoding — sin/cos pair for hour-of-day (0-23 UTC).
//
// Encodes the hour angle as `(sin, cos)` in [-1, 1], giving a continuous
// cyclic representation that wraps correctly (hour 0 and hour 23 are adjacent
// in the embedding). Price-free: the wall-clock coordinate only, no kline source.

/// Cyclic time encoder: emits `(sin, cos)` of the UTC hour-of-day angle.
///
/// The two outputs are in [-1, 1] and together uniquely identify the hour
/// without discontinuity at the day boundary.
#[derive(Debug, Clone)]
pub struct TimeEncoders {
    sin: f64,
    cos: f64,
}

impl Default for TimeEncoders {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeEncoders {
    pub fn new() -> Self {
        Self { sin: 0.0, cos: 0.0 }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.sin = 0.0;
        self.cos = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }


    /// Named getter for the `sin` brace output (sine of the hour-of-day angle).
    pub fn sin(&self) -> f64 {
        self.sin
    }

    /// Named getter for the `cos` brace output (cosine of the hour-of-day angle).
    pub fn cos(&self) -> f64 {
        self.cos
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    ///
    /// Computes the cyclic hour-of-day angle: `2π * hour / 24`, then stores
    /// `(sin, cos)` of that angle in [-1, 1].
    pub fn feed(&mut self, ts_ms: i64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let hour = (ts_secs.rem_euclid(86_400) / 3600) as f64;
        let angle = std::f64::consts::TAU * hour / 24.0;
        self.sin = angle.sin();
        self.cos = angle.cos();
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, RenderSpec, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`TimeEncoders`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct TimeEncodersConfig;

impl Indicator for TimeEncoders {
    const ID: IndicatorId = IndicatorId::Tenc;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::TencSin),
        Output::centered(IndicatorOutputId::TencCos),
    ];

    type Config = TimeEncodersConfig;
    type Runtime = TimeEncoders;

    fn create(_cfg: TimeEncodersConfig) -> TimeEncoders {
        TimeEncoders::new()
    }
}

impl crate::contract::Config for TimeEncodersConfig {
    fn defaults() -> Self {
        TimeEncodersConfig
    }
    fn machine_defaults() -> Self {
        // Unit struct — no Param fields; machine_defaults_auto() = defaults().
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for TimeEncoders {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::TencSin, "TENC sin", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::TencCos, "TENC cos", Color::hex(0x4CAF50))
            .bounds(-1.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_encoders_creation() {
        let te = TimeEncoders::new();
        assert!(te.is_ready());
        assert_eq!(te.sin(), 0.0);
        assert_eq!(te.cos(), 0.0);
    }

    #[test]
    fn test_time_encoders_midnight() {
        let mut te = TimeEncoders::new();
        // 1970-01-01 00:00:00 UTC — hour 0 (canonical ms ×1000)
        te.feed(0);
        let (s, c) = (te.sin(), te.cos());
        // hour 0 → angle 0 → sin=0, cos=1
        assert!((s - 0.0).abs() < 1e-12, "sin(0) should be 0, got {s}");
        assert!((c - 1.0).abs() < 1e-12, "cos(0) should be 1, got {c}");
    }

    #[test]
    fn test_time_encoders_noon() {
        let mut te = TimeEncoders::new();
        // hour 12 → angle π → sin≈0, cos≈-1 (canonical ms ×1000)
        te.feed(12 * 3_600_000);
        let (s, c) = (te.sin(), te.cos());
        assert!(s.abs() < 1e-10, "sin(π) should be ~0, got {s}");
        assert!((c - (-1.0)).abs() < 1e-10, "cos(π) should be -1, got {c}");
    }

    #[test]
    fn test_time_encoders_outputs_in_range() {
        let mut te = TimeEncoders::new();
        for hour in 0..24_i64 {
            te.feed(hour * 3_600_000);
            let (s, c) = (te.sin(), te.cos());
            assert!(s >= -1.0 && s <= 1.0, "sin out of range: {s}");
            assert!(c >= -1.0 && c <= 1.0, "cos out of range: {c}");
        }
    }

    #[test]
    fn test_time_encoders_reset() {
        let mut te = TimeEncoders::new();
        te.feed(6 * 3_600_000);
        te.reset();
        assert_eq!(te.sin(), 0.0);
        assert_eq!(te.cos(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Tenc(TimeEncodersConfig).build_solo().unwrap();
        // ts=0 → midnight UTC → sin=0, cos=1 (canonical ms)
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        // main() returns first output = sin; at midnight sin(0)=0
        assert!(v.is_finite(), "tenc sin should be finite, got {v}");
    }
}
