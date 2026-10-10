//! GPU bottom of the indicator contract — sibling of [`super::Indicator`]
//! (compute) and [`super::Render`] (draw).
//!
//! `+cube(formula)` in the manifest is the cube registry. One kernel in
//! `contract::kernels` (feature `gpu`) runs every formula. An indicator does
//! not implement [`GpuCube`] to join that table.
//!
//! [`GpuShader`] is the escape hatch for a formula the cube subset cannot
//! express. `+shader` on a manifest row marks it and [`shader_of`] names its WGSL. Feature `gpu-shader` is the wgpu
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
    /// Donchian channel bands. Columns: upper, middle, lower. All 0 until `period` bars. Multi-column: use `launch_cube_columns`.
    DonchianBands = 100,
    /// Donchian metrics. Columns: width (`upper - lower`), position (`0.5` when the width is not positive).
    DonchianMetrics = 101,
    /// Aroon. Columns: up, down, oscillator. 0 until `period` bars. Ties pick the newest bar, as the feed.
    AroonCols = 102,
    /// Central pivot range of this bar. Columns: bc, pivot, tc.
    CentralPivotRange = 103,
    /// Heikin Ashi candle. Columns: open, high, low, close.
    HeikinAshiCols = 104,
    /// Candle anatomy. Columns: body, upper wick, lower wick, long upper (0/1), long lower (0/1). [`CubeParams::a`] is the long-wick ratio threshold. All 0 when the range is ~0.
    CandleAnatomyCols = 105,
    /// Qstick: smoother of `close - open`. Smoother and period come from [`CubeParams::smoother`] / [`CubeParams::smooth_period`]. Smoothed formulas run through `launch_cube_smoothed` (prep, smoother, combine).
    QstickSmoothed = 120,
    /// Force index: smoother of `volume * (close - prev_close)`. Bar 0 is 0 and is not fed to the smoother.
    ForceIndexSmoothed = 121,
    /// Coppock curve: smoother of `ROC(period) + ROC(fast)` (percent, 0 until the lag exists).
    CoppockSmoothed = 122,
    /// Volume oscillator: smoother(volume, `smooth_period`) minus smoother2(volume, `smooth_period2`).
    VolumeOscSmoothed = 123,
    /// Chaikin oscillator: smoother of the A/D line minus smoother2 of the same line.
    ChaikinOscSmoothed = 124,
    /// Intraday intensity: `100 * smoother(II) / smoother(volume)`, both with the first smoother. 0 when the volume mean is ~0.
    IntradayIntensitySmoothed = 125,
    /// Ease of movement: smoother of the raw EOM. [`CubeParams::a`] is the scale factor. Bar 0 is 0 and is not fed to the smoother.
    EaseOfMovementSmoothed = 126,
    /// Normalized ATR: `100 * smoother(true range) / |close|`. The first true range is `high - low`.
    NatrSmoothed = 127,
    /// Volume rate of change in percent: `(v - v[period]) / v[period] * 100`. Zero until `period` bars or when the old volume is ~0. The signal smoother of the core is not part of the output.
    Vroc = 91,
    /// Donchian breakout signal packed as f32: `+1` close above the upper band, `-1` below the lower band, else `0`. The band is 0 / 0 until `period` bars (at least 2), as the feed.
    DonchianBreakout = 92,
    /// Heikin Ashi trend packed as f32: `+1` HA close above HA open, `-1` below, else `0`.
    HeikinAshiTrend = 93,
    /// Hampel filter: the sample, or `median +- k * 1.4826 * MAD` when it lies more than `k` robust sigmas out. `period` is the window (3..=512), [`CubeParams::a`] is `k` (3 when not positive). Zero until the window is full. Upper median for both statistics, as the feed.
    Hampel = 94,
    /// Weekday effect: running mean log return of the current weekday bucket (`GpuTimes::weekday`). Calendar formula: use `launch_cube_timed`. Zero on the first bar.
    WeekdayEffect = 140,
    /// Session effect: running mean log return of the current session bucket (hour < 6 overnight, < 12 Asia, < 18 Europe, else US). Use `launch_cube_timed`.
    SessionEffect = 141,
    /// Month/quarter effect readout: mean of the non-zero monthly mean returns (the `month` output of the feed). Use `launch_cube_timed`.
    MonthEffect = 142,
    /// Day-of-month / week-of-quarter effect readout: mean of the non-zero day-of-month mean returns (the feed's `woq()` value). Use `launch_cube_timed`.
    DayOfMonthEffect = 143,
    /// Parabolic SAR of high and low. [`CubeParams::a`] is the start AF, [`CubeParams::b`] the AF step, [`CubeParams::c`] the AF cap. Output is the SAR level. Shared by `Psar` and `Psars` (`PSARStop` returns the SAR of its inner `ParabolicSAR`). UNTESTED on GPU.
    Psar = 200,
    /// Supertrend level with the default Wilder (RMA) ATR. `period` is the ATR period, [`CubeParams::a`] the multiplier. UNTESTED on GPU.
    Supertrend = 201,
    /// ADX with Wilder (RMA) ATR as in `Adx::feed`: bar 0 is 0, the ATR value (not the true range) feeds the DM sums, ADX stays 0 until `period` bars. UNTESTED on GPU.
    Adx = 202,
    /// ADX slope: `AdxSlope::feed`. `period` below 2 is 2. Holds the last slope until the ADX is ready (`count > 2 * period`). UNTESTED on GPU.
    AdxSlope = 203,
    /// `CusumBreakDetector`: [`CubeParams::a`] is the threshold, [`CubeParams::b`] the decay `kappa`. First bar is 0. UNTESTED on GPU.
    Cusum = 204,
    /// HAR-RV: `0.6 rv(period) + 0.3 rv(fast) + 0.1 rv(slow)` with the `RealizedVol` kernel arm. [`CubeParams::a`] is the annualize factor (multiplies when > 0). UNTESTED on GPU.
    Har = 205,
    /// Realized bipower jump test: `max(rv^2 - bv, 0) / (bv + 1e-9)` when `bv > 0`. `period` is the window (at least 2), [`CubeParams::a`] the annualize factor of `rv`. Uses the `Bipower` ring-slot products. UNTESTED on GPU.
    Rbvj = 206,
    /// Volatility of volatility, `AbsReturn` source only: population std of the last `period` (at least 2) `|ln(c/c_prev)|` values. Zero until two such values exist. The `Atr` source stays on the CPU. UNTESTED on GPU.
    VolOfVol = 207,
    /// Ehlers cyber cycle of `(high + low) / 2`: `EhlersCyberCycle`. [`CubeParams::a`] is alpha clamped to `[0.01, 0.99]`. Holds 0 until six bars. UNTESTED on GPU.
    EhlersCc = 208,
    /// Vortex indicator. Columns: VI+, VI-. Wilder ATR as in the feed (bar 0 returns `1, 1`); sums of `|high - low_prev|`, `|low - high_prev|` and ATR over the last `period` bars; holds the previous pair when the ATR sum is ~0. Multi-column. UNTESTED on GPU.
    Vortex = 300,
    /// `Dm::feed`. Columns: +DI, -DI, ADX. Window sums over the last `period` bars of `1..=i` (true range with the previous close); DI from the first `period - 1` entries, ADX is the mean of the last `period` DX values. UNTESTED on GPU.
    Dm = 301,
    /// `DiPlusMinus`: +DI, -DI of the Wilder-smoothed `Adx` state (0 until `period` bars). UNTESTED on GPU.
    DiPlusMinus = 302,
    /// `Rwi`. Columns: up (`high`), down (`low`). Wilder ATR over `period` (at least 2), `(high - low_prev)+ / (atr * sqrt(period))` and the mirror. UNTESTED on GPU.
    Rwi = 303,
    /// `HigherMoments`. Columns: skew, kurtosis of the last `period` (at least 3) log returns; 0 until `period + 1` closes. UNTESTED on GPU.
    HigherMoments = 304,
    /// `SwingAge`. Columns: bars since a new window high, bars since a new window low (as f32). Window is `period` (at least 2); holds 0 until it is full. UNTESTED on GPU.
    SwingAge = 305,
    /// EWMAC: smoother(lane, `smooth_period`) minus smoother2(lane, `smooth_period2`). Smoothed path (`launch_cube_smoothed_gx`). UNTESTED on GPU.
    Ewmac = 410,
    /// Gator oscillator: same math as [`CubeFormula::Ewmac`] (`fast - slow`); the slow period is at least 2. UNTESTED on GPU.
    Gator = 411,
    /// RAVI: `100 * |fast - slow| / slow`, 0 when `slow` is ~0; slow period at least 2. UNTESTED on GPU.
    Ravi = 412,
    /// Twiggs money flow: `smoother(mf * volume) / smoother(volume)` with the first smoother and `smooth_period` for both; 0 when the volume mean is ~0. UNTESTED on GPU.
    Tmf = 413,
    /// Volatility ratio: `smoother(TR, smooth_period2) / smoother(TR, smooth_period)` (slow ATR over fast ATR), 0 when the fast ATR is not positive. Both legs use the first smoother. UNTESTED on GPU.
    VolRatio = 414,
    /// Range over ATR: `max(high - low, 0) / smoother(TR, smooth_period)`, 0 when the ATR is ~0. UNTESTED on GPU.
    RangeAtr = 415,
    /// Keltner bandwidth: `(upper - lower) / |centre|` with centre = smoother(lane, `smooth_period`), ATR = smoother2(TR, `smooth_period`), `a` the multiplier; 0 until `smooth_period` bars are in. UNTESTED on GPU.
    KeltBw = 416,
    /// Keltner distance: `(close - centre) / ATR`, 0 while not ready or ATR is not positive. UNTESTED on GPU.
    KeltDist = 417,
    /// Keltner position: `(close - lower) / (upper - lower)`, 0.5 while not ready or the width is 0. UNTESTED on GPU.
    KeltPos = 418,
    /// ATR through the first smoother (`smooth_period`) of the Wilder true range. Used as an inner series. UNTESTED on GPU.
    AtrSm = 419,
    /// `max(high - low, 0)`. UNTESTED on GPU.
    HlRange = 420,
    /// `|ln(close / previous close)|`, 0 on the first bar. UNTESTED on GPU.
    AbsLogRet = 421,
    /// `(high + low) / 2`. UNTESTED on GPU.
    Hl2 = 422,
    /// A smoother (first smoother, `smooth_period`) of the lane. Used as an inner series. UNTESTED on GPU.
    SmoothLane = 423,
    /// Composite (post stage): share of the last `slow` |ATR| values `<=` the current one. Inner: [`CubeFormula::AtrSm`]. UNTESTED on GPU.
    AtrPct = 500,
    /// Composite: `p - ema(p)` of [`CubeFormula::AtrPct`]; `a` is alpha (clamped to 0.01..1). UNTESTED on GPU.
    AtrPctTrend = 501,
    /// Composite: population z-score of ATR over `slow` (at least 2) bars. UNTESTED on GPU.
    AtrZ = 502,
    /// Composite: share of the last `slow` |vol-of-vol| values `<=` the current one. Inner: [`CubeFormula::VolOfVol`]. UNTESTED on GPU.
    VovPct = 503,
    /// Composite: `p - ema(p)` of [`CubeFormula::VovPct`]. UNTESTED on GPU.
    VovPctTrend = 504,
    /// Composite: RSI(`period`) percentile rank 0..100 over `slow` (5..=1024) bars; 50 until the window is full and `period` bars passed. UNTESTED on GPU.
    RsiPctRank = 505,
    /// Rolling quartiles `[q1, q2, q3]` of the lane over `slow` bars: order statistics `len/4`, `len/2`, `3len/4` of the partial window. UNTESTED on GPU.
    RollQuart = 600,
    /// Percentile channels `[upper, middle, lower]` of the lane over `slow` bars; lower quantile `a`, upper `b`; order statistic `round(q * (len - 1))`. UNTESTED on GPU.
    PctChannels = 601,
    /// RSI percentile bands `[upper, middle, lower]`: RSI(`period`), window `slow` (10..=1024), 80th / 20th order statistics once full. UNTESTED on GPU.
    RsiPctBands = 602,
    /// Hour of day 0..=23. Calendar adapter (`launch_cube_timed`). UNTESTED on GPU.
    HourOfDay = 700,
    /// Week in month `((dom - 1) / 7) + 1`. UNTESTED on GPU.
    WeekInMonth = 701,
    /// Weekday occurrence within the month (host-computed). UNTESTED on GPU.
    WeekdayOccurrence = 702,
    /// Month turn: `1 - near / w` with `w = clamp(period, 1, 10)`. UNTESTED on GPU.
    MonthTurn = 703,
    /// Quarter turn: `1 - near / w` with `w = clamp(period, 1, 15)`. UNTESTED on GPU.
    QuarterTurn = 704,
    /// Weekend proximity with `w = clamp(period, 1, 5)`. UNTESTED on GPU.
    WeekendProx = 705,
    /// Start / end of month flags, `w = clamp(period, 1, 5)`. UNTESTED on GPU.
    StartEndMonth = 706,
    /// Start / end of quarter flags, `w = clamp(period, 1, 7)`. UNTESTED on GPU.
    StartEndQuarter = 707,
    /// Start / end of week flags, `w = clamp(period, 1, 3)`. UNTESTED on GPU.
    StartEndWeek = 708,
    /// Hour sin / cos encoding. UNTESTED on GPU.
    TimeEnc = 709,
    /// Direction of change of the lane: 1 / -1 / 0 (0 on the first bar). Signal path. UNTESTED on GPU.
    DirDetect = 800,
    /// Regime gate on the lane: threshold `a`, `flag` 0 above / 1 below; +1 entry, -1 exit. UNTESTED on GPU.
    RegimeGateSig = 801,
    /// Threshold edge on the lane: `a` upper, `b` lower, `flag` kind 0 above / 1 below / 2 in range / 3 out of range. UNTESTED on GPU.
    ThresholdEdge = 802,
    /// RSI(`period`) threshold gate: `a` upper (50..100), `b` lower (0..50), sticky. UNTESTED on GPU.
    ThresholdGateSig = 803,
    /// RSI(`period`) hysteresis gate with the same thresholds. UNTESTED on GPU.
    HysteresisGateSig = 804,
    /// Volume spike: volume lane above `a` times its mean over `period` bars. UNTESTED on GPU.
    VolEventSig = 805,
    /// Slope direction of a smoother of the lane (first smoother, `smooth_period`). UNTESTED on GPU.
    SlopeDirLine = 806,
    /// AND gate of two RSIs (`fast`, `slow` periods): 1 when both are beyond 70 or both below 30. UNTESTED on GPU.
    LogicAnd = 810,
    /// OR gate: either RSI outside 30..=70. UNTESTED on GPU.
    LogicOr = 811,
    /// XOR gate: exactly one RSI outside 30..=70. UNTESTED on GPU.
    LogicXor = 812,
    /// Sign combiner of the two RSI signals, clamped to -1..=1. UNTESTED on GPU.
    LogicSign = 813,
    /// Volatility regime transitions on the lane: thresholds `a` low, `b` high. UNTESTED on GPU.
    VolRegimeSig = 814,
    /// Relative position of two smoothers of the lane (first and second smoother). UNTESTED on GPU.
    RelPositionSig = 815,
    /// CUSUM event filter on the lane (`a` threshold): 1 / -1 / 0. UNTESTED on GPU.
    CusumFilter = 816,
    /// Williams fractals `[up, down]` as 1 / 0, from bar 4. UNTESTED on GPU.
    Fractals = 830,
    /// Fair value gap: 1 bull / -1 bear / 0, from bar 2. UNTESTED on GPU.
    FvgSig = 831,
    /// N-bar pivot of the lane: `fast` bars left, `slow` bars right; 1 high / -1 low. UNTESTED on GPU.
    NbarPivotSig = 832,
    /// Break of structure over `period` (at least 2) bars: 1 / -1 / 0. UNTESTED on GPU.
    BosSig = 833,
    /// Event frame: sign flip of the funding rate. UNTESTED on GPU.
    FundingDirShift = 900,
    /// Event frame: long ratio above `a` / below `b`. UNTESTED on GPU.
    LsExtreme = 901,
    /// Event frame: predicted rate beyond `+-a`. UNTESTED on GPU.
    PredFundingExtreme = 902,
    /// Event frame: max leverage down vs previous. UNTESTED on GPU.
    LeverageReduction = 903,
    /// Event frame: (high_24h - low_24h) / last. UNTESTED on GPU.
    HlRangeRatio = 904,
    /// Event frame: (ask - bid) / last. UNTESTED on GPU.
    TickerSpread = 905,
    /// Event frame: bid_iv - ask_iv. UNTESTED on GPU.
    IvSkewEv = 906,
    /// Event frame: (mmr + imr) / 2. UNTESTED on GPU.
    RiskProximity = 907,
    /// Event frame: mmr. UNTESTED on GPU.
    MmrTrack = 908,
    /// Event frame: sum of theta over `period` events. UNTESTED on GPU.
    ThetaDecay = 909,
    /// Event frame: z-score of the 24h percent change over `period` (at least 2). UNTESTED on GPU.
    PctChangeZ = 910,
    /// Event frame: HV above `a` times its mean over `period`. UNTESTED on GPU.
    HvSpikeEv = 911,
    /// Event frame: insurance balance slope below `-|a|` over `period`. UNTESTED on GPU.
    FundStress = 912,
    /// Event frame: |delta| within `b` of `a` and |theta| >= `c`. UNTESTED on GPU.
    PinRisk = 913,
    /// Event frame: (buys - sells) / count over `period` ticks. UNTESTED on GPU.
    AggressorImb = 914,
    /// Event frame: indicative qty / mean over `period`. UNTESTED on GPU.
    AuctionLiq = 915,
    /// Event frame: z-score of block trade size over `period` (at least 2). UNTESTED on GPU.
    BlockSizeZ = 916,
    /// Event frame: `[side, run_length]` of the current same-side run. UNTESTED on GPU.
    TradeRun = 917,
    /// Event frame (time window): liquidations per second over `a` ms. UNTESTED on GPU.
    LiqRate = 920,
    /// Event frame (time window): seconds since the previous liquidation. UNTESTED on GPU.
    LiqCooldown = 921,
    /// Event frame (time window): quote value per minute over `a` ms. UNTESTED on GPU.
    LiqVolVelocity = 922,
    /// Event frame (time window): `[imbalance, long, short]` over `a` ms. UNTESTED on GPU.
    LiqVolImbalance = 923,
    /// Event frame (time window): `[flag, count]` over `a` ms, threshold `period`. UNTESTED on GPU.
    LiqCascade = 924,
    /// Event frame (time window): agg-trade buy/sell imbalance over `a` ms. UNTESTED on GPU.
    AggFlowImb = 925,
    /// Event frame (time window): `[rate per s, current]` open interest. UNTESTED on GPU.
    OiChangeRateEv = 926,
    /// Event frame (time window): block-trade net flow over `a` ms. UNTESTED on GPU.
    BlockFlow = 927,
    /// Event frame (time window): block trades per minute over `a` ms. UNTESTED on GPU.
    BlockRate = 928,
    /// Event frame (time window): 1 - countdown / `a` ms, held when invalid. UNTESTED on GPU.
    FundingTimeDecayEv = 929,
    /// Event frame (time window): 1 - countdown / `a` ms, held when invalid. UNTESTED on GPU.
    SettleApproach = 930,
    /// Event frame (time window): delta change per second. UNTESTED on GPU.
    Charm = 931,
    /// Event frame (time window): warnings per minute over `a` ms. UNTESTED on GPU.
    WarnRate = 932,
    /// Event frame (time window): short (`a` ms) over long (`b` ms) tick rate. UNTESTED on GPU.
    TickFreqAnomaly = 933,
    /// Event frame (time window): directional burst over `a` ms: `period` min count, `c` threshold. UNTESTED on GPU.
    AggBurst = 934,
    /// Event frame (time window): momentum of ticks above size `b` over `a` ms. UNTESTED on GPU.
    LargeTickMom = 935,
    /// Event frame (time window): size-weighted directional momentum over `a` ms. UNTESTED on GPU.
    SizeWtMom = 936,
    /// Composite (smoother chain): Ppo line / signal / histogram. UNTESTED on GPU.
    PpoCols = 1000,
    /// Composite (smoother chain): Pvo line / signal / histogram. UNTESTED on GPU.
    PvoCols = 1001,
    /// Composite (smoother chain): Trix line / signal. UNTESTED on GPU.
    TrixCols = 1002,
    /// Composite (smoother chain): True strength index line / signal / histogram. UNTESTED on GPU.
    TsiCols = 1003,
    /// Composite (smoother chain): Know sure thing kst / signal. UNTESTED on GPU.
    KstCols = 1004,
    /// Composite (smoother chain): Price momentum oscillator pmo / signal. UNTESTED on GPU.
    PmoCols = 1005,
    /// Composite (smoother chain): Klinger volume oscillator line / signal. UNTESTED on GPU.
    KvoCols = 1006,
    /// Composite (smoother chain): RSI smoothed by a smoother slot. UNTESTED on GPU.
    RsiOmaCols = 1007,
    /// Composite (smoother chain): Detrended price oscillator. UNTESTED on GPU.
    DpoCols = 1008,
    /// Composite (smoother chain): Elder ray bull / bear. UNTESTED on GPU.
    ElderRayCols = 1009,
    /// Composite: Bollinger bands upper/middle/lower/std/bandwidth/percent_b. UNTESTED on GPU.
    BbCols = 1010,
    /// Composite: Bollinger on typical price middle/upper/lower. UNTESTED on GPU.
    BbPeriodCols = 1011,
    /// Composite: Bollinger percent_b / bandwidth (SMA). UNTESTED on GPU.
    BbMetricsCols = 1012,
    /// Composite: Envelope channels upper/middle/lower. UNTESTED on GPU.
    EnvelopeCols = 1013,
    /// Composite: Stochastics k/d. UNTESTED on GPU.
    StochCols = 1014,
    /// Composite: KDJ k/d/j. UNTESTED on GPU.
    KdjCols = 1015,
    /// Composite: Keltner channel upper/middle/lower. UNTESTED on GPU.
    KcCols = 1016,
    /// Composite: Keltner width/position. UNTESTED on GPU.
    KcMetricsCols = 1017,
    /// Composite: ATR channels (volatility) upper/middle/lower. UNTESTED on GPU.
    AtrcCols = 1018,
    /// Composite: ATR channels (channels) upper/middle/lower. UNTESTED on GPU.
    AtrChanCols = 1019,
    /// Composite: STARC bands upper/middle/lower. UNTESTED on GPU.
    StarcCols = 1020,
    /// Composite: Keltner on SMA(typical) upper/middle/lower. UNTESTED on GPU.
    VoKcCols = 1021,
    /// Composite: CCI. UNTESTED on GPU.
    CciComp = 1022,
    /// Composite: Chaikin volatility. UNTESTED on GPU.
    CvComp = 1023,
    /// Composite: Mass index. UNTESTED on GPU.
    MiComp = 1024,
    /// Composite: Relative momentum index. UNTESTED on GPU.
    RmiComp = 1025,
    /// Composite: Relative volatility index. UNTESTED on GPU.
    RviComp = 1026,
    /// Composite: Detrended synthetic price. UNTESTED on GPU.
    DspComp = 1027,
    /// Composite: Didi index short/long. UNTESTED on GPU.
    DidiCols = 1028,
    /// Composite: SSL channel up/down. UNTESTED on GPU.
    SslCols = 1029,
    /// Composite: Relative vigor index rvgi/signal. UNTESTED on GPU.
    RvgiCols = 1030,
    /// Composite: EMA slope. UNTESTED on GPU.
    EmaSlopeComp = 1031,
    /// Composite: Trend intensity index. UNTESTED on GPU.
    TiiComp = 1032,
    /// Composite: Price z-score (smoother mean/variance). UNTESTED on GPU.
    PriceZComp = 1033,
    /// Composite: Volume price trend. UNTESTED on GPU.
    VptComp = 1034,
    /// Composite: Rolling linear regression line/gradient/intercept/r2. UNTESTED on GPU.
    LrCols = 1035,
    /// Composite: Regression channels upper/middle/lower. UNTESTED on GPU.
    RegChanCols = 1036,
    /// Composite: Regression channel width. UNTESTED on GPU.
    RegChanWidthComp = 1037,
    /// Composite: Standard deviation channels upper/middle/lower. UNTESTED on GPU.
    StdDevChanCols = 1038,
    /// Composite: Standard deviation channel width. UNTESTED on GPU.
    StdDevWidthComp = 1039,
    /// Composite: Basic Kalman filter (2-state, optional adaptive noise). UNTESTED on GPU.
    KalmanComp = 1040,
    /// Composite: RTS smoother value (Kalman 1/1/1). UNTESTED on GPU.
    RtsComp = 1041,
    /// Composite: Kalman trend slope slope/slope_z. UNTESTED on GPU.
    KslopeCols = 1042,
    /// Composite: Kalman regime score. UNTESTED on GPU.
    KscrComp = 1043,
    /// Composite: Kalman slope z-score. UNTESTED on GPU.
    KslopezComp = 1044,
    /// Composite: Alpha-beta-gamma filter pos/vel/acc. UNTESTED on GPU.
    AbgCols = 1045,
    /// Composite: Butterworth filter. UNTESTED on GPU.
    ButterComp = 1050,
    /// Composite: Savitzky-Golay filter. UNTESTED on GPU.
    SgComp = 1051,
    /// Composite: Roofing filter. UNTESTED on GPU.
    RoofComp = 1052,
    /// Composite: TRIMA bands upper/middle/lower. UNTESTED on GPU.
    TrimaBandsCols = 1053,
    /// Composite: Keltner position. UNTESTED on GPU.
    KpComp = 1054,
    /// Composite: Chandelier stop long level. UNTESTED on GPU.
    ChandComp = 1055,
    /// Composite: Chande-Kroll stop long level. UNTESTED on GPU.
    CksComp = 1056,
    /// Composite: ATR trailing stop long level. UNTESTED on GPU.
    AtrtsComp = 1057,
    /// Composite: Gann HiLo activator activator/side. UNTESTED on GPU.
    GannHiloCols = 1058,
    /// Composite: Buy/sell pressure. UNTESTED on GPU.
    PressureComp = 1059,
    /// Composite: MACD histogram z-score. UNTESTED on GPU.
    MacdHistZComp = 1060,
    /// Composite: funding drift. UNTESTED on GPU.
    FundingDriftMg = 1070,
    /// Composite: funding OI pressure funding/oi_delta/pressure. UNTESTED on GPU.
    FundingOiPressureMg = 1071,
    /// Composite: funding price divergence. UNTESTED on GPU.
    FundingPriceDivMg = 1072,
    /// Composite: funding sentiment alignment. UNTESTED on GPU.
    FundingSentimentMg = 1073,
    /// Composite: IV HV spread. UNTESTED on GPU.
    IvHvSpreadMg = 1074,
    /// Composite: long squeeze detector. UNTESTED on GPU.
    LongSqueezeMg = 1075,
    /// Composite: mark vs last deviation/pct. UNTESTED on GPU.
    MarkVsLastMg = 1076,
    /// Composite: index tracking error. UNTESTED on GPU.
    IndexTrackingMg = 1077,
    /// Composite: OI price correlation. UNTESTED on GPU.
    OiPriceCorrMg = 1078,
    /// Composite: price vs index spread price/index/spread. UNTESTED on GPU.
    PriceVsIndexMg = 1079,
    /// Composite: vol regime entry. UNTESTED on GPU.
    VolRegimeEntryMg = 1080,
    /// Composite: settlement vs mark settlement/mark/spread. UNTESTED on GPU.
    SettleVsMarkMg = 1081,
    /// Composite: squeeze probability prob/direction. UNTESTED on GPU.
    SqueezeProbMg = 1082,
    /// Composite: risk-off detector. UNTESTED on GPU.
    RiskOffMg = 1083,
    /// Composite: market stress composite. UNTESTED on GPU.
    MarketStressMg = 1084,
    /// Composite: sentiment composite. UNTESTED on GPU.
    SentimentCompMg = 1085,
    /// Composite: book churn rate. UNTESTED on GPU.
    BookChurnEv = 980,
    /// Composite: level replenishment rate. UNTESTED on GPU.
    LevelReplenishEv = 981,
    /// Composite: quote stuffing rate/signal. UNTESTED on GPU.
    QuoteStuffingEv = 982,
    /// Composite: basis extreme. UNTESTED on GPU.
    BasisExtremeEv = 983,
    /// Composite: liquidation cluster detector price/count/volume. UNTESTED on GPU.
    LiqClusterEv = 959,
    /// Composite: large trade filter signal/ratio. UNTESTED on GPU.
    LargeTradeFilterEv = 957,
    /// Composite: agg trade size distribution median/p95/current. UNTESTED on GPU.
    AggSizeDistEv = 958,
    /// Composite: spread distribution spread/percentile. UNTESTED on GPU.
    SpreadDistributionBk = 967,
    /// Composite: layer concentration gini bid/ask/max. UNTESTED on GPU.
    LayerConcentrationBk = 968,
    /// Composite: order book velocity. UNTESTED on GPU.
    OrderBookVelocityBk = 969,
    /// Composite: tick volume delta. UNTESTED on GPU.
    TickVolumeEv = 949,
    /// Composite: trade flow imbalance/volume. UNTESTED on GPU.
    TradeFlowImbEv = 950,
    /// Composite: uptick/downtick volume. UNTESTED on GPU.
    UpDownTickVolEv = 951,
    /// Composite: funding extreme alert. UNTESTED on GPU.
    FundingExtremeEv = 952,
    /// Composite: funding momentum ema/slope. UNTESTED on GPU.
    FundingMomEv = 953,
    /// Composite: index price momentum ema/slope. UNTESTED on GPU.
    IndexPriceMomEv = 954,
    /// Composite: mark price gap signal/jump/sigma. UNTESTED on GPU.
    MarkGapEv = 955,
    /// Composite: adaptive threshold mean/std/threshold. UNTESTED on GPU.
    AdaptiveThresholdEv = 956,
    /// Composite: bid/ask bounce rate. UNTESTED on GPU.
    BidAskBounceBk = 960,
    /// Composite: mid price velocity. UNTESTED on GPU.
    MidPriceVelBk = 961,
    /// Composite: book depth change bid/ask. UNTESTED on GPU.
    BookDepthChangeBk = 962,
    /// Composite: wall detector bid_price/ask_price/total_size. UNTESTED on GPU.
    WallDetectorBk = 963,
    /// Composite: best level volatility. UNTESTED on GPU.
    BestLevelVolBk = 964,
    /// Composite: price level density. UNTESTED on GPU.
    PriceLevelDensityBk = 965,
    /// Composite: liquidity sweep direction/magnitude. UNTESTED on GPU.
    LiquiditySweepBk = 966,
    /// Composite: L3 cancel ratio. UNTESTED on GPU.
    L3CancelRatioEv = 940,
    /// Composite: auction price deviation (always 0). UNTESTED on GPU.
    AuctionPriceDeviationEv = 941,
    /// Composite: L3 order rate. UNTESTED on GPU.
    L3OrderRateEv = 942,
    /// Composite: L3 spoofer score. UNTESTED on GPU.
    L3SpooferScoreEv = 943,
    /// Composite: L3 large order side/size/price. UNTESTED on GPU.
    L3LargeOrderEv = 944,
    /// Composite: trade cluster signal/price/size. UNTESTED on GPU.
    TradeClusterEv = 945,
    /// Composite: volume imbalance zone side/low/high. UNTESTED on GPU.
    VolImbZoneEv = 946,
    /// Composite: VWAP deviation price/vwap/deviation. UNTESTED on GPU.
    VwapDevEv = 947,
    /// Composite: vol-index spike. UNTESTED on GPU.
    VolIdxSpikeEv = 948,
    /// Composite: Spectral flatness percentile. UNTESTED on GPU.
    SflatpComp = 1120,
    /// Composite: Spectral rolloff percentile. UNTESTED on GPU.
    SrollpComp = 1121,
    /// Composite: Spectral rolloff robust percentile. UNTESTED on GPU.
    SrollrpComp = 1122,
    /// Composite: Spectral slope percentile. UNTESTED on GPU.
    SslopepComp = 1123,
    /// Composite: Spectral slope robust percentile. UNTESTED on GPU.
    SsloperpComp = 1124,
    /// Composite: Spectral slope z-score. UNTESTED on GPU.
    SslopezComp = 1125,
    /// Composite: Spectral crest percentile. UNTESTED on GPU.
    ScrestpComp = 1126,
    /// Composite: Spectral entropy of entropy. UNTESTED on GPU.
    SententComp = 1127,
    /// Composite: Spectral entropy rate. UNTESTED on GPU.
    SentrComp = 1128,
    /// Composite: Spectral flux proxy. UNTESTED on GPU.
    SfluxComp = 1129,
    /// Composite: STFT band energy ratio. UNTESTED on GPU.
    StftComp = 1136,
    /// Composite: Spectral flatness. UNTESTED on GPU.
    SflatComp = 1100,
    /// Composite: Spectral slope. UNTESTED on GPU.
    SslopeComp = 1101,
    /// Composite: Spectral band power low/mid/high. UNTESTED on GPU.
    SbpCols = 1102,
    /// Composite: Spectral band power ratio high/low. UNTESTED on GPU.
    SbprhlComp = 1103,
    /// Composite: Spectral bandwidth feature. UNTESTED on GPU.
    SbwfComp = 1104,
    /// Composite: Spectral centroid feature. UNTESTED on GPU.
    ScfComp = 1105,
    /// Composite: Spectral crest. UNTESTED on GPU.
    ScrestComp = 1106,
    /// Composite: Spectral entropy. UNTESTED on GPU.
    SentComp = 1107,
    /// Composite: Spectral energy ratio. UNTESTED on GPU.
    SerComp = 1108,
    /// Composite: Spectral high/mid power ratio. UNTESTED on GPU.
    ShmprComp = 1109,
    /// Composite: Spectral low/mid power ratio. UNTESTED on GPU.
    SlmprComp = 1110,
    /// Composite: Spectral rolloff. UNTESTED on GPU.
    SrollComp = 1111,
    /// Composite: Spectral rolloff 95. UNTESTED on GPU.
    Sroll95Comp = 1112,
}

/// Smoother a smoothed cube formula applies to its pre-smoother series.
/// Mirrors the `+smoother` members of the manifest (`SmootherId`). `Tma` and
/// `Trima` are the same math. The kernel arms are the single-lane formulas of
/// the same name, run on a derived series instead of an OHLCV lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CubeSmoother {
    Sma = 0,
    Ema = 1,
    Wma = 2,
    Rma = 3,
    Dema = 4,
    Tema = 5,
    /// Also `Trima`.
    Tma = 6,
    Hma = 7,
    /// Offset is [`CubeParams::a`], sigma is [`CubeParams::b`].
    Alma = 8,
}

impl CubeSmoother {
    /// Discriminant the smoother kernels compare against.
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// The cube smoother for a manifest `SmootherId`.
    pub fn from_id(id: crate::engine::contract_engine::SmootherId) -> Self {
        use crate::engine::contract_engine::SmootherId as S;
        match id {
            S::Sma => CubeSmoother::Sma,
            S::Ema => CubeSmoother::Ema,
            S::Wma => CubeSmoother::Wma,
            S::Rma => CubeSmoother::Rma,
            S::Dema => CubeSmoother::Dema,
            S::Tema => CubeSmoother::Tema,
            S::Tma | S::Trima => CubeSmoother::Tma,
            S::Hma => CubeSmoother::Hma,
            S::Alma => CubeSmoother::Alma,
        }
    }
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
    /// Third scalar coefficient (Parabolic SAR AF cap and the like). Default `0.0`.
    pub c: f32,
    pub flag: u32,
    /// Book levels to sum. `1` is top of book.
    pub levels: u32,
    /// Scalar slot for window formulas. `0` is `s0`, `3` is `s3`.
    pub slot: u32,
    /// Smoother of a smoothed formula (code 120..=127). Default `Sma`.
    pub smoother: CubeSmoother,
    /// Period of that smoother. Default is `period`.
    pub smooth_period: u32,
    /// Second smoother (volume oscillator, Chaikin oscillator slow leg).
    pub smoother2: CubeSmoother,
    /// Period of the second smoother. Default is `period`.
    pub smooth_period2: u32,
    /// Third smoother (signal legs of the composite oscillators). Default `Sma`.
    pub smoother3: CubeSmoother,
    /// Period of the third smoother. Default is `period`.
    pub smooth_period3: u32,
    /// Extra integer parameters of composite formulas (e.g. KST ROC periods in `0..4` and
    /// smoother periods in `4..8`). Default zeros.
    pub ext: [u32; 8],
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
            c: 0.0,
            flag: 0,
            levels: 1,
            slot: 0,
            smoother: CubeSmoother::Sma,
            smooth_period: period,
            smoother2: CubeSmoother::Sma,
            smooth_period2: period,
            smoother3: CubeSmoother::Sma,
            smooth_period3: period,
            ext: [0; 8],
        }
    }
}

impl CubeFormula {
    /// Discriminant the kernel compares against.
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Number of output columns one launch writes. `1` for every single-output
    /// formula. A multi-column formula (code 100 and up) is read with
    /// `launch_cube_columns`; column `c` is the `c`-th name in the manifest braces.
    /// `true` for the smoothed formulas (code 120..=127). They run as
    /// prep -> smoother -> combine and read [`CubeParams::smoother`].
    pub const fn is_smoothed(self) -> bool {
        self.code() >= 120 && self.code() < 200
    }

    /// Leading bars the smoother does not see (their output is 0).
    pub const fn smooth_skip(self) -> u32 {
        match self {
            CubeFormula::ForceIndexSmoothed | CubeFormula::EaseOfMovementSmoothed => 1,
            _ => 0,
        }
    }

    /// Smoother passes the combine step reads: `2` for the oscillators built from
    /// two smoothed legs, otherwise `1`.
    pub const fn smoother_stages(self) -> u32 {
        match self {
            CubeFormula::VolumeOscSmoothed
            | CubeFormula::ChaikinOscSmoothed
            | CubeFormula::IntradayIntensitySmoothed => 2,
            _ => 1,
        }
    }

    /// `true` for the calendar formulas (code 140..=149). They need the time
    /// adapter and run through `launch_cube_timed`.
    pub const fn needs_time(self) -> bool {
        (self.code() >= 140 && self.code() < 150) || (self.code() >= 700 && self.code() < 710)
    }

    pub const fn output_count(self) -> u32 {
        match self {
            CubeFormula::DonchianBands => 3,
            CubeFormula::DonchianMetrics => 2,
            CubeFormula::AroonCols => 3,
            CubeFormula::CentralPivotRange => 3,
            CubeFormula::HeikinAshiCols => 4,
            CubeFormula::CandleAnatomyCols => 5,
            CubeFormula::Vortex => 2,
            CubeFormula::Fractals => 2,
            CubeFormula::TradeRun => 2,
            CubeFormula::BbCols => 6,
            CubeFormula::BbPeriodCols => 3,
            CubeFormula::BbMetricsCols => 2,
            CubeFormula::EnvelopeCols => 3,
            CubeFormula::StochCols => 2,
            CubeFormula::KdjCols => 3,
            CubeFormula::KcCols => 3,
            CubeFormula::KcMetricsCols => 2,
            CubeFormula::AtrcCols => 3,
            CubeFormula::AtrChanCols => 3,
            CubeFormula::StarcCols => 3,
            CubeFormula::VoKcCols => 3,
            CubeFormula::DidiCols => 2,
            CubeFormula::SslCols => 2,
            CubeFormula::RvgiCols => 2,
            CubeFormula::LrCols => 4,
            CubeFormula::RegChanCols => 3,
            CubeFormula::StdDevChanCols => 3,
            CubeFormula::KslopeCols => 2,
            CubeFormula::AbgCols => 3,
            CubeFormula::SbpCols => 3,
            CubeFormula::TrimaBandsCols => 3,
            CubeFormula::GannHiloCols => 2,
            CubeFormula::L3LargeOrderEv => 3,
            CubeFormula::TradeClusterEv => 3,
            CubeFormula::VolImbZoneEv => 3,
            CubeFormula::VwapDevEv => 3,
            CubeFormula::BookDepthChangeBk => 2,
            CubeFormula::WallDetectorBk => 3,
            CubeFormula::BestLevelVolBk => 3,
            CubeFormula::PriceLevelDensityBk => 3,
            CubeFormula::LiquiditySweepBk => 2,
            CubeFormula::TradeFlowImbEv => 2,
            CubeFormula::UpDownTickVolEv => 2,
            CubeFormula::FundingExtremeEv => 2,
            CubeFormula::FundingMomEv => 2,
            CubeFormula::IndexPriceMomEv => 2,
            CubeFormula::MarkGapEv => 3,
            CubeFormula::AdaptiveThresholdEv => 3,
            CubeFormula::LargeTradeFilterEv => 2,
            CubeFormula::AggSizeDistEv => 3,
            CubeFormula::SpreadDistributionBk => 2,
            CubeFormula::LayerConcentrationBk => 3,
            CubeFormula::LiqClusterEv => 3,
            CubeFormula::QuoteStuffingEv => 2,
            CubeFormula::FundingOiPressureMg => 3,
            CubeFormula::FundingPriceDivMg => 3,
            CubeFormula::IvHvSpreadMg => 3,
            CubeFormula::MarkVsLastMg => 2,
            CubeFormula::PriceVsIndexMg => 3,
            CubeFormula::SettleVsMarkMg => 3,
            CubeFormula::SqueezeProbMg => 2,
            CubeFormula::PpoCols => 3,
            CubeFormula::PvoCols => 3,
            CubeFormula::TrixCols => 2,
            CubeFormula::TsiCols => 3,
            CubeFormula::KstCols => 2,
            CubeFormula::PmoCols => 2,
            CubeFormula::KvoCols => 2,
            CubeFormula::ElderRayCols => 2,
            CubeFormula::LiqVolImbalance => 3,
            CubeFormula::LiqCascade | CubeFormula::OiChangeRateEv => 2,
            CubeFormula::StartEndMonth
            | CubeFormula::StartEndQuarter
            | CubeFormula::StartEndWeek
            | CubeFormula::TimeEnc => 2,
            CubeFormula::RollQuart | CubeFormula::PctChannels | CubeFormula::RsiPctBands => 3,
            CubeFormula::Dm => 3,
            CubeFormula::DiPlusMinus => 2,
            CubeFormula::Rwi => 2,
            CubeFormula::HigherMoments => 2,
            CubeFormula::SwingAge => 2,
            _ => 1,
        }
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

/// A catalog row backed by hand-written WGSL: the text and its entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderSpec {
    pub source: &'static str,
    pub entry: &'static str,
}

/// The WGSL behind a `+shader` catalog row, or `None` for every other id.
/// A row joins this table in two places: `+shader` on its manifest line (which
/// makes `gpu_of` answer [`GpuMode::Shader`]) and an arm here that names its
/// [`GpuShader`] impl. A formula that fits the cube subset stays `+cube`.
pub fn shader_of(id: crate::engine::indicator_id::IndicatorId) -> Option<ShaderSpec> {
    use crate::engine::indicator_id::IndicatorId;
    match id {
        IndicatorId::Decyc => Some(ShaderSpec {
            source: <crate::indicators::signal_processing::decycler::Decycler as GpuShader>::shader_source(),
            entry: <crate::indicators::signal_processing::decycler::Decycler as GpuShader>::shader_entry(),
        }),
        _ => None,
    }
}

/// Decycler needs `cos` and `sqrt` of its period for the high-pass coefficients,
/// so it is the first `+shader` row. See `shaders/decycler.wgsl` for the bindings.
impl GpuShader for crate::indicators::signal_processing::decycler::Decycler {
    fn shader_source() -> &'static str {
        include_str!("shaders/decycler.wgsl")
    }

    fn shader_entry() -> &'static str {
        "decycler"
    }
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
        assert_eq!(formula_of(IndicatorId::ClQueueImb), Some(CubeFormula::BookImbalance));
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
        assert_eq!(formula_of(IndicatorId::Dc), Some(CubeFormula::DonchianBands));
        assert_eq!(formula_of(IndicatorId::Dcmetrics), Some(CubeFormula::DonchianMetrics));
        assert_eq!(formula_of(IndicatorId::Aroon), Some(CubeFormula::AroonCols));
        assert_eq!(formula_of(IndicatorId::Cpr), Some(CubeFormula::CentralPivotRange));
        assert_eq!(formula_of(IndicatorId::Heikinashi), Some(CubeFormula::HeikinAshiCols));
        assert_eq!(formula_of(IndicatorId::Candleanatomy), Some(CubeFormula::CandleAnatomyCols));
        for (id, f) in [
            (IndicatorId::Dc, CubeFormula::DonchianBands),
            (IndicatorId::Dcmetrics, CubeFormula::DonchianMetrics),
            (IndicatorId::Aroon, CubeFormula::AroonCols),
            (IndicatorId::Cpr, CubeFormula::CentralPivotRange),
            (IndicatorId::Heikinashi, CubeFormula::HeikinAshiCols),
            (IndicatorId::Candleanatomy, CubeFormula::CandleAnatomyCols),
            (IndicatorId::Vortex, CubeFormula::Vortex),
            (IndicatorId::Dm, CubeFormula::Dm),
            (IndicatorId::DiPlusMinus, CubeFormula::DiPlusMinus),
            (IndicatorId::Rwi, CubeFormula::Rwi),
            (IndicatorId::Hmom, CubeFormula::HigherMoments),
            (IndicatorId::SwingAge, CubeFormula::SwingAge),
            (IndicatorId::VoDc, CubeFormula::DonchianBands),
        ] {
            assert_eq!(
                crate::engine::contract_engine::outputs_of(id).map(|o| o.len() as u32),
                Some(f.output_count()),
                "{id:?}"
            );
        }
        assert_eq!(formula_of(IndicatorId::Qstick), Some(CubeFormula::QstickSmoothed));
        assert_eq!(formula_of(IndicatorId::Fi), Some(CubeFormula::ForceIndexSmoothed));
        assert_eq!(formula_of(IndicatorId::Coppock), Some(CubeFormula::CoppockSmoothed));
        assert_eq!(formula_of(IndicatorId::Vo), Some(CubeFormula::VolumeOscSmoothed));
        assert_eq!(formula_of(IndicatorId::Cho), Some(CubeFormula::ChaikinOscSmoothed));
        assert_eq!(formula_of(IndicatorId::Ii), Some(CubeFormula::IntradayIntensitySmoothed));
        assert_eq!(formula_of(IndicatorId::Eom), Some(CubeFormula::EaseOfMovementSmoothed));
        assert_eq!(formula_of(IndicatorId::Natr), Some(CubeFormula::NatrSmoothed));
        assert_eq!(formula_of(IndicatorId::Vroc), Some(CubeFormula::Vroc));
        assert_eq!(formula_of(IndicatorId::Donbo), Some(CubeFormula::DonchianBreakout));
        assert_eq!(formula_of(IndicatorId::HaTrend), Some(CubeFormula::HeikinAshiTrend));
        assert_eq!(formula_of(IndicatorId::Weekday), Some(CubeFormula::WeekdayEffect));
        assert_eq!(formula_of(IndicatorId::Session), Some(CubeFormula::SessionEffect));
        assert_eq!(formula_of(IndicatorId::MonthQtr), Some(CubeFormula::MonthEffect));
        assert_eq!(formula_of(IndicatorId::DomWoq), Some(CubeFormula::DayOfMonthEffect));
        assert_eq!(formula_of(IndicatorId::Hampel), Some(CubeFormula::Hampel));
        assert_eq!(formula_of(IndicatorId::Psar), Some(CubeFormula::Psar));
        assert_eq!(formula_of(IndicatorId::Psars), Some(CubeFormula::Psar));
        assert_eq!(formula_of(IndicatorId::Supertrend), Some(CubeFormula::Supertrend));
        assert_eq!(formula_of(IndicatorId::Adx), Some(CubeFormula::Adx));
        assert_eq!(formula_of(IndicatorId::AdxSlope), Some(CubeFormula::AdxSlope));
        assert_eq!(formula_of(IndicatorId::Cusum), Some(CubeFormula::Cusum));
        assert_eq!(formula_of(IndicatorId::Har), Some(CubeFormula::Har));
        assert_eq!(formula_of(IndicatorId::Rbvj), Some(CubeFormula::Rbvj));
        assert_eq!(formula_of(IndicatorId::Vov), Some(CubeFormula::VolOfVol));
        assert_eq!(formula_of(IndicatorId::EhlersCc), Some(CubeFormula::EhlersCc));
        assert_eq!(formula_of(IndicatorId::Vortex), Some(CubeFormula::Vortex));
        assert_eq!(formula_of(IndicatorId::Dm), Some(CubeFormula::Dm));
        assert_eq!(formula_of(IndicatorId::DiPlusMinus), Some(CubeFormula::DiPlusMinus));
        assert_eq!(formula_of(IndicatorId::Rwi), Some(CubeFormula::Rwi));
        assert_eq!(formula_of(IndicatorId::Hmom), Some(CubeFormula::HigherMoments));
        assert_eq!(formula_of(IndicatorId::SwingAge), Some(CubeFormula::SwingAge));
        assert_eq!(formula_of(IndicatorId::VoDc), Some(CubeFormula::DonchianBands));
        assert_eq!(formula_of(IndicatorId::Ewmac), Some(CubeFormula::Ewmac));
        assert_eq!(formula_of(IndicatorId::Gator), Some(CubeFormula::Gator));
        assert_eq!(formula_of(IndicatorId::Ravi), Some(CubeFormula::Ravi));
        assert_eq!(formula_of(IndicatorId::Tmf), Some(CubeFormula::Tmf));
        assert_eq!(formula_of(IndicatorId::VoVr), Some(CubeFormula::VolRatio));
        assert_eq!(formula_of(IndicatorId::RangeAtr), Some(CubeFormula::RangeAtr));
        assert_eq!(formula_of(IndicatorId::Atrbw), Some(CubeFormula::RangeAtr));
        assert_eq!(formula_of(IndicatorId::Atrp), Some(CubeFormula::AtrPct));
        assert_eq!(formula_of(IndicatorId::Atrpt), Some(CubeFormula::AtrPctTrend));
        assert_eq!(formula_of(IndicatorId::Atrz), Some(CubeFormula::AtrZ));
        assert_eq!(formula_of(IndicatorId::Vovp), Some(CubeFormula::VovPct));
        assert_eq!(formula_of(IndicatorId::Vovpt), Some(CubeFormula::VovPctTrend));
        assert_eq!(formula_of(IndicatorId::Rp), Some(CubeFormula::HlRange));
        assert_eq!(formula_of(IndicatorId::RiskOffDetector), Some(CubeFormula::RiskOffMg));
        assert_eq!(formula_of(IndicatorId::MarketStressComposite), Some(CubeFormula::MarketStressMg));
        assert_eq!(formula_of(IndicatorId::SentimentComposite), Some(CubeFormula::SentimentCompMg));
        assert_eq!(formula_of(IndicatorId::OiPriceCorrelation), Some(CubeFormula::OiPriceCorrMg));
        assert_eq!(formula_of(IndicatorId::PriceVsIndexSpread), Some(CubeFormula::PriceVsIndexMg));
        assert_eq!(formula_of(IndicatorId::VolRegimeEntry), Some(CubeFormula::VolRegimeEntryMg));
        assert_eq!(formula_of(IndicatorId::SettlementVsMarkSpread), Some(CubeFormula::SettleVsMarkMg));
        assert_eq!(formula_of(IndicatorId::SqueezeProbability), Some(CubeFormula::SqueezeProbMg));
        assert_eq!(formula_of(IndicatorId::FundingDrift), Some(CubeFormula::FundingDriftMg));
        assert_eq!(formula_of(IndicatorId::FundingOiPressure), Some(CubeFormula::FundingOiPressureMg));
        assert_eq!(formula_of(IndicatorId::FundingPriceDivergence), Some(CubeFormula::FundingPriceDivMg));
        assert_eq!(formula_of(IndicatorId::FundingSentimentAlignment), Some(CubeFormula::FundingSentimentMg));
        assert_eq!(formula_of(IndicatorId::IvHvSpread), Some(CubeFormula::IvHvSpreadMg));
        assert_eq!(formula_of(IndicatorId::LongSqueezeDetector), Some(CubeFormula::LongSqueezeMg));
        assert_eq!(formula_of(IndicatorId::MarkPriceVsLast), Some(CubeFormula::MarkVsLastMg));
        assert_eq!(formula_of(IndicatorId::IndexTrackingError), Some(CubeFormula::IndexTrackingMg));
        assert_eq!(formula_of(IndicatorId::BookChurnRate), Some(CubeFormula::BookChurnEv));
        assert_eq!(formula_of(IndicatorId::LevelReplenishRate), Some(CubeFormula::LevelReplenishEv));
        assert_eq!(formula_of(IndicatorId::QuoteStuffingDetector), Some(CubeFormula::QuoteStuffingEv));
        assert_eq!(formula_of(IndicatorId::BasisExtreme), Some(CubeFormula::BasisExtremeEv));
        assert_eq!(formula_of(IndicatorId::LiquidationClusterDetector), Some(CubeFormula::LiqClusterEv));
        assert_eq!(formula_of(IndicatorId::LargeTradeFilter), Some(CubeFormula::LargeTradeFilterEv));
        assert_eq!(formula_of(IndicatorId::AggTradeSizeDistribution), Some(CubeFormula::AggSizeDistEv));
        assert_eq!(formula_of(IndicatorId::SpreadDistribution), Some(CubeFormula::SpreadDistributionBk));
        assert_eq!(formula_of(IndicatorId::LayerConcentration), Some(CubeFormula::LayerConcentrationBk));
        assert_eq!(formula_of(IndicatorId::OrderBookVelocity), Some(CubeFormula::OrderBookVelocityBk));
        assert_eq!(formula_of(IndicatorId::TickVolume), Some(CubeFormula::TickVolumeEv));
        assert_eq!(formula_of(IndicatorId::TradeFlowImbalance), Some(CubeFormula::TradeFlowImbEv));
        assert_eq!(formula_of(IndicatorId::UptickDowntickVolume), Some(CubeFormula::UpDownTickVolEv));
        assert_eq!(formula_of(IndicatorId::FundingExtremeAlert), Some(CubeFormula::FundingExtremeEv));
        assert_eq!(formula_of(IndicatorId::FundingMomentum), Some(CubeFormula::FundingMomEv));
        assert_eq!(formula_of(IndicatorId::IndexPriceMomentum), Some(CubeFormula::IndexPriceMomEv));
        assert_eq!(formula_of(IndicatorId::MarkPriceGapDetector), Some(CubeFormula::MarkGapEv));
        assert_eq!(formula_of(IndicatorId::AdaptiveThreshold), Some(CubeFormula::AdaptiveThresholdEv));
        assert_eq!(formula_of(IndicatorId::BidAskBounceRate), Some(CubeFormula::BidAskBounceBk));
        assert_eq!(formula_of(IndicatorId::MidPriceVelocity), Some(CubeFormula::MidPriceVelBk));
        assert_eq!(formula_of(IndicatorId::BookDepthChange), Some(CubeFormula::BookDepthChangeBk));
        assert_eq!(formula_of(IndicatorId::WallDetector), Some(CubeFormula::WallDetectorBk));
        assert_eq!(formula_of(IndicatorId::BestLevelVolatility), Some(CubeFormula::BestLevelVolBk));
        assert_eq!(formula_of(IndicatorId::PriceLevelDensity), Some(CubeFormula::PriceLevelDensityBk));
        assert_eq!(formula_of(IndicatorId::LiquiditySweep), Some(CubeFormula::LiquiditySweepBk));
        assert_eq!(formula_of(IndicatorId::MacdHistZ), Some(CubeFormula::MacdHistZComp));
        assert_eq!(formula_of(IndicatorId::L3CancelRatio), Some(CubeFormula::L3CancelRatioEv));
        assert_eq!(formula_of(IndicatorId::AuctionPriceDeviation), Some(CubeFormula::AuctionPriceDeviationEv));
        assert_eq!(formula_of(IndicatorId::L3OrderRate), Some(CubeFormula::L3OrderRateEv));
        assert_eq!(formula_of(IndicatorId::L3SpooferScore), Some(CubeFormula::L3SpooferScoreEv));
        assert_eq!(formula_of(IndicatorId::L3LargeOrderTracker), Some(CubeFormula::L3LargeOrderEv));
        assert_eq!(formula_of(IndicatorId::TradeClusterDetector), Some(CubeFormula::TradeClusterEv));
        assert_eq!(formula_of(IndicatorId::VolumeImbalanceZone), Some(CubeFormula::VolImbZoneEv));
        assert_eq!(formula_of(IndicatorId::VwapDeviation), Some(CubeFormula::VwapDevEv));
        assert_eq!(formula_of(IndicatorId::VolIdxSpike), Some(CubeFormula::VolIdxSpikeEv));
        assert_eq!(formula_of(IndicatorId::Chand), Some(CubeFormula::ChandComp));
        assert_eq!(formula_of(IndicatorId::Cks), Some(CubeFormula::CksComp));
        assert_eq!(formula_of(IndicatorId::Atrts), Some(CubeFormula::AtrtsComp));
        assert_eq!(formula_of(IndicatorId::GannHilo), Some(CubeFormula::GannHiloCols));
        assert_eq!(formula_of(IndicatorId::Pressure), Some(CubeFormula::PressureComp));
        assert_eq!(formula_of(IndicatorId::Trimabands), Some(CubeFormula::TrimaBandsCols));
        assert_eq!(formula_of(IndicatorId::Kp), Some(CubeFormula::KpComp));
        assert_eq!(formula_of(IndicatorId::Butter), Some(CubeFormula::ButterComp));
        assert_eq!(formula_of(IndicatorId::Sg), Some(CubeFormula::SgComp));
        assert_eq!(formula_of(IndicatorId::Roof), Some(CubeFormula::RoofComp));
        assert_eq!(formula_of(IndicatorId::Sflatp), Some(CubeFormula::SflatpComp));
        assert_eq!(formula_of(IndicatorId::Srollp), Some(CubeFormula::SrollpComp));
        assert_eq!(formula_of(IndicatorId::Srollrp), Some(CubeFormula::SrollrpComp));
        assert_eq!(formula_of(IndicatorId::Sslopep), Some(CubeFormula::SslopepComp));
        assert_eq!(formula_of(IndicatorId::Ssloperp), Some(CubeFormula::SsloperpComp));
        assert_eq!(formula_of(IndicatorId::Sslopez), Some(CubeFormula::SslopezComp));
        assert_eq!(formula_of(IndicatorId::Screstp), Some(CubeFormula::ScrestpComp));
        assert_eq!(formula_of(IndicatorId::Sentent), Some(CubeFormula::SententComp));
        assert_eq!(formula_of(IndicatorId::Sentr), Some(CubeFormula::SentrComp));
        assert_eq!(formula_of(IndicatorId::Sflux), Some(CubeFormula::SfluxComp));
        assert_eq!(formula_of(IndicatorId::Stft), Some(CubeFormula::StftComp));
        assert_eq!(formula_of(IndicatorId::Sflat), Some(CubeFormula::SflatComp));
        assert_eq!(formula_of(IndicatorId::Sslope), Some(CubeFormula::SslopeComp));
        assert_eq!(formula_of(IndicatorId::Sbp), Some(CubeFormula::SbpCols));
        assert_eq!(formula_of(IndicatorId::Sbprhl), Some(CubeFormula::SbprhlComp));
        assert_eq!(formula_of(IndicatorId::Sbwf), Some(CubeFormula::SbwfComp));
        assert_eq!(formula_of(IndicatorId::Scf), Some(CubeFormula::ScfComp));
        assert_eq!(formula_of(IndicatorId::Screst), Some(CubeFormula::ScrestComp));
        assert_eq!(formula_of(IndicatorId::Sent), Some(CubeFormula::SentComp));
        assert_eq!(formula_of(IndicatorId::Ser), Some(CubeFormula::SerComp));
        assert_eq!(formula_of(IndicatorId::Shmpr), Some(CubeFormula::ShmprComp));
        assert_eq!(formula_of(IndicatorId::Slmpr), Some(CubeFormula::SlmprComp));
        assert_eq!(formula_of(IndicatorId::Sroll), Some(CubeFormula::SrollComp));
        assert_eq!(formula_of(IndicatorId::Sroll95), Some(CubeFormula::Sroll95Comp));
        assert_eq!(formula_of(IndicatorId::Kalman), Some(CubeFormula::KalmanComp));
        assert_eq!(formula_of(IndicatorId::Rts), Some(CubeFormula::RtsComp));
        assert_eq!(formula_of(IndicatorId::Kslope), Some(CubeFormula::KslopeCols));
        assert_eq!(formula_of(IndicatorId::Kscr), Some(CubeFormula::KscrComp));
        assert_eq!(formula_of(IndicatorId::Kslopez), Some(CubeFormula::KslopezComp));
        assert_eq!(formula_of(IndicatorId::Abgfilter), Some(CubeFormula::AbgCols));
        assert_eq!(formula_of(IndicatorId::Lr), Some(CubeFormula::LrCols));
        assert_eq!(formula_of(IndicatorId::Regchan), Some(CubeFormula::RegChanCols));
        assert_eq!(formula_of(IndicatorId::Regchanwidth), Some(CubeFormula::RegChanWidthComp));
        assert_eq!(formula_of(IndicatorId::Stddevchan), Some(CubeFormula::StdDevChanCols));
        assert_eq!(formula_of(IndicatorId::Stddevwidth), Some(CubeFormula::StdDevWidthComp));
        assert_eq!(formula_of(IndicatorId::Didi), Some(CubeFormula::DidiCols));
        assert_eq!(formula_of(IndicatorId::Ssl), Some(CubeFormula::SslCols));
        assert_eq!(formula_of(IndicatorId::Rvgi), Some(CubeFormula::RvgiCols));
        assert_eq!(formula_of(IndicatorId::EmaSlope), Some(CubeFormula::EmaSlopeComp));
        assert_eq!(formula_of(IndicatorId::Tii), Some(CubeFormula::TiiComp));
        assert_eq!(formula_of(IndicatorId::PriceZscore), Some(CubeFormula::PriceZComp));
        assert_eq!(formula_of(IndicatorId::Vpt), Some(CubeFormula::VptComp));
        assert_eq!(formula_of(IndicatorId::Cci), Some(CubeFormula::CciComp));
        assert_eq!(formula_of(IndicatorId::Cv), Some(CubeFormula::CvComp));
        assert_eq!(formula_of(IndicatorId::Mi), Some(CubeFormula::MiComp));
        assert_eq!(formula_of(IndicatorId::Rmi), Some(CubeFormula::RmiComp));
        assert_eq!(formula_of(IndicatorId::Rvi), Some(CubeFormula::RviComp));
        assert_eq!(formula_of(IndicatorId::Dsp), Some(CubeFormula::DspComp));
        assert_eq!(formula_of(IndicatorId::Kc), Some(CubeFormula::KcCols));
        assert_eq!(formula_of(IndicatorId::Kcmetrics), Some(CubeFormula::KcMetricsCols));
        assert_eq!(formula_of(IndicatorId::Atrc), Some(CubeFormula::AtrcCols));
        assert_eq!(formula_of(IndicatorId::Atrchan), Some(CubeFormula::AtrChanCols));
        assert_eq!(formula_of(IndicatorId::Starc), Some(CubeFormula::StarcCols));
        assert_eq!(formula_of(IndicatorId::VoKc), Some(CubeFormula::VoKcCols));
        assert_eq!(formula_of(IndicatorId::Bb), Some(CubeFormula::BbCols));
        assert_eq!(formula_of(IndicatorId::BbPeriod), Some(CubeFormula::BbPeriodCols));
        assert_eq!(formula_of(IndicatorId::Bbmetrics), Some(CubeFormula::BbMetricsCols));
        assert_eq!(formula_of(IndicatorId::Envelope), Some(CubeFormula::EnvelopeCols));
        assert_eq!(formula_of(IndicatorId::Stoch), Some(CubeFormula::StochCols));
        assert_eq!(formula_of(IndicatorId::Kdj), Some(CubeFormula::KdjCols));
        assert_eq!(formula_of(IndicatorId::Ppo), Some(CubeFormula::PpoCols));
        assert_eq!(formula_of(IndicatorId::Pvo), Some(CubeFormula::PvoCols));
        assert_eq!(formula_of(IndicatorId::Trix), Some(CubeFormula::TrixCols));
        assert_eq!(formula_of(IndicatorId::Tsi), Some(CubeFormula::TsiCols));
        assert_eq!(formula_of(IndicatorId::Kst), Some(CubeFormula::KstCols));
        assert_eq!(formula_of(IndicatorId::Pmo), Some(CubeFormula::PmoCols));
        assert_eq!(formula_of(IndicatorId::Kvo), Some(CubeFormula::KvoCols));
        assert_eq!(formula_of(IndicatorId::Rsioma), Some(CubeFormula::RsiOmaCols));
        assert_eq!(formula_of(IndicatorId::Dpo), Some(CubeFormula::DpoCols));
        assert_eq!(formula_of(IndicatorId::ElderRay), Some(CubeFormula::ElderRayCols));
        assert_eq!(formula_of(IndicatorId::LiquidationRate), Some(CubeFormula::LiqRate));
        assert_eq!(formula_of(IndicatorId::LiquidationCooldown), Some(CubeFormula::LiqCooldown));
        assert_eq!(formula_of(IndicatorId::LiquidationVolumeVelocity), Some(CubeFormula::LiqVolVelocity));
        assert_eq!(formula_of(IndicatorId::LiquidationVolumeImbalance), Some(CubeFormula::LiqVolImbalance));
        assert_eq!(formula_of(IndicatorId::LiquidationCascade), Some(CubeFormula::LiqCascade));
        assert_eq!(formula_of(IndicatorId::AggTradeFlowImbalance), Some(CubeFormula::AggFlowImb));
        assert_eq!(formula_of(IndicatorId::OiChangeRate), Some(CubeFormula::OiChangeRateEv));
        assert_eq!(formula_of(IndicatorId::BlockTradeFlow), Some(CubeFormula::BlockFlow));
        assert_eq!(formula_of(IndicatorId::BlockTradeImpact), Some(CubeFormula::BlockRate));
        assert_eq!(formula_of(IndicatorId::FundingTimeDecay), Some(CubeFormula::FundingTimeDecayEv));
        assert_eq!(formula_of(IndicatorId::SettlementApproachSignal), Some(CubeFormula::SettleApproach));
        assert_eq!(formula_of(IndicatorId::CharmTracker), Some(CubeFormula::Charm));
        assert_eq!(formula_of(IndicatorId::WarningRate), Some(CubeFormula::WarnRate));
        assert_eq!(formula_of(IndicatorId::TickFrequencyAnomaly), Some(CubeFormula::TickFreqAnomaly));
        assert_eq!(formula_of(IndicatorId::AggressorBurstDetector), Some(CubeFormula::AggBurst));
        assert_eq!(formula_of(IndicatorId::LargeTickMomentum), Some(CubeFormula::LargeTickMom));
        assert_eq!(formula_of(IndicatorId::SizeWeightedDirectionalMomentum), Some(CubeFormula::SizeWtMom));
        assert_eq!(formula_of(IndicatorId::FundingDirectionShift), Some(CubeFormula::FundingDirShift));
        assert_eq!(formula_of(IndicatorId::LongShortExtremeDetector), Some(CubeFormula::LsExtreme));
        assert_eq!(formula_of(IndicatorId::PredictedFundingExtreme), Some(CubeFormula::PredFundingExtreme));
        assert_eq!(formula_of(IndicatorId::LeverageReductionWarning), Some(CubeFormula::LeverageReduction));
        assert_eq!(formula_of(IndicatorId::HighLowRangeRatio), Some(CubeFormula::HlRangeRatio));
        assert_eq!(formula_of(IndicatorId::TickerSpreadRatio), Some(CubeFormula::TickerSpread));
        assert_eq!(formula_of(IndicatorId::IvSkew), Some(CubeFormula::IvSkewEv));
        assert_eq!(formula_of(IndicatorId::RiskLimitProximity), Some(CubeFormula::RiskProximity));
        assert_eq!(formula_of(IndicatorId::MmrTracker), Some(CubeFormula::MmrTrack));
        assert_eq!(formula_of(IndicatorId::ThetaDecayTracker), Some(CubeFormula::ThetaDecay));
        assert_eq!(formula_of(IndicatorId::PriceChange24hZScore), Some(CubeFormula::PctChangeZ));
        assert_eq!(formula_of(IndicatorId::HvSpike), Some(CubeFormula::HvSpikeEv));
        assert_eq!(formula_of(IndicatorId::FundStressDetector), Some(CubeFormula::FundStress));
        assert_eq!(formula_of(IndicatorId::PinRiskDetector), Some(CubeFormula::PinRisk));
        assert_eq!(formula_of(IndicatorId::AggressorImbalance), Some(CubeFormula::AggressorImb));
        assert_eq!(formula_of(IndicatorId::AuctionLiquidityScore), Some(CubeFormula::AuctionLiq));
        assert_eq!(formula_of(IndicatorId::BlockTradeSizeAnomaly), Some(CubeFormula::BlockSizeZ));
        assert_eq!(formula_of(IndicatorId::TradeRunDetector), Some(CubeFormula::TradeRun));
        assert_eq!(formula_of(IndicatorId::Fractals), Some(CubeFormula::Fractals));
        assert_eq!(formula_of(IndicatorId::Fvg), Some(CubeFormula::FvgSig));
        assert_eq!(formula_of(IndicatorId::NbarPivot), Some(CubeFormula::NbarPivotSig));
        assert_eq!(formula_of(IndicatorId::Bos), Some(CubeFormula::BosSig));
        assert_eq!(formula_of(IndicatorId::Logicand), Some(CubeFormula::LogicAnd));
        assert_eq!(formula_of(IndicatorId::Logicor), Some(CubeFormula::LogicOr));
        assert_eq!(formula_of(IndicatorId::Logicxor), Some(CubeFormula::LogicXor));
        assert_eq!(formula_of(IndicatorId::Logicsign), Some(CubeFormula::LogicSign));
        assert_eq!(formula_of(IndicatorId::VolRegimeDetect), Some(CubeFormula::VolRegimeSig));
        assert_eq!(formula_of(IndicatorId::RelPosition), Some(CubeFormula::RelPositionSig));
        assert_eq!(formula_of(IndicatorId::StCusum), Some(CubeFormula::CusumFilter));
        assert_eq!(formula_of(IndicatorId::DirDetect), Some(CubeFormula::DirDetect));
        assert_eq!(formula_of(IndicatorId::RegimeGate), Some(CubeFormula::RegimeGateSig));
        assert_eq!(formula_of(IndicatorId::ThreshEdge), Some(CubeFormula::ThresholdEdge));
        assert_eq!(formula_of(IndicatorId::Thresh), Some(CubeFormula::ThresholdGateSig));
        assert_eq!(formula_of(IndicatorId::Hyst), Some(CubeFormula::HysteresisGateSig));
        assert_eq!(formula_of(IndicatorId::VolEvent), Some(CubeFormula::VolEventSig));
        assert_eq!(formula_of(IndicatorId::Sdl), Some(CubeFormula::SlopeDirLine));
        assert_eq!(formula_of(IndicatorId::HourDay), Some(CubeFormula::HourOfDay));
        assert_eq!(formula_of(IndicatorId::WeekMonth), Some(CubeFormula::WeekInMonth));
        assert_eq!(formula_of(IndicatorId::DayWeekMonth), Some(CubeFormula::WeekdayOccurrence));
        assert_eq!(formula_of(IndicatorId::MonthTurn), Some(CubeFormula::MonthTurn));
        assert_eq!(formula_of(IndicatorId::QtrTurn), Some(CubeFormula::QuarterTurn));
        assert_eq!(formula_of(IndicatorId::HolidayProx), Some(CubeFormula::WeekendProx));
        assert_eq!(formula_of(IndicatorId::SomEom), Some(CubeFormula::StartEndMonth));
        assert_eq!(formula_of(IndicatorId::SoqEoq), Some(CubeFormula::StartEndQuarter));
        assert_eq!(formula_of(IndicatorId::SowEow), Some(CubeFormula::StartEndWeek));
        assert_eq!(formula_of(IndicatorId::Tenc), Some(CubeFormula::TimeEnc));
        assert_eq!(formula_of(IndicatorId::RsiPctRank), Some(CubeFormula::RsiPctRank));
        assert_eq!(formula_of(IndicatorId::Rquart), Some(CubeFormula::RollQuart));
        assert_eq!(formula_of(IndicatorId::Percentilech), Some(CubeFormula::PctChannels));
        assert_eq!(formula_of(IndicatorId::RsiPctBands), Some(CubeFormula::RsiPctBands));
        assert_eq!(formula_of(IndicatorId::C2cvp), Some(CubeFormula::AbsLogRet));
        assert_eq!(formula_of(IndicatorId::Hlva), Some(CubeFormula::Hl2));
        assert_eq!(formula_of(IndicatorId::Keltbw), Some(CubeFormula::KeltBw));
        assert_eq!(formula_of(IndicatorId::Keltdist), Some(CubeFormula::KeltDist));
        assert_eq!(formula_of(IndicatorId::Keltpos), Some(CubeFormula::KeltPos));
        assert_eq!(gpu_of(IndicatorId::Decyc), GpuMode::Shader);
        assert_eq!(formula_of(IndicatorId::Decyc), None);
        let spec = shader_of(IndicatorId::Decyc).expect("decycler shader");
        assert_eq!(spec.entry, "decycler");
        assert!(spec.source.contains("fn decycler"));
        assert!(shader_of(IndicatorId::Sma).is_none());
    }

    #[cfg(feature = "gpu-shader")]
    #[test]
    fn decycler_shader_parses() {
        let spec = shader_of(IndicatorId::Decyc).expect("decycler shader");
        naga::front::wgsl::parse_str(spec.source).expect("decycler shader");
    }

    #[cfg(feature = "gpu-shader")]
    #[test]
    fn shader_hatch_parses() {
        let src = "@compute @workgroup_size(1)\nfn hatch() {}\n";
        naga::front::wgsl::parse_str(src).expect("shader hatch");
    }
}
