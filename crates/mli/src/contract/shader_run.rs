//! wgpu dispatcher for the hand-written WGSL rows (`+shader`). Feature `gpu-shader`.
//!
//! Binding convention every shader in `shaders/` follows (group 0):
//! `0..k` storage read-only `array<f32>` inputs, `k..k+m` storage read-write `array<f32>`
//! outputs, then one `uniform` block. One workgroup of one invocation walks the series, like the
//! cube bar entries. UNTESTED on GPU (no GPU on the authoring box).

use wgpu::util::DeviceExt;

use super::gpu::{shader_of, ShaderSpec};
use super::gpu_sample::GpuSample;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;

/// Drive a future to completion on the calling thread (no executor dependency).
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// Run `spec` once: `inputs` become read-only storage buffers, `uniform` the uniform block
/// (already laid out in WGSL std140-compatible bytes), `out_lens` the element counts of the
/// read-write outputs. Returns one `Vec<f32>` per output. `None` when no adapter exists.
pub fn run_wgsl(
    spec: ShaderSpec,
    inputs: &[&[f32]],
    uniform: &[u8],
    out_lens: &[usize],
) -> Option<Vec<Vec<f32>>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    let module = super::gpu::create_shader_module(&device, spec.source, spec.entry);

    let k = inputs.len();
    let m = out_lens.len();
    let mut entries = Vec::new();
    for i in 0..(k + m) {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: i < k },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: (k + m) as u32,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mli-shader"),
        entries: &entries,
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mli-shader"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(spec.entry),
        layout: Some(&pl),
        module: &module,
        entry_point: Some(spec.entry),
        compilation_options: Default::default(),
        cache: None,
    });

    let in_bufs: Vec<wgpu::Buffer> = inputs
        .iter()
        .map(|s| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck_cast(s),
                usage: wgpu::BufferUsages::STORAGE,
            })
        })
        .collect();
    let out_bufs: Vec<wgpu::Buffer> = out_lens
        .iter()
        .map(|n| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (*n * 4).max(4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        })
        .collect();
    let ubuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: uniform,
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let mut bind = Vec::new();
    for (i, b) in in_bufs.iter().chain(out_bufs.iter()).enumerate() {
        bind.push(wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() });
    }
    bind.push(wgpu::BindGroupEntry { binding: (k + m) as u32, resource: ubuf.as_entire_binding() });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &bind,
    });
    let reads: Vec<wgpu::Buffer> = out_lens
        .iter()
        .map(|n| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (*n * 4).max(4) as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
        .collect();
    let mut enc = device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    for (o, r) in out_bufs.iter().zip(&reads) {
        enc.copy_buffer_to_buffer(o, 0, r, 0, r.size());
    }
    queue.submit(Some(enc.finish()));
    let mut out = Vec::new();
    for (r, n) in reads.iter().zip(out_lens) {
        let slice = r.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        let data = slice.get_mapped_range().ok()?;
        let v: Vec<f32> = data
            .chunks_exact(4)
            .take(*n)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(data);
        r.unmap();
        out.push(v);
    }
    Some(out)
}

fn bytemuck_cast(s: &[f32]) -> &[u8] {
    // SAFETY: f32 has no padding and alignment 4 >= 1; the byte view covers exactly the slice.
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}

fn lane(samples: &[GpuSample], field: OhlcvField) -> Vec<f32> {
    super::kernels::launch_cube(super::gpu::CubeFormula::Identity, samples, {
        let mut p = super::gpu::CubeParams::period(1);
        p.lane = field;
        p
    })
}

/// Pack `[n: u32, f32...]` into 16-byte-multiple uniform bytes (std140 pads to 16).
pub fn uniform_bytes(n: u32, floats: &[f32]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&n.to_le_bytes());
    for f in floats {
        v.extend_from_slice(&f.to_le_bytes());
    }
    while v.len() % 16 != 0 {
        v.push(0);
    }
    v
}

/// Run a `+shader` catalog row over bar `samples`. `period` is the row's first parameter.
/// Returns one `Vec<f32>` per output column, or `None` without an adapter / for rows that have
/// no bar-sample entry.
pub fn launch_shader(
    id: IndicatorId,
    samples: &[GpuSample],
    period: f32,
) -> Option<Vec<Vec<f32>>> {
    let spec = shader_of(id)?;
    let n = samples.len();
    match id {
        IndicatorId::Decyc => {
            let close = lane(samples, OhlcvField::Close);
            run_wgsl(spec, &[&close], &uniform_bytes(n as u32, &[period]), &[n])
        }
        _ => None,
    }
}
