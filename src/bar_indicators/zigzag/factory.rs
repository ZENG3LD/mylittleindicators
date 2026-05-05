// Minimal ZigZag factory/wrapper for divergence module
// (c) 2024

use super::{
    zigzag_classic::ZigZagClassic,
    zigzag_atr::ZigZagAtr,
    zigzag_candle::ZigZagCandle,
    zigzag_lookahead::ZigZagLookahead,
    zigzag_time::ZigZagTime,
};

pub enum ZigZagAlgo {
    Classic(ZigZagClassic),
    Atr(ZigZagAtr),
    Candle(ZigZagCandle),
    Lookahead(ZigZagLookahead),
    Time(ZigZagTime),
}

impl ZigZagAlgo {
    pub fn swings(&self) -> &[(usize, f64)] {
        match self {
            ZigZagAlgo::Classic(z) => z.swings().as_slice(),
            ZigZagAlgo::Atr(z) => z.swings().as_slice(),
            ZigZagAlgo::Candle(z) => z.swings().as_slice(),
            ZigZagAlgo::Lookahead(z) => z.swings().as_slice(),
            ZigZagAlgo::Time(z) => z.swings().as_slice(),
        }
    }
    pub fn last_swing(&self) -> Option<(usize, f64)> {
        match self {
            ZigZagAlgo::Classic(z) => z.last_swing(),
            ZigZagAlgo::Atr(z) => z.last_swing(),
            ZigZagAlgo::Candle(z) => z.last_swing(),
            ZigZagAlgo::Lookahead(z) => z.last_swing(),
            ZigZagAlgo::Time(z) => z.last_swing(),
        }
    }
}

pub struct ZigZagFactory;

impl ZigZagFactory {
    pub fn create_classic(period: usize, threshold_percent: Option<f64>, threshold_abs: Option<f64>) -> ZigZagAlgo {
        ZigZagAlgo::Classic(ZigZagClassic::new(period, threshold_percent, threshold_abs))
    }
    pub fn create_atr(period: usize, atr_mult: f64, atr: crate::bar_indicators::volatility::atr::Atr) -> ZigZagAlgo {
        ZigZagAlgo::Atr(ZigZagAtr::new_compatible(period, atr_mult, atr))
    }
    pub fn create_candle(period: usize, swing_bars: usize) -> ZigZagAlgo {
        ZigZagAlgo::Candle(ZigZagCandle::new(period, swing_bars))
    }
    pub fn create_lookahead(period: usize, lookahead: usize) -> ZigZagAlgo {
        ZigZagAlgo::Lookahead(ZigZagLookahead::new(period, lookahead))
    }
    pub fn create_time(period: usize, min_bars: usize) -> ZigZagAlgo {
        ZigZagAlgo::Time(ZigZagTime::new(period, min_bars))
    }
}






















