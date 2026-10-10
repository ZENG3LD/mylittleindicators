//! GPU bottom of the indicator contract — sibling of [`super::Indicator`]
//! (compute) and [`super::Render`] (draw).
//!
//! `+cube(formula)` in the manifest is the cube registry. One kernel in
//! `contract::kernels` (feature `gpu`) runs every formula. An indicator does
//! not implement [`GpuCube`] to join that table.
//!
//! [`GpuShader`] is the escape hatch for a formula the cube subset cannot
//! express. No catalog member uses it. Feature `gpu-shader` is the wgpu
//! submitter for that text.

/// How an indicator may leave the CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMode {
    /// CPU `feed`. Not a kernel and not a shader.
    None,
    /// CubeCL kernel. The launch is behind feature `gpu`.
    Cube,
    /// WGSL. The source is static text; the submitter is not linked.
    Shader,
}

/// Closed set behind `+cube(name)`. The manifest name is the snake_case of
/// the variant (`window_mean`). One kernel switches on [`CubeFormula::code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CubeFormula {
    /// `output[i]` is the selected OHLCV lane. Period is ignored.
    Identity = 0,
    /// Mean of the last `period` samples, or of the prefix while the window fills.
    WindowMean = 1,
    /// Max of that same window.
    WindowMax = 2,
    /// Min of that same window.
    WindowMin = 3,
    /// Linear weights, newest = `period`. Until the window is full the sample
    /// itself is the value, matching `Wma::feed`.
    WindowWeighted = 4,
    /// `Ema::feed`. Seed is the first sample. Alpha is `2 / (period + 1)`.
    Ema = 5,
    /// `Rma::feed`. Seed is the first sample. Then `(prev * (n - 1) + x) / n`.
    Rma = 6,
    /// `Dema::feed`. Two cascaded EMAs: `2 e1 - e2`.
    Dema = 7,
    /// `Tema::feed`. Three cascaded EMAs: `3 e1 - 3 e2 + e3`.
    Tema = 8,
    /// `Tma::feed` and `Trima::feed`. SMA of an SMA, both of length `period`.
    Tma = 9,
    /// `Hma::feed`. WMA of `2 WMA(n/2) - WMA(n)` with length `floor(sqrt(n))`.
    Hma = 10,
    /// `Alma::with_params`. Offset is [`CubeParams::a`], sigma is [`CubeParams::b`].
    /// Zero until the window is full.
    Alma = 11,
    /// `T3::with_alpha`. Volume factor is [`CubeParams::a`].
    T3 = 12,
    /// `McGinleyDynamic::feed`.
    Mcginley = 13,
    /// `Roc`. Zero until `period` samples. `flag == 0` is `(x - x_lag) / x_lag`.
    /// `flag == 1` is `log10(x / x_lag)`.
    Roc = 14,
    /// `Rsi::new`. Wilder RMA of gains and losses. Stays `0` until that RMA is ready.
    Rsi = 15,
    /// `Cmo::new`. Wilder RMA of gains and losses.
    Cmo = 16,
    /// `Bias::new`. `x / SMA(x) - 1` once the SMA is ready, otherwise `0`.
    Bias = 17,
    /// `TrueRange::feed` on high, low, close. First bar is `|high - low|`.
    TrueRange = 18,
    /// `Atr::new_wilder`. RMA of true range. First bar is `high - low`.
    Atr = 19,
    /// `Bop::feed`. `(close - open) / max(|high - low|, 1e-12)`.
    Bop = 20,
    /// `Vwma::feed`. `sum(lane * lane2) / sum(lane2)` over `period`.
    /// A non-positive weight sum holds the previous value.
    Vwma = 21,
    /// MACD line: EMA(`fast`) of `lane` minus EMA(`slow`) of `lane2`.
    /// `signal` is an input and does not enter this line.
    Macd = 22,
    /// APO: EMA(`fast`) of `lane` minus EMA(`slow`) of the same lane.
    Apo = 23,
    /// Top-of-book microprice. Holds the previous value when a side is missing
    /// or the top sizes sum to zero.
    Microprice = 24,
    /// `(bid_depth - ask_depth) / (bid_depth + ask_depth)` over [`CubeParams::levels`].
    /// Zero when the depth sum is zero. Shared by book imbalance and bid/ask asymmetry.
    BookImbalance = 25,
    /// Bid depth slope minus ask depth slope. `period` is the snapshot window,
    /// [`CubeParams::levels`] is the depth.
    BookPressure = 26,
    /// `output[i] = s0[i]`. Primary number of a non-bar sample.
    Scalar = 27,
    /// `(latest - oldest) / (n - 1)` over the last `period` values of
    /// [`CubeParams::slot`]. Zero until two samples. `period` below 2 is 2.
    EndpointSlope = 28,
    /// Population z-score of that same window: `(x - mean) / std`, `std`
    /// divides by `n`. Zero when fewer than two samples or `std` is zero.
    PopZScore = 29,
    /// `output[i] = s0[i] * a`. Stateless scale of the primary number.
    Scale = 30,
    /// Population standard deviation of the [`CubeParams::slot`] window.
    /// Zero until two samples. `period` below 2 is 2.
    PopStd = 31,
    /// Share of the previous window strictly below the current sample.
    /// The current sample is not in that window. Zero on the first sample.
    PercentileRank = 32,
    /// Current slot value over the mean of the window that includes it.
    /// `1` when that mean is zero.
    RatioToMean = 33,
    /// One-step change of an EMA. Alpha is `2 / (period + 1)`. The first
    /// sample is zero. `period` below 2 is 2.
    EmaStep = 34,
    /// Mean of the bid and ask OLS slopes. Cumulative size against absolute
    /// distance from mid, over [`CubeParams::levels`] (at least 2). Holds the
    /// previous value when either side is missing.
    BookSlope = 35,
    /// Williams %R. `-50` until `period` bars, and when the high-low range is ~0.
    WilliamsR = 36,
    /// On-balance volume. First sample is 0. Price lane up adds volume, down subtracts it.
    Obv = 37,
    /// Price-volume trend. First sample is 0. Adds `(x - prev) / prev * volume`.
    Pvt = 38,
    /// Money flow index. `50` until `period` bars. First typical-price step is positive.
    Mfi = 39,
    /// Accumulation/distribution line. Multiplier is 0 when the high-low range is ~0.
    AdLine = 40,
    /// DeMarker. First sample is 0. Wilder sums of up-high and down-low, then `up / (up + down)`.
    Demarker = 41,
    /// Ulcer index of [`CubeParams`] lane. Zero until two windows of drawdown squares exist.
    Ulcer = 42,
    /// Close-to-close realized vol of the lane. RMS of squared log returns.
    /// [`CubeParams::a`] > 0 multiplies the result. The first sample is 0.
    RealizedVol = 43,
    /// Kaufman efficiency ratio of the lane. Zero until the window is full. Clamped to `[0, 1]`.
    Efficiency = 44,
    /// Gopalakrishnan range index of this bar: `ln(max(high-low, 1e-9)) / ln(window)`.
    /// `window` below 2 is 2.
    Gapo = 45,
    /// Williams VIX fix. Zero until `lookback` bars. `(highest close - low) / highest close * 100`.
    Wvf = 46,
    /// Mean of fourth-power log returns, times `1e6`. First sample is 0.
    Quarticity = 47,
    /// Population std of close-to-close log returns, times `sqrt(252)`.
    /// Zero until `window` returns. `window` is clamped to 5..=1024.
    HvC2c = 48,
    /// Psychological line of the lane. Zero until `period` samples. Up-bar share times 100.
    Psl = 49,
    /// Intraday momentum index. `100 * up / (up + down)` over the last `period` bars.
    Imi = 50,
    /// Price zone oscillator. `100 * sum(diff) / sum(|diff|)` over the last `period` closes.
    /// The first diff is 0. `period` is clamped to 2..=1024.
    Pzo = 51,
    /// Ehlers center of gravity of the lane. Zero until the window is full.
    /// `period` is clamped to 2..=512.
    Cog = 52,
    /// Bipower variance of the lane. First sample is 0. Scaled by `(pi/2) * 252 * 10000`.
    /// Products follow the feed's ring slots, not time order.
    Bipower = 53,
    /// Vertical horizontal filter of the lane. Zero until `period` samples.
    Vhf = 54,
    /// Polarized fractal efficiency of the lane, in `[-100, 100]`. Zero until the window
    /// is full. `period` is clamped to 5..=1024.
    Pfe = 55,
    /// Population z-score of volume. Zero until two samples.
    VolumeZ = 56,
    /// Z-score of `close - close[period]`. The z window is [`CubeParams::fast`] (at least 2).
    /// Zero until two such diffs exist.
    MomZ = 57,
    /// Bollinger %B with an SMA center. `0.5` until `period` samples.
    /// [`CubeParams::a`] is the standard-deviation multiple (at least 0.1).
    PercentB = 58,
    /// Chande forecast oscillator. `100 * (price - regression endpoint) / price`.
    /// Zero until `period` samples. `period` below 2 is 2.
    Cfo = 59,
    /// Rolling mean of `(high + low) / 2`. Partial window from the first bar.
    Rmid = 60,
    /// Williams accumulation/distribution. First sample is 0, then a running sum.
    Wad = 61,
    /// Median-absolute-deviation z-score of the lane. Zero until the window is full.
    /// `period` below 3 is 3. Scale is `1.4826`.
    MadZ = 62,
    /// Chaikin money flow. Zero until `period` bars, then rolling money-flow volume over volume.
    Cmf = 63,
    /// Rolling VWAP of typical price. Partial from the first bar.
    /// Holds the previous value when window volume is not positive.
    Vwap = 64,
    /// Jurik RSX in `[0, 1]`. First sample is 0. Wilder gain/loss, then three
    /// EMA passes at alpha `0.0625`.
    Rsx = 65,
    /// Accumulative swing index. First sample is 0. Limit move is `close * 0.03`.
    Asi = 66,
    /// Historical VaR of the lane, as a positive loss. Zero until `period` log returns.
    /// `period` below 2 is 2. [`CubeParams::a`] is the confidence, clamped to `[0.5, 0.9999]`.
    Var = 67,
    /// Choppiness index. Stays at 50 until `period` true ranges exist.
    /// Then `100 * log10(sum(TR) / range) / log10(period)`, clamped to `[0, 100]`.
    /// Holds the previous value when the range or the sum is ~0.
    Chop = 68,
    /// Awesome oscillator. SMA(5) minus SMA(34) of `(high + low) / 2`, partial from bar 0.
    Ao = 69,
    /// DPO divided by the current lane. Zero until `period + period/2 + 1` samples.
    /// `period` below 2 is 2.
    DpoPct = 70,
    /// Envelope bandwidth. Zero until the SMA is full, then `2 * pct / 100`.
    /// [`CubeParams::a`] is the percent, at least `0.01`.
    Envbw = 71,
    /// Acceleration/deceleration. Awesome oscillator minus its SMA(5), partial from bar 0.
    Ac = 72,
    /// Williams market facilitation index. `(high - low) / volume`, or 0 when volume is not positive.
    WilliamsMfi = 73,
    /// Volume flow indicator. Wilder-style decayed sums of typical-price flow and volume, `sum_flow / sum_vol`. Zero when the volume sum is ~0.
    Vfi = 74,
    /// Volume zone oscillator. `100 * (up_vol - down_vol) / max(|sum|, 1e-9)` over the last `period` bars; the first bar counts as up. `period` is clamped to 2..=1024.
    Vzo = 75,
    /// Intraday intensity percent of this bar: `((2c - h - l) / max(|h - l|, 1e-9)) * volume`.
    IntradayPct = 76,
    /// Intraday intensity ratio. Seed-0 EMA (alpha `1 / period`) of the intraday intensity, clamped to +-1e9.
    IntradayRatio = 77,
    /// Donchian position of the close in the high/low channel. `0.5` until `period` bars (at least 2) or when the width is 0.
    DonchianPos = 78,
    /// Donchian width, `highest high - lowest low`. Zero until `period` bars (at least 2).
    DonchianWidth = 79,
    /// Price channel oscillator `2 * pos - 1`. `pos` is the close inside the partial-window high/low channel (`period` clamped to 2..=512). `0.5` until the window is full or when the channel is flat.
    PriceChannelOsc = 80,
    /// Price channel width: partial-window highest high minus lowest low (`period` clamped to 2..=512).
    PriceChannelWidth = 81,
    /// Efficiency ratio over the whole history: `|x - x0| / sum|dx|`. Zero on the first sample and when the path length is 0. Period is ignored.
    ErFull = 82,
    /// Efficiency ratio over a ring window of `period` samples (at least 2), partial from bar 0. Zero on the first sample.
    ErRing = 83,
    /// R-squared of the lane against the ring-slot index (as `RSquared::feed`, slots follow the ring, not time). Zero until `period` samples. `period` clamped to 5..=1024.
    RSquared = 84,
    /// `(close - rolling VWAP) / VWAP` with the same rolling VWAP as [`CubeFormula::Vwap`]. Zero when the VWAP is ~0.
    VwapDistance = 85,
    /// Ehlers cyber cycle of the lane. [`CubeParams::a`] is alpha, clamped to `[0, 1]`. Previous inputs start at 0.
    CyberCycle = 86,
    /// `Ama::feed`: ER over a ring window of `period` (at least 2), smoothing constant `(er * (fast_alpha - slow_alpha) + slow_alpha)^2`. [`CubeParams::fast`] and [`CubeParams::slow`] are the fast and slow periods. Seed is the first sample.
    Ama = 87,
    /// `VolatilityBreakExp`: 1 when `|x - ema| > threshold * max(|x - prev_ema|, 1e-9)`. [`CubeParams::a`] is alpha (0.01..=1), [`CubeParams::b`] the sigma threshold (at least 0.5). Zero on the first sample.
    VolBreak = 88,
    /// Autocorrelation of log returns of the lane. `period` is the window (at least 2), [`CubeParams::fast`] the lag (at least 1). Holds 0 until `period + lag + 1` samples.
    Autocorr = 89,
    /// Variance ratio of log returns. `period` is the window (at least 20), [`CubeParams::fast`] the aggregation (2..=window/2). `1` until a window of returns exists.
    VarianceRatio = 90,
}

/// Scalar axes a cube launch reads beside the OHLCV columns.
///
/// `lane` / `lane2` are [`crate::engine::ohlcv_field::OhlcvField`] codes.
/// `a` is the ALMA offset or the T3 volume factor. `b` is the ALMA sigma.
/// `flag == 1` selects log ROC. `signal` is the MACD signal period; the
/// kernel's one output for MACD is still the line. `slot` selects `s0`..`s3`
/// for the scalar-window formulas (`0` is `s0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubeParams {
    pub lane: crate::engine::ohlcv_field::OhlcvField,
    pub lane2: crate::engine::ohlcv_field::OhlcvField,
    pub period: u32,
    pub fast: u32,
    pub slow: u32,
    pub signal: u32,
    pub a: f32,
    pub b: f32,
    pub flag: u32,
    /// Book levels to sum. `1` is top of book.
    pub levels: u32,
    /// Scalar slot for window formulas. `0` is `s0`, `3` is `s3`.
    pub slot: u32,
}

impl CubeParams {
    /// Close on both lanes, ROC ratio, ALMA offset `0.85` / sigma `6`, signal `9`.
    /// `fast` and `slow` start equal to `period`. T3 callers set `a` to the volume factor.
    pub fn period(period: u32) -> Self {
        Self {
            lane: crate::engine::ohlcv_field::OhlcvField::Close,
            lane2: crate::engine::ohlcv_field::OhlcvField::Close,
            period,
            fast: period,
            slow: period,
            signal: 9,
            a: 0.85,
            b: 6.0,
            flag: 0,
            levels: 1,
            slot: 0,
        }
    }
}

impl CubeFormula {
    /// Discriminant the kernel compares against.
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// A hand-written `#[cube]` kernel that is not one of the shared formulas.
/// The catalog does not consult this trait. `+cube(formula)` is the registry.
pub trait GpuCube: super::Indicator {}

/// Compile `source` as a wgpu shader module. The device is the caller's.
#[cfg(feature = "gpu-shader")]
pub fn create_shader_module<'a>(
    device: &wgpu::Device,
    source: &'a str,
    entry: &str,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
    })
}

/// Hand-written WGSL. Use this only when the update is not a [`CubeFormula`].
/// The submitter is not linked in the default build.
pub trait GpuShader: super::Indicator {
    /// Shader source. A file becomes this `&'static str` via `include_str!`.
    fn shader_source() -> &'static str;

    /// Entry-point name inside [`Self::shader_source`].
    fn shader_entry() -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::{formula_of, gpu_of};
    use crate::engine::indicator_id::IndicatorId;

    #[test]
    fn catalog_names_the_written_paths() {
        assert_eq!(gpu_of(IndicatorId::Volume), GpuMode::Cube);
        assert_eq!(formula_of(IndicatorId::Volume), Some(CubeFormula::Identity));
        assert_eq!(gpu_of(IndicatorId::Sma), GpuMode::Cube);
        assert_eq!(formula_of(IndicatorId::Sma), Some(CubeFormula::WindowMean));
        assert_eq!(formula_of(IndicatorId::Wma), Some(CubeFormula::WindowWeighted));
        assert_eq!(formula_of(IndicatorId::Highest), Some(CubeFormula::WindowMax));
        assert_eq!(formula_of(IndicatorId::Lowest), Some(CubeFormula::WindowMin));
        assert_eq!(gpu_of(IndicatorId::Rsi), GpuMode::Cube);
        assert_eq!(formula_of(IndicatorId::Rsi), Some(CubeFormula::Rsi));
        assert_eq!(gpu_of(IndicatorId::Ema), GpuMode::Cube);
        assert_eq!(formula_of(IndicatorId::Ema), Some(CubeFormula::Ema));
        assert_eq!(formula_of(IndicatorId::Macd), Some(CubeFormula::Macd));
        assert_eq!(formula_of(IndicatorId::Apo), Some(CubeFormula::Apo));
        assert_eq!(formula_of(IndicatorId::Atr), Some(CubeFormula::Atr));
        assert_eq!(formula_of(IndicatorId::Tr), Some(CubeFormula::TrueRange));
        assert_eq!(formula_of(IndicatorId::Bop), Some(CubeFormula::Bop));
        assert_eq!(formula_of(IndicatorId::Vwma), Some(CubeFormula::Vwma));
        assert_eq!(formula_of(IndicatorId::BookMicroprice), Some(CubeFormula::Microprice));
        assert_eq!(formula_of(IndicatorId::BookImb), Some(CubeFormula::BookImbalance));
        assert_eq!(formula_of(IndicatorId::BidAskAsymmetry), Some(CubeFormula::BookImbalance));
        assert_eq!(formula_of(IndicatorId::BookPressure), Some(CubeFormula::BookPressure));
        assert_eq!(formula_of(IndicatorId::OiMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::MarkPriceMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::BasisMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::SettledFundingMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::HvMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::VolIdxMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::Volume24hMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::DeltaExposureFlow), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::VegaExposureFlow), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::FundDepletionRate), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::SettlementPriceMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::OiZScore), Some(CubeFormula::PopZScore));
        assert_eq!(formula_of(IndicatorId::FundingZScore), Some(CubeFormula::PopZScore));
        assert_eq!(formula_of(IndicatorId::BasisZScore), Some(CubeFormula::PopZScore));
        assert_eq!(formula_of(IndicatorId::Volume24hZScore), Some(CubeFormula::PopZScore));
        assert_eq!(formula_of(IndicatorId::AnnualizedFundingRate), Some(CubeFormula::Scale));
        assert_eq!(formula_of(IndicatorId::LongShortRatioMomentum), Some(CubeFormula::EndpointSlope));
        assert_eq!(formula_of(IndicatorId::MarkPriceVolatility), Some(CubeFormula::PopStd));
        assert_eq!(formula_of(IndicatorId::OiPercentile), Some(CubeFormula::PercentileRank));
        assert_eq!(formula_of(IndicatorId::AuctionImbalance), Some(CubeFormula::RatioToMean));
        assert_eq!(formula_of(IndicatorId::InsuranceFundMomentum), Some(CubeFormula::EmaStep));
        assert_eq!(formula_of(IndicatorId::BookSlope), Some(CubeFormula::BookSlope));
        assert_eq!(formula_of(IndicatorId::WilliamsR), Some(CubeFormula::WilliamsR));
        assert_eq!(formula_of(IndicatorId::Obv), Some(CubeFormula::Obv));
        assert_eq!(formula_of(IndicatorId::Pvt), Some(CubeFormula::Pvt));
        assert_eq!(formula_of(IndicatorId::Mfi), Some(CubeFormula::Mfi));
        assert_eq!(formula_of(IndicatorId::Ad), Some(CubeFormula::AdLine));
        assert_eq!(formula_of(IndicatorId::Demarker), Some(CubeFormula::Demarker));
        assert_eq!(formula_of(IndicatorId::Ui), Some(CubeFormula::Ulcer));
        assert_eq!(formula_of(IndicatorId::Rv), Some(CubeFormula::RealizedVol));
        assert_eq!(formula_of(IndicatorId::TrEr), Some(CubeFormula::Efficiency));
        assert_eq!(formula_of(IndicatorId::Gapo), Some(CubeFormula::Gapo));
        assert_eq!(formula_of(IndicatorId::Wvf), Some(CubeFormula::Wvf));
        assert_eq!(formula_of(IndicatorId::Rq), Some(CubeFormula::Quarticity));
        assert_eq!(formula_of(IndicatorId::Hvc2c), Some(CubeFormula::HvC2c));
        assert_eq!(formula_of(IndicatorId::Psl), Some(CubeFormula::Psl));
        assert_eq!(formula_of(IndicatorId::Imi), Some(CubeFormula::Imi));
        assert_eq!(formula_of(IndicatorId::Pzo), Some(CubeFormula::Pzo));
        assert_eq!(formula_of(IndicatorId::Cog), Some(CubeFormula::Cog));
        assert_eq!(formula_of(IndicatorId::Bpv), Some(CubeFormula::Bipower));
        assert_eq!(formula_of(IndicatorId::Vhf), Some(CubeFormula::Vhf));
        assert_eq!(formula_of(IndicatorId::Pfe), Some(CubeFormula::Pfe));
        assert_eq!(formula_of(IndicatorId::Vz), Some(CubeFormula::VolumeZ));
        assert_eq!(formula_of(IndicatorId::MomZscore), Some(CubeFormula::MomZ));
        assert_eq!(formula_of(IndicatorId::Percentb), Some(CubeFormula::PercentB));
        assert_eq!(formula_of(IndicatorId::Cfo), Some(CubeFormula::Cfo));
        assert_eq!(formula_of(IndicatorId::Rmid), Some(CubeFormula::Rmid));
        assert_eq!(formula_of(IndicatorId::Wad), Some(CubeFormula::Wad));
        assert_eq!(formula_of(IndicatorId::Zmad), Some(CubeFormula::MadZ));
        assert_eq!(formula_of(IndicatorId::Cmf), Some(CubeFormula::Cmf));
        assert_eq!(formula_of(IndicatorId::Vwap), Some(CubeFormula::Vwap));
        assert_eq!(formula_of(IndicatorId::Rsx), Some(CubeFormula::Rsx));
        assert_eq!(formula_of(IndicatorId::Asi), Some(CubeFormula::Asi));
        assert_eq!(formula_of(IndicatorId::Var), Some(CubeFormula::Var));
        assert_eq!(formula_of(IndicatorId::Chop), Some(CubeFormula::Chop));
        assert_eq!(formula_of(IndicatorId::Ao), Some(CubeFormula::Ao));
        assert_eq!(formula_of(IndicatorId::DpoPct), Some(CubeFormula::DpoPct));
        assert_eq!(formula_of(IndicatorId::Envbw), Some(CubeFormula::Envbw));
        assert_eq!(formula_of(IndicatorId::Ac), Some(CubeFormula::Ac));
        assert_eq!(formula_of(IndicatorId::WilliamsMfi), Some(CubeFormula::WilliamsMfi));
        assert_eq!(formula_of(IndicatorId::Vfi), Some(CubeFormula::Vfi));
        assert_eq!(formula_of(IndicatorId::Vzo), Some(CubeFormula::Vzo));
        assert_eq!(formula_of(IndicatorId::Iip), Some(CubeFormula::IntradayPct));
        assert_eq!(formula_of(IndicatorId::Iir), Some(CubeFormula::IntradayRatio));
        assert_eq!(formula_of(IndicatorId::Dcpos), Some(CubeFormula::DonchianPos));
        assert_eq!(formula_of(IndicatorId::Dcwidth), Some(CubeFormula::DonchianWidth));
        assert_eq!(formula_of(IndicatorId::Pchosc), Some(CubeFormula::PriceChannelOsc));
        assert_eq!(formula_of(IndicatorId::Pchwidth), Some(CubeFormula::PriceChannelWidth));
        assert_eq!(formula_of(IndicatorId::Er), Some(CubeFormula::ErFull));
        assert_eq!(formula_of(IndicatorId::ErRing), Some(CubeFormula::ErRing));
        assert_eq!(formula_of(IndicatorId::RSquared), Some(CubeFormula::RSquared));
        assert_eq!(formula_of(IndicatorId::VwapDist), Some(CubeFormula::VwapDistance));
        assert_eq!(formula_of(IndicatorId::Cyber), Some(CubeFormula::CyberCycle));
        assert_eq!(formula_of(IndicatorId::Ama), Some(CubeFormula::Ama));
        assert_eq!(formula_of(IndicatorId::Vbexp), Some(CubeFormula::VolBreak));
        assert_eq!(formula_of(IndicatorId::Autocorr), Some(CubeFormula::Autocorr));
        assert_eq!(formula_of(IndicatorId::Vr), Some(CubeFormula::VarianceRatio));
    }

    #[cfg(feature = "gpu-shader")]
    #[test]
    fn shader_hatch_parses() {
        let src = "@compute @workgroup_size(1)\nfn hatch() {}\n";
        naga::front::wgsl::parse_str(src).expect("shader hatch");
    }
}
