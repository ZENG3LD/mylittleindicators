//! CubeCL bar-map for [`crate::indicators::volume::volume::Volume`].
//!
//! The CPU core stays f64. This kernel is the f32 batch map of that same
//! lane: one element in, the same element out. It is compiled only with
//! the `gpu` feature, which links the CubeCL wgpu runtime.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

#[cube(launch_unchecked)]
fn volume_map(input: &[f32], output: &mut [f32]) {
    let i = UNIT_POS as usize;
    if i < input.len() {
        output[i] = input[i];
    }
}

/// Run the volume lane on the CubeCL wgpu runtime and return the mapped buffer.
pub fn launch_volume_map(volumes: &[f32]) -> Vec<f32> {
    if volumes.is_empty() {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let input = client.create_from_slice(f32::as_bytes(volumes));
    let output = client.empty(volumes.len() * core::mem::size_of::<f32>());
    unsafe {
        volume_map::launch_unchecked(
            &client,
            CubeCount::new_single(),
            CubeDim::new_1d(volumes.len() as u32),
            BufferArg::from_raw_parts(input, volumes.len()),
            BufferArg::from_raw_parts(output.clone(), volumes.len()),
        );
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_lane_is_copied() {
        let input = [1.0f32, 2.5, 0.0, 40.0];
        let output = launch_volume_map(&input);
        assert_eq!(output, input);
    }
}
