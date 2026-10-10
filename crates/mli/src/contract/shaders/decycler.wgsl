// Ehlers decycler over one close series. Not a cube formula: the high-pass
// coefficients come from cos and sqrt of the period, which the shared cube
// subset does not carry. One invocation walks the series, as the cube bar
// entries do.
//
// Bindings (group 0):
//   0  storage, read        close[n]
//   1  storage, read_write  out[n]
//   2  uniform              Params { n, period }
// `period` is `Decycler::new(period)` (clamped to at least 2). `Decycler::from_alpha(a)`
// is `period = 2 / clamp(a, 0.001, 1)`. The first two outputs are the close itself.

struct Params {
    n: u32,
    period: f32,
}

@group(0) @binding(0) var<storage, read> close: array<f32>;
@group(0) @binding(1) var<storage, read_write> out: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

const TAU: f32 = 6.28318530717958647692;

@compute @workgroup_size(1)
fn decycler() {
    let p = max(params.period, 2.0);
    let cos_val = cos(TAU / p);
    let alpha = cos_val + 2.0 - sqrt(2.0 * (1.0 + cos_val));
    let c0 = (1.0 - alpha / 2.0) * (1.0 - alpha / 2.0);
    let c1 = 2.0 * (1.0 - alpha);
    let c2 = (1.0 - alpha) * (1.0 - alpha);

    var price0 = 0.0;
    var price1 = 0.0;
    var hp0 = 0.0;
    var hp1 = 0.0;
    for (var i: u32 = 0u; i < params.n; i = i + 1u) {
        let c = close[i];
        if (i < 2u) {
            price0 = price1;
            hp0 = hp1;
            price1 = c;
            hp1 = 0.0;
            out[i] = c;
        } else {
            let hp = c0 * (c - 2.0 * price1 + price0) + c1 * hp1 - c2 * hp0;
            out[i] = c - hp;
            price0 = price1;
            price1 = c;
            hp0 = hp1;
            hp1 = hp;
        }
    }
}
