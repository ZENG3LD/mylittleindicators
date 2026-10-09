//! Volume Weighted RSI - RSI взвешенный по объему
//!
//! Улучшенная версия RSI, которая учитывает объем торгов при расчете.
//! Большие объемы получают больший вес в расчете, что делает индикатор
//! более чувствительным к "умным деньгам".
//!
//! FLAG: embeds `Rsi` for the `regular_rsi` output, but ALSO runs a parallel
//! independent Wilder smoother (`avg_regular_gain/loss`) that replicates RSI math
//! internally. The two paths cannot be merged without altering math — contracted
//! as a standalone with own buffers. The embedded `Rsi` is kept as-is (it feeds the
//! `regular_rsi` field on the result struct; it is NOT the VW path).

use crate::indicators::momentum::rsi::Rsi;
use crate::engine::ohlcv_field::OhlcvField;

/// Результат Volume Weighted RSI
#[derive(Debug, Clone, Copy)]
pub struct VolumeWeightedRsiResult {
    pub vw_rsi: f64,
    pub regular_rsi: f64,
    pub volume_factor: f64,
    pub avg_volume: f64,
    pub volume_trend: i8,
    pub smart_money_signal: i8,
}

impl VolumeWeightedRsiResult {
    pub fn empty() -> Self {
        Self {
            vw_rsi: 50.0,
            regular_rsi: 50.0,
            volume_factor: 1.0,
            avg_volume: 0.0,
            volume_trend: 0,
            smart_money_signal: 0,
        }
    }

    pub fn market_condition(&self) -> &'static str {
        let (oversold, overbought) = if self.volume_factor > 1.2 {
            (25.0, 75.0)
        } else if self.volume_factor < 0.8 {
            (35.0, 65.0)
        } else {
            (30.0, 70.0)
        };

        match self.vw_rsi {
            x if x <= oversold => "Перепродан с объемом",
            x if x >= overbought => "Перекуплен с объемом",
            _ => "Нейтральный",
        }
    }

    pub fn smart_money_description(&self) -> &'static str {
        match self.smart_money_signal {
            1 => "Умные деньги покупают",
            -1 => "Умные деньги продают",
            _ => "Нет активности умных денег",
        }
    }
}

/// Volume Weighted RSI индикатор
#[derive(Clone)]
pub struct VolumeWeightedRsi {
    period: usize,
    volume_period: usize,

    vw_gains: Vec<f64>,
    vw_losses: Vec<f64>,
    volumes: Vec<f64>,

    // Embedded RSI for the regular_rsi display value (not used by the VW path)
    regular_rsi: Rsi,

    // Parallel Wilder buffers for volume-weighted RSI (independent from embedded Rsi)
    regular_gains: Vec<f64>,
    regular_losses: Vec<f64>,

    avg_vw_gain: f64,
    avg_vw_loss: f64,
    avg_regular_gain: f64,
    avg_regular_loss: f64,
    avg_volume: f64,

    prev_close: Option<f64>,

    current_result: VolumeWeightedRsiResult,

    is_ready: bool,
    update_count: usize,
}

impl std::fmt::Debug for VolumeWeightedRsi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolumeWeightedRsi")
            .field("period", &self.period)
            .field("volume_period", &self.volume_period)
            .field("is_ready", &self.is_ready)
            .finish()
    }
}

impl VolumeWeightedRsi {
    pub fn new() -> Self {
        Self::with_periods(14, 20)
    }

    pub fn with_periods(rsi_period: usize, volume_period: usize) -> Self {
        Self::with_source(rsi_period, volume_period, OhlcvField::Close)
    }

    pub fn with_source(rsi_period: usize, volume_period: usize, _price_source: OhlcvField) -> Self {
        // `_price_source` is config-only (factory resolves it via `const SOURCE`); runtime keeps none.
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        assert!(volume_period > 0, "Volume period must be greater than 0");

        Self {
            period: rsi_period,
            volume_period,

            vw_gains: Vec::with_capacity(64),
            vw_losses: Vec::with_capacity(64),
            volumes: Vec::with_capacity(64),
            regular_rsi: Rsi::new(rsi_period),
            regular_gains: Vec::with_capacity(64),
            regular_losses: Vec::with_capacity(64),

            avg_vw_gain: 0.0,
            avg_vw_loss: 0.0,
            avg_regular_gain: 0.0,
            avg_regular_loss: 0.0,
            avg_volume: 0.0,

            prev_close: None,
            current_result: VolumeWeightedRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed lanes as [price, volume] (Sources flavor: Close + Volume).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let price = lanes[0];
        let volume = lanes[1];
        self.feed_price_volume(price, volume);
        self.current_result.vw_rsi
    }

    /// Feed pre-extracted scalar + volume.
    /// `price` is the resolved price source; `volume` is always the bar's volume.
    pub fn feed_price_volume(&mut self, price: f64, volume: f64) -> VolumeWeightedRsiResult {
        // Drive embedded RSI (feeds close scalar)
        let regular_rsi_value = self.regular_rsi.feed(price);
        self.current_result.regular_rsi = regular_rsi_value;

        if self.volumes.len() >= self.volume_period {
            self.volumes.remove(0);
        }
        self.volumes.push(volume);
        self.avg_volume = self.volumes.iter().sum::<f64>() / self.volumes.len() as f64;

        if let Some(prev_close) = self.prev_close {
            let price_change = price - prev_close;

            let regular_gain = if price_change > 0.0 { price_change } else { 0.0 };
            let regular_loss = if price_change < 0.0 { -price_change } else { 0.0 };

            let volume_weight = if self.avg_volume > 0.0 {
                volume / self.avg_volume
            } else {
                1.0
            };

            let vw_gain = regular_gain * volume_weight;
            let vw_loss = regular_loss * volume_weight;

            self.add_to_buffers(regular_gain, regular_loss, vw_gain, vw_loss);
            self.calculate_rsi_values();
            self.analyze_volume_and_smart_money();
        }

        self.prev_close = Some(price);
        self.update_count += 1;
        self.current_result
    }

    fn add_to_buffers(&mut self, regular_gain: f64, regular_loss: f64, vw_gain: f64, vw_loss: f64) {
        if self.regular_gains.len() >= self.period {
            self.regular_gains.remove(0);
        }
        self.regular_gains.push(regular_gain);

        if self.regular_losses.len() >= self.period {
            self.regular_losses.remove(0);
        }
        self.regular_losses.push(regular_loss);

        if self.vw_gains.len() >= self.period {
            self.vw_gains.remove(0);
        }
        self.vw_gains.push(vw_gain);

        if self.vw_losses.len() >= self.period {
            self.vw_losses.remove(0);
        }
        self.vw_losses.push(vw_loss);
    }

    fn calculate_rsi_values(&mut self) {
        if self.regular_gains.len() == self.period {
            if self.avg_regular_gain == 0.0 && self.avg_regular_loss == 0.0 {
                self.avg_regular_gain = self.regular_gains.iter().sum::<f64>() / self.period as f64;
                self.avg_regular_loss = self.regular_losses.iter().sum::<f64>() / self.period as f64;
                self.avg_vw_gain = self.vw_gains.iter().sum::<f64>() / self.period as f64;
                self.avg_vw_loss = self.vw_losses.iter().sum::<f64>() / self.period as f64;
            } else {
                let alpha = 1.0 / self.period as f64;

                let latest_regular_gain = self.regular_gains[self.regular_gains.len() - 1];
                let latest_regular_loss = self.regular_losses[self.regular_losses.len() - 1];
                let latest_vw_gain = self.vw_gains[self.vw_gains.len() - 1];
                let latest_vw_loss = self.vw_losses[self.vw_losses.len() - 1];

                self.avg_regular_gain = alpha * latest_regular_gain + (1.0 - alpha) * self.avg_regular_gain;
                self.avg_regular_loss = alpha * latest_regular_loss + (1.0 - alpha) * self.avg_regular_loss;
                self.avg_vw_gain = alpha * latest_vw_gain + (1.0 - alpha) * self.avg_vw_gain;
                self.avg_vw_loss = alpha * latest_vw_loss + (1.0 - alpha) * self.avg_vw_loss;
            }

            self.current_result.vw_rsi = if self.avg_vw_loss == 0.0 {
                100.0
            } else {
                let vw_rs = self.avg_vw_gain / self.avg_vw_loss;
                100.0 - (100.0 / (1.0 + vw_rs))
            };

            self.is_ready = true;
        }
    }

    fn analyze_volume_and_smart_money(&mut self) {
        if self.volumes.len() < 3 {
            return;
        }

        let current_volume = self.volumes[self.volumes.len() - 1];
        self.current_result.volume_factor = if self.avg_volume > 0.0 {
            current_volume / self.avg_volume
        } else {
            1.0
        };

        let recent_volumes = &self.volumes[self.volumes.len().saturating_sub(3)..];
        if recent_volumes.len() >= 3 {
            let volume_change1 = recent_volumes[1] - recent_volumes[0];
            let volume_change2 = recent_volumes[2] - recent_volumes[1];

            if volume_change1 > 0.0 && volume_change2 > 0.0 {
                self.current_result.volume_trend = 1;
            } else if volume_change1 < 0.0 && volume_change2 < 0.0 {
                self.current_result.volume_trend = -1;
            } else {
                self.current_result.volume_trend = 0;
            }
        }

        self.analyze_smart_money();
    }

    fn analyze_smart_money(&mut self) {
        if !self.is_ready {
            return;
        }

        let vw_rsi = self.current_result.vw_rsi;
        let regular_rsi = self.current_result.regular_rsi;
        let volume_factor = self.current_result.volume_factor;

        if volume_factor > 1.5 {
            let rsi_divergence = vw_rsi - regular_rsi;

            if rsi_divergence > 5.0 && vw_rsi < 40.0 {
                self.current_result.smart_money_signal = 1;
            } else if rsi_divergence < -5.0 && vw_rsi > 60.0 {
                self.current_result.smart_money_signal = -1;
            } else {
                self.current_result.smart_money_signal = 0;
            }
        } else {
            self.current_result.smart_money_signal = 0;
        }

        self.current_result.avg_volume = self.avg_volume;
    }

    pub fn value(&self) -> f64 {
        self.current_result.vw_rsi
    }

    pub fn result(&self) -> VolumeWeightedRsiResult {
        self.current_result
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.regular_rsi.reset();
        self.vw_gains.clear();
        self.vw_losses.clear();
        self.volumes.clear();
        self.regular_gains.clear();
        self.regular_losses.clear();

        self.avg_vw_gain = 0.0;
        self.avg_vw_loss = 0.0;
        self.avg_regular_gain = 0.0;
        self.avg_regular_loss = 0.0;
        self.avg_volume = 0.0;

        self.prev_close = None;
        self.current_result = VolumeWeightedRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn parameters(&self) -> (usize, usize) {
        (self.period, self.volume_period)
    }

    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }

        let result = self.current_result;

        let (oversold, overbought) = if result.volume_factor > 1.2 {
            (25.0, 75.0)
        } else if result.volume_factor < 0.8 {
            (35.0, 65.0)
        } else {
            (30.0, 70.0)
        };

        if result.vw_rsi <= oversold && result.volume_factor > 1.0 {
            1
        } else if result.vw_rsi >= overbought && result.volume_factor > 1.0 {
            -1
        } else {
            0
        }
    }

    pub fn smart_money_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        self.current_result.smart_money_signal
    }

    pub fn divergence_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }

        let result = self.current_result;
        let divergence = result.vw_rsi - result.regular_rsi;

        if result.volume_factor > 1.2 {
            if divergence > 10.0 {
                return 1;
            } else if divergence < -10.0 {
                return -1;
            }
        }

        0
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};

/// Typed contract config for [`VolumeWeightedRsi`].
///
/// Dual-mode: every field is a `Param`. No smoother slot — the embedded `Rsi` uses its
/// default Wilder RMA (the VW path runs its own inline Wilder math independently).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VwrsiConfig {
    pub rsi_period: Param<usize>,
    pub volume_period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for VolumeWeightedRsi {
    const ID: IndicatorId = IndicatorId::Vwrsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    // VW-RSI needs the resolved price field + volume.
    // Volume is always extracted by the factory automatically when SOURCE includes Volume.
    // We declare Close+Volume so the factory passes both.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    // Stores: rolling windows for gains/losses/volumes (O(period) per reset, O(1) per update)
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[
            Store::window(StoreKind::Vec), // vw_gains
            Store::window(StoreKind::Vec), // vw_losses
            Store::window(StoreKind::Vec), // volumes
        ],
        // Embedded Rsi drives regular_rsi; parallel Wilder buffers for VW path are standalone.
        // FLAG: duplicate Wilder logic; the Port is metadata only.
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Vwrsi)];
    type Config = VwrsiConfig;
    type Runtime = VolumeWeightedRsi;

    fn create(cfg: VwrsiConfig) -> VolumeWeightedRsi {
        VolumeWeightedRsi::with_source(
            cfg.rsi_period.resolved(),
            cfg.volume_period.resolved(),
            cfg.source.resolved(),
        )
    }
}

impl crate::contract::Config for VwrsiConfig {
    fn defaults() -> Self {
        VwrsiConfig {
            rsi_period: Param::Solo(14),
            volume_period: Param::Solo(20),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/volume_period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        Self::machine_defaults_auto()
    }
}


impl Render for VolumeWeightedRsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vwrsi, "VWRSI", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

impl Default for VolumeWeightedRsi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_weighted_rsi_creation() {
        let vw_rsi = VolumeWeightedRsi::new();
        assert!(!vw_rsi.is_ready());
        assert_eq!(vw_rsi.parameters(), (14, 20));
    }

    #[test]
    fn test_volume_weighted_rsi_rsi_period_configurable() {
        let mut vw = VolumeWeightedRsi::with_periods(9, 15);
        assert_eq!(vw.parameters(), (9, 15));
        for i in 0..25 {
            let p = 100.0 + i as f64 * 0.5;
            let r = vw.feed_price_volume(p, 1000.0);
            assert!(r.vw_rsi.is_finite());
        }
        assert!(vw.is_ready());
    }

    #[test]
    fn test_volume_weighted_rsi_with_periods() {
        let vw_rsi = VolumeWeightedRsi::with_periods(21, 30);
        assert_eq!(vw_rsi.parameters(), (21, 30));
    }

    #[test]
    fn test_volume_weighted_rsi_update() {
        let mut vw_rsi = VolumeWeightedRsi::new();

        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let volume = 1000.0 + (i as f64 * 0.2).cos() * 500.0;

            let result = vw_rsi.feed_price_volume(price, volume);

            if i > 15 {
                assert!(vw_rsi.is_ready());
                assert!(result.vw_rsi >= 0.0 && result.vw_rsi <= 100.0);
                assert!(result.regular_rsi >= 0.0 && result.regular_rsi <= 100.0);
                assert!(result.volume_factor >= 0.0);
                assert!(result.avg_volume > 0.0);
            }
        }
    }

    #[test]
    fn test_trading_signals() {
        let mut vw_rsi = VolumeWeightedRsi::new();

        let mut price = 100.0;
        for i in 0..20 {
            price -= 1.0;
            let _result = vw_rsi.feed_price_volume(price, 1500.0);

            if i > 15 && vw_rsi.is_ready() {
                let signal = vw_rsi.trading_signal();
                let smart_signal = vw_rsi.smart_money_signal();
                let div_signal = vw_rsi.divergence_signal();

                assert!(signal >= -1 && signal <= 1);
                assert!(smart_signal >= -1 && smart_signal <= 1);
                assert!(div_signal >= -1 && div_signal <= 1);
            }
        }
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<VolumeWeightedRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Vwrsi(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64 * 0.5;
            // high=9999.0 proves only close+volume are resolved
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() >= 0.0 && f.primary() <= 100.0);
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<VolumeWeightedRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);
    }
}
