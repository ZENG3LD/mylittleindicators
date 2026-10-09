// High-performance Efficiency Ratio (Kaufman Efficiency)
// (c) 2024


#[derive(Debug, Clone)]
pub struct EfficiencyRatioFullHistory {
    pub period: usize,
    values: Vec<f64>,    // динамический вектор всех значений (как в Nautilus)
    deltas: Vec<f64>,    // динамический вектор всех дельт (как в Nautilus)
    value: f64,
    initialized: bool,
}




impl EfficiencyRatioFullHistory {
    /// Returns up to the last n price values (newest first)
    pub fn get_last_prices(&self, n: usize) -> Vec<f64> {
        let len = self.values.len();
        let n = n.min(len);
        self.values[len - n..].iter().rev().cloned().collect()
    }
    /// Returns up to the last n delta values (newest first)
    pub fn get_last_deltas(&self, n: usize) -> Vec<f64> {
        let len = self.deltas.len();
        let n = n.min(len);
        self.deltas[len - n..].iter().rev().cloned().collect()
    }
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            values: Vec::with_capacity(512), // pre-allocate для ускорения
            deltas: Vec::with_capacity(512),
            value: 0.0,
            initialized: false,
        }
    }
    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, v: f64) -> f64 {
        self.update_raw(v)
    }

    /// Обновить Efficiency Ratio новым значением (аналог Nautilus)
    pub fn update_raw(&mut self, value: f64) -> f64 {
        self.values.push(value);
        if self.values.len() < 2 {
            self.value = 0.0;
            self.initialized = false;
            return self.value;
        }
        let last_diff = (self.values[self.values.len() - 1] - self.values[self.values.len() - 2]).abs();
        self.deltas.push(last_diff);
        if !self.initialized && self.values.len() >= self.period {
            self.initialized = true;
        }
        let net_diff = (self.values[self.values.len() - 1] - self.values[0]).abs();
        let sum_deltas: f64 = self.deltas.iter().sum();
        self.value = if sum_deltas == 0.0 { 0.0 } else { net_diff / sum_deltas };
        self.value
    }

    /// Возвращает первые n значений буфера (от старого к новому)
    pub fn get_first_n_prices(&self, n: usize) -> Vec<f64> {
        let n = n.min(self.values.len());
        self.values.iter().take(n).cloned().collect()
    }
    /// Возвращает первые n дельт (от старого к новому)
    pub fn get_first_n_deltas(&self, n: usize) -> Vec<f64> {
        let n = n.min(self.deltas.len());
        self.deltas.iter().take(n).cloned().collect()
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    // --- Debug getters ---
    pub fn get_buf(&self) -> &[f64] {
        &self.values
    }
    pub fn get_deltas(&self) -> &[f64] {
        &self.deltas
    }
    // get_buf_start, get_buf_len, get_deltas_start, get_deltas_len удалены как нерелевантные для Vec

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    pub fn reset(&mut self) {
        self.values.clear();
        self.deltas.clear();
        self.value = 0.0;
        self.initialized = false;
    }
}

impl Default for EfficiencyRatioFullHistory {
    fn default() -> Self {
        Self::new(10)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

impl Indicator for EfficiencyRatioFullHistory {
    const ID: IndicatorId = IndicatorId::Er;
    /// No family — the ER ratio is a specific named output (KAMA's inner), not a
    /// pluggable oscillator member. FLAG: could be Trend if needed later.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field — default close (standard ER usage).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(N) per update: `update_raw` rescans deltas via `.iter().sum()` over all history.
    /// Two unbounded `Vec` buffers — grows without limit (full-history variant).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Er)];
    type Config = ErConfig;
    type Runtime = EfficiencyRatioFullHistory;

    fn create(cfg: ErConfig) -> EfficiencyRatioFullHistory {
        EfficiencyRatioFullHistory::new(cfg.period.resolved())
    }
}

/// Own config for [`EfficiencyRatioFullHistory`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ErConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for ErConfig {
    fn defaults() -> Self {
        ErConfig { period: Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (ER lookback window) → auto range(2,4048,1). No other axes.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for EfficiencyRatioFullHistory {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Er, "Efficiency Ratio", Color::hex(0x4CAF50))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_ratio_creation() {
        let ind = EfficiencyRatioFullHistory::new(10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_efficiency_ratio_warmup() {
        let mut ind = EfficiencyRatioFullHistory::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.feed(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_efficiency_ratio_values() {
        let mut ind = EfficiencyRatioFullHistory::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.feed(price);
        }
        assert!(ind.value().is_finite());
        assert!(ind.value() >= 0.0);
    }

    #[test]
    fn test_efficiency_ratio_reset() {
        let mut ind = EfficiencyRatioFullHistory::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.feed(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    /// The factory resolves the close field (ignoring open/high/low/volume) and feeds the
    /// scalar; pure uptrend => ER → 1.0 once warmed up.
    #[test]
    fn factory_feeds_resolved_scalar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Er(<<EfficiencyRatioFullHistory as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..15 {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "ER should be in [0,1], got {v}");
    }
}






















