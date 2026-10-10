// Sequential window mean of a close lane. Each output is the mean of the
// last `period` prices ending at that index, or of the prefix when the
// window is not full yet. That matches Sma::feed (sum / count).

struct Params {
    period: u32,
    count: u32,
}

@group(0) @binding(0) var<storage, read> prices: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn sma_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.count) {
        return;
    }
    let window = min(max(params.period, 1u), i + 1u);
    var acc = 0.0;
    for (var k = 0u; k < window; k = k + 1u) {
        acc = acc + prices[i - k];
    }
    output[i] = acc / f32(window);
}
