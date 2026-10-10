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
}

/// Scalar axes a cube launch reads beside the OHLCV columns.
///
/// `lane` / `lane2` are [`crate::engine::ohlcv_field::OhlcvField`] codes.
/// `a` is the ALMA offset or the T3 volume factor. `b` is the ALMA sigma.
/// `flag == 1` selects log ROC. `signal` is the MACD signal period; the
/// kernel's one output for MACD is still the line.
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
    }

    #[cfg(feature = "gpu-shader")]
    #[test]
    fn shader_hatch_parses() {
        let src = "@compute @workgroup_size(1)\nfn hatch() {}\n";
        naga::front::wgsl::parse_str(src).expect("shader hatch");
    }
}
