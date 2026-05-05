// Day-of-Month and Week-of-Quarter Effect: mean log-returns by day (1..31) and by week (1..13)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct DayOfMonthWeekOfQuarterEffect {
    dom_counts: [usize; 31],
    dom_sums: [f64; 31],
    woq_counts: [usize; 13],
    woq_sums: [f64; 13],
    last_close: Option<f64>,
    pub dom_means: [f64; 31],
    pub woq_means: [f64; 13],
}

impl Default for DayOfMonthWeekOfQuarterEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl DayOfMonthWeekOfQuarterEffect {
    pub fn new() -> Self {
        Self {
            dom_counts: [0; 31],
            dom_sums: [0.0; 31],
            woq_counts: [0; 13],
            woq_sums: [0.0; 13],
            last_close: None,
            dom_means: [0.0; 31],
            woq_means: [0.0; 13],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.dom_counts = [0; 31];
        self.dom_sums = [0.0; 31];
        self.woq_counts = [0; 13];
        self.woq_sums = [0.0; 13];
        self.last_close = None;
        self.dom_means = [0.0; 31];
        self.woq_means = [0.0; 13];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dom_counts.iter().any(|&c| c > 0) || self.woq_counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> IndicatorValue {
        // Return average of all non-zero day-of-month means
        let dom_avg = self.dom_means.iter().filter(|&&x| x != 0.0).sum::<f64>()
            / self.dom_means.iter().filter(|&&x| x != 0.0).count().max(1) as f64;
        IndicatorValue::Single(dom_avg)
    }

    // Caller supplies dom in 1..=31 and woq in 1..=13
    pub fn update_with_calendar(&mut self, close: f64, day_of_month: u8, week_of_quarter: u8) {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            let d = (day_of_month.clamp(1, 31) as usize) - 1;
            let w = (week_of_quarter.clamp(1, 13) as usize) - 1;
            self.dom_counts[d] += 1;
            self.dom_sums[d] += r;
            self.dom_means[d] = self.dom_sums[d] / self.dom_counts[d] as f64;
            self.woq_counts[w] += 1;
            self.woq_sums[w] += r;
            self.woq_means[w] = self.woq_sums[w] / self.woq_counts[w] as f64;
        }
        self.last_close = Some(close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dayofmonth_weekofquarter_creation() {
        let eff = DayOfMonthWeekOfQuarterEffect::new();
        assert!(!eff.is_ready());
    }

    #[test]
    fn test_dayofmonth_weekofquarter_update() {
        let mut eff = DayOfMonthWeekOfQuarterEffect::new();
        eff.update_with_calendar(100.0, 1, 1);
        eff.update_with_calendar(101.0, 2, 1);
        assert!(eff.is_ready());
    }

    #[test]
    fn test_dayofmonth_weekofquarter_means() {
        let mut eff = DayOfMonthWeekOfQuarterEffect::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            let dom = (i % 31 + 1) as u8;
            let woq = (i % 13 + 1) as u8;
            eff.update_with_calendar(price, dom, woq);
        }
        // Check means are finite
        for mean in &eff.dom_means {
            assert!(mean.is_finite());
        }
        for mean in &eff.woq_means {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_dayofmonth_weekofquarter_reset() {
        let mut eff = DayOfMonthWeekOfQuarterEffect::new();
        eff.update_with_calendar(100.0, 1, 1);
        eff.update_with_calendar(101.0, 2, 1);
        eff.reset();
        assert!(!eff.is_ready());
    }
}
