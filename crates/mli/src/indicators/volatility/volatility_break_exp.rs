// Volatility Break detector with exponential sensitivity

/// Detects a volatility expansion event: fires 1.0 when the current volatility
/// reading deviates from its EMA by more than `threshold_sigma` standard-deviation
/// units (measured as `|vol - prev_ema|`).
///
/// Input: a single scalar volatility measure (e.g. realised vol, ATR, or any
/// configurable OHLCV-derived value). Use `Source` flavor — the factory resolves
/// the field and feeds `feed(vol)`.
#[derive(Debug, Clone)]
pub struct VolatilityBreakExp {
    alpha: f64,
    ema: f64,
    init: bool,
    threshold: f64,
    last_value: f64,
}

impl VolatilityBreakExp {
    pub fn new(alpha: f64, threshold_sigma: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.01, 1.0),
            ema: 0.0,
            init: false,
            threshold: threshold_sigma.max(0.5),
            last_value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema = 0.0;
        self.init = false;
        self.last_value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.init
    }

    /// Feed one volatility scalar. Returns 1.0 on expansion, 0.0 otherwise.
    pub fn feed(&mut self, vol: f64) -> f64 {
        if !self.init {
            self.ema = vol;
            self.init = true;
            self.last_value = 0.0;
            return self.last_value;
        }
        let prev = self.ema;
        self.ema = self.alpha * vol + (1.0 - self.alpha) * self.ema;
        let sigma = (vol - prev).abs().max(1e-9);
        self.last_value = if (vol - self.ema).abs() > self.threshold * sigma {
            1.0
        } else {
            0.0
        };
        self.last_value
    }

    pub fn value(&self) -> f64 {
        self.last_value
    }
}

impl Default for VolatilityBreakExp {
    fn default() -> Self {
        Self::new(0.1, 2.0)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`VolatilityBreakExp`] — alpha, threshold, source.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VbexpConfig {
    /// EMA smoothing factor for the baseline.
    pub alpha: Param<f64>,
    /// Deviation threshold in sigma units.
    pub threshold_sigma: Param<f64>,
    /// Source price field fed as the volatility proxy.
    pub source: Param<OhlcvField>,
}

impl Indicator for VolatilityBreakExp {
    const ID: IndicatorId = IndicatorId::Vbexp;
    /// Not a pluggable family — a binary expansion detector, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable source field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) per bar: one EMA update + threshold check; all state is inline scalars.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Vbexp)];
    type Config = VbexpConfig;
    type Runtime = VolatilityBreakExp;

    fn create(cfg: VbexpConfig) -> VolatilityBreakExp {
        VolatilityBreakExp::new(cfg.alpha.resolved(), cfg.threshold_sigma.resolved())
    }

    fn source_fields(cfg: &VbexpConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for VbexpConfig {
    fn defaults() -> Self {
        VbexpConfig {
            alpha: Param::Solo(0.1),
            threshold_sigma: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // source: Class O OhlcvField — auto all 8 variants
        let mut s = Self::machine_defaults_auto();
        s.alpha            = Param::many(crate::contract::sweep_f64(0.01, 0.99, 0.01)); // Class E alpha/decay
        s.threshold_sigma  = Param::many(crate::contract::sweep_f64(0.1, 5.0, 0.1));    // Class F threshold/sigma
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolatilityBreakExp {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vbexp, "Vol Expansion", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_break_exp_creation() {
        let vbe = VolatilityBreakExp::new(0.1, 2.0);
        assert!(!vbe.is_ready());
        assert_eq!(vbe.value(), 0.0);
    }

    #[test]
    fn test_volatility_break_exp_warmup() {
        let mut vbe = VolatilityBreakExp::new(0.1, 2.0);
        vbe.feed(0.02);
        assert!(vbe.is_ready());
    }

    #[test]
    fn test_volatility_break_exp_values() {
        let mut vbe = VolatilityBreakExp::new(0.1, 2.0);
        for i in 0..20 {
            let vol = 0.02 + (i as f64 * 0.1).sin() * 0.01;
            let value = vbe.feed(vol);
            assert!(value == 0.0 || value == 1.0, "Break signal should be 0 or 1");
        }
    }

    #[test]
    fn test_volatility_break_exp_reset() {
        let mut vbe = VolatilityBreakExp::new(0.1, 2.0);
        vbe.feed(0.02);
        vbe.feed(0.03);
        vbe.reset();
        assert!(!vbe.is_ready());
        assert_eq!(vbe.value(), 0.0);
    }

    /// The factory resolves the configured source field and feeds the scalar.
    /// With `source: OhlcvField::Close`, open/high/low/volume are ignored.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Vbexp(<<VolatilityBreakExp as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..20 {
            let close = 0.02 + (i as f64 * 0.1).sin() * 0.01;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.read(IndicatorOutputId::Vbexp);
        assert!(v == 0.0 || v == 1.0, "output should be 0 or 1, got {v}");
    }
}
