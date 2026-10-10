//! GPU bottom of the indicator contract — sibling of [`super::Indicator`]
//! (compute) and [`super::Render`] (draw). No kernel and no submitter are
//! compiled here.

/// How an indicator may leave the CPU. The catalog reports [`GpuMode::None`]
/// for every id until a [`GpuCube`] or [`GpuShader`] impl exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMode {
    /// CPU `feed`. Not a kernel and not a shader.
    None,
    /// CubeCL. Declared only; this change writes no `#[cube]` kernel.
    Cube,
    /// WGSL. The source is static text; the submitter is not linked.
    Shader,
}

/// GPU+ only when this trait is implemented. An unmarked `feed` is not a kernel.
/// Loop-carried bar state is not rewritten into a parallel map.
pub trait GpuCube: super::Indicator {}

/// WGSL text. `include_str!` is how a file becomes that `&'static str`.
/// The submitter is not linked in the default build.
pub trait GpuShader: super::Indicator {
    /// Shader source. A file becomes this `&'static str` via `include_str!`.
    fn shader_source() -> &'static str;

    /// Entry-point name inside [`Self::shader_source`].
    fn shader_entry() -> &'static str;
}
