//! Keyed-state frame: string keys are interned on the host to integer ids `0..k` (first-appearance
//! order) and each snapshot row carries one value + presence flag per key (`val[i * k + id]`,
//! `pres[i * k + id]`). Duplicate keys inside one snapshot keep the LAST value, like the CPU `HashMap`
//! collect. Bounded per-key device state = the `k` columns. UNTESTED on GPU.

use crate::core::types::CompositeIndex;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuKeyedFrame {
    pub n: usize,
    pub k: usize,
    pub val: Vec<f32>,
    pub pres: Vec<f32>,
}

impl GpuKeyedFrame {
    pub fn from_composite_index(v: &[CompositeIndex]) -> Self {
        let mut keys: Vec<&str> = Vec::new();
        for ci in v {
            for (s, _) in &ci.components {
                if !keys.contains(&s.as_str()) {
                    keys.push(s.as_str());
                }
            }
        }
        let (n, k) = (v.len(), keys.len().max(1));
        let mut f = GpuKeyedFrame { n, k, val: vec![0.0; n * k], pres: vec![0.0; n * k] };
        for (i, ci) in v.iter().enumerate() {
            for (s, w) in &ci.components {
                let id = keys.iter().position(|x| *x == s.as_str()).unwrap_or(0);
                f.val[i * k + id] = *w as f32;
                f.pres[i * k + id] = 1.0;
            }
        }
        f
    }
}
