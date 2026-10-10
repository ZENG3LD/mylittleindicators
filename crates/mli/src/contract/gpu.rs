//! GPU bottom of the indicator contract — sibling of [`super::Indicator`]
//! (compute) and [`super::Render`] (draw).
//!
//! `gpu_of` reports the mode from the manifest flag. The CubeCL kernel lives
//! in `contract::kernels` under feature `gpu`. The WGSL text is on the
//! [`GpuShader`] impl. Feature `gpu-shader` is the wgpu submitter.

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

/// GPU+ only when this trait is implemented. An unmarked `feed` is not a kernel.
/// Loop-carried bar state is not rewritten into a parallel map.
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

/// WGSL text. `include_str!` is how a file becomes that `&'static str`.
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
    use crate::engine::contract_engine::gpu_of;
    use crate::engine::indicator_id::IndicatorId;
    use crate::indicators::average::sma::Sma;

    #[test]
    fn catalog_names_the_written_paths() {
        assert_eq!(gpu_of(IndicatorId::Volume), GpuMode::Cube);
        assert_eq!(gpu_of(IndicatorId::Sma), GpuMode::Shader);
        assert_eq!(gpu_of(IndicatorId::Rsi), GpuMode::None);
        assert!(Sma::shader_source().contains("fn sma_main"));
        assert_eq!(Sma::shader_entry(), "sma_main");
    }

    #[cfg(feature = "gpu-shader")]
    #[test]
    fn sma_wgsl_parses() {
        naga::front::wgsl::parse_str(Sma::shader_source()).expect("sma shader");
    }
}
