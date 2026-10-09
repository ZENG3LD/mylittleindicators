//! Variable Index Dynamic Average (VIDYA) indicator.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::indicators::momentum::cmo::Cmo;
use crate::engine::ohlcv_field::OhlcvField;

/// Variable Index Dynamic Average (VIDYA) - volatility-adaptive moving average.
///
/// VIDYA = α × CMO% × Price + (1 - α × CMO%) × VIDYA_prev
///
/// where α = 2/(period+1) and CMO% is the absolute Chande Momentum Oscillator
/// value as a percentage.
///
/// Created by Tushar Chande. Adapts its smoothing factor based on market
/// volatility measured by CMO. More volatile = faster adaptation.
///
/// # Implementation
///
/// Uses internal CMO for volatility measurement. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Vidya {
    period: usize,
    /// The smoother-family member the inner CMO smooths with (the absorbed
    /// `cmo_ma_type` axis of the old separate VIDYA id — config-chosen, default SMA).
    cmo_ma: SmootherId,
    value: f64,
    count: usize,
    alpha: f64,
    // Boxed: VIDYA embeds a full `Cmo` (which itself holds two `SmootherSlot`
    // smoothers) as its volatility gauge. `Vidya` is a MovingAverage member living
    // inside the whole-universe `ContractFactory` enum — the Box keeps the variant
    // small so the factory is not bloated by an inlined oscillator.
    cmo: Box<Cmo>,
    cmo_pct: f64,
    ready: bool,
}

impl Vidya {
    /// Returns the period of this VIDYA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new VIDYA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for CMO and base alpha calculation
    /// * `ma_type` - Moving average type (ignored, uses SMA internally)
    pub fn new(period: usize) -> Self {
        Self::with_cmo(period, SmootherId::Sma)
    }

    /// Creates a new VIDYA whose inner CMO smooths with the given smoother-family
    /// member (`cmo_ma`) — the absorbed `cmo_ma_type` config axis.
    pub fn with_cmo(period: usize, cmo_ma: SmootherId) -> Self {
        Self {
            period,
            cmo_ma,
            value: 0.0,
            count: 0,
            alpha: 2.0 / (period as f64 + 1.0),
            cmo: Box::new(Cmo::from_smoother(period, cmo_ma)),
            cmo_pct: 0.0,
            ready: false,
        }
    }

    /// Updates the VIDYA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn feed(&mut self, value: f64) -> f64 {
        let value = value;
        self.cmo.feed(value);
        self.cmo_pct = (self.cmo.value() / 100.0).abs();
        if self.ready {
            self.value = (self.alpha * self.cmo_pct) * value + (1.0 - self.alpha * self.cmo_pct) * self.value;
        }
        if !self.ready && self.cmo.is_ready() {
            self.ready = true;
            self.value = value;
        }
        self.count += 1;
        self.value
    }

    /// Returns the current VIDYA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the VIDYA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the VIDYA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
        self.cmo_pct = 0.0;
        self.ready = false;
        self.cmo = Box::new(Cmo::from_smoother(self.period, self.cmo_ma));
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Vidya`]: period, price source, and the CMO smoother family
/// member (`cmo_ma`) — the absorbed `cmo_ma_type` axis of the old separate VIDYA id.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VidyaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    pub cmo_ma: Param<SmootherId>,
}

impl crate::contract::Config for VidyaConfig {
    fn defaults() -> Self {
        VidyaConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            cmo_ma: Param::Solo(SmootherId::Sma),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        // cmo_ma: Class P (SmootherId enum) — left Solo (smoother sweep deferred wave).
        Self::machine_defaults_auto()
    }
}

impl Indicator for Vidya {
    // The canonical VIDYA node. The ugly `AvVidya` alias is annihilated; the nice
    // `Vidya` id survives and now carries the `cmo_ma` config axis the old separate
    // `Vidya` id (a second VariableIndexDynamicAverage impl) had.
    const ID: IndicatorId = IndicatorId::Vidya;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) VIDYA adaptive EMA driven by a CMO (its volatility gauge). The CMO is a
    /// fixed edge — the barometer charges its full cost (own base + its two
    /// MovingAverage slot smoothers) recursively. One source of truth for CMO.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Cmo, &[IndicatorOutputId::Cmo])],
    };
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Vidya)];
    type Config = VidyaConfig;
    type Runtime = Vidya;

    fn create(cfg: VidyaConfig) -> Vidya {
        Vidya::with_cmo(cfg.period.resolved(), cfg.cmo_ma.resolved())
    }

    /// Field-source core: the factory variant holds cfg.source and feeds the
    /// resolved scalar; the core ingests via feed, knowing no OHLCV fields.
    fn source_fields(cfg: &Self::Config) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}


impl Render for Vidya {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Vidya, "VIDYA", Color::hex(0x00BCD4))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vidya_basic_calculation() {
        let mut vidya = Vidya::new(10);

        for i in 1..=20 {
            vidya.feed(i as f64 * 10.0);
        }

        assert!(vidya.is_ready());
        assert!(vidya.value() > 0.0);
    }

    #[test]
    fn test_vidya_reset() {
        let mut vidya = Vidya::new(5);
        for i in 1..=10 {
            vidya.feed(i as f64 * 10.0);
        }
        assert!(vidya.is_ready());

        vidya.reset();
        assert!(!vidya.is_ready());
    }

}
