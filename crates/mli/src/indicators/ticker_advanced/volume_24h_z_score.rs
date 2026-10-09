//! Volume24hZScore — z-score of current 24h volume relative to rolling history.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::ticker_consumer::TickerConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::Ticker;
use crate::engine::stream_kind::StreamKind;

/// Z-score of current 24h traded volume within a rolling window.
///
/// `z = (current - mean) / std`
///
/// Returns 0.0 when std is near zero or window is not yet full.
///
/// Output: `Single(z_score)`.
#[derive(Debug, Clone)]
pub struct Volume24hZScore {
    period: usize,
    history: Vec<f64>,
    cursor: usize,
    count: usize,
    last_z: f64,
}

impl Volume24hZScore {
    /// Create with given rolling window size (clamped ≥ 2).
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: vec![0.0; period],
            cursor: 0,
            count: 0,
            last_z: 0.0,
        }
    }

    fn compute_z(&self, current: f64) -> f64 {
        let n = self.count.min(self.period);
        if n < 2 {
            return 0.0;
        }
        let slice: Vec<f64> = if self.count >= self.period {
            self.history.clone()
        } else {
            self.history[..self.count].to_vec()
        };
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let variance = slice.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / slice.len() as f64;
        let std = variance.sqrt();
        if std < 1e-15 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl Default for Volume24hZScore {
    fn default() -> Self {
        Self::new(20)
    }
}

/// Typed configuration for [`Volume24hZScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct Volume24hZScoreConfig {
    pub period: Param<usize>,
}

impl Indicator for Volume24hZScore {
    const ID: IndicatorId = IndicatorId::Volume24hZScore;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Ticker];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Volume24hZScore)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    type Config = Volume24hZScoreConfig;
    type Runtime = Volume24hZScore;

    fn create(cfg: Volume24hZScoreConfig) -> Volume24hZScore {
        Volume24hZScore::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for Volume24hZScoreConfig {
    fn defaults() -> Self {
        Volume24hZScoreConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period is Class A (rolling z-score window) — auto provides range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Volume24hZScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Volume24hZScore, "Volume 24h Z-Score", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl TickerConsumer for Volume24hZScore {
    fn update_ticker(&mut self, ticker: &Ticker) {
        let vol = ticker.volume_24h.unwrap_or(0.0);
        // Insert first, then compute z so the current value is included in the window
        self.history[self.cursor] = vol;
        self.cursor = (self.cursor + 1) % self.period;
        if self.count < self.period {
            self.count += 1;
        }
        self.last_z = self.compute_z(vol);
    }


    fn reset(&mut self) {
        self.history.fill(0.0);
        self.cursor = 0;
        self.count = 0;
        self.last_z = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker_with_vol(vol: f64) -> Ticker {
        Ticker {
            last_price: 50_000.0,
            bid_price: None,
            ask_price: None,
            high_24h: None,
            low_24h: None,
            volume_24h: Some(vol),
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = Volume24hZScore::new(5);
        for _ in 0..4 {
            ind.update_ticker(&ticker_with_vol(100.0));
        }
        assert!(!ind.is_ready());
        ind.update_ticker(&ticker_with_vol(100.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn constant_volume_gives_zero_z() {
        let mut ind = Volume24hZScore::new(5);
        for _ in 0..10 {
            ind.update_ticker(&ticker_with_vol(100.0));
        }
        let z = ind.value();
        assert!(z.abs() < 1e-9, "constant volume → z=0, got {z}");
    }

    #[test]
    fn spike_volume_gives_positive_z() {
        let mut ind = Volume24hZScore::new(5);
        for _ in 0..5 {
            ind.update_ticker(&ticker_with_vol(100.0));
        }
        ind.update_ticker(&ticker_with_vol(1000.0));
        let z = ind.value();
        assert!(z > 0.0, "spike should give positive z, got {z}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = Volume24hZScore::new(5);
        for _ in 0..5 {
            ind.update_ticker(&ticker_with_vol(100.0));
        }
        ind.reset();
        assert!(!ind.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_volume_24h_z_score() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::core::types::Ticker as TickerType;
        use super::Volume24hZScoreConfig;

        let mut f = IndicatorOrder::Volume24hZScore(Volume24hZScoreConfig { period: Param::Solo(3) })
            .build_solo()
            .unwrap();
        let t = TickerType {
            last_price: 9999.0,
            bid_price: None,
            ask_price: None,
            high_24h: None,
            low_24h: None,
            volume_24h: Some(100.0),
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 0,
            ..Default::default()
        };
        for _ in 0..3 {
            f.feed(0, MarketSample::Ticker(&t));
        }
        let z = f.primary();
        assert!(z.abs() < 1e-9, "constant volume → z=0, got {z}");
    }
}

impl Volume24hZScore {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_z
    }
}
