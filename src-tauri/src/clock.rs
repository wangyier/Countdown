// Pure time-of-day calculation, independent of the desktop runtime and time zone.
// Can be tested with rustc alone when Cargo dependencies are unavailable.
const GUANDAN_TARGET_SECONDS: i64 = 11 * 3600 + 45 * 60;
const GUANDAN_END_SECONDS: i64 = 13 * 3600 + 15 * 60;

#[derive(Debug)]
pub struct ClockStats {
    pub status: &'static str,
    pub progress_percent: f64,
    pub remaining_seconds: i64,
    pub guandan_remaining_seconds: i64,
    pub guandan_ready: bool,
}

pub fn calculate(times: [i64; 4], now: i64, workday: bool) -> Result<ClockStats, &'static str> {
    if now < 0
        || now >= 86400
        || times[0] < 0
        || times[3] >= 86400
        || !times.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err("无效的工作时间");
    }
    let [start, lunch_start, lunch_end, end] = times;
    let total = lunch_start - start + end - lunch_end;
    let (status, elapsed) = if !workday {
        ("day_off", 0)
    } else if now < start {
        ("before_work", 0)
    } else if now < lunch_start {
        ("working", now - start)
    } else if now < lunch_end {
        ("lunch_break", lunch_start - start)
    } else if now < end {
        ("working", lunch_start - start + now - lunch_end)
    } else {
        ("after_work", total)
    };
    Ok(ClockStats {
        status,
        progress_percent: (elapsed as f64 / total as f64 * 100.0).clamp(0.0, 100.0),
        remaining_seconds: if workday { (end - now).max(0) } else { 0 },
        guandan_remaining_seconds: if workday {
            (GUANDAN_TARGET_SECONDS - now).max(0)
        } else {
            0
        },
        guandan_ready: workday && (GUANDAN_TARGET_SECONDS..GUANDAN_END_SECONDS).contains(&now),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const TIMES: [i64; 4] = [32400, 42300, 47700, 63000];

    #[test]
    fn boundary_states() {
        for (time, status, remaining) in [
            (32399, "before_work", 30601),
            (32400, "working", 30600),
            (42299, "working", 20701),
            (42300, "lunch_break", 20700),
            (47699, "lunch_break", 15301),
            (47700, "working", 15300),
            (62999, "working", 1),
            (63000, "after_work", 0),
        ] {
            let stats = calculate(TIMES, time, true).unwrap();
            assert_eq!(stats.status, status);
            assert_eq!(stats.remaining_seconds, remaining);
        }
    }

    #[test]
    fn lunch_and_day_off() {
        let start = calculate(TIMES, 42300, true).unwrap();
        let end = calculate(TIMES, 47700, true).unwrap();
        assert_eq!(start.progress_percent, end.progress_percent);
        assert_eq!(start.remaining_seconds - end.remaining_seconds, 5400);
        let rest = calculate(TIMES, 50000, false).unwrap();
        assert_eq!(rest.status, "day_off");
        assert_eq!(rest.progress_percent, 0.0);
        assert_eq!(rest.remaining_seconds, 0);
    }

    #[test]
    fn full_day_invariants_and_midnight_reset() {
        let mut previous = 0.0;
        for time in 0..86400 {
            let stats = calculate(TIMES, time, true).unwrap();
            assert!((0.0..=100.0).contains(&stats.progress_percent));
            assert!(stats.progress_percent >= previous);
            assert!(stats.remaining_seconds >= 0);
            previous = stats.progress_percent;
        }
        assert_eq!(calculate(TIMES, 0, true).unwrap().progress_percent, 0.0);
        assert_eq!(
            calculate(TIMES, 86399, true).unwrap().progress_percent,
            100.0
        );
    }

    #[test]
    fn guandan_target_boundaries_and_next_day_reset() {
        for (now, ready) in [
            (42299, false),
            (42300, true),
            (47699, true),
            (47700, false),
            (47701, false),
        ] {
            assert_eq!(calculate(TIMES, now, true).unwrap().guandan_ready, ready);
        }
        assert!(!calculate(TIMES, 45000, false).unwrap().guandan_ready);
        for (now, remaining) in [
            (0, 42300),
            (32400, 9900),
            (42299, 1),
            (42300, 0),
            (86399, 0),
        ] {
            assert_eq!(
                calculate(TIMES, now, true)
                    .unwrap()
                    .guandan_remaining_seconds,
                remaining
            );
        }
        assert_eq!(
            calculate(TIMES, 32400, false)
                .unwrap()
                .guandan_remaining_seconds,
            0
        );
        // Changing lunch does not change the fixed 11:45 target.
        assert_eq!(
            calculate([32400, 43200, 48600, 63000], 42299, true)
                .unwrap()
                .guandan_remaining_seconds,
            1
        );
    }

    #[test]
    fn invalid_schedules_and_time_are_rejected() {
        for times in [
            [32400, 42300, 47700, 32400],
            [32400, 42300, 42300, 63000],
            [-1, 42300, 47700, 63000],
            [0, 42300, 47700, 86400],
        ] {
            assert!(calculate(times, 50000, true).is_err());
        }
        assert!(calculate(TIMES, -1, true).is_err());
        assert!(calculate(TIMES, 86400, true).is_err());
    }
}
