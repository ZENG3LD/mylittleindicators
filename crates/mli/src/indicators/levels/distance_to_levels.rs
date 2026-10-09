// Distance-to-Levels: distances to rolling_midline and percentile_channels mid

use crate::indicators::channels::percentile_channels::{
    PercentileBasis, PercentileChannels,
};
use crate::indicators::levels::rolling_midline::RollingMidline;

#[derive(Debug, Clone)]
pub struct DistanceToLevels {
    mid: RollingMidline,
    pct: PercentileChannels,
    last_dist_mid: f64,
    last_dist_mid_pct: f64,
}

impl DistanceToLevels {
    pub fn new(mid_window: usize, pct_window: usize, low_pct: f64, high_pct: f64) -> Self {
        Self {
            mid: RollingMidline::new(mid_window),
            pct: PercentileChannels::new_with_basis(pct_window, PercentileBasis::Close, low_pct, high_pct),
            last_dist_mid: 0.0,
            last_dist_mid_pct: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.mid.reset();
        self.pct.reset();
        self.last_dist_mid = 0.0;
        self.last_dist_mid_pct = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.mid.is_ready()
    }

    /// Feed resolved OHLCV lanes `[open, high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high   = lanes[1];
        let low    = lanes[2];
        let close  = lanes[3];
        let mid = self.mid.feed(&[high, low]);
        let (_lower, mid_pct, _upper) = self.pct.feed(close);
        let dist_mid = if mid != 0.0 {
            (close - mid) / mid.abs().max(1e-9)
        } else {
            0.0
        };
        let dist_mid_pct = if mid_pct != 0.0 {
            (close - mid_pct) / mid_pct.abs().max(1e-9)
        } else {
            0.0
        };
        self.last_dist_mid = dist_mid;
        self.last_dist_mid_pct = dist_mid_pct;
        (dist_mid, dist_mid_pct)
    }

    /// Named output getter: brace `dist`.
    #[inline]
    pub fn dist(&self) -> f64 { self.last_dist_mid }

    /// Named output getter: brace `mid_pct`.
    #[inline]
    pub fn mid_pct(&self) -> f64 { self.last_dist_mid_pct }

    #[inline]
    pub fn values(&self) -> (f64, f64) {
        (self.last_dist_mid, self.last_dist_mid_pct)
    }

}

impl Default for DistanceToLevels {
    /// Factory default: `new(50, 50, 0.1, 0.9)`
    /// (mid_window=`unwrap_or(50)`, pct_window=`unwrap_or(50)`,
    ///  low_pct=`unwrap_or(0.1)`, high_pct=`unwrap_or(0.9)`).
    fn default() -> Self {
        Self::new(50, 50, 0.1, 0.9)
    }
}

// ── Contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, Render, RenderSpec, SourceAxis,
    UpdateComplexity, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`DistanceToLevels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DistanceToLevelsConfig {
    /// Rolling window for the midline (High+Low average).
    pub mid_window: Param<usize>,
    /// Rolling window for the percentile channel.
    pub pct_window: Param<usize>,
    /// Lower percentile (e.g. 0.1).
    pub low_pct: Param<f64>,
    /// Upper percentile (e.g. 0.9).
    pub high_pct: Param<f64>,
}

impl crate::contract::Config for DistanceToLevelsConfig {
    fn defaults() -> Self {
        DistanceToLevelsConfig {
            mid_window: Param::Solo(50),
            pct_window: Param::Solo(50),
            low_pct: Param::Solo(0.1),
            high_pct: Param::Solo(0.9),
        }
    }
    fn machine_defaults() -> Self {
        // mid_window / pct_window: Class A period — auto range(2,4048,1) is correct.
        // low_pct / high_pct: Class D fraction (percentile bounds 0..1) — spec says sweep_f64(0.0,1.0,0.05).
        let mut s = Self::machine_defaults_auto();
        s.low_pct  = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s.high_pct = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Indicator for DistanceToLevels {
    const ID: IndicatorId = IndicatorId::DistLevels;
    /// Not a pluggable family — a structural level-distance indicator.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// All five OHLCV fields forwarded to both inner indicators.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(1) outer update (two running scalars); inners carry their own cost via Ports.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Rmid, &[IndicatorOutputId::Rmid]),
            Port::new(IndicatorId::Percentilech, &[
                IndicatorOutputId::PercentilechMiddle,
            ]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::DistLevelsDist),
        Output::centered(IndicatorOutputId::DistLevelsMidPct),
    ];
    type Config = DistanceToLevelsConfig;
    type Runtime = DistanceToLevels;

    fn create(cfg: DistanceToLevelsConfig) -> DistanceToLevels {
        DistanceToLevels::new(
            cfg.mid_window.resolved(),
            cfg.pct_window.resolved(),
            cfg.low_pct.resolved(),
            cfg.high_pct.resolved(),
        )
    }
}


impl Render for DistanceToLevels {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(IndicatorId::DistLevels)
            .sub_pane()
            .line_output(
                IndicatorOutputId::DistLevelsDist,
                "Dist to Levels",
                Color::hex(0xFF9800),
            )
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_to_levels_creation() {
        let dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        assert!(!dtl.is_ready());
    }

    #[test]
    fn test_distance_to_levels_warmup() {
        let mut dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dtl.feed(&[price, price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(dtl.is_ready());
    }

    #[test]
    fn test_distance_to_levels_values() {
        let mut dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (d1, d2) = dtl.feed(&[price, price + 1.0, price - 1.0, price, 1000.0]);
            assert!(d1.is_finite(), "Distance should be finite");
            assert!(d2.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_distance_to_levels_reset() {
        let mut dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        for i in 0..25 {
            dtl.feed(&[100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0]);
        }
        dtl.reset();
        assert!(!dtl.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_dist_levels() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::DistLevels(<<DistanceToLevels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..60 {
            let price = 100.0 + i as f64;
            // volume=9999.0 is a wild value that const SOURCE maps to lane[4] — proves
            // the factory resolves lanes in [open,high,low,close,volume] order.
            f.feed(0, MarketSample::Bar {
                open: price,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "dist_mid should be finite: {v}");
    }
}
