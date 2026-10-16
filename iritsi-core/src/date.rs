fn epoch_day(year: i32, month: u32, day: u32) -> i32 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year as i32;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_epoch_day(epoch_day: i32) -> (i32, u32, u32) {
    let z = epoch_day + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = year_of_era + era * 400 + (month <= 2) as i32;
    (year, month, day)
}

const DAY: i64 = 24 * 60 * 60;
const HOUR: i64 = 60 * 60;

fn last_sunday(year: i32, month: u32) -> i32 {
    let last = epoch_day(year, month, 31);
    last - ((weekday_of_epoch_day(last) + 1) % 7) as i32
}

pub fn madrid_utc_offset(unix: i64) -> i64 {
    let (year, _, _) = civil_from_epoch_day(unix.div_euclid(DAY) as i32);
    let summer_start = last_sunday(year, 3) as i64 * DAY + HOUR;
    let summer_end = last_sunday(year, 10) as i64 * DAY + HOUR;
    if (summer_start..summer_end).contains(&unix) {
        2 * HOUR
    } else {
        HOUR
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn from_yyyymmdd(date: u32) -> Self {
        Self {
            year: (date / 10_000) as i32,
            month: date / 100 % 100,
            day: date % 100,
        }
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalTime {
    pub date: Date,
    pub hour: u32,
    pub minute: u32,
}

impl LocalTime {
    pub fn madrid(unix: i64) -> Self {
        let local = unix + madrid_utc_offset(unix);
        let (year, month, day) = civil_from_epoch_day(local.div_euclid(DAY) as i32);
        let (hour, minute) = madrid_clock(unix);
        Self {
            date: Date { year, month, day },
            hour,
            minute,
        }
    }
}

impl std::fmt::Display for LocalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:02}:{:02}", self.date, self.hour, self.minute)
    }
}

pub fn madrid_clock(unix: i64) -> (u32, u32) {
    let secs = (unix + madrid_utc_offset(unix)).rem_euclid(DAY);
    ((secs / HOUR) as u32, (secs % HOUR / 60) as u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceDay {
    epoch_day: i32,
}

impl ServiceDay {
    pub fn containing(unix: i64) -> Self {
        let local = unix + madrid_utc_offset(unix);
        Self {
            epoch_day: local.div_euclid(DAY) as i32,
        }
    }

    pub fn previous(self) -> Self {
        Self {
            epoch_day: self.epoch_day - 1,
        }
    }

    pub fn date(self) -> u32 {
        let (year, month, day) = civil_from_epoch_day(self.epoch_day);
        year as u32 * 10_000 + month * 100 + day
    }

    pub fn weekday(self) -> u32 {
        weekday_of_epoch_day(self.epoch_day)
    }

    pub fn start(self) -> i64 {
        let noon = self.epoch_day as i64 * DAY + 12 * HOUR;
        noon - madrid_utc_offset(noon) - 12 * HOUR
    }
}

// taken from jiff (https://github.com/BurntSushi/jiff, MIT), based on
// https://www.benjoffe.com/fast-day-of-week
#[inline]
const fn weekday_of_epoch_day(epoch_day: i32) -> u32 {
    const M: u32 = (1u64 << 32).div_ceil(7) as u32;
    const Z: u32 = 0x90000000;
    ((epoch_day as u32).wrapping_mul(M).wrapping_add(Z) >> 29) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_weekdays() {
        let weekday = |y, m, d| weekday_of_epoch_day(epoch_day(y, m, d));
        assert_eq!(weekday(2026, 9, 10), 3);
        assert_eq!(weekday(2026, 10, 16), 4);
        assert_eq!(weekday(2026, 10, 18), 6);
        assert_eq!(weekday(2024, 2, 29), 3);
        assert_eq!(weekday(2026, 1, 1), 3);
    }

    #[test]
    fn computes_epoch_days() {
        assert_eq!(epoch_day(1970, 1, 1), 0);
        assert_eq!(epoch_day(1969, 12, 31), -1);
        assert_eq!(epoch_day(2000, 3, 1), 11_017);
        assert_eq!(weekday_of_epoch_day(0), 3);
        assert_eq!(weekday_of_epoch_day(-1), 2);
    }

    #[test]
    fn roundtrips_civil_dates() {
        for day in -800_000..800_000 {
            let (year, month, date) = civil_from_epoch_day(day);
            assert_eq!(epoch_day(year, month, date), day);
        }
        assert_eq!(civil_from_epoch_day(0), (1970, 1, 1));
        assert_eq!(civil_from_epoch_day(11_016), (2000, 2, 29));
    }

    #[test]
    fn switches_to_summer_time_on_the_last_sundays() {
        let utc = |y, m, d, h| epoch_day(y, m, d) as i64 * DAY + h * HOUR;
        assert_eq!(madrid_utc_offset(utc(2026, 3, 29, 1) - 1), HOUR);
        assert_eq!(madrid_utc_offset(utc(2026, 3, 29, 1)), 2 * HOUR);
        assert_eq!(madrid_utc_offset(utc(2026, 10, 25, 1) - 1), 2 * HOUR);
        assert_eq!(madrid_utc_offset(utc(2026, 10, 25, 1)), HOUR);
        assert_eq!(madrid_utc_offset(utc(2026, 1, 1, 12)), HOUR);
        assert_eq!(madrid_clock(utc(2026, 9, 17, 22) + 90), (0, 1));
        assert_eq!(madrid_clock(utc(2026, 12, 1, 7) + 59 * 60), (8, 59));
        assert_eq!(
            LocalTime::madrid(utc(2026, 9, 17, 22) + 90).to_string(),
            "2026-09-18 00:01"
        );
        assert_eq!(Date::from_yyyymmdd(20261116).to_string(), "2026-11-16");
    }

    #[test]
    fn maps_instants_to_madrid_service_days() {
        let utc = |y, m, d, h| epoch_day(y, m, d) as i64 * DAY + h * HOUR;

        let day = ServiceDay::containing(utc(2026, 9, 17, 22));
        assert_eq!(day.date(), 20260918);
        assert_eq!(day.weekday(), 4);
        assert_eq!(day.start(), utc(2026, 9, 17, 22));
        assert_eq!(day.previous().date(), 20260917);

        let winter = ServiceDay::containing(utc(2026, 12, 1, 0));
        assert_eq!(winter.date(), 20261201);
        assert_eq!(winter.start(), utc(2026, 11, 30, 23));

        assert_eq!(
            ServiceDay::containing(utc(2026, 3, 29, 12)).start(),
            utc(2026, 3, 28, 22)
        );
    }
}
