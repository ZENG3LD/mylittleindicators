//! position_catalog.rs: Catalog of Position/Seasonality/Temporal indicators
//!
//! Position indicators track temporal patterns, seasonality effects, and price positioning.
//! Contains IndicatorSignature definitions for 19 position-based indicators.

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Position;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Anchored VWAP Distance - Distance from anchored VWAP
pub fn signature_avwap_distance() -> IndicatorSignature {
    IndicatorSignature::builder("AVWAP_DIST", CATEGORY)
        .name("Anchored VWAP Distance")
        .description("Relative distance to anchored VWAP")
        .metadata("type", "distance")
        .machine_id(BarIndicatorId::AvwapDist) // TODO: Add to enum
        // Note: "AVWAP_DIST" is already the main ID, no need for alias
        .alias("AvwapDist")
        .alias("avwap_dist")
        .alias("ANCHOREDVWAPDISTANCE")
        .alias("AnchoredVWAPDistance")
        .alias("anchoredvwapdistance")
        .alias("anchored_vwap_distance")
        .alias("ANCHORED_VWAP_DISTANCE")
        .alias("Anchored_Vwap_Distance")
        .build()
}

/// VWAP Distance - Distance from standard VWAP
pub fn signature_vwap_distance() -> IndicatorSignature {
    IndicatorSignature::builder("VWAP_DIST", CATEGORY)
        .name("VWAP Distance")
        .description("Relative distance to VWAP")
        .metadata("type", "distance")
        .machine_id(BarIndicatorId::VwapDist) // TODO: Add to enum
        // Note: "VWAP_DIST" is already the main ID, no need for alias
        .alias("VwapDist")
        .alias("vwap_dist")
        .alias("VWAPDISTANCE")
        .alias("VWAPDistance")
        .alias("vwapdistance")
        .alias("vwap_distance")
        .alias("VWAP_DISTANCE")
        .alias("Vwap_Distance")
        .build()
}

/// Central Pivot Range - Central pivot point for intraday trading
pub fn signature_central_pivot_range() -> IndicatorSignature {
    IndicatorSignature::builder("CPR", CATEGORY)
        .name("Central Pivot Range")
        .description("Central pivot range for support/resistance")
        .metadata("type", "pivot")
        .metadata("timeframe", "daily")
        .machine_id(BarIndicatorId::Cpr) // TODO: Add to enum
        // Note: "CPR" is already the main ID, no need for alias
        .alias("Cpr")
        .alias("cpr")
        .alias("CENTRALPIVOTRANGE")
        .alias("CentralPivotRange")
        .alias("centralpivotrange")
        .alias("central_pivot_range")
        .alias("CENTRAL_PIVOT_RANGE")
        .alias("Central_Pivot_Range")
        .build()
}

/// Day of Week in Month - Which week of the month (1-5)
pub fn signature_day_of_week_in_month() -> IndicatorSignature {
    IndicatorSignature::builder("DAY_WEEK_MONTH", CATEGORY)
        .name("Day of Week in Month")
        .description("Which week of the month (1st, 2nd, 3rd, 4th, 5th)")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::DayWeekMonth) // TODO: Add to enum
        // Note: "DAY_WEEK_MONTH" is already the main ID, no need for alias
        .alias("DayWeekMonth")
        .alias("day_week_month")
        .alias("DAYOFWEEKINMONTH")
        .alias("DayofWeekinMonth")
        .alias("dayofweekinmonth")
        .alias("day_of_week_in_month")
        .alias("DAY_OF_WEEK_IN_MONTH")
        .alias("Day_Of_Week_In_Month")
        .build()
}

/// Day of Month & Week of Quarter Effect
pub fn signature_dayofmonth_weekofquarter_effect() -> IndicatorSignature {
    IndicatorSignature::builder("DOM_WOQ", CATEGORY)
        .name("Day of Month & Week of Quarter Effect")
        .description("Combined day of month and week of quarter seasonality")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::DomWoq) // TODO: Add to enum
        // Note: "DOM_WOQ" is already the main ID, no need for alias
        .alias("DomWoq")
        .alias("dom_woq")
        .alias("DAYOFMONTH&WEEKOFQUARTEREFFECT")
        .alias("DayofMonth&WeekofQuarterEffect")
        .alias("dayofmonth&weekofquartereffect")
        .alias("day_of_month_&_week_of_quarter_effect")
        .alias("DAY_OF_MONTH_&_WEEK_OF_QUARTER_EFFECT")
        .alias("Day_Of_Month_&_Week_Of_Quarter_Effect")
        .build()
}

/// Distance to Levels - Distance to key support/resistance levels
pub fn signature_distance_to_levels() -> IndicatorSignature {
    IndicatorSignature::builder("DIST_LEVELS", CATEGORY)
        .name("Distance to Levels")
        .description("Distance to support/resistance levels")
        .add_constraint(ParamConstraint::period(10, 500, 50))
        .metadata("type", "distance")
        .machine_id(BarIndicatorId::DistLevels) // TODO: Add to enum
        // Note: "DIST_LEVELS" is already the main ID, no need for alias
        .alias("DistLevels")
        .alias("dist_levels")
        .alias("DISTANCETOLEVELS")
        .alias("DistancetoLevels")
        .alias("distancetolevels")
        .alias("distance_to_levels")
        .alias("DISTANCE_TO_LEVELS")
        .alias("Distance_To_Levels")
        .build()
}

/// Holiday & Weekend Proximity - Days until/since holiday or weekend
pub fn signature_holiday_weekend_proximity() -> IndicatorSignature {
    IndicatorSignature::builder("HOLIDAY_PROX", CATEGORY)
        .name("Holiday & Weekend Proximity")
        .description("Proximity to holidays and weekends")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::HolidayProx) // TODO: Add to enum
        // Note: "HOLIDAY_PROX" is already the main ID, no need for alias
        .alias("HolidayProx")
        .alias("holiday_prox")
        .alias("HOLIDAY&WEEKENDPROXIMITY")
        .alias("Holiday&WeekendProximity")
        .alias("holiday&weekendproximity")
        .alias("holiday_&_weekend_proximity")
        .alias("HOLIDAY_&_WEEKEND_PROXIMITY")
        .alias("Holiday_&_Weekend_Proximity")
        .build()
}

/// Hour of Day Effect - Intraday hour effect (0-23)
pub fn signature_hour_of_day_effect() -> IndicatorSignature {
    IndicatorSignature::builder("HOUR_DAY", CATEGORY)
        .name("Hour of Day Effect")
        .description("Hour of the day effect (0-23)")
        .metadata("type", "temporal")
        .metadata("category", "intraday")
        .machine_id(BarIndicatorId::HourDay) // TODO: Add to enum
        // Note: "HOUR_DAY" is already the main ID, no need for alias
        .alias("HourDay")
        .alias("hour_day")
        .alias("HOUROFDAYEFFECT")
        .alias("HourofDayEffect")
        .alias("hourofdayeffect")
        .alias("hour_of_day_effect")
        .alias("HOUR_OF_DAY_EFFECT")
        .alias("Hour_Of_Day_Effect")
        .build()
}

/// Month & Quarter Effect - Month (1-12) and Quarter (1-4)
pub fn signature_month_quarter_effect() -> IndicatorSignature {
    IndicatorSignature::builder("MONTH_QTR", CATEGORY)
        .name("Month & Quarter Effect")
        .description("Month and quarter seasonality effects")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::MonthQtr) // TODO: Add to enum
        // Note: "MONTH_QTR" is already the main ID, no need for alias
        .alias("MonthQtr")
        .alias("month_qtr")
        .alias("MONTH&QUARTEREFFECT")
        .alias("Month&QuarterEffect")
        .alias("month&quartereffect")
        .alias("month_&_quarter_effect")
        .alias("MONTH_&_QUARTER_EFFECT")
        .alias("Month_&_Quarter_Effect")
        .build()
}

/// Month Turn Effect - Beginning/end of month effect
pub fn signature_month_turn_effect() -> IndicatorSignature {
    IndicatorSignature::builder("MONTH_TURN", CATEGORY)
        .name("Month Turn Effect")
        .description("Turn of the month effect")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::MonthTurn) // TODO: Add to enum
        // Note: "MONTH_TURN" is already the main ID, no need for alias
        .alias("MonthTurn")
        .alias("month_turn")
        .alias("MONTHTURNEFFECT")
        .alias("MonthTurnEffect")
        .alias("monthturneffect")
        .alias("month_turn_effect")
        .alias("MONTH_TURN_EFFECT")
        .alias("Month_Turn_Effect")
        .build()
}

/// Quarter Turn Effect - Beginning/end of quarter effect
pub fn signature_quarter_turn_effect() -> IndicatorSignature {
    IndicatorSignature::builder("QTR_TURN", CATEGORY)
        .name("Quarter Turn Effect")
        .description("Turn of the quarter effect")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::QtrTurn) // TODO: Add to enum
        // Note: "QTR_TURN" is already the main ID, no need for alias
        .alias("QtrTurn")
        .alias("qtr_turn")
        .alias("QUARTERTURNEFFECT")
        .alias("QuarterTurnEffect")
        .alias("quarterturneffect")
        .alias("quarter_turn_effect")
        .alias("QUARTER_TURN_EFFECT")
        .alias("Quarter_Turn_Effect")
        .build()
}

/// Relative Trend Position - Position relative to trend indicators
pub fn signature_relative_trend_position() -> IndicatorSignature {
    IndicatorSignature::builder("REL_TREND_POS", CATEGORY)
        .name("Relative Trend Position")
        .description("Relative position to SMA200 and anchored VWAP")
        .add_constraint(ParamConstraint::period(50, 500, 200))
        .metadata("type", "position")
        .machine_id(BarIndicatorId::RelTrendPos) // TODO: Add to enum
        // Note: "REL_TREND_POS" is already the main ID, no need for alias
        .alias("RelTrendPos")
        .alias("rel_trend_pos")
        .alias("RELATIVETRENDPOSITION")
        .alias("RelativeTrendPosition")
        .alias("relativetrendposition")
        .alias("relative_trend_position")
        .alias("RELATIVE_TREND_POSITION")
        .alias("Relative_Trend_Position")
        .build()
}

/// Session Effect - Trading session indicator (pre-market, market, post-market)
pub fn signature_session_effect() -> IndicatorSignature {
    IndicatorSignature::builder("SESSION", CATEGORY)
        .name("Session Effect")
        .description("Trading session indicator")
        .metadata("type", "temporal")
        .metadata("category", "intraday")
        .machine_id(BarIndicatorId::Session) // TODO: Add to enum
        // Note: "SESSION" is already the main ID, no need for alias
        .alias("Session")
        .alias("session")
        .alias("SESSIONEFFECT")
        .alias("SessionEffect")
        .alias("sessioneffect")
        .alias("session_effect")
        .alias("SESSION_EFFECT")
        .alias("Session_Effect")
        .build()
}

/// Start/End of Month Flags
pub fn signature_start_end_of_month_flags() -> IndicatorSignature {
    IndicatorSignature::builder("SOM_EOM", CATEGORY)
        .name("Start/End of Month Flags")
        .description("Binary flags for start and end of month")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::SomEom) // TODO: Add to enum
        // Note: "SOM_EOM" is already the main ID, no need for alias
        .alias("SomEom")
        .alias("som_eom")
        .alias("STARTENDOFMONTHFLAGS")
        .alias("StartEndofMonthFlags")
        .alias("startendofmonthflags")
        .alias("start_end_of_month_flags")
        .alias("START_END_OF_MONTH_FLAGS")
        .alias("Start_End_Of_Month_Flags")
        .build()
}

/// Start/End of Quarter Flags
pub fn signature_start_end_of_quarter_flags() -> IndicatorSignature {
    IndicatorSignature::builder("SOQ_EOQ", CATEGORY)
        .name("Start/End of Quarter Flags")
        .description("Binary flags for start and end of quarter")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::SoqEoq) // TODO: Add to enum
        // Note: "SOQ_EOQ" is already the main ID, no need for alias
        .alias("SoqEoq")
        .alias("soq_eoq")
        .alias("STARTENDOFQUARTERFLAGS")
        .alias("StartEndofQuarterFlags")
        .alias("startendofquarterflags")
        .alias("start_end_of_quarter_flags")
        .alias("START_END_OF_QUARTER_FLAGS")
        .alias("Start_End_Of_Quarter_Flags")
        .build()
}

/// Start/End of Week Flags
pub fn signature_start_end_of_week_flags() -> IndicatorSignature {
    IndicatorSignature::builder("SOW_EOW", CATEGORY)
        .name("Start/End of Week Flags")
        .description("Binary flags for start and end of week")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::SowEow) // TODO: Add to enum
        // Note: "SOW_EOW" is already the main ID, no need for alias
        .alias("SowEow")
        .alias("sow_eow")
        .alias("STARTENDOFWEEKFLAGS")
        .alias("StartEndofWeekFlags")
        .alias("startendofweekflags")
        .alias("start_end_of_week_flags")
        .alias("START_END_OF_WEEK_FLAGS")
        .alias("Start_End_Of_Week_Flags")
        .build()
}

/// Week in Month Effect - Which week of the month
pub fn signature_week_in_month_effect() -> IndicatorSignature {
    IndicatorSignature::builder("WEEK_MONTH", CATEGORY)
        .name("Week in Month Effect")
        .description("Week of the month effect (1-5)")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::WeekMonth) // TODO: Add to enum
        // Note: "WEEK_MONTH" is already the main ID, no need for alias
        .alias("WeekMonth")
        .alias("week_month")
        .alias("WEEKINMONTHEFFECT")
        .alias("WeekinMonthEffect")
        .alias("weekinmontheffect")
        .alias("week_in_month_effect")
        .alias("WEEK_IN_MONTH_EFFECT")
        .alias("Week_In_Month_Effect")
        .build()
}

/// Weekday Effect - Day of week (Monday=1, Sunday=7)
pub fn signature_weekday_effect() -> IndicatorSignature {
    IndicatorSignature::builder("WEEKDAY", CATEGORY)
        .name("Weekday Effect")
        .description("Day of the week seasonality")
        .metadata("type", "temporal")
        .metadata("category", "calendar")
        .machine_id(BarIndicatorId::Weekday) // TODO: Add to enum
        // Note: "WEEKDAY" is already the main ID, no need for alias
        .alias("Weekday")
        .alias("weekday")
        .alias("WEEKDAYEFFECT")
        .alias("WeekdayEffect")
        .alias("weekdayeffect")
        .alias("weekday_effect")
        .alias("WEEKDAY_EFFECT")
        .alias("Weekday_Effect")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all Position indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("AVWAP_DIST", signature_avwap_distance as fn() -> IndicatorSignature),
    ("VWAP_DIST", signature_vwap_distance as fn() -> IndicatorSignature),
    ("CPR", signature_central_pivot_range as fn() -> IndicatorSignature),
    ("DAY_WEEK_MONTH", signature_day_of_week_in_month as fn() -> IndicatorSignature),
    ("DOM_WOQ", signature_dayofmonth_weekofquarter_effect as fn() -> IndicatorSignature),
    ("DIST_LEVELS", signature_distance_to_levels as fn() -> IndicatorSignature),
    ("HOLIDAY_PROX", signature_holiday_weekend_proximity as fn() -> IndicatorSignature),
    ("HOUR_DAY", signature_hour_of_day_effect as fn() -> IndicatorSignature),
    ("MONTH_QTR", signature_month_quarter_effect as fn() -> IndicatorSignature),
    ("MONTH_TURN", signature_month_turn_effect as fn() -> IndicatorSignature),
    ("QTR_TURN", signature_quarter_turn_effect as fn() -> IndicatorSignature),
    ("REL_TREND_POS", signature_relative_trend_position as fn() -> IndicatorSignature),
    ("SESSION", signature_session_effect as fn() -> IndicatorSignature),
    ("SOM_EOM", signature_start_end_of_month_flags as fn() -> IndicatorSignature),
    ("SOQ_EOQ", signature_start_end_of_quarter_flags as fn() -> IndicatorSignature),
    ("SOW_EOW", signature_start_end_of_week_flags as fn() -> IndicatorSignature),
    ("WEEK_MONTH", signature_week_in_month_effect as fn() -> IndicatorSignature),
    ("WEEKDAY", signature_weekday_effect as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static POSITION_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();

    for &(main_id, func) in BASE_CATALOG {
        // Call function once to get signature with aliases
        let sig = func();

        // Insert main ID
        m.insert(main_id.to_string(), func);

        // Auto-insert all aliases from signature
        for alias in &sig.aliases {
            m.insert(alias.clone(), func);
        }
    }

    m
});

// ============================================================================
// Public API
// ============================================================================

/// Get indicator signature by ID
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    POSITION_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators in this category
pub fn count() -> usize {
    BASE_CATALOG.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count() {
        assert_eq!(count(), 18); // 18 position indicators
    }

    #[test]
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }
}
