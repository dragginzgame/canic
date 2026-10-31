/// Render an observed Unix-nanosecond deadline without consulting the local clock.
pub fn utc_timestamp_ns(nanoseconds: u64) -> String {
    let seconds = nanoseconds / 1_000_000_000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds % 86_400 / 3_600;
    let minute = seconds % 3_600 / 60;
    let second = seconds % 60;
    let fraction = nanoseconds % 1_000_000_000;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:09}Z")
}

// Convert days since 1970-01-01 into a proleptic Gregorian UTC date.
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + (month <= 2) as i64;

    (year, month, day)
}
