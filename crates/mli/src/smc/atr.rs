//! Private window-form Average True Range for `smc`'s pure detectors.
//!
//! The crate's streaming `Atr` (`bar_indicators::volatility::atr::Atr`)
//! drives a `MovingAverageProvider` supporting 19 moving-average types and
//! carries `prev_close` / mutable-update state meant for a live bar-by-bar
//! feed. `smc`'s detectors instead receive a whole slice at once and need to
//! read the ATR value AT AN ARBITRARY PAST BAR (`atr_at_start` on a
//! `DisplacementLeg` that started in the middle of the window) — streaming
//! that slice through the mutable `Atr` and snapshotting its value at every
//! index would mean carrying the whole streaming machinery just to throw
//! away 18 of its 19 moving-average options. This helper reproduces only
//! the Wilder-smoothed variant (`Atr::new_wilder`, this crate's own default)
//! as a plain function over a slice, reusing the crate's own `true_range`.

use super::types::SmcBar;
use crate::indicators::utils::true_range::true_range;

/// Wilder ATR over a bar slice. Returns one entry per bar; `None` until
/// `period` true-range samples have accumulated (index `period - 1` onward
/// is `Some`).
pub(crate) fn atr(bars: &[SmcBar], period: usize) -> Vec<Option<f64>> {
    let mut result = vec![None; bars.len()];
    if period == 0 || bars.len() < period {
        return result;
    }

    let mut trs = Vec::with_capacity(bars.len());
    for (i, bar) in bars.iter().enumerate() {
        let tr = if i == 0 {
            bar.high - bar.low
        } else {
            true_range(bar.high, bar.low, bars[i - 1].close)
        };
        trs.push(tr);
    }

    let mut value = trs[..period].iter().sum::<f64>() / period as f64;
    result[period - 1] = Some(value);
    for (i, &tr) in trs.iter().enumerate().skip(period) {
        value = (value * (period - 1) as f64 + tr) / period as f64;
        result[i] = Some(value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(h: f64, l: f64, c: f64) -> SmcBar {
        SmcBar { open: c, high: h, low: l, close: c }
    }

    #[test]
    fn not_ready_before_period() {
        let bars = vec![bar(101.0, 99.0, 100.0); 5];
        let values = atr(&bars, 14);
        assert!(values.iter().all(|v| v.is_none()));
    }

    #[test]
    fn ready_at_period_minus_one() {
        // Constant true range of 2.0 every bar (high-low=2, no gaps).
        let bars: Vec<SmcBar> = (0..20).map(|_| bar(101.0, 99.0, 100.0)).collect();
        let values = atr(&bars, 14);
        assert!(values[12].is_none());
        let v13 = values[13].expect("ATR should warm up at index period-1");
        assert!((v13 - 2.0).abs() < 1e-9, "constant TR=2.0 should give ATR=2.0, got {v13}");
        let v19 = values[19].expect("ATR should stay ready");
        assert!((v19 - 2.0).abs() < 1e-9, "steady state ATR should hold at 2.0, got {v19}");
    }
}
