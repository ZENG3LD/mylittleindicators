// Calendar / seasonality indicators (Phase 1 reorg 2026-06-16).
// Dissolved out of position/ (which mixed calendar effects with price-distance):
// these answer "what time-period is it?" — bucket indices / proximity / flags.
// position/'s price-distance indicators went to levels/. position_catalog kept
// here intact (legacy metadata, keyed by IndicatorId — dies at the split).
pub mod day_of_week_in_month;
pub mod dayofmonth_weekofquarter_effect;
pub mod holiday_weekend_proximity;
pub mod hour_of_day_effect;
pub mod month_quarter_effect;
pub mod month_turn_effect;
pub mod quarter_turn_effect;
pub mod session_effect;
pub mod start_end_of_month_flags;
pub mod start_end_of_quarter_flags;
pub mod start_end_of_week_flags;
pub mod week_in_month_effect;
pub mod weekday_effect;
pub mod time_encoders;

pub use day_of_week_in_month::*;
pub use dayofmonth_weekofquarter_effect::*;
pub use holiday_weekend_proximity::*;
pub use hour_of_day_effect::*;
pub use month_quarter_effect::*;
pub use month_turn_effect::*;
pub use quarter_turn_effect::*;
pub use session_effect::*;
pub use start_end_of_month_flags::*;
pub use start_end_of_quarter_flags::*;
pub use start_end_of_week_flags::*;
pub use week_in_month_effect::*;
pub use weekday_effect::*;
pub use time_encoders::*;
