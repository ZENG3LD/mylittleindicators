//! SSL (Semaphore Signal Level) Channel.
//! Trend-following indicator: MAs of the bar highs and lows form dynamic
//! support/resistance levels that cross to signal trend changes.

use crate::engine::contract_engine::{SmootherChoice, SmootherSlot, SmootherId};

/// SSL Channel.
///
/// 1. `MA_High = MA(High, period)`, `MA_Low = MA(Low, period)`
/// 2. `SSL_Up = MA_High` if `Close > MA_High[1]` else `MA_Low`
/// 3. `SSL_Down = MA_Low` if `Close < MA_Low[1]` else `MA_High`
///
/// Bullish when `SSL_Up > SSL_Down`, bearish otherwise; a crossover flags a trend
/// change. ONE smoother choice, applied to both the high and the low series at the same
/// period (a single configurable slot, two instances).
#[derive(Clone, Debug)]
pub struct SslChannel {
    period: usize,

    ma_high: SmootherSlot,
    ma_low: SmootherSlot,

    ssl_up: f64,
    ssl_down: f64,

    prev_ssl_up: f64,
    prev_ssl_down: f64,
    prev_close: f64,

    trend_direction: i8, // 1 = bullish, -1 = bearish, 0 = undetermined

    count: usize,
    is_ready: bool,
}

impl SslChannel {
    /// Create an SSL Channel — both MAs are SMA (the classic SSL kernel) over high/low.
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for both the high and low MAs.
    /// Legacy bridge; the contract path goes through `SslConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            ma_high: SmootherSlot::new(smoother, p),
            ma_low: SmootherSlot::new(smoother, p),
            ssl_up: 0.0,
            ssl_down: 0.0,
            prev_ssl_up: 0.0,
            prev_ssl_down: 0.0,
            prev_close: 0.0,
            trend_direction: 0,
            count: 0,
            is_ready: false,
        }
    }

    /// Create an SSL Channel with the standard period (10).
    pub fn default_instance() -> Self {
        Self::new(10)
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). Returns `(SSL_Up, SSL_Down)`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let ma_high_val = self.ma_high.feed(high);
        let ma_low_val = self.ma_low.feed(low);

        if self.count > 0 {
            self.prev_ssl_up = self.ssl_up;
            self.prev_ssl_down = self.ssl_down;

            if close > self.ma_high.value() {
                self.ssl_up = ma_high_val;
            } else {
                self.ssl_up = ma_low_val;
            }

            if close < self.ma_low.value() {
                self.ssl_down = ma_low_val;
            } else {
                self.ssl_down = ma_high_val;
            }

            if self.ssl_up > self.ssl_down {
                self.trend_direction = 1;
            } else if self.ssl_up < self.ssl_down {
                self.trend_direction = -1;
            }

            if self.count >= self.period {
                self.is_ready = true;
            }
        } else {
            self.ssl_up = ma_high_val;
            self.ssl_down = ma_low_val;
        }

        self.prev_close = close;
        self.count += 1;

        (self.ssl_up, self.ssl_down)
    }


    pub fn ssl_up(&self) -> f64 {
        self.ssl_up
    }

    pub fn ssl_down(&self) -> f64 {
        self.ssl_down
    }

    /// Brace-named getter for the `up` output (delegates to `ssl_up()`).
    #[inline]
    pub fn up(&self) -> f64 {
        self.ssl_up
    }

    /// Brace-named getter for the `down` output (delegates to `ssl_down()`).
    #[inline]
    pub fn down(&self) -> f64 {
        self.ssl_down
    }

    /// Trend direction: 1 = bullish, -1 = bearish, 0 = undetermined.
    pub fn trend_direction(&self) -> i8 {
        self.trend_direction
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn period(&self) -> usize {
        self.period
    }

    /// Reset the indicator — resets the slots in place (never rebuilds them, which would
    /// drop a configured smoother's shape).
    pub fn reset(&mut self) {
        self.ma_high.reset();
        self.ma_low.reset();
        self.ssl_up = 0.0;
        self.ssl_down = 0.0;
        self.prev_ssl_up = 0.0;
        self.prev_ssl_down = 0.0;
        self.prev_close = 0.0;
        self.trend_direction = 0;
        self.count = 0;
        self.is_ready = false;
    }

    /// Trade signal on a trend flip: 1 = buy, -1 = sell, 0 = none.
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        if self.trend_direction == 1 && self.prev_ssl_up <= self.prev_ssl_down {
            return 1;
        }
        if self.trend_direction == -1 && self.prev_ssl_up >= self.prev_ssl_down {
            return -1;
        }
        0
    }

    /// SSL line crossover: 1 = up cross, -1 = down cross, 0 = none.
    pub fn crossover(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        if self.prev_ssl_up <= self.prev_ssl_down && self.ssl_up > self.ssl_down {
            return 1;
        }
        if self.prev_ssl_up >= self.prev_ssl_down && self.ssl_up < self.ssl_down {
            return -1;
        }
        0
    }

    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        match self.trend_direction {
            1 => "Bullish Trend",
            -1 => "Bearish Trend",
            _ => "Sideways",
        }
    }

    /// Trend strength in [0, 1] from the relative SSL line separation.
    pub fn trend_strength(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        let ssl_diff = (self.ssl_up - self.ssl_down).abs();
        let ssl_avg = (self.ssl_up + self.ssl_down) / 2.0;
        if ssl_avg.abs() < 1e-12 {
            return 0.0;
        }
        ((ssl_diff / ssl_avg) * 10.0).min(1.0)
    }

    /// The active support (bull) / resistance (bear) level.
    pub fn support_resistance_level(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        match self.trend_direction {
            1 => self.ssl_down,
            -1 => self.ssl_up,
            _ => (self.ssl_up + self.ssl_down) / 2.0,
        }
    }

    /// Breakout vs the active level: 1 = up, -1 = down, 0 = none.
    pub fn breakout_signal(&self, close: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }
        let support_resistance = self.support_resistance_level();
        let threshold = support_resistance * 0.001;
        match self.trend_direction {
            1 => if close < support_resistance - threshold { -1 } else { 0 },
            -1 => if close > support_resistance + threshold { 1 } else { 0 },
            _ => 0,
        }
    }

    pub fn info(&self) -> String {
        format!(
            "SSL: Up={:.3}, Down={:.3}, Trend: {}, Strength: {:.3}",
            self.ssl_up,
            self.ssl_down,
            self.market_condition(),
            self.trend_strength()
        )
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Typed contract config for [`SslChannel`]. ONE smoother choice (`ma`) is applied to
/// BOTH the high and low series at the same period — a single configurable slot, two
/// instances. Reads the raw high/low/close slices, so there is no configurable source.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct SslConfig {
    pub period: Param<usize>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl SslChannel {
    /// Build from a smoother CHOICE (kind + follow/own period) at the given host period.
    pub fn from_choice(choice: SmootherChoice, host_period: usize) -> Self {
        let p = choice.period.resolve(host_period).max(1);
        Self {
            period: p,
            ma_high: SmootherSlot::new(choice.id(), p),
            ma_low: SmootherSlot::new(choice.id(), p),
            ssl_up: 0.0,
            ssl_down: 0.0,
            prev_ssl_up: 0.0,
            prev_ssl_down: 0.0,
            prev_close: 0.0,
            trend_direction: 0,
            count: 0,
            is_ready: false,
        }
    }
}

impl Indicator for SslChannel {
    const ID: IndicatorId = IndicatorId::Ssl;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads the raw high/low (MAs) + close (level select) — fixed slices, not a field.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1): the up/down level selection over two smoothers; both smoother buffers land
    /// recursively through the single `SLOTS` choice (realized into high + low MAs).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = SslConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::SslUp),
        Output::price(IndicatorOutputId::SslDown),
    ];
    type Config = SslConfig;
    type Runtime = SslChannel;

    fn create(cfg: SslConfig) -> SslChannel {
        SslChannel::from_choice(cfg.ma.resolved(), cfg.period.resolved())
    }

    fn slot_members(cfg: &SslConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for SslConfig {
    fn defaults() -> Self {
        SslConfig {
            period: Param::Solo(10),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1).
        // ma: #[slot] SmootherChoice — left Solo (deferred wave). [FLAG: slot]
        Self::machine_defaults_auto()
    }
}


impl Render for SslChannel {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::SslUp, "SSL Up", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::SslDown, "SSL Down", Color::hex(0xF44336), 2.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_channel_new() {
        let ssl = SslChannel::new(10);
        assert_eq!(ssl.period(), 10);
        assert!(!ssl.is_ready());
        assert_eq!(ssl.trend_direction(), 0);
    }

    #[test]
    fn test_ssl_channel_default() {
        let ssl = SslChannel::default_instance();
        assert_eq!(ssl.period(), 10);
    }

    #[test]
    fn test_ssl_channel_calculation() {
        let mut ssl = SslChannel::new(5);
        let test_data = vec![
            (100.0, 102.0, 98.0, 101.0),
            (101.0, 103.0, 99.0, 102.0),
            (102.0, 104.0, 100.0, 103.0),
            (103.0, 105.0, 101.0, 104.0),
            (104.0, 106.0, 102.0, 105.0),
            (105.0, 107.0, 103.0, 106.0),
            (106.0, 108.0, 104.0, 107.0),
            (107.0, 109.0, 105.0, 108.0),
        ];
        for (_open, high, low, close) in test_data {
            ssl.feed(&[high, low, close]);
        }
        assert!(ssl.is_ready());
        assert!(ssl.ssl_up() > 0.0);
        assert!(ssl.ssl_down() > 0.0);
        assert_ne!(ssl.trend_direction(), 0);
    }

    #[test]
    fn test_ssl_channel_reset() {
        let mut ssl = SslChannel::new(10);
        ssl.feed(&[102.0, 98.0, 101.0]);
        ssl.feed(&[103.0, 99.0, 102.0]);
        ssl.reset();
        assert!(!ssl.is_ready());
        assert_eq!(ssl.trend_direction(), 0);
        assert_eq!(ssl.ssl_up(), 0.0);
        assert_eq!(ssl.ssl_down(), 0.0);
    }

    #[test]
    fn test_market_conditions() {
        let mut ssl = SslChannel::new(5);
        ssl.is_ready = true;
        ssl.trend_direction = 1;
        assert_eq!(ssl.market_condition(), "Bullish Trend");
        ssl.trend_direction = -1;
        assert_eq!(ssl.market_condition(), "Bearish Trend");
        ssl.trend_direction = 0;
        assert_eq!(ssl.market_condition(), "Sideways");
    }

    #[test]
    fn test_ssl_channel_smoother_choice() {
        // The smoother is a free slot choice — SMA vs EMA produce different levels.
        let mut ssl_sma = SslChannel::from_smoother(5, SmootherId::Sma);
        let mut ssl_ema = SslChannel::from_smoother(5, SmootherId::Ema);
        let test_data = vec![
            (100.0, 102.0, 98.0, 101.0),
            (101.0, 103.0, 99.0, 102.0),
            (102.0, 104.0, 100.0, 103.0),
            (103.0, 105.0, 101.0, 104.0),
            (104.0, 106.0, 102.0, 105.0),
            (105.0, 107.0, 103.0, 106.0),
        ];
        for (_open, high, low, close) in test_data {
            ssl_sma.feed(&[high, low, close]);
            ssl_ema.feed(&[high, low, close]);
        }
        assert!(ssl_sma.is_ready());
        assert!(ssl_ema.is_ready());
        assert_ne!(ssl_sma.ssl_up(), ssl_ema.ssl_up());
    }

    #[test]
    fn test_ssl_channel_contract_create() {
        let cfg = <<SslChannel as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut ssl = <SslChannel as Indicator>::create(cfg);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ssl.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ssl.is_ready());
    }
}
