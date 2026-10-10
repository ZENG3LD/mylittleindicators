//! Spectral family: a faithful device port of `FastFourierTransform` as the spectral
//! indicators drive it, plus one statistic kernel per indicator. UNTESTED on GPU.
//!
//! CPU behaviour reproduced (not "an FFT"):
//! * the FFT window is `ws = min(next_pow2(window), 256)`, Hamming `0.54 - 0.46 cos(2 pi i / (ws-1))`,
//!   power `|X_k|^2` for `k < ws / 2`, frequency `k * 0.5 / (ws / 2)`;
//! * it computes only once `2 * ws` samples have been pushed;
//! * the exposed power / magnitude spectrum is the plain average of the last up to 8 computed
//!   spectra (`apply_overlapped_averaging`), while centroid, bandwidth and dominant period come
//!   from the newest, unaveraged spectrum;
//! * "block" indicators (Sflat, Sslope, Sbp, Screst, Sent, Ser, Shmpr, Slmpr, Sroll ...) push the
//!   whole demeaned `n`-bar window (`n` samples) into the FFT on every bar once full, so the 8
//!   averaged spectra are the 8 sliding windows ending at the last 8 samples of that block, and
//!   earlier blocks form the history; "stream" indicators (Sbprhl, Sbwf, Scf) push the raw
//!   price once per bar.
//! The device re-derives those 8 spectra per bar with a naive DFT against host-built Hamming,
//! cos and sin tables (O(ws^2 / 2) per spectrum), instead of keeping an 8-deep ring.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::gpu_sample::GpuSample;
use super::kernels::smooth_series;
use crate::engine::ohlcv_field::OhlcvField;

#[cube]
fn spec_scan(
    x: &[f32],
    means: &[f32],
    win: &[f32],
    cs: &[f32],
    sn: &[f32],
    pw: &mut [f32],
    mg: &mut [f32],
    sc: &mut [f32],
    n_blk: u32,
    ws: u32,
    mode: u32,
) {
    let n = x.len();
    let w = ws as usize;
    let nb = n_blk as usize;
    let bins = w / 2;
    let mut samp = Array::<f32>::new(256usize);
    let mut fin_p = Array::<f32>::new(128usize);
    let mut fin_m = Array::<f32>::new(128usize);
    for t in 0..n {
        let mut count = 0usize;
        for k in 0..bins {
            pw[t * bins + k] = 0.0f32;
            mg[t * bins + k] = 0.0f32;
        }
        let mut centroid = 0.0f32;
        let mut bandwidth = 0.0f32;
        let mut dom = 0.0f32;
        for q in 0..8usize {
            let mut valid = false;
            let mut e = 0usize; // global sample index of the window end (block) or bar index (stream)
            if mode == 0u32 {
                if t + 1 >= nb {
                    let m = t + 1 - nb;
                    e = m * nb + (nb - 8 + q);
                    if e + 1 >= 2 * w {
                        valid = true;
                    }
                }
            } else if t + q >= 7 {
                e = t + q - 7;
                if e + 1 >= 2 * w {
                    valid = true;
                }
            }
            if valid {
                for j in 0..w {
                    let h = e + 1 + j - w;
                    if mode == 0u32 {
                        let m2 = h / nb;
                        let i2 = h - m2 * nb;
                        samp[j] = (x[m2 + i2] - means[m2 + nb - 1]) * win[j];
                    } else {
                        samp[j] = x[h] * win[j];
                    }
                }
                let mut fsum = 0.0f32;
                let mut fw = 0.0f32;
                let mut maxm = 0.0f32;
                let mut maxk = 0usize;
                for k in 0..bins {
                    let mut re = 0.0f32;
                    let mut im = 0.0f32;
                    for j in 0..w {
                        let idx = (j * k) % w;
                        re = re + samp[j] * cs[idx];
                        im = im - samp[j] * sn[idx];
                    }
                    let power = re * re + im * im;
                    let mag = power.sqrt();
                    pw[t * bins + k] = pw[t * bins + k] + power;
                    mg[t * bins + k] = mg[t * bins + k] + mag;
                    if q == 7usize {
                        fin_p[k] = power;
                        fin_m[k] = mag;
                        let f = (k as f32) * 0.5f32 / (bins as f32);
                        fsum = fsum + power;
                        fw = fw + f * power;
                        if k > 0 && mag > maxm {
                            maxm = mag;
                            maxk = k;
                        }
                    }
                }
                count = count + 1;
                if q == 7usize {
                    if fsum > 0.0f32 {
                        centroid = fw / fsum;
                        let mut bs = 0.0f32;
                        for k in 0..bins {
                            let f = (k as f32) * 0.5f32 / (bins as f32);
                            let d = f - centroid;
                            bs = bs + d * d * fin_p[k];
                        }
                        bandwidth = (bs / fsum).sqrt();
                    }
                    let fd = (maxk as f32) * 0.5f32 / (bins as f32);
                    if fd > 0.0f32 {
                        dom = 1.0f32 / fd;
                    }
                }
            }
        }
        if count > 0 {
            let cf = count as f32;
            for k in 0..bins {
                pw[t * bins + k] = pw[t * bins + k] / cf;
                mg[t * bins + k] = mg[t * bins + k] / cf;
            }
        }
        sc[t * 4] = count as f32;
        sc[t * 4 + 1] = centroid;
        sc[t * 4 + 2] = bandwidth;
        sc[t * 4 + 3] = dom;
    }
}

#[cube(launch_unchecked)]
fn spec_map(
    x: &[f32],
    means: &[f32],
    win: &[f32],
    cs: &[f32],
    sn: &[f32],
    pw: &mut [f32],
    mg: &mut [f32],
    sc: &mut [f32],
    n_blk: u32,
    ws: u32,
    mode: u32,
) {
    spec_scan(x, means, win, cs, sn, pw, mg, sc, n_blk, ws, mode);
}

/// Per-bar spectra: `(power matrix, magnitude matrix, scalars)`; matrices are `n * ws / 2`,
/// scalars `n * 4` = `[count, centroid, bandwidth, dominant_period]`.
pub(crate) struct Spectra {
    pub bins: usize,
    pub pw: Vec<f32>,
    pub mg: Vec<f32>,
    pub sc: Vec<f32>,
}

fn next_pow2(n: usize) -> usize {
    let mut p = 1;
    while p < n {
        p *= 2;
    }
    p
}

/// `block`: `Some(n)` for the demeaned-window drive, `None` for one raw price per bar.
pub(crate) fn spectra(x: &[f32], fft_window: usize, block: Option<usize>) -> Spectra {
    let ws = next_pow2(fft_window).min(256).max(2);
    let bins = ws / 2;
    let n = x.len();
    if n == 0 {
        return Spectra { bins, pw: Vec::new(), mg: Vec::new(), sc: Vec::new() };
    }
    let nb = block.unwrap_or(0);
    let means = match block {
        Some(nn) => smooth_series(x, super::gpu::CubeSmoother::Sma, nn as u32, 0, 0.0, 0.0),
        None => x.to_vec(),
    };
    let win: Vec<f32> = (0..ws)
        .map(|i| (0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (ws - 1) as f64).cos()) as f32)
        .collect();
    let cs: Vec<f32> = (0..ws)
        .map(|i| (2.0 * std::f64::consts::PI * i as f64 / ws as f64).cos() as f32)
        .collect();
    let sn: Vec<f32> = (0..ws)
        .map(|i| (2.0 * std::f64::consts::PI * i as f64 / ws as f64).sin() as f32)
        .collect();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let mb = client.create_from_slice(f32::as_bytes(&means));
    let wb = client.create_from_slice(f32::as_bytes(&win));
    let cb = client.create_from_slice(f32::as_bytes(&cs));
    let sb = client.create_from_slice(f32::as_bytes(&sn));
    let pb = client.empty(n * bins * 4);
    let gb = client.empty(n * bins * 4);
    let scb = client.empty(n * 4 * 4);
    unsafe {
        spec_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, n),
            BufferArg::from_raw_parts(mb, n),
            BufferArg::from_raw_parts(wb, ws),
            BufferArg::from_raw_parts(cb, ws),
            BufferArg::from_raw_parts(sb, ws),
            BufferArg::from_raw_parts(pb.clone(), n * bins),
            BufferArg::from_raw_parts(gb.clone(), n * bins),
            BufferArg::from_raw_parts(scb.clone(), n * 4),
            nb as u32,
            ws as u32,
            if block.is_some() { 0u32 } else { 1u32 },
        );
    }
    Spectra {
        bins,
        pw: f32::from_bytes(&client.read_one_unchecked(pb)).to_vec(),
        mg: f32::from_bytes(&client.read_one_unchecked(gb)).to_vec(),
        sc: f32::from_bytes(&client.read_one_unchecked(scb)).to_vec(),
    }
}

/// Statistic over each bar's averaged spectrum (see the op list in [`spec_stat_scan`]).
/// Three output columns per bar, row-major `n * 3`. `first` is the first bar the CPU indicator
/// computes on (the window-full bar for block indicators); before it the outputs are `init`.
/// Outputs that the CPU only assigns on some bars are held.
#[cube]
fn spec_stat_scan(
    pw: &[f32],
    mg: &[f32],
    sc: &[f32],
    out: &mut [f32],
    bins: u32,
    op: u32,
    first: u32,
    a: f32,
    b: f32,
    init: f32,
) {
    let n = sc.len() / 4;
    let nbins = bins as usize;
    let mut h0 = init;
    let mut h1 = 0.0f32;
    let mut h2 = 0.0f32;
    for t in 0..n {
        if t >= (first as usize) {
            let mut len = 0usize;
            if sc[t * 4] > 0.5f32 {
                len = nbins;
            }
            let base = t * nbins;
            if op == 1u32 {
                // flatness
                let mut k = len;
                if k < 1 {
                    k = 1;
                }
                let mut am = 0.0f32;
                let mut lg = 0.0f32;
                for i in 0..len {
                    let mut p = pw[base + i];
                    if p < 1.0e-18f32 {
                        p = 1.0e-18f32;
                    }
                    am = am + p;
                    lg = lg + p.ln();
                }
                let amean = am / (k as f32);
                let gmean = (lg / (k as f32)).exp();
                h0 = 0.0f32;
                if amean > 0.0f32 {
                    let mut r = gmean / amean;
                    if r < 0.0f32 {
                        r = 0.0f32;
                    }
                    if r > 1.0f32 {
                        r = 1.0f32;
                    }
                    h0 = r;
                }
            } else if op == 2u32 {
                // slope of ln power vs ln frequency (held unless >= 2 points)
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxx = 0.0f32;
                let mut sxy = 0.0f32;
                let mut cnt = 0.0f32;
                for i in 1..len {
                    let f = (i as f32) * 0.5f32 / (nbins as f32);
                    let mut p = pw[base + i];
                    if p < 1.0e-18f32 {
                        p = 1.0e-18f32;
                    }
                    let xv = f.ln();
                    let yv = p.ln();
                    sx = sx + xv;
                    sy = sy + yv;
                    sxx = sxx + xv * xv;
                    sxy = sxy + xv * yv;
                    cnt = cnt + 1.0f32;
                }
                if cnt >= 2.0f32 {
                    let den = sxx * cnt - sx * sx;
                    let mut ad = den;
                    if ad < 0.0f32 {
                        ad = -ad;
                    }
                    if ad > 1.0e-12f32 {
                        h0 = (sxy * cnt - sx * sy) / den;
                    } else {
                        h0 = 0.0f32;
                    }
                }
            } else if op == 3u32 {
                // band power shares: low <= a, mid <= b, high above
                let mut ls = 0.0f32;
                let mut ms = 0.0f32;
                let mut hs = 0.0f32;
                for i in 0..len {
                    let f = (i as f32) * 0.5f32 / (nbins as f32);
                    let p = pw[base + i];
                    if f <= a {
                        ls = ls + p;
                    } else if f <= b {
                        ms = ms + p;
                    } else {
                        hs = hs + p;
                    }
                }
                let mut tot = ls + ms + hs;
                if tot < 1.0e-18f32 {
                    tot = 1.0e-18f32;
                }
                h0 = ls / tot;
                h1 = ms / tot;
                h2 = hs / tot;
            } else if op == 4u32 {
                // high / low ratio on fractions below a * 0.5 and from b * 0.5 (held when low = 0)
                let mut lo = 0.0f32;
                let mut hi = 0.0f32;
                for i in 0..len {
                    let frac = (i as f32) * 0.5f32 / (nbins as f32);
                    let p = pw[base + i];
                    if frac < a * 0.5f32 {
                        lo = lo + p;
                    } else if frac >= b * 0.5f32 {
                        hi = hi + p;
                    }
                }
                if len > 0 && lo > 0.0f32 {
                    h0 = hi / lo;
                }
            } else if op == 5u32 {
                // newest-spectrum bandwidth (a == 0) or centroid (a == 1)
                if a < 0.5f32 {
                    h0 = sc[t * 4 + 2];
                } else {
                    h0 = sc[t * 4 + 1];
                }
            } else if op == 6u32 {
                // crest: max magnitude / rms of the power spectrum
                let mut mx = 0.0f32;
                let mut sp = 0.0f32;
                for i in 0..len {
                    if mg[base + i] > mx {
                        mx = mg[base + i];
                    }
                    sp = sp + pw[base + i];
                }
                let mut rms = 0.0f32;
                if len > 0 {
                    rms = (sp / (len as f32)).sqrt();
                }
                h0 = 0.0f32;
                if rms > 0.0f32 {
                    h0 = mx / rms;
                }
            } else if op == 7u32 {
                // normalised spectral entropy
                let mut ps = 0.0f32;
                for i in 0..len {
                    ps = ps + pw[base + i];
                }
                h0 = 0.0f32;
                if ps > 1.0e-12f32 {
                    let mut hh = 0.0f32;
                    for i in 0..len {
                        let mut p = pw[base + i] / ps;
                        if p < 1.0e-15f32 {
                            p = 1.0e-15f32;
                        }
                        hh = hh - p * p.ln();
                    }
                    let nn = len as f32;
                    if hh >= 0.0f32 && nn > 1.0f32 {
                        let mut r = hh / nn.ln();
                        if r < 0.0f32 {
                            r = 0.0f32;
                        }
                        if r > 1.0f32 {
                            r = 1.0f32;
                        }
                        h0 = r;
                    } else {
                        h0 = 0.5f32;
                    }
                }
            } else if op == 8u32 {
                // energy ratio: low (f <= a * 0.5) / total
                let mut lo = 0.0f32;
                let mut hi = 0.0f32;
                for i in 0..len {
                    let f = (i as f32) * 0.5f32 / (nbins as f32);
                    let p = pw[base + i];
                    if f <= a * 0.5f32 {
                        lo = lo + p;
                    } else {
                        hi = hi + p;
                    }
                }
                h0 = 0.0f32;
                if lo + hi > 0.0f32 {
                    h0 = lo / (lo + hi);
                }
            } else if op == 9u32 {
                // high / mid (a = low cut, b = high cut), power floored at 1e-18
                let mut mid = 0.0f32;
                let mut hi = 0.0f32;
                for i in 0..len {
                    let f = (i as f32) * 0.5f32 / (nbins as f32);
                    let mut p = pw[base + i];
                    if p < 1.0e-18f32 {
                        p = 1.0e-18f32;
                    }
                    if f > a {
                        if f <= b {
                            mid = mid + p;
                        } else {
                            hi = hi + p;
                        }
                    }
                }
                h0 = 0.0f32;
                if mid > 0.0f32 {
                    h0 = hi / mid;
                }
            } else if op == 10u32 {
                // low / mid
                let mut lo = 0.0f32;
                let mut mid = 0.0f32;
                for i in 0..len {
                    let f = (i as f32) * 0.5f32 / (nbins as f32);
                    let mut p = pw[base + i];
                    if p < 1.0e-18f32 {
                        p = 1.0e-18f32;
                    }
                    if f <= a {
                        lo = lo + p;
                    } else if f <= b {
                        mid = mid + p;
                    }
                }
                h0 = 0.0f32;
                if mid > 0.0f32 {
                    h0 = lo / mid;
                }
            } else if op == 11u32 {
                // rolloff frequency at cumulative fraction a
                let mut tot = 0.0f32;
                for i in 0..len {
                    tot = tot + pw[base + i];
                }
                h0 = 0.0f32;
                if tot > 0.0f32 {
                    let mut cum = 0.0f32;
                    let mut found = false;
                    for i in 0..len {
                        if !found {
                            cum = cum + pw[base + i];
                            if cum / tot >= a {
                                h0 = (i as f32) * 0.5f32 / (nbins as f32);
                                found = true;
                            }
                        }
                    }
                }
            }
        }
        out[t * 3] = h0;
        out[t * 3 + 1] = h1;
        out[t * 3 + 2] = h2;
    }
}

#[cube(launch_unchecked)]
fn spec_stat_map(
    pw: &[f32],
    mg: &[f32],
    sc: &[f32],
    out: &mut [f32],
    bins: u32,
    op: u32,
    first: u32,
    a: f32,
    b: f32,
    init: f32,
) {
    spec_stat_scan(pw, mg, sc, out, bins, op, first, a, b, init);
}

/// Run one statistic; returns the three columns.
pub(crate) fn spec_stat(
    s: &Spectra,
    n: usize,
    op: u32,
    first: u32,
    a: f32,
    b: f32,
    init: f32,
) -> Vec<Vec<f32>> {
    if n == 0 {
        return vec![Vec::new(); 3];
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let pb = client.create_from_slice(f32::as_bytes(&s.pw));
    let gb = client.create_from_slice(f32::as_bytes(&s.mg));
    let scb = client.create_from_slice(f32::as_bytes(&s.sc));
    let ob = client.empty(n * 3 * 4);
    unsafe {
        spec_stat_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(pb, n * s.bins),
            BufferArg::from_raw_parts(gb, n * s.bins),
            BufferArg::from_raw_parts(scb, n * 4),
            BufferArg::from_raw_parts(ob.clone(), n * 3),
            s.bins as u32,
            op,
            first,
            a,
            b,
            init,
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(ob)).to_vec();
    (0..3).map(|c| (0..n).map(|t| flat[t * 3 + c]).collect()).collect()
}

fn close_series(samples: &[GpuSample], p: CubeParams) -> Vec<f32> {
    let mut q = p;
    q.lane = p.lane;
    super::kernels::launch_cube(super::CubeFormula::Identity, samples, q)
}

/// Spectral formulas (codes 1100..=1119). `period` = window, `a` / `b` the cut fractions or
/// target, as each indicator's constructor clamps them.
pub fn launch_cube_spectral(
    formula: super::CubeFormula,
    samples: &[GpuSample],
    p: CubeParams,
) -> Vec<Vec<f32>> {
    use super::CubeFormula as F;
    if samples.is_empty() {
        return Vec::new();
    }
    let x = close_series(samples, p);
    let n = x.len();
    let w = p.period as usize;
    // (window clamp, drive) per formula
    let (lo, hi, block) = match formula {
        F::SflatComp => (16, 256, true),
        F::SslopeComp => (32, 512, true),
        F::SbpCols => (32, 512, true),
        F::ScrestComp => (16, 256, true),
        F::SentComp => (16, 256, true),
        F::SerComp => (32, 256, true),
        F::ShmprComp | F::SlmprComp => (32, 1024, true),
        F::SrollComp | F::Sroll95Comp => (16, 256, true),
        F::SbprhlComp => (32, 1024, false),
        F::SbwfComp | F::ScfComp => (1, usize::MAX, false),
        _ => return Vec::new(),
    };
    let wc = w.clamp(lo, hi.min(1 << 20)).max(1);
    let spec = spectra(&x, wc, if block { Some(wc) } else { None });
    let first = if block { (wc - 1) as u32 } else { 0 };
    let cut = |v: f32, lo: f32, hi: f32| v.clamp(lo, hi);
    let cols = match formula {
        F::SflatComp => spec_stat(&spec, n, 1, first, 0.0, 0.0, 0.0),
        F::SslopeComp => spec_stat(&spec, n, 2, first, 0.0, 0.0, 0.0),
        F::SbpCols => {
            let l = cut(p.a, 0.05, 0.45);
            let h = cut(p.b, l + 0.01, 0.49);
            spec_stat(&spec, n, 3, first, l, h, 0.0)
        }
        F::SbprhlComp => {
            let lc = cut(p.a, 1e-6, 0.49);
            let hc = cut(p.b, lc + 1e-6, 0.499);
            spec_stat(&spec, n, 4, first, lc, hc, 1.0)
        }
        F::SbwfComp => spec_stat(&spec, n, 5, first, 0.0, 0.0, 0.0),
        F::ScfComp => spec_stat(&spec, n, 5, first, 1.0, 0.0, 0.0),
        F::ScrestComp => spec_stat(&spec, n, 6, first, 0.0, 0.0, 0.0),
        F::SentComp => spec_stat(&spec, n, 7, first, 0.0, 0.0, 0.0),
        F::SerComp => spec_stat(&spec, n, 8, first, cut(p.a, 0.05, 0.95), 0.0, 0.0),
        F::ShmprComp | F::SlmprComp => {
            let lc = cut(p.a, 1e-6, 0.49);
            let hc = cut(p.b, lc + 1e-6, 0.499);
            spec_stat(&spec, n, if formula == F::ShmprComp { 9 } else { 10 }, first, lc, hc, 0.0)
        }
        F::SrollComp => spec_stat(&spec, n, 11, first, cut(p.a, 0.01, 0.99), 0.0, 0.0),
        F::Sroll95Comp => spec_stat(&spec, n, 11, first, 0.95, 0.0, 0.0),
        _ => return Vec::new(),
    };
    let take = formula.output_count() as usize;
    cols.into_iter().take(take).collect()
}
