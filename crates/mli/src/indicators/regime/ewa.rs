// EWA (Elliott Wave Analysis) contracted indicator.
// Wraps `crate::ewa::EwaRuntime` as a regime/structure-detection node.
// Returns the highest-confidence hypothesis's confidence score (lane 0)
// and a stable pattern index (lane 1) from the first world's top hypothesis.

use crate::core::types::Bar;
use crate::ewa::{EwaConfig as EwaInnerConfig, EwaRuntime};


// ---- EwaProfileSel ----

/// Which EWA execution profile to use.
/// `Runtime` = light (1 percent-swing world, minimal overhead).
/// `DevExplore` = heavy (12+ worlds, full diagonal/harmonic scan).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum EwaProfileSel {
    Runtime,
    DevExplore,
}

// ---- EwaIndicatorConfig ----

/// Typed config for the EWA contracted indicator.
///
/// The profile axis is a real typed enum (`EwaProfileSel`) — no `u8` newtype, no stringly name.
#[derive(Debug, Clone, Copy)]
pub struct EwaIndicatorConfig {
    /// Execution profile: light `Runtime` vs heavy `DevExplore`.
    pub profile: EwaProfileSel,
}

impl EwaIndicatorConfig {
    pub const RUNTIME: Self = Self { profile: EwaProfileSel::Runtime };
    pub const DEV_EXPLORE: Self = Self { profile: EwaProfileSel::DevExplore };
}

// ---- Ewa core struct ----

/// EWA runtime wrapped as a contracted indicator.
///
/// On each bar `feed` delegates to `EwaRuntime::push_bar`. The contract
/// projects the first world's top hypothesis into two scalars:
/// - lane 0 (`confidence`): hypothesis confidence in [0, 1].
/// - lane 1 (`pattern`):    stable numeric index of `EwaPatternKind`
///   (implicit discriminant — Impulse=0, Zigzag=10, …; same order as
///   the enum definition in `crate::ewa::types`).
///
/// EWA is the heaviest per-bar node in the factory: it maintains a full
/// pivot/segment/candidate data structure across N swing worlds.
#[derive(Debug, Clone)]
pub struct Ewa {
    runtime: EwaRuntime,
    /// Kept for `reset()` to reconstruct `EwaRuntime` without re-building the config.
    ewa_config: EwaInnerConfig,
    confidence: f64,
    pattern_disc: f64,
    is_ready: bool,
}

impl Ewa {
    pub fn new(cfg: EwaIndicatorConfig) -> Self {
        let ewa_config = Self::build_ewa_config(cfg);
        let runtime = EwaRuntime::new(ewa_config.clone());
        Self {
            runtime,
            ewa_config,
            confidence: 0.0,
            pattern_disc: 0.0,
            is_ready: false,
        }
    }

    fn build_ewa_config(cfg: EwaIndicatorConfig) -> EwaInnerConfig {
        match cfg.profile {
            EwaProfileSel::Runtime => EwaInnerConfig::default(),
            EwaProfileSel::DevExplore => EwaInnerConfig::dev_explore(),
        }
    }

    pub fn reset(&mut self) {
        self.runtime = EwaRuntime::new(self.ewa_config.clone());
        self.confidence = 0.0;
        self.pattern_disc = 0.0;
        self.is_ready = false;
    }

    /// Feed a bar via the `Fields +time` contract arm.
    /// `lanes` order matches `SOURCE`: `[open, high, low, close, volume]`.
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) {
        let bar = Bar::new(
            ts_ms,
            lanes[0], // open
            lanes[1], // high
            lanes[2], // low
            lanes[3], // close
            lanes[4], // volume
        );
        let update = self.runtime.push_bar(bar);
        if let Some(world) = update.worlds.first() {
            if world.strict_candidates > 0 || !world.hypotheses.is_empty() {
                self.is_ready = true;
            }
            if let Some(hyp) = world.hypotheses.first() {
                self.confidence = hyp.confidence;
                self.pattern_disc = hyp.pattern as usize as f64;
            }
        }
    }


    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Confidence score of the top hypothesis in [0, 1].
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Numeric discriminant of the top hypothesis pattern kind.
    /// Field `pattern_disc` stores the implicit enum index.
    pub fn pattern(&self) -> f64 {
        self.pattern_disc
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`Ewa`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EwaConfig {
    pub profile: Param<EwaProfileSel>,
}

impl Indicator for Ewa {
    const ID: IndicatorId = IndicatorId::Ewa;
    /// No pluggable family — EWA is a standalone regime/structure classifier.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full OHLCV slice — EWA needs all five fields to construct a complete `Bar`.
    /// Lane order: Open / High / Low / Close / Volume (matches `feed` destructuring).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::EwaConfidence),
        Output::discrete(IndicatorOutputId::EwaPattern),
    ];
    /// EWA is the heaviest per-bar node in the factory.
    /// Each bar may rebuild pivots, segments, fib-relations, and candidate sets across
    /// N swing worlds; the inner data structures grow with bar count (Vec stores).
    /// `Quadratic` update complexity because the candidate/segment rescan is O(pivots²)
    /// in the worst case; the store declares three heap Vecs (bars, worlds, candidates).
    const COST: Cost = Cost {
        update: UpdateComplexity::Quadratic,
        stores: &[
            Store::window(StoreKind::Vec),  // bars buffer
            Store::window(StoreKind::Vec),  // world pivot/segment/candidate state
            Store::window(StoreKind::Vec),  // pending_affected ring
        ],
        inner: &[],
    };
    type Config = EwaConfig;
    type Runtime = Ewa;

    fn create(cfg: EwaConfig) -> Ewa {
        Ewa::new(EwaIndicatorConfig { profile: cfg.profile.resolved() })
    }

}

impl crate::contract::Config for EwaConfig {
    fn defaults() -> Self {
        EwaConfig { profile: Param::Solo(EwaProfileSel::Runtime) }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // profile: Class Q plain enum — all EwaProfileSel variants.
        s.profile = Param::many(vec![EwaProfileSel::Runtime, EwaProfileSel::DevExplore]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Ewa {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EwaConfidence, "EWA conf", Color::hex(0x4CAF50))
            .line_output(IndicatorOutputId::EwaPattern, "EWA pattern", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::{MarketSample, Render};

    /// Build 300 zigzag synthetic bars: price alternates +3% / -2% to create
    /// swing pivots and give EWA a realistic structure to analyze.
    fn zigzag_bars() -> Vec<(i64, f64, f64, f64, f64, f64)> {
        let mut bars = Vec::with_capacity(300);
        let mut price = 100.0_f64;
        let mut ts_ms = 1_700_000_000_000_i64;
        for i in 0..300 {
            let next = if i % 2 == 0 { price * 1.03 } else { price * 0.98 };
            let (open, close) = (price, next);
            let high = open.max(close) * 1.002;
            let low = open.min(close) * 0.998;
            bars.push((ts_ms, open, high, low, close, 1000.0));
            price = next;
            ts_ms += 60_000;
        }
        bars
    }

    #[test]
    fn create_and_not_ready() {
        let ewa = Ewa::new(EwaIndicatorConfig::RUNTIME);
        assert!(!ewa.is_ready());
        assert_eq!((ewa.confidence(), ewa.pattern()), (0.0, 0.0));
    }

    #[test]
    fn feed_zigzag_becomes_ready_and_finite() {
        let mut ewa = Ewa::new(EwaIndicatorConfig::RUNTIME);
        for (ts, o, h, l, c, v) in zigzag_bars() {
            ewa.feed(ts, &[o, h, l, c, v]);
        }
        assert!(ewa.is_ready(), "EWA should be ready after 300 zigzag bars");
        let conf = ewa.confidence();
        let pat = ewa.pattern();
        assert!(conf.is_finite(), "confidence must be finite, got {conf}");
        assert!(pat.is_finite(), "pattern index must be finite, got {pat}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ewa = Ewa::new(EwaIndicatorConfig::RUNTIME);
        for (ts, o, h, l, c, v) in zigzag_bars() {
            ewa.feed(ts, &[o, h, l, c, v]);
        }
        assert!(ewa.is_ready());
        ewa.reset();
        assert!(!ewa.is_ready());
        assert_eq!((ewa.confidence(), ewa.pattern()), (0.0, 0.0));
    }

    /// `Fields +time` arm: the factory feeds `(ts, &[O,H,L,C,V])` to `feed`.
    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Ewa(<<Ewa as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let bars = zigzag_bars();
        for (ts, o, h, l, c, v) in &bars {
            f.feed(*ts, MarketSample::Bar {
                open: *o,
                high: *h,
                low: *l,
                close: *c,
                volume: *v,
            });
        }
        assert!(f.is_ready(), "factory EWA should be ready after 300 bars");
        let conf = f.primary();
        assert!(conf.is_finite(), "confidence must be finite, got {conf}");
        // rendering() must not panic
        let _spec = Ewa::rendering();
    }
}
