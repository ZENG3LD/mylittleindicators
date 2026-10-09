//! Fractal Adaptive Moving Average (FRAMA) indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// The fractal-dimension method a [`Frama`] uses — a CONFIG AXIS, not a separate
/// indicator. The old `Framaadv` ("advanced FRAMA") id was a second file with these
/// four modes baked in; it is absorbed here as the `method` axis of the one `Frama`
/// node (order the mode you want, the runtime switches).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum FractalMethod {
    /// Ehlers' standard fractal dimension: `log2(N2/N1)` over the window.
    #[default]
    Standard,
    /// Standard, scaled by a volatility correction factor.
    Improved,
    /// Adaptive period: the dimension window shrinks in high volatility, grows in low.
    Dynamic,
    /// Outlier-robust: median price change instead of mean for the variation term.
    Robust,
}

/// Fractal Adaptive Moving Average (FRAMA) - adapts to market fractal dimension.
///
/// α = exp(-4.6 × (D - 1)), clamped to `[min_alpha, max_alpha]`
/// FRAMA = α × Price + (1-α) × FRAMA_prev
///
/// where D is the fractal dimension estimated from price movements via the configured
/// [`FractalMethod`]. Created by John Ehlers. Trending markets get a faster response,
/// choppy markets get more smoothing.
///
/// # Implementation
///
/// Sliding price/high/low windows for the fractal-dimension calculation. O(period) per
/// update. Maximum period 512 bars.
#[derive(Debug, Clone)]
pub struct Frama {
    period: usize,
    method: FractalMethod,
    /// Window of the source price series (drives the EMA + total variation).
    window: Vec<f64>,
    /// Window of bar highs (drives the range term).
    high_window: Vec<f64>,
    /// Window of bar lows (drives the range term).
    low_window: Vec<f64>,
    min_alpha: f64,
    max_alpha: f64,
    smoothed_dimension: f64,
    dimension_ema_alpha: f64,
    alpha: f64,
    value: f64,
    initialized: bool,
}

impl Frama {
    /// Returns the period of this FRAMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the FRAMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.window.len() >= self.period
    }

    /// Resets the FRAMA to its initial state.
    pub fn reset(&mut self) {
        self.window.clear();
        self.high_window.clear();
        self.low_window.clear();
        self.smoothed_dimension = 0.0;
        self.alpha = 0.0;
        self.value = 0.0;
        self.initialized = false;
    }

    /// Creates a new FRAMA with the given period, Close source, Standard method.
    pub fn new(period: usize) -> Self {
        Self::with_method(period, FractalMethod::Standard)
    }

    /// Creates a new FRAMA with an explicit fractal-dimension method (the absorbed
    /// `Framaadv` modes). The price field is resolved by the factory via
    /// [`Frama::source_fields`]; the core ingests already-resolved lanes.
    pub fn with_method(period: usize, method: FractalMethod) -> Self {
        Self {
            period,
            method,
            window: Vec::with_capacity(period),
            high_window: Vec::with_capacity(period),
            low_window: Vec::with_capacity(period),
            min_alpha: 0.01,
            max_alpha: 1.0,
            smoothed_dimension: 0.0,
            dimension_ema_alpha: 0.2,
            alpha: 0.0,
            value: 0.0,
            initialized: false,
        }
    }

    /// Updates the FRAMA with a new bar and returns the current value.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let price = lanes[0];
        let high = lanes[1];
        let low = lanes[2];
        if self.window.len() == self.period {
            self.window.remove(0);
            self.high_window.remove(0);
            self.low_window.remove(0);
        }
        self.window.push(price);
        self.high_window.push(high);
        self.low_window.push(low);
        if self.window.len() < self.period {
            self.value = price;
            return self.value;
        }
        let dimension = self.fractal_dimension();
        // The adaptive modes EMA-smooth the dimension; the rest use it raw.
        self.smoothed_dimension =
            if matches!(self.method, FractalMethod::Improved | FractalMethod::Dynamic) {
                self.dimension_ema_alpha * dimension
                    + (1.0 - self.dimension_ema_alpha) * self.smoothed_dimension
            } else {
                dimension
            };
        self.alpha = (-4.6 * (self.smoothed_dimension - 1.0))
            .exp()
            .clamp(self.min_alpha, self.max_alpha);
        if !self.initialized {
            self.value = price;
            self.initialized = true;
        } else {
            self.value = self.alpha * price + (1.0 - self.alpha) * self.value;
        }
        self.value
    }

    /// The fractal dimension under the configured method.
    fn fractal_dimension(&self) -> f64 {
        match self.method {
            FractalMethod::Standard => self.standard_dimension(),
            FractalMethod::Improved => self.improved_dimension(),
            FractalMethod::Dynamic => self.dynamic_dimension(),
            FractalMethod::Robust => self.robust_dimension(),
        }
    }

    fn max_high(&self) -> f64 {
        self.high_window.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    fn min_low(&self) -> f64 {
        self.low_window.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    fn total_variation(&self) -> f64 {
        self.window.windows(2).map(|w| (w[1] - w[0]).abs()).sum()
    }

    /// Relative price volatility normalized to `[0, 1]` (drives the adaptive modes).
    fn volatility(&self) -> f64 {
        if self.window.len() < 2 {
            return 0.5;
        }
        let mean = self.window.iter().sum::<f64>() / self.window.len() as f64;
        let var = self.window.iter().map(|&x| (x - mean).powi(2)).sum::<f64>()
            / self.window.len() as f64;
        let rel = var.sqrt() / mean.abs().max(1e-12);
        (rel * 100.0).clamp(0.0, 1.0)
    }

    fn standard_dimension(&self) -> f64 {
        let n = self.period as f64;
        let n1 = (self.max_high() - self.min_low()) / n;
        let n2 = self.total_variation() / (n - 1.0);
        if n1 <= 1e-12 || n2 <= 1e-12 {
            return 1.0;
        }
        ((n2 / n1).ln() / 2.0_f64.ln()).clamp(1.0, 2.0)
    }

    fn improved_dimension(&self) -> f64 {
        let base = self.standard_dimension();
        let factor = 1.0 + (self.volatility() - 0.5).max(0.0) * 0.2;
        (base * factor).clamp(1.0, 2.0)
    }

    fn dynamic_dimension(&self) -> f64 {
        let vol = self.volatility();
        let adaptive = if vol > 0.7 {
            (self.period as f64 * 0.7) as usize
        } else if vol < 0.3 {
            (self.period as f64 * 1.3) as usize
        } else {
            self.period
        }
        .min(self.window.len())
        .max(2);
        let start = self.window.len() - adaptive;
        let max_val = self.high_window[start..].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_val = self.low_window[start..].iter().cloned().fold(f64::INFINITY, f64::min);
        let n1 = (max_val - min_val) / adaptive as f64;
        let mut tv = 0.0;
        for i in (start + 1)..self.window.len() {
            tv += (self.window[i] - self.window[i - 1]).abs();
        }
        let n2 = tv / (adaptive - 1) as f64;
        if n1 <= 1e-12 || n2 <= 1e-12 {
            return 1.0;
        }
        ((n2 / n1).ln() / 2.0_f64.ln()).clamp(1.0, 2.0)
    }

    fn robust_dimension(&self) -> f64 {
        let mut changes: Vec<f64> =
            self.window.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        changes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if changes.is_empty() {
            0.0
        } else {
            let mid = changes.len() / 2;
            if changes.len() % 2 == 0 {
                (changes[mid - 1] + changes[mid]) / 2.0
            } else {
                changes[mid]
            }
        };
        let n1 = (self.max_high() - self.min_low()) / self.period as f64;
        let n2 = median;
        if n1 <= 1e-12 || n2 <= 1e-12 {
            return 1.0;
        }
        ((n2 / n1).ln() / 2.0_f64.ln()).clamp(1.0, 2.0)
    }

    /// Returns the current FRAMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the FRAMA has been fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Frama`]: period, price source, and the [`FractalMethod`] axis
/// (the absorbed `Framaadv` modes — `additional_params["fractal_method"]` 1=Improved,
/// 2=Dynamic, 3=Robust, else Standard).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FramaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    pub method: Param<FractalMethod>,
}

impl crate::contract::Config for FramaConfig {
    fn defaults() -> Self {
        FramaConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            method: Param::Solo(FractalMethod::Standard),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(1,10000,1); source: Class O → auto all-8.
        // A1 2026-07-04: floor from valid_params (runtime — `standard_dimension`'s
        // `n2 = total_variation() / (period - 1)` is `0.0 / 0.0 = NaN` at period=1, a
        // single-bar window has zero total variation AND a zero divisor; no `valid_params`
        // gate exists to reject it).
        let mut s = Self::machine_defaults_auto();
        s.period = Param::range(2, 10000, 1);
        // method: Class Q (FractalMethod enum) — all 4 variants.
        s.method = Param::many(vec![
            FractalMethod::Standard,
            FractalMethod::Improved,
            FractalMethod::Dynamic,
            FractalMethod::Robust,
        ]);
        s
    }
}

impl Indicator for Frama {
    const ID: IndicatorId = IndicatorId::Frama;
    const FAMILY: &'static [Family] = &[Family::MovingAverage];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads close (the smoothed series) + high/low (the fractal-dimension windows).
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::Close, OhlcvField::High, OhlcvField::Low]));
    /// O(period): rescans period-deep price/high/low windows for the fractal dimension
    /// each bar. Its OWN state, no sub-deps. The `method` axis selects the dimension
    /// formula (absorbed `Framaadv` modes).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Frama)];
    type Config = FramaConfig;
    type Runtime = Frama;

    fn create(cfg: FramaConfig) -> Frama {
        Frama::with_method(cfg.period.resolved(), cfg.method.resolved())
    }

    /// Configurable price lane: splice the chosen source into lane[0]; high/low stay
    /// fixed (the fractal dimension always needs the bar's true high/low range).
    fn source_fields(cfg: &FramaConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved(), OhlcvField::High, OhlcvField::Low].into_iter().collect()
    }
}


impl Render for Frama {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Frama, "FRAMA", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frama_basic_calculation() {
        let mut frama = Frama::new(10);
        for i in 1..=20 {
            let p = i as f64 * 10.0;
            frama.feed(&[p, p, p]);
        }
        assert!(frama.is_ready());
        assert!(frama.value() > 0.0);
    }

    #[test]
    fn test_frama_reset() {
        let mut frama = Frama::new(5);
        for i in 1..=10 {
            let p = i as f64 * 10.0;
            frama.feed(&[p, p, p]);
        }
        assert!(frama.is_ready());
        frama.reset();
        assert!(!frama.is_ready());
        assert!(!frama.is_initialized());
    }

    #[test]
    fn test_frama_methods_all_run() {
        // Every absorbed Framaadv mode produces a finite value on the one Frama node.
        for method in [
            FractalMethod::Standard,
            FractalMethod::Improved,
            FractalMethod::Dynamic,
            FractalMethod::Robust,
        ] {
            let mut frama = Frama::with_method(20, method);
            for i in 0..60 {
                let p = 100.0 + (i as f64 * 0.3).sin() * 8.0;
                frama.feed(&[p, p, p]);
            }
            assert!(frama.is_ready(), "{method:?} ready");
            assert!(frama.value().is_finite(), "{method:?} finite");
        }
    }
}
