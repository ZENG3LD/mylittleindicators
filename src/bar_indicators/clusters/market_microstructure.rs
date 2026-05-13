//! Market Microstructure — real spread, depth, and efficiency from L2 orderbook.
//!
//! Primary path: `update_orderbook(&OrderBook)` — computes actual spread from
//! `book.spread()`, actual depth from `book.bid_depth(N) + book.ask_depth(N)`,
//! and real bid/ask imbalance.
//!
//! OHLCV path: `update_bar(o,h,l,c,v)` accumulates price-change and volume-change
//! statistics (efficiency, clustering) but spread and depth are always 0 without L2.
//! Use `update_orderbook` for meaningful liquidity metrics.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::types::Bar;

/// Liquidity metrics from L2 data.
#[derive(Debug, Clone)]
pub struct LiquidityMetrics {
    /// Absolute best bid/ask spread.
    pub bid_ask_spread: f64,
    /// Spread as percentage of mid price.
    pub spread_pct: f64,
    /// Total depth (bid + ask) at top N levels.
    pub market_depth: f64,
    /// Bid depth / (bid depth + ask depth) — >0.5 means bid-heavy.
    pub depth_imbalance: f64,
    /// Correlation of price change to volume change (price impact proxy).
    pub price_impact: f64,
    /// Composite 0.0–1.0 score.
    pub liquidity_score: f64,
}

/// Market efficiency metrics (derived from OHLCV price-change series).
#[derive(Debug, Clone)]
pub struct EfficiencyMetrics {
    pub price_discovery_speed: f64,
    pub information_ratio: f64,
    pub volatility_clustering: f64,
    pub mean_reversion_strength: f64,
    pub trend_persistence: f64,
    pub efficiency_score: f64,
}

/// Execution quality estimates.
#[derive(Debug, Clone)]
pub struct ExecutionQuality {
    pub slippage_estimate: f64,
    pub timing_risk: f64,
    pub adverse_selection: f64,
    pub order_flow_toxicity: f64,
    pub execution_score: f64,
}

/// Market regime classification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarketRegime {
    HighLiquidity,
    MediumLiquidity,
    LowLiquidity,
    StressedMarket,
    NormalMarket,
    VolatileMarket,
}

/// Market Microstructure analyser.
#[derive(Clone)]
pub struct MarketMicrostructure {
    period: usize,
    /// Number of L2 levels used for depth calculations.
    depth_levels: usize,

    volume_bars: Vec<Bar>,
    price_changes: Vec<f64>,
    volume_changes: Vec<f64>,

    liquidity_metrics: LiquidityMetrics,
    efficiency_metrics: EfficiencyMetrics,
    execution_quality: ExecutionQuality,

    cumulative_volume: f64,
    cumulative_price_volume: f64,

    market_regime: MarketRegime,
    microstructure_score: f64,
}

impl MarketMicrostructure {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            depth_levels: 10,
            volume_bars: Vec::with_capacity(period),
            price_changes: Vec::with_capacity(period),
            volume_changes: Vec::with_capacity(period),
            liquidity_metrics: LiquidityMetrics {
                bid_ask_spread: 0.0,
                spread_pct: 0.0,
                market_depth: 0.0,
                depth_imbalance: 0.5,
                price_impact: 0.0,
                liquidity_score: 0.5,
            },
            efficiency_metrics: EfficiencyMetrics {
                price_discovery_speed: 0.0,
                information_ratio: 0.0,
                volatility_clustering: 0.0,
                mean_reversion_strength: 0.0,
                trend_persistence: 0.0,
                efficiency_score: 0.5,
            },
            execution_quality: ExecutionQuality {
                slippage_estimate: 0.0,
                timing_risk: 0.0,
                adverse_selection: 0.0,
                order_flow_toxicity: 0.0,
                execution_score: 0.5,
            },
            cumulative_volume: 0.0,
            cumulative_price_volume: 0.0,
            market_regime: MarketRegime::NormalMarket,
            microstructure_score: 0.5,
        }
    }

    // -------------------------------------------------------------------------
    // L2 path
    // -------------------------------------------------------------------------

    fn apply_l2_liquidity(&mut self, book: &OrderBook) {
        // Real spread
        if let Some(spread) = book.spread() {
            self.liquidity_metrics.bid_ask_spread = spread;
            if let Some(mid) = book.mid_price() {
                if mid > 0.0 {
                    self.liquidity_metrics.spread_pct = (spread / mid) * 100.0;
                }
            }
        }

        // Real depth
        let bid_depth = book.bid_depth(self.depth_levels);
        let ask_depth = book.ask_depth(self.depth_levels);
        let total_depth = bid_depth + ask_depth;
        self.liquidity_metrics.market_depth = total_depth;
        self.liquidity_metrics.depth_imbalance = if total_depth > 0.0 {
            bid_depth / total_depth
        } else {
            0.5
        };

        // Liquidity score: spread component + depth component
        let spread_score = (1.0 - (self.liquidity_metrics.spread_pct / 1.0).min(1.0)).max(0.0);
        let depth_score = (total_depth / 10_000.0).min(1.0);
        self.liquidity_metrics.price_impact = (self.liquidity_metrics.spread_pct / 0.1).min(1.0);
        let impact_score = (1.0 - self.liquidity_metrics.price_impact).max(0.0);

        self.liquidity_metrics.liquidity_score = (spread_score + depth_score + impact_score) / 3.0;
    }

    // -------------------------------------------------------------------------
    // OHLCV path
    // -------------------------------------------------------------------------

    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);

        self.update_price_volume_changes(volume_bar);
        // Spread is 0 on OHLCV path — only efficiency metrics are available
        self.calculate_efficiency_metrics();
        self.calculate_execution_quality();
        self.determine_market_regime();
        self.calculate_microstructure_score();

        self.microstructure_score
    }

    fn update_price_volume_changes(&mut self, volume_bar: &Bar) {
        let n = self.volume_bars.len();
        if n >= 2 {
            if let Some(prev_bar) = self.volume_bars.get(n.saturating_sub(2)) {
                let price_change = (volume_bar.close - prev_bar.close) / prev_bar.close;
                if self.price_changes.len() >= self.period {
                    self.price_changes.remove(0);
                }
                self.price_changes.push(price_change);

                let volume_change = (volume_bar.volume - prev_bar.volume) / prev_bar.volume.max(1.0);
                if self.volume_changes.len() >= self.period {
                    self.volume_changes.remove(0);
                }
                self.volume_changes.push(volume_change);
            }
        }

        self.cumulative_volume += volume_bar.volume;
        let typical_price = (volume_bar.high + volume_bar.low + volume_bar.close) / 3.0;
        self.cumulative_price_volume += typical_price * volume_bar.volume;
    }

    fn calculate_efficiency_metrics(&mut self) {
        if self.price_changes.len() < 10 {
            return;
        }

        let autocorr = self.calculate_autocorrelation(&self.price_changes, 1);
        self.efficiency_metrics.price_discovery_speed = 1.0 - autocorr.abs();

        let mean_return = self.price_changes.iter().sum::<f64>() / self.price_changes.len() as f64;
        let volatility = self.calculate_volatility(&self.price_changes);
        if volatility > 0.0 {
            self.efficiency_metrics.information_ratio = mean_return / volatility;
        }

        let vol_changes: Vec<f64> = self.price_changes.windows(2)
            .map(|w| (w[1].abs() - w[0].abs()).abs())
            .collect();
        if vol_changes.len() > 1 {
            self.efficiency_metrics.volatility_clustering =
                self.calculate_autocorrelation(&vol_changes, 1);
        }

        if self.price_changes.len() > 2 {
            self.efficiency_metrics.mean_reversion_strength =
                self.calculate_mean_reversion_strength();
        }

        self.efficiency_metrics.trend_persistence = self.calculate_trend_persistence();

        let discovery_score = self.efficiency_metrics.price_discovery_speed;
        let clustering_score = 1.0 - self.efficiency_metrics.volatility_clustering.abs();
        let reversion_score = self.efficiency_metrics.mean_reversion_strength.abs();
        self.efficiency_metrics.efficiency_score =
            (discovery_score + clustering_score + reversion_score) / 3.0;
    }

    fn calculate_execution_quality(&mut self) {
        if self.volume_bars.len() < 5 {
            return;
        }

        let volatility = self.calculate_volatility(&self.price_changes);
        self.execution_quality.slippage_estimate = volatility * 0.5;

        if let Some(last_bar) = self.volume_bars.last() {
            self.execution_quality.timing_risk =
                (last_bar.high - last_bar.low) / last_bar.close;
        }

        self.execution_quality.adverse_selection = self.liquidity_metrics.price_impact * 0.7;

        self.execution_quality.order_flow_toxicity =
            if self.efficiency_metrics.price_discovery_speed < 0.5 { 0.8 } else { 0.2 };

        let slippage_score =
            (1.0 - (self.execution_quality.slippage_estimate / 0.01).min(1.0)).max(0.0);
        let timing_score =
            (1.0 - (self.execution_quality.timing_risk / 0.05).min(1.0)).max(0.0);
        let selection_score = (1.0 - self.execution_quality.adverse_selection).max(0.0);
        let toxicity_score = 1.0 - self.execution_quality.order_flow_toxicity;

        self.execution_quality.execution_score =
            (slippage_score + timing_score + selection_score + toxicity_score) / 4.0;
    }

    fn determine_market_regime(&mut self) {
        let avg_score = (self.liquidity_metrics.liquidity_score
            + self.efficiency_metrics.efficiency_score
            + self.execution_quality.execution_score)
            / 3.0;

        self.market_regime = if avg_score >= 0.8 {
            MarketRegime::HighLiquidity
        } else if avg_score >= 0.6 {
            MarketRegime::MediumLiquidity
        } else if avg_score >= 0.4 {
            MarketRegime::LowLiquidity
        } else if avg_score >= 0.2 {
            MarketRegime::VolatileMarket
        } else {
            MarketRegime::StressedMarket
        };
    }

    fn calculate_microstructure_score(&mut self) {
        self.microstructure_score = self.liquidity_metrics.liquidity_score * 0.4
            + self.efficiency_metrics.efficiency_score * 0.3
            + self.execution_quality.execution_score * 0.3;
    }

    // -------------------------------------------------------------------------
    // Math helpers
    // -------------------------------------------------------------------------

    fn calculate_correlation(&self, x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.len() < 2 {
            return 0.0;
        }
        let n = x.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
        let sum_x2: f64 = x.iter().map(|a| a * a).sum();
        let sum_y2: f64 = y.iter().map(|b| b * b).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

        if denominator == 0.0 { 0.0 } else { numerator / denominator }
    }

    fn calculate_autocorrelation(&self, data: &[f64], lag: usize) -> f64 {
        if data.len() <= lag {
            return 0.0;
        }
        let n = data.len() - lag;
        let x1: Vec<f64> = data.iter().take(n).cloned().collect();
        let x2: Vec<f64> = data.iter().skip(lag).cloned().collect();
        self.calculate_correlation(&x1, &x2)
    }

    fn calculate_volatility(&self, data: &[f64]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
            / (data.len() - 1) as f64;
        variance.sqrt()
    }

    fn calculate_mean_reversion_strength(&self) -> f64 {
        let autocorr_1 = self.calculate_autocorrelation(&self.price_changes, 1);
        let autocorr_5 = self.calculate_autocorrelation(&self.price_changes, 5);
        if autocorr_1 < 0.0 && autocorr_5.abs() < autocorr_1.abs() {
            autocorr_1.abs()
        } else {
            0.0
        }
    }

    fn calculate_trend_persistence(&self) -> f64 {
        if self.price_changes.len() < 10 {
            return 0.0;
        }
        let positive = self.price_changes.iter().filter(|&&x| x > 0.0).count();
        let total = self.price_changes.len();
        (positive as f64 / total as f64 - 0.5).abs() * 2.0
    }

    // -------------------------------------------------------------------------
    // Public accessors
    // -------------------------------------------------------------------------

    pub fn liquidity_metrics(&self) -> &LiquidityMetrics { &self.liquidity_metrics }
    pub fn efficiency_metrics(&self) -> &EfficiencyMetrics { &self.efficiency_metrics }
    pub fn execution_quality(&self) -> &ExecutionQuality { &self.execution_quality }
    pub fn market_regime(&self) -> MarketRegime { self.market_regime }
    pub fn microstructure_score(&self) -> f64 { self.microstructure_score }

    pub fn is_ready(&self) -> bool {
        self.volume_bars.len() >= (self.period / 2).max(5)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let bar = Bar { time: 0, open: o, high: h, low: l, close: c, volume: v };
        self.update_volume_bar(&bar)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.microstructure_score)
    }

    pub fn reset(&mut self) {
        self.volume_bars.clear();
        self.price_changes.clear();
        self.volume_changes.clear();
        self.cumulative_volume = 0.0;
        self.cumulative_price_volume = 0.0;
        self.market_regime = MarketRegime::NormalMarket;
        self.microstructure_score = 0.5;
        self.liquidity_metrics.bid_ask_spread = 0.0;
        self.liquidity_metrics.spread_pct = 0.0;
        self.liquidity_metrics.market_depth = 0.0;
        self.liquidity_metrics.depth_imbalance = 0.5;
        self.liquidity_metrics.price_impact = 0.0;
        self.liquidity_metrics.liquidity_score = 0.5;
    }
}

impl OrderBookConsumer for MarketMicrostructure {
    /// Real spread, depth, and depth imbalance from L2 snapshot.
    /// Also triggers efficiency/execution recalculation.
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        self.apply_l2_liquidity(book);
        self.calculate_efficiency_metrics();
        self.calculate_execution_quality();
        self.determine_market_regime();
        self.calculate_microstructure_score();
        self.value()
    }

    fn value(&self) -> IndicatorValue { self.value() }
    fn reset(&mut self) { self.reset() }
    fn is_ready(&self) -> bool { self.is_ready() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    #[test]
    fn test_market_microstructure_creation() {
        let ind = MarketMicrostructure::new(20);
        assert!(!ind.is_ready());
        assert_eq!(ind.microstructure_score(), 0.5);
    }

    #[test]
    fn test_market_microstructure_warmup() {
        let mut ind = MarketMicrostructure::new(10);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0 + (i as f64 * 0.1).sin(),
                high: 101.0 + (i as f64 * 0.1).sin(),
                low: 99.0 + (i as f64 * 0.1).sin(),
                close: 100.5 + (i as f64 * 0.1).sin(),
                volume: 1000.0 + i as f64 * 10.0,
            };
            ind.update_volume_bar(&bar);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_market_microstructure_score_range() {
        let mut ind = MarketMicrostructure::new(10);
        for i in 0..20 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            let score = ind.update_volume_bar(&bar);
            assert!(score >= 0.0 && score <= 1.0);
        }
    }

    #[test]
    fn test_market_microstructure_reset() {
        let mut ind = MarketMicrostructure::new(10);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.microstructure_score(), 0.5);
    }

    #[test]
    fn test_l2_spread_populated() {
        let mut ind = MarketMicrostructure::new(10);
        let book = OrderBook::from_tuples(
            &[(100.0, 5.0), (99.0, 3.0)],
            &[(100.5, 5.0), (101.0, 3.0)],
            1000,
        );
        ind.update_orderbook(&book);
        // spread = 100.5 - 100.0 = 0.5
        assert!((ind.liquidity_metrics().bid_ask_spread - 0.5).abs() < 1e-9);
        assert!(ind.liquidity_metrics().market_depth > 0.0);
    }

    #[test]
    fn test_l2_depth_imbalance_bid_heavy() {
        let mut ind = MarketMicrostructure::new(10);
        let book = OrderBook::from_tuples(
            &[(100.0, 100.0)],  // large bid
            &[(101.0, 1.0)],    // tiny ask
            1000,
        );
        ind.update_orderbook(&book);
        assert!(
            ind.liquidity_metrics().depth_imbalance > 0.5,
            "bid-heavy book should have depth_imbalance > 0.5"
        );
    }
}
