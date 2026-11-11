use crate::config::WorkConfig;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct HolidayCalendar {
    pub years: Vec<i32>,
    pub ranges: Vec<[NaiveDate; 2]>,
}

impl HolidayCalendar {
    pub fn bundled() -> Self {
        Self::parse(include_str!("../../src/data/holidays.json"))
            .expect("bundled holiday calendar is valid")
    }

    pub fn parse(json: &str) -> Result<Self, String> {
        let calendar: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        for [start, end] in &calendar.ranges {
            if start > end || start.year() != end.year() || !calendar.years.contains(&start.year())
            {
                return Err("节假日区间须在已声明的同一年内，且开始不晚于结束".into());
            }
        }
        Ok(calendar)
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        self.ranges
            .iter()
            .any(|[start, end]| *start <= date && date <= *end)
    }
}

pub fn is_last_saturday(date: NaiveDate) -> bool {
    date.weekday() == Weekday::Sat
        && date
            .checked_add_signed(Duration::days(7))
            .is_none_or(|next| next.month() != date.month() || next.year() != date.year())
}

pub fn is_workday(date: NaiveDate, calendar: &HolidayCalendar) -> bool {
    if calendar.contains(date) {
        return false;
    }
    match date.weekday() {
        Weekday::Sun => false,
        Weekday::Sat => is_last_saturday(date),
        _ => true,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CountdownStats {
    pub progress_percent: f64,
    pub remaining_seconds: i64,
    pub guandan_remaining_seconds: i64,
    pub guandan_ready: bool,
    pub is_workday: bool,
    pub status: &'static str,
    pub date: String,
    pub calendar_known: bool,
    pub schedule: WorkConfig,
}

pub fn calculate(
    config: &WorkConfig,
    now: NaiveDateTime,
    calendar: &HolidayCalendar,
) -> Result<CountdownStats, String> {
    let times = config
        .times()?
        .map(|time| i64::from(time.num_seconds_from_midnight()));
    let date = now.date();
    let workday = is_workday(date, calendar);
    let result = crate::clock::calculate(
        times,
        i64::from(now.time().num_seconds_from_midnight()),
        workday,
    )?;
    Ok(CountdownStats {
        progress_percent: result.progress_percent,
        remaining_seconds: result.remaining_seconds,
        guandan_remaining_seconds: result.guandan_remaining_seconds,
        guandan_ready: result.guandan_ready,
        is_workday: workday,
        status: result.status,
        date: date.to_string(),
        calendar_known: calendar.years.contains(&date.year()),
        schedule: config.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        value.parse().unwrap()
    }
    fn stats(value: &str) -> CountdownStats {
        calculate(
            &WorkConfig::default(),
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap(),
            &HolidayCalendar::bundled(),
        )
        .unwrap()
    }

    #[test]
    fn all_working_boundaries_and_lunch_pause() {
        let cases = [
            ("08:59:59", "before_work", 30601, 0.0),
            ("09:00:00", "working", 30600, 0.0),
            ("11:45:00", "lunch_break", 20700, 9900.0 / 25200.0 * 100.0),
            ("12:30:00", "lunch_break", 18000, 9900.0 / 25200.0 * 100.0),
            ("13:15:00", "working", 15300, 9900.0 / 25200.0 * 100.0),
            ("17:30:00", "after_work", 0, 100.0),
            ("23:59:59", "after_work", 0, 100.0),
        ];
        for (time, status, remaining, progress) in cases {
            let result = stats(&format!("2026-11-11 {time}"));
            assert_eq!(result.status, status, "{time}");
            assert_eq!(result.remaining_seconds, remaining, "{time}");
            assert!(
                (result.progress_percent - progress).abs() < 0.0001,
                "{time}"
            );
        }
    }

    #[test]
    fn custom_saturdays_and_holiday_priority() {
        let calendar = HolidayCalendar::bundled();
        assert!(is_workday(date("2026-11-28"), &calendar));
        assert!(!is_workday(date("2026-11-21"), &calendar));
        assert!(!is_workday(date("2026-11-29"), &calendar));
        assert!(is_last_saturday(date("2026-09-26")));
        assert!(!is_workday(date("2026-09-26"), &calendar));
        // Extra national make-up working days are deliberately excluded.
        assert!(!is_workday(date("2026-02-14"), &calendar));
        assert!(!is_workday(date("2026-01-04"), &calendar));
        let result = stats("2026-10-01 12:00:00");
        assert_eq!(result.status, "day_off");
        assert_eq!(result.progress_percent, 0.0);
        assert_eq!(result.remaining_seconds, 0);
    }

    #[test]
    fn midnight_month_year_and_leap_day() {
        assert_eq!(stats("2026-11-30 23:59:59").status, "after_work");
        assert_eq!(stats("2026-12-01 00:00:00").progress_percent, 0.0);
        assert!(is_last_saturday(date("2026-12-26")));
        assert!(is_last_saturday(date("2028-02-26")));
        assert_eq!(stats("2028-02-29 09:00:00").status, "working");
        assert!(!stats("2027-01-01 09:00:00").calendar_known);
    }

    #[test]
    fn rejects_bad_calendar_and_zero_duration() {
        assert!(
            HolidayCalendar::parse(r#"{"years":[2026],"ranges":[["2026-05-05","2026-05-01"]]}"#)
                .is_err()
        );
        let config = WorkConfig {
            end_time: "09:00".into(),
            ..WorkConfig::default()
        };
        assert!(
            calculate(
                &config,
                date("2026-11-11").and_hms_opt(9, 0, 0).unwrap(),
                &HolidayCalendar::bundled()
            )
            .is_err()
        );
    }
}
