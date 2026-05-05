//! Synthetic data generators for indicator testing
//!
//! Different indicators require different types of price action to produce
//! meaningful (non-zero) values. This module provides specialized data generators
//! for each category of indicator.

/// A single OHLCV bar with timestamp
#[derive(Debug, Clone, Copy)]
pub struct Bar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: i64,
}

impl Bar {
    pub fn new(open: f64, high: f64, low: f64, close: f64, volume: f64, timestamp: i64) -> Self {
        Self { open, high, low, close, volume, timestamp }
    }
}

/// Type of synthetic data to generate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Smooth sinusoidal - default for most indicators
    Smooth,
    /// Contains candle patterns (hammers, dojis, engulfings, etc.)
    CandlePatterns,
    /// Contains price gaps (FVG, liquidity gaps)
    Gaps,
    /// Trending with divergences (price up, momentum down or vice versa)
    Divergence,
    /// High volatility with jumps and breakouts
    VolatilityBreakout,
    /// Volatility clustering (ARCH effects)
    VolatilityClustering,
    /// Structural breaks in the data
    StructuralBreaks,
    /// Range compression followed by expansion
    Squeeze,
    /// With specific timestamps for calendar indicators
    Calendar,
    /// Correlated returns between price and volume
    Correlated,
    /// Strong trend for market structure detection
    StrongTrend,
    /// Choppy/ranging market
    Ranging,
    /// Sweep + reversion patterns (SweepRev)
    SweepReversion,
    /// Clear ZigZag swings with defined highs/lows
    ZigZagSwings,
    /// Tick-level data simulation
    TickData,
}

/// Generate synthetic bar data for testing
pub fn generate_bars(data_type: DataType, count: usize, start_ts: i64) -> Vec<Bar> {
    match data_type {
        DataType::Smooth => generate_smooth(count, start_ts),
        DataType::CandlePatterns => generate_candle_patterns(count, start_ts),
        DataType::Gaps => generate_gaps(count, start_ts),
        DataType::Divergence => generate_divergence(count, start_ts),
        DataType::VolatilityBreakout => generate_volatility_breakout(count, start_ts),
        DataType::VolatilityClustering => generate_volatility_clustering(count, start_ts),
        DataType::StructuralBreaks => generate_structural_breaks(count, start_ts),
        DataType::Squeeze => generate_squeeze(count, start_ts),
        DataType::Calendar => generate_calendar(count, start_ts),
        DataType::Correlated => generate_correlated(count, start_ts),
        DataType::StrongTrend => generate_strong_trend(count, start_ts),
        DataType::Ranging => generate_ranging(count, start_ts),
        DataType::SweepReversion => generate_sweep_reversion(count, start_ts),
        DataType::ZigZagSwings => generate_zigzag_swings(count, start_ts),
        DataType::TickData => generate_tick_data(count, start_ts),
    }
}

/// Smooth sinusoidal data (default)
fn generate_smooth(count: usize, start_ts: i64) -> Vec<Bar> {
    (0..count)
        .map(|i| {
            let base = 100.0 + (i as f64 * 0.1).sin() * 10.0;
            let open = base;
            let close = base + (i as f64 * 0.15).cos() * 1.0;
            // Ensure high >= max(open, close) and low <= min(open, close)
            let high = open.max(close) + 0.5 + (i as f64 * 0.3).sin().abs() * 1.0;
            let low = open.min(close) - 0.5 - (i as f64 * 0.2).cos().abs() * 1.0;
            let volume = 1000.0 + (i as f64 * 50.0);
            let timestamp = start_ts + (i as i64 * 3600);
            Bar::new(open, high, low, close, volume, timestamp)
        })
        .collect()
}

/// Generate data with specific candle patterns
/// Each pattern is designed to match the detection criteria of its indicator:
/// - Hammer: body_to_range >= 0.1 (10%), lower_shadow >= 2x body, upper_shadow <= 0.5x body
/// - ShootingStar: same but reversed
/// - Doji: body_to_range < 0.1 (10%)
/// - Engulfing: second candle engulfs first, opposite colors, size_ratio >= 1.2
fn generate_candle_patterns(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);
        let volume = 1000.0 + (i as f64 * 10.0);

        // Create patterns at specific intervals, ensuring proper setups
        let bar = match i % 40 {
            // Hammer: body at top (>60% of range), long lower shadow (>=2x body), minimal upper shadow
            // Body=1.0, Range=6.0, body_to_range=16.7% > 10% ✓
            // lower_shadow=5.0, lower_shadow/body=5.0 >= 2.0 ✓
            // upper_shadow=0.0, upper_shadow/body=0.0 <= 0.5 ✓
            5 => {
                let low = price - 5.0;
                let open = price;
                let close = price + 1.0;
                let high = price + 1.0; // No upper shadow
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Shooting Star: body at bottom, long upper shadow (>=2x body), minimal lower shadow
            // Body=1.0, Range=6.0, body_to_range=16.7% > 10% ✓
            10 => {
                let high = price + 5.0;
                let open = price;
                let close = price - 1.0;
                let low = price - 1.0; // No lower shadow
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Doji: body_to_range < 10%
            // Body=0.1, Range=4.0, body_to_range=2.5% < 10% ✓
            15 => {
                let open = price;
                let close = price + 0.1;
                let high = price + 2.0;
                let low = price - 2.0;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Setup for Bullish Engulfing (previous bearish bar)
            // This needs to be smaller so next bar can engulf it
            19 => {
                let open = price + 1.0;
                let close = price; // Bearish: close < open
                let high = price + 1.2;
                let low = price - 0.2;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bullish Engulfing: bullish candle engulfs previous bearish
            // Body must be >1.2x previous body and fully engulf it
            20 => {
                let open = price - 0.5;  // Below previous close
                let close = price + 1.8; // Above previous open (engulfs)
                let high = close + 0.2;
                let low = open - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.5, timestamp)
            }
            // Setup for Bearish Engulfing (previous bullish bar)
            24 => {
                let open = price;
                let close = price + 1.0; // Bullish: close > open
                let high = price + 1.2;
                let low = price - 0.2;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bearish Engulfing: bearish candle engulfs previous bullish
            25 => {
                let prev_open = price;
                let prev_close = price + 1.0;
                let open = prev_close + 0.5;  // Above previous close
                let close = prev_open - 0.8;  // Below previous open (engulfs)
                let high = open + 0.2;
                let low = close - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.5, timestamp)
            }
            // Marubozu: full body, no shadows (open=low or high, close=high or low)
            30 => {
                let direction = if i % 80 < 40 { 1.0 } else { -1.0 };
                let open = price;
                let close = price + direction * 3.0;
                let high = open.max(close);
                let low = open.min(close);
                price = close;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Morning Star: 3-bar pattern (bearish -> small star -> bullish)
            // star_high < first body low, third close > first midpoint
            // Bar 1: Bearish candle
            26 => {
                let open = price + 3.0;
                let close = price;
                let high = open + 0.5;
                let low = close - 0.5;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bar 2: Small star (doji-like, gapped down)
            27 => {
                let star_open = price - 2.0;  // Gap down below previous close
                let star_close = star_open + 0.2;  // Small body
                let star_high = star_open + 0.5;  // Must be < previous body low (price - 0.5)
                let star_low = star_open - 0.5;
                Bar::new(star_open, star_high, star_low, star_close, volume * 0.5, timestamp)
            }
            // Bar 3: Bullish candle closing above first midpoint
            28 => {
                let third_open = price - 1.5;
                let third_close = price + 2.5;  // Above first midpoint (price + 1.5)
                let third_high = third_close + 0.3;
                let third_low = third_open - 0.3;
                price = third_close;
                Bar::new(third_open, third_high, third_low, third_close, volume * 1.5, timestamp)
            }

            // Evening Star: 3-bar pattern (bullish -> small star -> bearish)
            // Bar 1: Bullish candle
            29 => {
                let open = price;
                let close = price + 3.0;
                let high = close + 0.5;
                let low = open - 0.5;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bar 3: Bearish candle closing below first midpoint
            31 => {
                let third_open = price + 4.5;
                let third_close = price + 0.5;  // Below first midpoint (price + 1.5)
                let third_high = third_open + 0.3;
                let third_low = third_close - 0.3;
                price = third_close;
                Bar::new(third_open, third_high, third_low, third_close, volume * 1.5, timestamp)
            }

            // Three White Soldiers: 3 consecutive strong bullish candles
            32 => {
                let open = price;
                let close = price + 2.5;
                let high = close + 0.2;
                let low = open - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.2, timestamp)
            }
            33 => {
                let open = price - 0.3;  // Opens within previous body
                let close = price + 2.3;  // Closes higher than previous
                let high = close + 0.2;
                let low = open - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.3, timestamp)
            }
            34 => {
                let open = price - 0.3;
                let close = price + 2.3;
                let high = close + 0.2;
                let low = open - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.4, timestamp)
            }

            // Three Black Crows: 3 consecutive strong bearish candles
            35 => {
                let open = price;
                let close = price - 2.5;
                let high = open + 0.2;
                let low = close - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.2, timestamp)
            }
            36 => {
                let open = price + 0.3;  // Opens within previous body
                let close = price - 2.3;  // Closes lower than previous
                let high = open + 0.2;
                let low = close - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.3, timestamp)
            }
            37 => {
                let open = price + 0.3;
                let close = price - 2.3;
                let high = open + 0.2;
                let low = close - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.4, timestamp)
            }

            // Piercing Pattern: bearish candle followed by bullish closing above midpoint
            // Bar 1: Bearish
            38 => {
                let open = price + 3.0;
                let close = price;
                let high = open + 0.3;
                let low = close - 0.3;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bar 2: Bullish opening below previous low, closing above midpoint
            39 => {
                let prev_mid = (price + 3.0 + price) / 2.0;  // ~1.5 above current
                let open = price - 0.5;  // Below previous low
                let close = prev_mid + 0.5;  // Above midpoint
                let high = close + 0.2;
                let low = open - 0.2;
                price = close;
                Bar::new(open, high, low, close, volume * 1.3, timestamp)
            }

            // Normal trending bars
            _ => {
                let change = (i as f64 * 0.1).sin() * 1.5;
                price += change;
                let open = price - change;
                let close = price;
                let high = open.max(close) + 0.5;
                let low = open.min(close) - 0.5;
                Bar::new(open, high, low, close, volume, timestamp)
            }
        };

        bars.push(bar);
    }

    bars
}

/// Generate data with Fair Value Gaps (FVG)
/// FVG is a 3-bar pattern where middle bar's range doesn't overlap with surrounding bars:
/// - Bullish FVG: low[1] > high[0] AND low[1] > high[2]
/// - Bearish FVG: high[1] < low[0] AND high[1] < low[2]
fn generate_gaps(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);
        let volume = 1000.0 + (i as f64 * 10.0);

        // Create FVG triplets at specific positions
        let bar = match i % 20 {
            // Bullish FVG setup: Bar 0 (anchor bar before gap)
            5 => {
                let open = price;
                let close = price + 1.0;
                let high = price + 1.5;  // High = 101.5
                let low = price - 0.5;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bullish FVG: Bar 1 (gap bar) - low must be > previous high
            6 => {
                // Previous high was ~101.5, so low must be > 101.5
                let low = price + 3.0;  // Low = 103.0 > 101.5 ✓
                let open = low + 1.0;
                let close = low + 2.0;
                let high = low + 3.0;
                price = close;
                Bar::new(open, high, low, close, volume * 2.0, timestamp)
            }
            // Bullish FVG: Bar 2 - high must be < bar 1's low
            7 => {
                // Bar 1's low was ~103.0, so high must be < 103.0
                let prev_gap_low = price - 2.0; // Approximate bar 1's low
                let high = prev_gap_low - 0.5;  // High < gap low ✓
                let open = high - 1.0;
                let close = high - 0.5;
                let low = high - 2.0;
                price = close;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bearish FVG setup: Bar 0 (anchor bar before gap)
            12 => {
                let open = price;
                let close = price - 1.0;
                let high = price + 0.5;
                let low = price - 1.5;  // Low = ~98.5
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bearish FVG: Bar 1 (gap bar) - high must be < previous low
            13 => {
                // Previous low was ~98.5, so high must be < 98.5
                let high = price - 3.0;  // High = ~97 < 98.5 ✓
                let open = high - 1.0;
                let close = high - 2.0;
                let low = high - 3.0;
                price = close;
                Bar::new(open, high, low, close, volume * 2.0, timestamp)
            }
            // Bearish FVG: Bar 2 - low must be > bar 1's high
            14 => {
                // Bar 1's high was ~97, so low must be > 97
                let prev_gap_high = price + 2.0; // Approximate bar 1's high
                let low = prev_gap_high + 0.5;  // Low > gap high ✓
                let open = low + 0.5;
                let close = low + 1.0;
                let high = low + 2.0;
                price = close;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Normal bars
            _ => {
                let change = (i as f64 * 0.05).sin() * 1.0;
                price += change;
                let open = price - change;
                let close = price;
                let high = open.max(close) + 0.8;
                let low = open.min(close) - 0.8;
                Bar::new(open, high, low, close, volume, timestamp)
            }
        };

        bars.push(bar);
    }

    bars
}

/// Generate data with divergences (price trending one way, structure suggesting reversal)
fn generate_divergence(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    // Create a bullish divergence scenario:
    // Price makes lower lows, but momentum/RSI makes higher lows
    // This means: steeper drops followed by smaller drops

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        let phase = i / 50; // Change phase every 50 bars
        let within_phase = i % 50;

        let (change, vol_mult) = match phase % 4 {
            // Phase 0: Sharp drop
            0 => {
                let drop = -0.5 - (within_phase as f64 * 0.02);
                (drop, 1.5)
            }
            // Phase 1: Weak recovery (divergence setup)
            1 => {
                let rise = 0.3 + (within_phase as f64 * 0.01);
                (rise, 0.8)
            }
            // Phase 2: Smaller drop (higher low in momentum)
            2 => {
                let drop = -0.3 - (within_phase as f64 * 0.005);
                (drop, 1.2)
            }
            // Phase 3: Strong recovery (divergence confirmed)
            3 => {
                let rise = 0.8 + (within_phase as f64 * 0.03);
                (rise, 2.0)
            }
            _ => (0.0, 1.0),
        };

        price += change;
        let open = price - change;
        let close = price;
        let volatility = 1.0 + change.abs() * 0.5;
        let high = open.max(close) + volatility;
        let low = open.min(close) - volatility;
        let volume = 1000.0 * vol_mult;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate data with volatility breakouts
fn generate_volatility_breakout(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        // Alternate between quiet periods and breakouts
        let cycle = i / 30;
        let within_cycle = i % 30;

        let (volatility, trend, vol_mult) = if within_cycle < 20 {
            // Quiet period - low volatility
            (0.5, (within_cycle as f64 * 0.02).sin() * 0.1, 0.5)
        } else {
            // Breakout - high volatility
            let breakout_strength = (within_cycle - 20) as f64 * 0.5;
            let direction = if cycle % 2 == 0 { 1.0 } else { -1.0 };
            (3.0 + breakout_strength, direction * 1.5, 3.0)
        };

        price += trend;
        let open = price;
        let close = price + trend;
        let high = open.max(close) + volatility;
        let low = open.min(close) - volatility;
        let volume = 1000.0 * vol_mult;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate data with volatility clustering (ARCH effects)
fn generate_volatility_clustering(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0_f64;
    let mut current_vol: f64 = 1.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        // GARCH-like: volatility depends on previous volatility
        // High vol -> likely high vol, low vol -> likely low vol
        let vol_shock = if i % 40 == 0 {
            // Occasional volatility shock
            3.0
        } else {
            0.0
        };

        // Mean reversion in volatility
        current_vol = 0.95 * current_vol + 0.05 * 1.0 + vol_shock;
        current_vol = current_vol.clamp(0.3, 5.0);

        let return_val = (i as f64 * 0.1).sin() * current_vol * 0.5;
        price *= 1.0 + return_val / 100.0;

        let open = price / (1.0 + return_val / 100.0);
        let close = price;
        let range = current_vol * 2.0;
        let high = open.max(close) + range * 0.6;
        let low = open.min(close) - range * 0.4;
        let volume = 1000.0 * (1.0 + current_vol * 0.5);

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate data with structural breaks
fn generate_structural_breaks(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;
    let mut trend = 0.1;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        // Structural break every ~100 bars
        if i % 100 == 50 {
            trend = -trend * 1.5; // Reverse and strengthen trend
            price += trend * 10.0; // Jump
        }

        price += trend;
        let noise = (i as f64 * 0.2).sin() * 0.5;
        let open = price - trend;
        let close = price + noise;
        let high = open.max(close) + 1.0;
        let low = open.min(close) - 1.0;
        let volume = 1000.0;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate squeeze pattern (compression -> expansion)
fn generate_squeeze(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        let cycle = i / 40;
        let within_cycle = i % 40;

        // First 30 bars: compression (decreasing range)
        // Last 10 bars: expansion (increasing range)
        let range = if within_cycle < 30 {
            let compression = (30 - within_cycle) as f64 / 30.0;
            0.5 + compression * 2.0
        } else {
            let expansion = (within_cycle - 30) as f64;
            0.5 + expansion * 1.5
        };

        // After squeeze, breakout in alternating direction
        let trend = if within_cycle >= 30 {
            let direction = if cycle % 2 == 0 { 1.0 } else { -1.0 };
            direction * (within_cycle - 30) as f64 * 0.5
        } else {
            (within_cycle as f64 * 0.05).sin() * 0.2
        };

        price += trend;
        let open = price - trend * 0.5;
        let close = price;
        let high = open.max(close) + range;
        let low = open.min(close) - range;
        let volume = 1000.0 * (1.0 + range * 0.3);

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate calendar-aware data (specific days/times)
fn generate_calendar(count: usize, _start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    // Start on Monday 00:00 UTC (adjust start_ts to be Monday)
    // 1704067200 is Monday, January 1, 2024 00:00:00 UTC
    let monday_start = 1704067200_i64;

    for i in 0..count {
        // Each bar is 1 hour
        let timestamp = monday_start + (i as i64 * 3600);

        // Calculate day of week (0 = Monday)
        let hours_from_start = i as i64;
        let day_of_week = (hours_from_start / 24) % 7;
        let hour_of_day = hours_from_start % 24;

        // Higher volatility during market open hours
        let session_vol = if (13..=20).contains(&hour_of_day) {
            2.0 // US session
        } else if (7..=15).contains(&hour_of_day) {
            1.5 // EU session
        } else if (0..=8).contains(&hour_of_day) {
            1.2 // Asia session
        } else {
            0.8 // Off-hours
        };

        // Monday open often has gaps
        let gap = if day_of_week == 0 && hour_of_day == 0 && i > 0 {
            (i as f64 * 0.1).sin() * 3.0
        } else {
            0.0
        };

        // End of month effect (every ~720 bars assuming 1 hour per bar)
        let eom_vol = if i % 720 > 690 { 1.5 } else { 1.0 };

        price += gap + (i as f64 * 0.05).sin() * session_vol * 0.5;
        let volatility = session_vol * eom_vol;
        let open = price - gap;
        let close = price;
        let high = open.max(close) + volatility;
        let low = open.min(close) - volatility;
        let volume = 1000.0 * session_vol;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate correlated price-volume data
fn generate_correlated(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;
    let mut prev_change = 0.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        // Create strong AUTOCORRELATION: today's return similar to yesterday's
        // This is key for mutual information (lag-1 dependency)
        let momentum = prev_change * 0.7; // 70% carryover from previous bar
        let phase = (i / 50) % 3;
        let base_trend = match phase {
            0 => 1.5,   // Uptrend period
            1 => -1.2,  // Downtrend period
            _ => 0.3,   // Consolidation
        };

        let noise = (i as f64 * 0.2).sin() * 0.3;
        let price_change = momentum + base_trend * 0.3 + noise;
        prev_change = price_change;

        // Volume correlates with absolute price change (for TE)
        let volume_factor = 1.0 + price_change.abs() * 1.5;

        price = (price + price_change).max(30.0);
        let open = price - price_change * 0.7;
        let close = price;
        let range = price_change.abs().max(0.5);
        let high = open.max(close) + range * 0.3;
        let low = open.min(close) - range * 0.3;
        let volume = 500.0 + 1000.0 * volume_factor;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate strong trending data
fn generate_strong_trend(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0_f64;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        // Alternate between uptrend and downtrend
        let phase = i / 100;
        let trend: f64 = if phase % 2 == 0 { 0.5 } else { -0.5 };

        // Add pullbacks
        let pullback = if i % 20 < 5 { -trend * 0.3 } else { 0.0 };

        price += trend + pullback;
        let open = price - trend;
        let close = price;

        // Higher highs/lows in trend direction
        let (high, low) = if trend > 0.0 {
            (close + 1.5, open - 0.5)
        } else {
            (open + 0.5, close - 1.5)
        };

        let volume = 1000.0 * (1.0 + trend.abs());

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate ranging/choppy data with sharp spikes for XOR divergence detection
/// XOR needs: short RSI(7) in extreme while long RSI(21) stays neutral
/// This requires 5-7 consecutive bars of strong movement after a stable period
fn generate_ranging(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);

        // Create pattern: 25 stable bars, then 7 extreme bars, then 8 reversal bars
        let cycle_phase = i % 40;
        let change = if (25..=31).contains(&cycle_phase) {
            // Sharp 7-bar spike up - pushes short RSI(7) above 70
            // Long RSI(21) needs more bars to reach extreme
            5.0
        } else if (32..=39).contains(&cycle_phase) {
            // Sharp reversal down - pushes short RSI(7) below 30
            -6.0
        } else {
            // Stable oscillation keeps long RSI near 50
            (i as f64 * 0.15).sin() * 0.3
        };

        price = (price + change).clamp(50.0, 150.0);

        let open = price - change * 0.4;
        let close = price;
        let high = open.max(close) + 0.3;
        let low = open.min(close) - 0.3;
        let volume = 1000.0;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate sweep + reversion patterns
/// SweepRev needs:
/// - Price to exceed previous high/low (sweep)
/// - Then close back within the range (reversion)
/// lookback_period is typically 40, so we need 40+ bar ranges
fn generate_sweep_reversion(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;
    let mut range_high = price + 5.0;
    let mut range_low = price - 5.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);
        let volume = 1000.0 + (i as f64 * 10.0);

        let cycle = i % 60;

        let bar = match cycle {
            // First 40 bars: establish range
            0..=39 => {
                let oscillation = (cycle as f64 * 0.15).sin() * 3.0;
                price = 100.0 + oscillation;
                let open = price - oscillation * 0.3;
                let close = price;
                let high = open.max(close) + 0.5;
                let low = open.min(close) - 0.5;

                // Track range
                if high > range_high { range_high = high; }
                if low < range_low { range_low = low; }

                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bar 40-42: Bearish sweep (high exceeds range high, closes in lower quartile)
            40..=42 => {
                let sweep_high = range_high + 2.0 + (cycle - 40) as f64;  // Exceeds range
                let close = range_low + (range_high - range_low) * 0.2;   // Closes in lower 20%
                let open = sweep_high - 1.0;
                let low = close - 0.5;
                price = close;
                Bar::new(open, sweep_high, low, close, volume * 2.0, timestamp)
            }
            // Bar 43-45: Continuation down
            43..=45 => {
                let change = -1.5;
                price += change;
                let open = price - change;
                let close = price;
                let high = open.max(close) + 0.3;
                let low = open.min(close) - 0.3;
                Bar::new(open, high, low, close, volume, timestamp)
            }
            // Bar 46-48: Bullish sweep (low exceeds range low, closes in upper quartile)
            46..=48 => {
                let sweep_low = range_low - 2.0 - (cycle - 46) as f64;  // Exceeds range
                let close = range_low + (range_high - range_low) * 0.8;  // Closes in upper 20%
                let open = sweep_low + 1.0;
                let high = close + 0.5;
                price = close;
                Bar::new(open, high, sweep_low, close, volume * 2.0, timestamp)
            }
            // Rest: reset and oscillate
            _ => {
                let oscillation = ((cycle - 49) as f64 * 0.2).sin() * 2.0;
                price = 100.0 + oscillation;
                range_high = price + 5.0;
                range_low = price - 5.0;
                let open = price - oscillation * 0.3;
                let close = price;
                let high = open.max(close) + 0.5;
                let low = open.min(close) - 0.5;
                Bar::new(open, high, low, close, volume, timestamp)
            }
        };

        bars.push(bar);
    }

    bars
}

/// Generate clear ZigZag swings with defined pivots
/// ZigZagCandle needs N-bar swing highs/lows where center > all N neighbors on both sides
/// We create clear peaks every 10 bars with 5-bar approach and 5-bar decline
fn generate_zigzag_swings(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars: Vec<Bar> = Vec::with_capacity(count);

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 3600);
        let volume = 1000.0;

        // Create alternating peaks and troughs every 10 bars
        // Pattern: 5 bars rising to peak, 5 bars falling to trough
        let cycle = i % 20;
        let base = 100.0;

        let close = match cycle {
            // Rising to peak at bar 5
            0 => base,
            1 => base + 2.0,
            2 => base + 4.0,
            3 => base + 6.0,
            4 => base + 8.0,
            5 => base + 10.0,  // PEAK - higher than 4 neighbors on each side
            6 => base + 8.0,
            7 => base + 6.0,
            8 => base + 4.0,
            9 => base + 2.0,
            // Falling to trough at bar 15
            10 => base,
            11 => base - 2.0,
            12 => base - 4.0,
            13 => base - 6.0,
            14 => base - 8.0,
            15 => base - 10.0,  // TROUGH - lower than 4 neighbors on each side
            16 => base - 8.0,
            17 => base - 6.0,
            18 => base - 4.0,
            19 => base - 2.0,
            _ => base,
        };

        let prev_close = if i > 0 { bars[i - 1].close } else { close };
        let open = prev_close;
        let high = open.max(close) + 0.5;
        let low = open.min(close) - 0.5;

        bars.push(Bar::new(open, high, low, close, volume, timestamp));
    }

    bars
}

/// Generate tick-level data simulation
/// Each bar represents aggregated tick activity with varying tick counts
/// Volume = number of ticks, and we create patterns in tick distribution
fn generate_tick_data(count: usize, start_ts: i64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(count);
    let mut price = 100.0;

    for i in 0..count {
        let timestamp = start_ts + (i as i64 * 60);  // 1-minute bars for ticks
        let hour = (i / 60) % 24;

        // Tick count varies by "session"
        let base_ticks = match hour {
            9..=11 => 500.0,   // Morning session - high activity
            12..=13 => 200.0,  // Lunch - low activity
            14..=16 => 600.0,  // Afternoon - highest
            _ => 100.0,        // Off-hours - minimal
        };

        // Add randomness using deterministic pattern
        let tick_noise = 1.0 + (i as f64 * 0.7).sin() * 0.5;
        let tick_count = base_ticks * tick_noise;

        // Price moves proportional to tick imbalance
        let buy_ratio = 0.5 + (i as f64 * 0.1).sin() * 0.2;
        let imbalance = buy_ratio - 0.5;  // -0.2 to +0.2
        let price_change = imbalance * tick_count.sqrt() * 0.01;

        price *= 1.0 + price_change;
        price = price.clamp(80.0, 120.0);

        let open = price / (1.0 + price_change);
        let close = price;
        let volatility = tick_count.sqrt() * 0.02;
        let high = open.max(close) + volatility;
        let low = open.min(close) - volatility;

        // Volume = tick count (this is what TickVolumeAnalyzer expects)
        bars.push(Bar::new(open, high, low, close, tick_count, timestamp));
    }

    bars
}

/// Get recommended data type for an indicator category
pub fn recommended_data_type(indicator_name: &str) -> DataType {
    let name_lower = indicator_name.to_lowercase();

    // Specific indicators that need exact matching (check before generic patterns)
    // ZigzagCandle needs ZigZagSwings, not CandlePatterns
    if name_lower == "zigzagcandle" || name_lower == "zigzag_candle" {
        return DataType::ZigZagSwings;
    }
    // TickVolume needs TickData, not Calendar
    if name_lower == "tickvolume" || name_lower == "tick_volume" || name_lower == "tickvol" {
        return DataType::TickData;
    }

    // Candle patterns
    if name_lower.contains("hammer") || name_lower.contains("doji") ||
       name_lower.contains("engulf") || name_lower.contains("harami") ||
       name_lower.contains("star") || name_lower.contains("marubozu") ||
       name_lower.contains("tweezer") || name_lower.contains("piercing") ||
       name_lower.contains("cloud") || name_lower.contains("soldiers") ||
       name_lower.contains("crows") || name_lower.contains("candle") ||
       name_lower.contains("pattern")
    {
        return DataType::CandlePatterns;
    }

    // Gaps/FVG
    if name_lower.contains("fvg") || name_lower.contains("gap") ||
       name_lower.contains("liq")
    {
        return DataType::Gaps;
    }

    // Divergence
    if name_lower.contains("div") {
        return DataType::Divergence;
    }

    // Volatility
    if name_lower.contains("breakout") || name_lower.contains("vb") ||
       name_lower.contains("jump") || name_lower.contains("rbv")
    {
        return DataType::VolatilityBreakout;
    }

    // ARCH/GARCH
    if name_lower.contains("arch") || name_lower.contains("garch") {
        return DataType::VolatilityClustering;
    }

    // Structural breaks
    if name_lower.contains("cusum") || name_lower.contains("break") ||
       name_lower.contains("bp")
    {
        return DataType::StructuralBreaks;
    }

    // Squeeze
    if name_lower.contains("squeeze") || name_lower.contains("compress") ||
       name_lower.contains("rcb") || name_lower.contains("wave")
    {
        return DataType::Squeeze;
    }

    // Calendar
    if name_lower.contains("month") || name_lower.contains("qtr") ||
       name_lower.contains("calendar") || name_lower.contains("tenc") ||
       name_lower.contains("som") || name_lower.contains("soq") ||
       name_lower.contains("tick")
    {
        return DataType::Calendar;
    }

    // Entropy/correlation
    if name_lower.contains("entropy") || name_lower.contains("mutual") ||
       name_lower.contains("transfer") || name_lower.contains("mi") ||
       name_lower.contains("te") || name_lower.contains("xmil")
    {
        return DataType::Correlated;
    }

    // Sweep reversion
    if name_lower.contains("sweep") {
        return DataType::SweepReversion;
    }

    // ZigZag patterns
    if name_lower.contains("zigzag") {
        return DataType::ZigZagSwings;
    }

    // Tick volume
    if name_lower.contains("tickvol") || name_lower.contains("tick_vol") {
        return DataType::TickData;
    }

    // Market structure
    if name_lower.contains("cipher") || name_lower.contains("mrf") ||
       name_lower.contains("fractal") || name_lower.contains("ichimoku") ||
       name_lower.contains("bos") || name_lower.contains("avwap")
    {
        return DataType::StrongTrend;
    }

    // Logic gates - need varied input
    if name_lower.contains("logic") || name_lower.contains("thresh") ||
       name_lower.contains("hyst")
    {
        return DataType::Ranging;
    }

    // STFT/spectral
    if name_lower.contains("stft") || name_lower.contains("spectral") ||
       name_lower.contains("fft")
    {
        return DataType::VolatilityClustering;
    }

    // Default
    DataType::Smooth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_all_types() {
        let types = [
            DataType::Smooth,
            DataType::CandlePatterns,
            DataType::Gaps,
            DataType::Divergence,
            DataType::VolatilityBreakout,
            DataType::VolatilityClustering,
            DataType::StructuralBreaks,
            DataType::Squeeze,
            DataType::Calendar,
            DataType::Correlated,
            DataType::StrongTrend,
            DataType::Ranging,
            DataType::SweepReversion,
            DataType::ZigZagSwings,
            DataType::TickData,
        ];

        for data_type in types {
            let bars = generate_bars(data_type, 100, 1704067200);
            assert_eq!(bars.len(), 100, "{:?} should generate 100 bars", data_type);

            for (i, bar) in bars.iter().enumerate() {
                assert!(bar.high >= bar.low, "{:?} bar {} high >= low", data_type, i);
                assert!(bar.high >= bar.open, "{:?} bar {} high >= open", data_type, i);
                assert!(bar.high >= bar.close, "{:?} bar {} high >= close", data_type, i);
                assert!(bar.low <= bar.open, "{:?} bar {} low <= open", data_type, i);
                assert!(bar.low <= bar.close, "{:?} bar {} low <= close", data_type, i);
                assert!(bar.volume > 0.0, "{:?} bar {} volume > 0", data_type, i);
            }
        }
    }

    #[test]
    fn test_candle_patterns_has_patterns() {
        let bars = generate_bars(DataType::CandlePatterns, 100, 1704067200);

        // Check for hammer pattern (long lower shadow)
        let has_hammer = bars.iter().any(|b| {
            let body = (b.close - b.open).abs();
            let lower_shadow = b.open.min(b.close) - b.low;
            lower_shadow > body * 2.0
        });
        assert!(has_hammer, "Should have hammer-like patterns");

        // Check for doji (small body)
        let has_doji = bars.iter().any(|b| {
            let body = (b.close - b.open).abs();
            let range = b.high - b.low;
            body < range * 0.1
        });
        assert!(has_doji, "Should have doji-like patterns");
    }

    #[test]
    fn test_gaps_has_gaps() {
        let bars = generate_bars(DataType::Gaps, 100, 1704067200);

        // Check for gaps between consecutive bars
        let mut gap_count = 0;
        for i in 1..bars.len() {
            let prev = &bars[i - 1];
            let curr = &bars[i];

            // Gap up: current low > previous high
            if curr.low > prev.high {
                gap_count += 1;
            }
            // Gap down: current high < previous low
            if curr.high < prev.low {
                gap_count += 1;
            }
        }

        assert!(gap_count >= 3, "Should have at least 3 gaps, found {}", gap_count);
    }

    #[test]
    fn test_recommended_data_type() {
        assert_eq!(recommended_data_type("Hammer"), DataType::CandlePatterns);
        assert_eq!(recommended_data_type("FvgDetector"), DataType::Gaps);
        assert_eq!(recommended_data_type("RsiDiv"), DataType::Divergence);
        assert_eq!(recommended_data_type("VolatilityBreakout"), DataType::VolatilityBreakout);
        assert_eq!(recommended_data_type("ArchLm"), DataType::VolatilityClustering);
        assert_eq!(recommended_data_type("BpCusum"), DataType::StructuralBreaks);
        assert_eq!(recommended_data_type("Rcb"), DataType::Squeeze);
        assert_eq!(recommended_data_type("MonthTurn"), DataType::Calendar);
        assert_eq!(recommended_data_type("MutualInformation"), DataType::Correlated);
        assert_eq!(recommended_data_type("Sma"), DataType::Smooth);
        // New types
        assert_eq!(recommended_data_type("SweepRev"), DataType::SweepReversion);
        assert_eq!(recommended_data_type("ZigzagCandle"), DataType::ZigZagSwings);
        assert_eq!(recommended_data_type("TickVolume"), DataType::TickData);
    }

    #[test]
    fn test_sweep_reversion_has_sweeps() {
        let bars = generate_bars(DataType::SweepReversion, 200, 1704067200);

        // Check for sweep pattern: high exceeds range then closes low
        let mut found_sweep = false;
        for i in 41..bars.len() {
            let bar = &bars[i];
            // Sweep: large range where high is far from close
            let range = bar.high - bar.low;
            let close_position = (bar.close - bar.low) / range;
            if range > 5.0 && close_position < 0.3 {
                found_sweep = true;
                break;
            }
        }
        assert!(found_sweep, "Should have sweep patterns");
    }

    #[test]
    fn test_zigzag_has_swings() {
        let bars = generate_bars(DataType::ZigZagSwings, 100, 1704067200);

        // Check for clear swing high/low points
        let mut swing_highs = 0;
        let mut swing_lows = 0;

        for i in 20..bars.len() - 20 {
            // Local max
            if (0..20).all(|j| bars[i].high >= bars[i - j - 1].high) &&
               (0..20).all(|j| bars[i].high >= bars[i + j + 1].high)
            {
                swing_highs += 1;
            }
            // Local min
            if (0..20).all(|j| bars[i].low <= bars[i - j - 1].low) &&
               (0..20).all(|j| bars[i].low <= bars[i + j + 1].low)
            {
                swing_lows += 1;
            }
        }

        assert!(swing_highs >= 1 || swing_lows >= 1, "Should have swing points");
    }

    #[test]
    fn test_tick_data_has_varying_volume() {
        let bars = generate_bars(DataType::TickData, 1440, 1704067200);  // 24 hours of minute data

        // Check that volume varies significantly (session effects)
        let volumes: Vec<f64> = bars.iter().map(|b| b.volume).collect();
        let max_vol = volumes.iter().cloned().fold(0.0, f64::max);
        let min_vol = volumes.iter().cloned().fold(f64::MAX, f64::min);

        assert!(max_vol / min_vol > 3.0, "Volume should vary by at least 3x across sessions");
    }
}
