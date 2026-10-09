//! Chart layers the host paints itself.
//!
//! These are not [`super::Indicator`]s. Nothing here produces an
//! [`crate::engine::contract_engine::IndicatorOutputId`]. The chart keeps
//! the ring buffer and draws the layer. The contract is the stable id, the
//! pane, the stream the host must raise, and the typed settings.
//!
//! Ids are the public catalog strings (`overlay_volume_profile`, …) so a
//! later compatibility shim can find the same layer. An empty venue list
//! means every venue that serves the stream, not none.

use crate::engine::stream_kind::StreamKind;

/// Where the layer is drawn. One catalog category, two paint spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintSpace {
    /// On the price series.
    OnPrice,
    /// A pane of its own, under the price series.
    Subpane,
}

/// Which bars a volume profile reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileWindow {
    VisibleRange,
    Session,
    Composite,
}

/// Which chart edge a profile histogram grows from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileSide {
    Right,
    Left,
}

/// The seven host layers that landed in the public catalog after the fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostOverlayId {
    VolumeProfile,
    TpoProfile,
    OiDelta,
    FundingRate,
    DomHeatmap,
    LiquidationHeatmap,
    LiquidationProjection,
}

impl HostOverlayId {
    pub const ALL: [HostOverlayId; 7] = [
        HostOverlayId::VolumeProfile,
        HostOverlayId::TpoProfile,
        HostOverlayId::OiDelta,
        HostOverlayId::FundingRate,
        HostOverlayId::DomHeatmap,
        HostOverlayId::LiquidationHeatmap,
        HostOverlayId::LiquidationProjection,
    ];

    /// Catalog id. Stable across the fork.
    pub const fn as_str(self) -> &'static str {
        match self {
            HostOverlayId::VolumeProfile => "overlay_volume_profile",
            HostOverlayId::TpoProfile => "overlay_tpo_profile",
            HostOverlayId::OiDelta => "overlay_oi_delta",
            HostOverlayId::FundingRate => "overlay_funding_rate",
            HostOverlayId::DomHeatmap => "overlay_dom_heatmap",
            HostOverlayId::LiquidationHeatmap => "overlay_liquidation_heatmap",
            HostOverlayId::LiquidationProjection => "overlay_liquidation_projection",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            HostOverlayId::VolumeProfile => "Volume Profile",
            HostOverlayId::TpoProfile => "TPO Profile",
            HostOverlayId::OiDelta => "OI Delta",
            HostOverlayId::FundingRate => "Funding Rate",
            HostOverlayId::DomHeatmap => "DOM Heatmap",
            HostOverlayId::LiquidationHeatmap => "Liquidation Heatmap",
            HostOverlayId::LiquidationProjection => "Projected Liquidations",
        }
    }

    pub const fn paint(self) -> PaintSpace {
        match self {
            HostOverlayId::OiDelta | HostOverlayId::FundingRate => PaintSpace::Subpane,
            _ => PaintSpace::OnPrice,
        }
    }

    /// Stream the host must subscribe. `None` means the bars already on
    /// the chart: volume profile and TPO do not open a second feed.
    pub const fn stream(self) -> Option<StreamKind> {
        match self {
            HostOverlayId::VolumeProfile | HostOverlayId::TpoProfile => None,
            HostOverlayId::OiDelta | HostOverlayId::LiquidationProjection => {
                Some(StreamKind::OpenInterest)
            }
            HostOverlayId::FundingRate => Some(StreamKind::Funding),
            HostOverlayId::DomHeatmap => Some(StreamKind::OrderBook),
            HostOverlayId::LiquidationHeatmap => Some(StreamKind::Liquidation),
        }
    }

    pub fn default_config(self) -> HostOverlayConfig {
        match self {
            HostOverlayId::VolumeProfile => {
                HostOverlayConfig::VolumeProfile(VolumeProfileOverlay::default())
            }
            HostOverlayId::TpoProfile => HostOverlayConfig::TpoProfile(TpoProfileOverlay::default()),
            HostOverlayId::OiDelta => HostOverlayConfig::OiDelta(SignedStripOverlay::default()),
            HostOverlayId::FundingRate => {
                HostOverlayConfig::FundingRate(SignedStripOverlay::default())
            }
            HostOverlayId::DomHeatmap => HostOverlayConfig::DomHeatmap(DomHeatmapOverlay::default()),
            HostOverlayId::LiquidationHeatmap => {
                HostOverlayConfig::LiquidationHeatmap(LiquidationHeatmapOverlay::default())
            }
            HostOverlayId::LiquidationProjection => {
                HostOverlayConfig::LiquidationProjection(LiquidationProjectionOverlay::default())
            }
        }
    }
}

/// Typed settings for one host layer.
#[derive(Debug, Clone, PartialEq)]
pub enum HostOverlayConfig {
    VolumeProfile(VolumeProfileOverlay),
    TpoProfile(TpoProfileOverlay),
    OiDelta(SignedStripOverlay),
    FundingRate(SignedStripOverlay),
    DomHeatmap(DomHeatmapOverlay),
    LiquidationHeatmap(LiquidationHeatmapOverlay),
    LiquidationProjection(LiquidationProjectionOverlay),
}

/// Venues folded into one layer. Empty = every venue that serves the stream.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VenueFold {
    pub aggregate: bool,
    pub venues: Vec<String>,
}

/// Volume traded per price row, over a window `mode` chooses.
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeProfileOverlay {
    pub mode: ProfileWindow,
    /// Price buckets. Catalog range 6..=200.
    pub rows: u16,
    /// Percent of volume inside the value area. Catalog range 50..=95.
    pub value_area: f64,
    /// Histogram width as a percent of the chart. Catalog range 5..=50.
    pub width_pct: f64,
    pub opacity: f64,
    pub side: ProfileSide,
    pub show_poc: bool,
    pub show_value_area: bool,
    pub show_labels: bool,
    /// Default off: each extra venue is a kline subscription.
    pub venues: VenueFold,
}

impl Default for VolumeProfileOverlay {
    fn default() -> Self {
        Self {
            mode: ProfileWindow::VisibleRange,
            rows: 24,
            value_area: 70.0,
            width_pct: 18.0,
            opacity: 1.0,
            side: ProfileSide::Right,
            show_poc: true,
            show_value_area: true,
            show_labels: true,
            venues: VenueFold {
                aggregate: false,
                venues: Vec::new(),
            },
        }
    }
}

/// Time-at-price blocks over the bars on screen. `rows` is a target:
/// the row step snaps to a round price, so the count lands near this.
/// No `mode` — a session profile of touches is the letter TPO chart, not this layer.
#[derive(Debug, Clone, PartialEq)]
pub struct TpoProfileOverlay {
    /// Catalog range 20..=200.
    pub rows: u16,
    pub value_area: f64,
    pub width_pct: f64,
    pub opacity: f64,
    pub side: ProfileSide,
    pub show_poc: bool,
    pub show_value_area: bool,
    pub show_labels: bool,
    pub venues: VenueFold,
}

impl Default for TpoProfileOverlay {
    fn default() -> Self {
        Self {
            rows: 60,
            value_area: 70.0,
            width_pct: 18.0,
            opacity: 1.0,
            side: ProfileSide::Right,
            show_poc: true,
            show_value_area: true,
            show_labels: true,
            venues: VenueFold {
                aggregate: false,
                venues: Vec::new(),
            },
        }
    }
}

/// OI delta and funding rate. `zones` tints the area against zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SignedStripOverlay {
    pub zones: bool,
}

/// Resting book depth under the candles, grained to the chart's bars.
#[derive(Debug, Clone, PartialEq)]
pub struct DomHeatmapOverlay {
    /// Instrument ticks folded into one heat row. Catalog range 1..=200.
    pub row_ticks: u16,
    /// Half-width of the band around mid, in percent of price. Catalog range 0.05..=5.
    pub band_pct: f64,
    /// Gain on the on-screen depth normaliser. Catalog range 0.1..=10.
    pub sensitivity: f64,
    /// Percentile of visible depth that sets the normaliser. Catalog range 10..=100.
    pub norm_pct: f64,
    /// Exponent of the depth-to-alpha curve. Catalog range 0.4..=4.
    pub contrast: f64,
    pub opacity: f64,
    /// Multiple of the visible mean that paints a cell as a wall. Catalog range 1.5..=25.
    pub wall_x: f64,
    pub show_values: bool,
    /// Scale on the auto-fitted font. Catalog range 0.5..=2.
    pub value_size: f64,
    /// Default on: one venue's book is a sample of resting size.
    pub venues: VenueFold,
}

impl Default for DomHeatmapOverlay {
    fn default() -> Self {
        Self {
            row_ticks: 4,
            band_pct: 0.6,
            sensitivity: 1.0,
            norm_pct: 90.0,
            contrast: 1.7,
            opacity: 0.38,
            wall_x: 5.0,
            show_values: true,
            value_size: 1.0,
            venues: VenueFold {
                aggregate: true,
                venues: Vec::new(),
            },
        }
    }
}

/// Liquidations that already printed, as a time-by-price heat under the series.
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidationHeatmapOverlay {
    /// Gain on the heat normaliser. Catalog range 0.1..=10.
    pub sensitivity: f64,
    /// Row height as a percent of price. Catalog range 0.001..=1.
    pub row_pct: f64,
    /// Notional a row must collect before it is a level.
    pub min_usd: f64,
    pub show_levels: bool,
    pub show_dots: bool,
    pub dot_scale: f64,
    /// Half-reach of a print's price line, in bars, at the largest print in view.
    pub span_bars: f64,
    pub opacity: f64,
    /// Default on: liquidations are sparse, one venue misleads.
    pub venues: VenueFold,
}

impl Default for LiquidationHeatmapOverlay {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            row_pct: 0.02,
            min_usd: 25_000.0,
            show_levels: true,
            show_dots: true,
            dot_scale: 1.0,
            span_bars: 6.0,
            opacity: 0.55,
            venues: VenueFold {
                aggregate: true,
                venues: Vec::new(),
            },
        }
    }
}

/// Where leveraged positions are likely to be forced out, modelled from open interest.
/// Sibling of [`LiquidationHeatmapOverlay`]: that one is what printed, this one is still loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidationProjectionOverlay {
    /// Shares of the assumed leverage mix. Normalised by the host; they need not sum to 1.
    pub lev_5: f64,
    pub lev_10: f64,
    pub lev_25: f64,
    pub lev_50: f64,
    pub lev_100: f64,
    /// Maintenance margin, in percent of price. 0.4 means 0.4%.
    pub mmr: f64,
    /// Share of new exposure assumed long.
    pub long_share: f64,
    /// Band height as a percent of price.
    pub row_pct: f64,
    pub min_usd: f64,
    pub sensitivity: f64,
    /// Kept low: hundreds of bands over the window, under the candles.
    pub opacity: f64,
    /// How far a standing band may run past the newest bar. Display only.
    pub future_bars: u16,
    /// Dissolves only the `future_bars` extension.
    pub fade: bool,
    /// Bands price already traded through.
    pub show_consumed: bool,
    pub venues: VenueFold,
}

impl Default for LiquidationProjectionOverlay {
    fn default() -> Self {
        Self {
            lev_5: 0.10,
            lev_10: 0.25,
            lev_25: 0.30,
            lev_50: 0.25,
            lev_100: 0.10,
            mmr: 0.4,
            long_share: 0.5,
            row_pct: 0.05,
            min_usd: 0.0,
            sensitivity: 1.0,
            opacity: 0.3,
            future_bars: 0,
            fade: false,
            show_consumed: true,
            venues: VenueFold {
                aggregate: true,
                venues: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_the_public_catalog_strings() {
        let ids: Vec<&str> = HostOverlayId::ALL.iter().map(|id| id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "overlay_volume_profile",
                "overlay_tpo_profile",
                "overlay_oi_delta",
                "overlay_funding_rate",
                "overlay_dom_heatmap",
                "overlay_liquidation_heatmap",
                "overlay_liquidation_projection",
            ]
        );
    }

    #[test]
    fn paint_and_stream_match_the_public_catalog() {
        for id in HostOverlayId::ALL {
            match id {
                HostOverlayId::OiDelta | HostOverlayId::FundingRate => {
                    assert_eq!(id.paint(), PaintSpace::Subpane);
                }
                _ => assert_eq!(id.paint(), PaintSpace::OnPrice),
            }
        }
        assert_eq!(HostOverlayId::VolumeProfile.stream(), None);
        assert_eq!(HostOverlayId::TpoProfile.stream(), None);
        assert_eq!(
            HostOverlayId::OiDelta.stream(),
            Some(StreamKind::OpenInterest)
        );
        assert_eq!(HostOverlayId::FundingRate.stream(), Some(StreamKind::Funding));
        assert_eq!(
            HostOverlayId::DomHeatmap.stream(),
            Some(StreamKind::OrderBook)
        );
        assert_eq!(
            HostOverlayId::LiquidationHeatmap.stream(),
            Some(StreamKind::Liquidation)
        );
        assert_eq!(
            HostOverlayId::LiquidationProjection.stream(),
            Some(StreamKind::OpenInterest)
        );
    }

    #[test]
    fn defaults_match_the_public_catalog() {
        let vp = VolumeProfileOverlay::default();
        assert_eq!(vp.mode, ProfileWindow::VisibleRange);
        assert_eq!(vp.rows, 24);
        assert!(!vp.venues.aggregate);
        assert!(vp.venues.venues.is_empty());

        assert_eq!(TpoProfileOverlay::default().rows, 60);
        assert!(!SignedStripOverlay::default().zones);

        let dom = DomHeatmapOverlay::default();
        assert_eq!(dom.row_ticks, 4);
        assert_eq!(dom.band_pct, 0.6);
        assert_eq!(dom.norm_pct, 90.0);
        assert_eq!(dom.contrast, 1.7);
        assert_eq!(dom.opacity, 0.38);
        assert!(dom.venues.aggregate);

        let heat = LiquidationHeatmapOverlay::default();
        assert_eq!(heat.row_pct, 0.02);
        assert_eq!(heat.min_usd, 25_000.0);
        assert_eq!(heat.span_bars, 6.0);
        assert!(heat.venues.aggregate);

        let proj = LiquidationProjectionOverlay::default();
        assert_eq!(proj.lev_25, 0.30);
        assert_eq!(proj.mmr, 0.4);
        assert_eq!(proj.opacity, 0.3);
        assert_eq!(proj.future_bars, 0);
        assert!(!proj.fade);
        assert!(proj.show_consumed);
    }
}
