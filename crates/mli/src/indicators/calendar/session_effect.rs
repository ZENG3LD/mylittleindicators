//! Session effect — rolling mean log-return per 4-bucket trading session.


/// Session buckets (UTC hours):
/// 0 = Asia (06-11), 1 = Europe (12-17), 2 = US (18-23), 3 = Overnight (00-05).
///
/// Accumulates log-returns by session and emits the running mean for the
/// current bar's session. The session bucket is derived from the UTC hour
/// inside `feed`.
#[derive(Debug, Clone)]
pub struct SessionEffect {
    counts: [usize; 4],
    sums: [f64; 4],
    last_close: Option<f64>,
    pub last_bucket: usize,
    pub mean_returns: [f64; 4],
}

impl Default for SessionEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionEffect {
    pub fn new() -> Self {
        Self {
            counts: [0; 4],
            sums: [0.0; 4],
            last_close: None,
            last_bucket: 0,
            mean_returns: [0.0; 4],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.counts = [0; 4];
        self.sums = [0.0; 4];
        self.last_close = None;
        self.last_bucket = 0;
        self.mean_returns = [0.0; 4];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> f64 {
        self.mean_returns[self.last_bucket]
    }

    #[inline]
    pub fn session(&self) -> f64 {
        self.mean_returns[self.last_bucket]
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS) + close — `(ts, value)`, the
    /// orthogonal time coordinate plus the source-resolved close scalar. The session bucket
    /// is the UTC hour: 0=Asia(6-11), 1=Europe(12-17), 2=US(18-23), 3=Overnight(0-5).
    pub fn feed(&mut self, ts_ms: i64, close: f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let hour = (ts_secs.rem_euclid(86_400) / 3600) as u32;
        let b = if hour < 6 {
            3usize // Overnight
        } else if hour < 12 {
            0 // Asia
        } else if hour < 18 {
            1 // Europe
        } else {
            2 // US
        };
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.counts[b] += 1;
            self.sums[b] += r;
            self.mean_returns[b] = self.sums[b] / self.counts[b] as f64;
            self.last_bucket = b;
        }
        self.last_close = Some(close);
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

/// Typed config for [`SessionEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct SessionEffectConfig;

impl Indicator for SessionEffect {
    const ID: IndicatorId = IndicatorId::Session;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field {
        default: crate::engine::ohlcv_field::OhlcvField::Close,
    });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::SessionSession)];

    type Config = SessionEffectConfig;
    type Runtime = SessionEffect;

    fn create(_cfg: SessionEffectConfig) -> SessionEffect {
        SessionEffect::new()
    }
}

impl crate::contract::Config for SessionEffectConfig {
    fn defaults() -> Self {
        SessionEffectConfig
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


impl Render for SessionEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SessionSession, "Session", Color::hex(0x2196F3))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_effect_creation() {
        let se = SessionEffect::new();
        assert!(!se.is_ready());
    }

    #[test]
    fn test_session_effect_update() {
        let mut se = SessionEffect::new();
        // ts=0 → hour 0 → bucket 3 (overnight)
        se.feed(0, 100.0);
        se.feed(3_600_000, 101.0);
        assert!(se.is_ready());
    }

    #[test]
    fn test_session_effect_buckets_finite() {
        let mut se = SessionEffect::new();
        for i in 0..8_i64 {
            let price = 100.0 + i as f64;
            let ts = i * 3 * 3_600_000; // step by 3h to hit all sessions (ms)
            se.feed(ts, price);
        }
        for mean in &se.mean_returns {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_session_effect_reset() {
        let mut se = SessionEffect::new();
        se.feed(0, 100.0);
        se.feed(3_600_000, 101.0);
        se.reset();
        assert!(!se.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Session(SessionEffectConfig).build_solo().unwrap();
        f.feed(1_700_000_000_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 100.0, volume: 0.0,
        });
        f.feed(1_700_003_600_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 101.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v.is_finite(), "session mean return should be finite, got {v}");
    }
}
