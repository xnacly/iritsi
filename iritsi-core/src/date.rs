pub fn weekday(date: u32) -> Option<u32> {
    let (year, month, day) = (date / 10_000, date / 100 % 100, date % 100);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > days_in_month {
        return None;
    }

    Some(weekday_of_epoch_day(epoch_day(year as i32, month, day)))
}

fn epoch_day(year: i32, month: u32, day: u32) -> i32 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year as i32;
    era * 146_097 + day_of_era - 719_468
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
        assert_eq!(weekday(20260910), Some(3));
        assert_eq!(weekday(20261016), Some(4));
        assert_eq!(weekday(20261018), Some(6));
        assert_eq!(weekday(20240229), Some(3));
        assert_eq!(weekday(20260101), Some(3));
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
    fn rejects_invalid_dates() {
        assert_eq!(weekday(20260229), None);
        assert_eq!(weekday(20261301), None);
        assert_eq!(weekday(20261000), None);
    }
}
