//! Session VWAP bands — cumulative volume-weighted mean and deviation that
//! reset when the UTC day (`bar_time_ms.div_euclid(86_400_000)`) changes.
//!
//! TradingView `ta.vwap(source, timeframe.change("D"), mult)`. This is not
//! [`super::session_vwap::SessionVwap`] (no clock, no bands) and not the rolling
//! `Vwapchan` window.

use crate::contract::{
    Color, Cost, Indicator, Output, Param, Render, RenderSpec, SourceAxis, UpdateComplexity,
};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

const UTC_DAY_MS: i64 = 86_400_000;

/// Price the bands weight. `hl2` is `(high + low) / 2`, `hlc3` is `(high + low + close) / 3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, mli_contract_macros::ParamScalar)]
pub enum SessionVwapBandsSource {
    /// `(high + low) / 2`. Cube `flag == 0`.
    Hl2,
    /// `(high + low + close) / 3`. Cube `flag == 1`.
    Hlc3,
}

impl SessionVwapBandsSource {
    /// Cube launch flag. `0` is hl2, `1` is hlc3.
    pub fn cube_flag(self) -> u32 {
        match self {
            Self::Hl2 => 0,
            Self::Hlc3 => 1,
        }
    }

    fn price(self, high: f64, low: f64, close: f64) -> f64 {
        match self {
            Self::Hl2 => (high + low) / 2.0,
            Self::Hlc3 => (high + low + close) / 3.0,
        }
    }
}

/// Cumulative session VWAP plus `mult` bands of the volume-weighted deviation.
#[derive(Debug, Clone)]
pub struct SessionVwapBands {
    source: SessionVwapBandsSource,
    mult: f64,
    sum_v: f64,
    sum_pv: f64,
    sum_p2v: f64,
    last_day: Option<i64>,
    center: f64,
    upper: f64,
    lower: f64,
    fed: bool,
}

impl SessionVwapBands {
    /// `mult` scales the volume-weighted deviation around the session VWAP.
    pub fn new(source: SessionVwapBandsSource, mult: f64) -> Self {
        Self {
            source,
            mult,
            sum_v: 0.0,
            sum_pv: 0.0,
            sum_p2v: 0.0,
            last_day: None,
            center: 0.0,
            upper: 0.0,
            lower: 0.0,
            fed: false,
        }
    }

    /// Feed `(ts_ms, &[high, low, close, volume])`. `ts_ms` is canonical unix
    /// milliseconds. A change of `ts_ms.div_euclid(86_400_000)` clears the sums
    /// before the bar is applied.
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        let day = ts_ms.div_euclid(UTC_DAY_MS);
        if self.last_day != Some(day) {
            self.sum_v = 0.0;
            self.sum_pv = 0.0;
            self.sum_p2v = 0.0;
            self.last_day = Some(day);
        }
        let src = self.source.price(high, low, close);
        let (center, dev) = if volume > 0.0 {
            self.sum_v += volume;
            self.sum_pv += src * volume;
            self.sum_p2v += src * src * volume;
            let vwap = self.sum_pv / self.sum_v;
            let var = (self.sum_p2v / self.sum_v - vwap * vwap).max(0.0);
            (vwap, var.sqrt())
        } else {
            (src, 0.0)
        };
        self.center = center;
        self.upper = center + self.mult * dev;
        self.lower = center - self.mult * dev;
        self.fed = true;
    }

    /// Session VWAP, or the bar source when this bar has no positive volume.
    pub fn center(&self) -> f64 {
        self.center
    }

    /// `center + mult * dev`. Equal to `center` when the deviation is zero.
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// `center - mult * dev`. Equal to `center` when the deviation is zero.
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// `true` after the first bar. A zero-volume bar still emits its source.
    pub fn is_ready(&self) -> bool {
        self.fed
    }

    /// Clear the session sums and the outputs.
    pub fn reset(&mut self) {
        self.sum_v = 0.0;
        self.sum_pv = 0.0;
        self.sum_p2v = 0.0;
        self.last_day = None;
        self.center = 0.0;
        self.upper = 0.0;
        self.lower = 0.0;
        self.fed = false;
    }
}

/// `source` selects hl2 or hlc3. `mult` defaults to `2.0`.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SessionVwapBandsConfig {
    /// hl2 or hlc3. Default is hlc3, matching `ta.vwap`.
    pub source: Param<SessionVwapBandsSource>,
    /// Band width in deviations. Default `2.0`.
    pub mult: Param<f64>,
}

impl Indicator for SessionVwapBands {
    const ID: IndicatorId = IndicatorId::SessionVwapBands;
    const FAMILY: &'static [crate::contract::Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::SessionVwapBandsCenter),
        Output::price(IndicatorOutputId::SessionVwapBandsUpper),
        Output::price(IndicatorOutputId::SessionVwapBandsLower),
    ];
    type Config = SessionVwapBandsConfig;
    type Runtime = SessionVwapBands;

    fn create(cfg: SessionVwapBandsConfig) -> SessionVwapBands {
        SessionVwapBands::new(cfg.source.resolved(), cfg.mult.resolved())
    }
}

impl crate::contract::Config for SessionVwapBandsConfig {
    fn defaults() -> Self {
        Self {
            source: Param::solo(SessionVwapBandsSource::Hlc3),
            mult: Param::solo(2.0),
        }
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Render for SessionVwapBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::SessionVwapBandsCenter, "Session VWAP", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::SessionVwapBandsUpper, "Upper", Color::hex(0xEF5350))
            .line_output(IndicatorOutputId::SessionVwapBandsLower, "Lower", Color::hex(0x26A69A))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn feed(core: &mut SessionVwapBands, ts: i64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        core.feed(ts, &[high, low, close, volume]);
        (core.center(), core.upper(), core.lower())
    }

    /// Paper series, hl2, mult 2. The factory arm is one `ContractFactory`
    /// (the enum is the whole catalog) checked against the same core.
    ///
    /// Day 20000 (`1_728_000_000_000`):
    /// bar1 H=10 L=8 C=9 V=100 → hl2=9
    /// bar2 H=12 L=10 C=11 V=300 → hl2=11
    /// sum_v=400, sum_pv=9*100+11*300=4200, vwap=10.5
    /// sum_p2v=81*100+121*300=44400
    /// var=44400/400 - 10.5² = 111 - 110.25 = 0.75
    ///
    /// A zero-volume bar does not move the sums. The last millisecond of the
    /// day still belongs to day 20000. The next UTC day is only its own bar.
    #[test]
    fn session_vwap_bands_utc_day_hl2_and_hlc3() {
        let day0 = 1_728_000_000_000;
        let day1 = day0 + UTC_DAY_MS;
        let mut core = SessionVwapBands::new(SessionVwapBandsSource::Hl2, 2.0);

        let (c, u, l) = feed(&mut core, day0, 10.0, 8.0, 9.0, 100.0);
        assert!((c - 9.0).abs() < 1e-12 && (u - 9.0).abs() < 1e-12 && (l - 9.0).abs() < 1e-12);

        let (c, u, l) = feed(&mut core, day0 + 60_000, 12.0, 10.0, 11.0, 300.0);
        let dev = 0.75_f64.sqrt();
        let upper = 10.5 + 2.0 * dev;
        let lower = 10.5 - 2.0 * dev;
        assert!((c - 10.5).abs() < 1e-12, "vwap {c}");
        assert!((u - upper).abs() < 1e-12, "upper {u} dev {dev}");
        assert!((l - lower).abs() < 1e-12, "lower {l} dev {dev}");

        // Zero volume: sums stay, this bar's center is its own hl2.
        let (c, u, l) = feed(&mut core, day0 + 120_000, 14.0, 10.0, 13.0, 0.0);
        assert!((c - 12.0).abs() < 1e-12 && (u - 12.0).abs() < 1e-12 && (l - 12.0).abs() < 1e-12);

        // Still day 20000, so the earlier sums remain under this bar.
        let (c, u, l) = feed(&mut core, day1 - 1, 10.0, 10.0, 10.0, 100.0);
        assert!((c - 10.4).abs() < 1e-9, "continued vwap {c}");
        assert!((u - 12.0).abs() < 1e-9, "continued upper {u}");
        assert!((l - 8.8).abs() < 1e-9, "continued lower {l}");

        // Next UTC day: only this bar. hl2 of (20, 16) is 18, deviation 0.
        let (c, u, l) = feed(&mut core, day1, 20.0, 16.0, 19.0, 50.0);
        assert!((c - 18.0).abs() < 1e-12, "day reset vwap {c}");
        assert!((u - 18.0).abs() < 1e-12 && (l - 18.0).abs() < 1e-12);

        // Same bar, close != hl2: hlc3 is 10, hl2 is 9.
        let mut hl2 = SessionVwapBands::new(SessionVwapBandsSource::Hl2, 2.0);
        let mut hlc3 = SessionVwapBands::new(SessionVwapBandsSource::Hlc3, 2.0);
        let (hl2_c, _, _) = feed(&mut hl2, day0, 10.0, 8.0, 12.0, 40.0);
        let (hlc3_c, hlc3_u, hlc3_l) = feed(&mut hlc3, day0, 10.0, 8.0, 12.0, 40.0);
        assert!((hl2_c - 9.0).abs() < 1e-12, "hl2 {hl2_c}");
        assert!((hlc3_c - 10.0).abs() < 1e-12, "hlc3 {hlc3_c}");
        assert!((hlc3_u - 10.0).abs() < 1e-12 && (hlc3_l - 10.0).abs() < 1e-12);
        assert!((hlc3_c - hl2_c).abs() > 0.5);

        // `ContractFactory` is the whole catalog. Debug `build_solo` / `feed`
        // frames overflow the 2 MiB test thread, so the `+time` check runs on
        // a larger stack. The paper numbers above stay on this thread.
        std::thread::Builder::new()
            .name("session-vwap-bands-factory".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(factory_sees_utc_day)
            .expect("factory thread")
            .join()
            .expect("factory thread");

        println!(
            "asserted day0 vwap=10.5 dev={dev} upper={upper} lower={lower}; continued vwap=10.4 dev=0.8 upper=12 lower=8.8; day1 center=18; hlc3=10 hl2=9"
        );
    }

    fn factory_sees_utc_day() {
        let day0 = 1_728_000_000_000;
        let day1 = day0 + UTC_DAY_MS;
        let cfg = SessionVwapBandsConfig {
            source: Param::solo(SessionVwapBandsSource::Hl2),
            mult: Param::solo(2.0),
        };
        let mut factory = IndicatorOrder::SessionVwapBands(cfg)
            .build_solo()
            .expect("solo config builds");
        let mut check = SessionVwapBands::new(SessionVwapBandsSource::Hl2, 2.0);
        let series = [
            (day0, 10.0, 8.0, 9.0, 100.0),
            (day0 + 60_000, 12.0, 10.0, 11.0, 300.0),
            (day1, 20.0, 16.0, 19.0, 50.0),
        ];
        for (ts, high, low, close, volume) in series {
            factory.feed(
                ts,
                MarketSample::Bar {
                    open: 0.0,
                    high,
                    low,
                    close,
                    volume,
                },
            );
            let (center, upper_b, lower_b) = feed(&mut check, ts, high, low, close, volume);
            let got = factory.read(IndicatorOutputId::SessionVwapBandsCenter);
            assert!((got - center).abs() < 1e-12, "factory center {got} core {center}");
            assert!((factory.read(IndicatorOutputId::SessionVwapBandsUpper) - upper_b).abs() < 1e-12);
            assert!((factory.read(IndicatorOutputId::SessionVwapBandsLower) - lower_b).abs() < 1e-12);
        }
    }
}
