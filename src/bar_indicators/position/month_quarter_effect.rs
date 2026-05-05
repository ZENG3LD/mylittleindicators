// Month/Quarter Effect: mean log-returns per month (12) and per quarter (4)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct MonthQuarterEffect {
    month_counts: [usize; 12],
    month_sums: [f64; 12],
    quarter_counts: [usize; 4],
    quarter_sums: [f64; 4],
    last_close: Option<f64>,
    pub month_means: [f64; 12],
    pub quarter_means: [f64; 4],
}

impl Default for MonthQuarterEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl MonthQuarterEffect {
    pub fn new() -> Self {
        Self {
            month_counts: [0; 12],
            month_sums: [0.0; 12],
            quarter_counts: [0; 4],
            quarter_sums: [0.0; 4],
            last_close: None,
            month_means: [0.0; 12],
            quarter_means: [0.0; 4],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.month_counts = [0; 12];
        self.month_sums = [0.0; 12];
        self.quarter_counts = [0; 4];
        self.quarter_sums = [0.0; 4];
        self.last_close = None;
        self.month_means = [0.0; 12];
        self.quarter_means = [0.0; 4];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.month_counts.iter().any(|&c| c > 0) || self.quarter_counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> IndicatorValue {
        // Return average of all non-zero month means
        let month_avg = self.month_means.iter().filter(|&&x| x != 0.0).sum::<f64>()
            / self.month_means.iter().filter(|&&x| x != 0.0).count().max(1) as f64;
        IndicatorValue::Single(month_avg)
    }

    // Caller supplies month (1..=12) and quarter (1..=4)
    pub fn update_with_calendar(&mut self, close: f64, month: u8, quarter: u8) {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            let m = (month.clamp(1, 12) as usize) - 1;
            let q = (quarter.clamp(1, 4) as usize) - 1;
            self.month_counts[m] += 1;
            self.month_sums[m] += r;
            self.month_means[m] = self.month_sums[m] / self.month_counts[m] as f64;
            self.quarter_counts[q] += 1;
            self.quarter_sums[q] += r;
            self.quarter_means[q] = self.quarter_sums[q] / self.quarter_counts[q] as f64;
        }
        self.last_close = Some(close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_quarter_effect_creation() {
        let mqe = MonthQuarterEffect::new();
        assert!(!mqe.is_ready());
    }

    #[test]
    fn test_month_quarter_effect_update() {
        let mut mqe = MonthQuarterEffect::new();
        mqe.update_with_calendar(100.0, 1, 1);
        mqe.update_with_calendar(101.0, 1, 1);
        assert!(mqe.is_ready());
    }

    #[test]
    fn test_month_quarter_effect_means() {
        let mut mqe = MonthQuarterEffect::new();
        for i in 0..12 {
            let price = 100.0 + i as f64;
            let month = (i % 12 + 1) as u8;
            let quarter = (i % 4 + 1) as u8;
            mqe.update_with_calendar(price, month, quarter);
        }
        // Check means are finite
        for mean in &mqe.month_means {
            assert!(mean.is_finite());
        }
        for mean in &mqe.quarter_means {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_month_quarter_effect_reset() {
        let mut mqe = MonthQuarterEffect::new();
        mqe.update_with_calendar(100.0, 1, 1);
        mqe.update_with_calendar(101.0, 1, 1);
        mqe.reset();
        assert!(!mqe.is_ready());
    }
}
