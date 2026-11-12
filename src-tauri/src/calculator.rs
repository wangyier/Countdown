use crate::config::WorkConfig;
use chrono::{NaiveDateTime, Timelike};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CountdownStats {
    pub progress_percent: f64,
    pub remaining_seconds: i64,
    pub guandan_remaining_seconds: i64,
    pub guandan_ready: bool,
    pub status: &'static str,
    pub date: String,
    pub schedule: WorkConfig,
}

pub fn calculate(config: &WorkConfig, now: NaiveDateTime) -> Result<CountdownStats, String> {
    let times = config
        .times()?
        .map(|time| i64::from(time.num_seconds_from_midnight()));
    let date = now.date();
    let result = crate::clock::calculate(times, i64::from(now.time().num_seconds_from_midnight()))?;
    Ok(CountdownStats {
        progress_percent: result.progress_percent,
        remaining_seconds: result.remaining_seconds,
        guandan_remaining_seconds: result.guandan_remaining_seconds,
        guandan_ready: result.guandan_ready,
        status: result.status,
        date: date.to_string(),
        schedule: config.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(value: &str) -> CountdownStats {
        calculate(
            &WorkConfig::default(),
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").unwrap(),
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
    fn every_date_uses_the_configured_schedule() {
        for date in [
            "2026-11-21",
            "2026-11-29",
            "2026-09-26",
            "2026-10-01",
            "2027-01-01",
        ] {
            let result = stats(&format!("{date} 09:00:00"));
            assert_eq!(result.status, "working");
            assert_eq!(result.remaining_seconds, 30600);
            assert_eq!(stats(&format!("{date} 12:00:00")).status, "lunch_break");
        }
    }

    #[test]
    fn midnight_month_year_and_leap_day() {
        assert_eq!(stats("2026-11-30 23:59:59").status, "after_work");
        assert_eq!(stats("2026-12-01 00:00:00").progress_percent, 0.0);
        assert_eq!(stats("2028-02-29 09:00:00").status, "working");
        assert_eq!(stats("2027-01-01 09:00:00").status, "working");
    }

    #[test]
    fn rejects_zero_duration() {
        let config = WorkConfig {
            end_time: "09:00".into(),
            ..WorkConfig::default()
        };
        assert!(
            calculate(
                &config,
                NaiveDateTime::parse_from_str("2026-11-11 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .is_err()
        );
    }
}
