//! Time: a moment, written down and read back, and the calendar arithmetic
//! that turns a moment into a date.
//!
//! A moment in Nail is a Unix timestamp - whole seconds since the start of
//! 1970 - because one number is the only representation two machines never
//! disagree about. Everything here reads and writes that number.
//!
//! Every calendar function works in UTC. A program that stores UTC and
//! converts once, at the edge where a person reads it, is a program that never
//! has a daylight-saving bug; a program that stores local time has already
//! lost an hour somewhere and does not know it yet.

use chrono::{DateTime, Datelike, Duration, NaiveDate, SecondsFormat, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep as tokio_sleep;

/// How a moment is spelled out as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TIME_Format {
    /// Whole seconds since 1970: `1234567890`.
    Unix,
    /// Milliseconds since 1970: `1234567890000`.
    UnixMillis,
    /// `2009-02-13T23:31:30Z` - the one to use in a file or an API.
    ISO8601,
    /// `2009-02-13T23:31:30+00:00` - ISO 8601 with the offset written out.
    RFC3339,
    /// `Fri, 13 Feb 2009 23:31:30 +0000` - what email headers and HTTP use.
    RFC2822,
}

/// A timestamp as a date, or an error naming the timestamp that has no date.
/// The range that fails is absurd - hundreds of thousands of years out - but
/// it is reachable by arithmetic on bad input, and a wrong date is worse than
/// a refusal.
fn to_datetime(function: &str, timestamp: i64) -> Result<DateTime<Utc>, String> {
    return DateTime::from_timestamp(timestamp, 0).ok_or_else(|| format!("{}: {} is too far from 1970 to be a date", function, timestamp));
}

// Current Unix timestamp in seconds. Total (never panics): a system clock set
// before 1970 yields a negative timestamp instead of crashing the program.
pub fn now() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

// Current Unix timestamp in milliseconds. Total like now() above.
pub fn now_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    }
}

pub async fn sleep(seconds: f64) {
    let duration = StdDuration::from_secs_f64(seconds);
    tokio_sleep(duration).await;
}

/// Writes a moment out in one of the standard spellings.
pub fn format(timestamp: i64, format: TIME_Format) -> Result<String, String> {
    match format {
        TIME_Format::Unix => Ok(timestamp.to_string()),
        TIME_Format::UnixMillis => Ok((timestamp * 1000).to_string()),
        TIME_Format::ISO8601 => Ok(to_datetime("time_format", timestamp)?.to_rfc3339_opts(SecondsFormat::Secs, true)),
        TIME_Format::RFC3339 => Ok(to_datetime("time_format", timestamp)?.to_rfc3339_opts(SecondsFormat::Secs, false)),
        TIME_Format::RFC2822 => Ok(to_datetime("time_format", timestamp)?.to_rfc2822()),
    }
}

/// Reads a moment back out of text. The format says which spelling to expect;
/// anything else is an error rather than a guess, because a date guessed wrong
/// is a date nobody notices until the report is late.
pub fn parse(time_str: String, format: TIME_Format) -> Result<i64, String> {
    match format {
        TIME_Format::Unix => time_str.trim().parse::<i64>().map_err(|_| format!("time_parse: could not read '{}' as a Unix timestamp", time_str)),
        TIME_Format::UnixMillis => time_str.trim().parse::<i64>().map(|milliseconds| milliseconds / 1000).map_err(|_| format!("time_parse: could not read '{}' as Unix milliseconds", time_str)),
        TIME_Format::ISO8601 | TIME_Format::RFC3339 => DateTime::parse_from_rfc3339(time_str.trim())
            .map(|moment| moment.timestamp())
            .map_err(|e| format!("time_parse: could not read '{}' as an ISO 8601 date: {}", time_str, e)),
        TIME_Format::RFC2822 => DateTime::parse_from_rfc2822(time_str.trim())
            .map(|moment| moment.timestamp())
            .map_err(|e| format!("time_parse: could not read '{}' as an RFC 2822 date: {}", time_str, e)),
    }
}

/// Writes a moment out in a layout of your own, in the strftime notation every
/// language borrowed from C: `%Y-%m-%d` is 2009-02-13, `%H:%M` is 23:31,
/// `%A %d %B %Y` is Friday 13 February 2009.
pub fn format_custom(timestamp: i64, layout: String) -> Result<String, String> {
    let moment = to_datetime("time_format_custom", timestamp)?;
    // A bad layout makes chrono's formatter fail at display time, so the
    // layout is checked here rather than allowed to panic mid-print.
    if chrono::format::StrftimeItems::new(&layout).any(|item| matches!(item, chrono::format::Item::Error)) {
        return Err(format!("time_format_custom: '{}' is not a valid layout - see the strftime notation, such as %Y-%m-%d", layout));
    }
    return Ok(moment.format(&layout).to_string());
}

/// Reads a moment out of text laid out the way you say, in the same strftime
/// notation. The text must name a date and a time; a layout with no time in it
/// leaves the time at midnight.
pub fn parse_custom(time_str: String, layout: String) -> Result<i64, String> {
    if let Ok(moment) = chrono::NaiveDateTime::parse_from_str(time_str.trim(), &layout) {
        return Ok(moment.and_utc().timestamp());
    }
    return match NaiveDate::parse_from_str(time_str.trim(), &layout) {
        Ok(date) => Ok(date.and_hms_opt(0, 0, 0).expect("midnight is a valid time").and_utc().timestamp()),
        Err(e) => Err(format!("time_parse_custom: could not read '{}' with the layout '{}': {}", time_str, layout, e)),
    };
}

/// Builds a moment out of the parts of a date, in UTC. A day that does not
/// exist - the thirty-first of February, the twenty-ninth of a year that is
/// not a leap year - is an error rather than the day it would spill over into.
pub fn from_parts(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> Result<i64, String> {
    let in_range = (1..=12).contains(&month) && (1..=31).contains(&day) && (0..=23).contains(&hour) && (0..=59).contains(&minute) && (0..=60).contains(&second);
    if !in_range {
        return Err(format!("time_from_parts: {}-{}-{} {}:{}:{} is not a date on the calendar", year, month, day, hour, minute, second));
    }

    let moment = Utc.with_ymd_and_hms(year as i32, month as u32, day as u32, hour as u32, minute as u32, second as u32);
    return match moment.single() {
        Some(moment) => Ok(moment.timestamp()),
        None => Err(format!("time_from_parts: {}-{}-{} {}:{}:{} is not a date on the calendar", year, month, day, hour, minute, second)),
    };
}

pub fn add_seconds(timestamp: i64, seconds: i64) -> i64 {
    return timestamp + seconds;
}

pub fn add_minutes(timestamp: i64, minutes: i64) -> i64 {
    return timestamp + minutes * 60;
}

pub fn add_hours(timestamp: i64, hours: i64) -> i64 {
    return timestamp + hours * 3600;
}

/// Moves a whole number of days, which in UTC is always exactly 24 hours.
pub fn add_days(timestamp: i64, days: i64) -> i64 {
    return timestamp + days * 86_400;
}

/// Moves a whole number of months, keeping the day of the month where it can.
/// The 31st of a month moved into a shorter one lands on that month's last
/// day, which is what a person means by "a month later" and what no amount of
/// second-counting can express.
pub fn add_months(timestamp: i64, months: i64) -> Result<i64, String> {
    let moment = to_datetime("time_add_months", timestamp)?;
    let shifted = if months >= 0 {
        moment.checked_add_months(chrono::Months::new(months as u32))
    } else {
        moment.checked_sub_months(chrono::Months::new(months.unsigned_abs() as u32))
    };
    return shifted.map(|moment| moment.timestamp()).ok_or_else(|| format!("time_add_months: {} months from {} is off the calendar", months, timestamp));
}

pub fn diff(t1: i64, t2: i64) -> i64 {
    return (t1 - t2).abs();
}

/// Midnight UTC at the start of the day this moment falls in. The building
/// block for "everything that happened today".
pub fn start_of_day(timestamp: i64) -> Result<i64, String> {
    let moment = to_datetime("time_start_of_day", timestamp)?;
    return Ok(moment.date_naive().and_hms_opt(0, 0, 0).expect("midnight is a valid time").and_utc().timestamp());
}

pub fn year(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_year", timestamp)?.year() as i64);
}

pub fn month(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_month", timestamp)?.month() as i64);
}

pub fn day(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_day", timestamp)?.day() as i64);
}

pub fn hour(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_hour", timestamp)?.hour() as i64);
}

pub fn minute(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_minute", timestamp)?.minute() as i64);
}

pub fn second(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_second", timestamp)?.second() as i64);
}

/// The day of the week, written out: `Monday` through `Sunday`.
pub fn weekday(timestamp: i64) -> Result<String, String> {
    let moment = to_datetime("time_weekday", timestamp)?;
    return Ok(match moment.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    }
    .to_string());
}

/// Which day of the year this is, from 1 to 366.
pub fn day_of_year(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_day_of_year", timestamp)?.ordinal() as i64);
}

/// A length of time written the way a person says it: `2d 3h 4m`, `45s`,
/// `0s`. Only the two largest units that matter are shown, because "2 days, 3
/// hours, 4 minutes and 11 seconds" is not how anyone reads a duration.
pub fn format_duration(seconds: i64) -> String {
    if seconds == 0 {
        return "0s".to_string();
    }

    let sign = if seconds < 0 { "-" } else { "" };
    let total = seconds.abs();
    let duration = Duration::seconds(total);

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let remaining_seconds = total % 60;

    if days > 0 {
        if hours > 0 {
            return format!("{}{}d {}h", sign, days, hours);
        }
        return format!("{}{}d", sign, days);
    }
    if hours > 0 {
        if minutes > 0 {
            return format!("{}{}h {}m", sign, hours, minutes);
        }
        return format!("{}{}h", sign, hours);
    }
    if minutes > 0 {
        if remaining_seconds > 0 {
            return format!("{}{}m {}s", sign, minutes, remaining_seconds);
        }
        return format!("{}{}m", sign, minutes);
    }
    return format!("{}{}s", sign, remaining_seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2009-02-13T23:31:30Z, a Friday. The timestamp everyone recognises.
    const REFERENCE: i64 = 1_234_567_890;

    #[test]
    fn a_moment_is_written_out_in_every_standard_spelling() {
        assert_eq!(format(REFERENCE, TIME_Format::Unix).expect("a real date"), "1234567890");
        assert_eq!(format(REFERENCE, TIME_Format::UnixMillis).expect("a real date"), "1234567890000");
        assert_eq!(format(REFERENCE, TIME_Format::ISO8601).expect("a real date"), "2009-02-13T23:31:30Z");
        assert_eq!(format(REFERENCE, TIME_Format::RFC3339).expect("a real date"), "2009-02-13T23:31:30+00:00");
        assert_eq!(format(REFERENCE, TIME_Format::RFC2822).expect("a real date"), "Fri, 13 Feb 2009 23:31:30 +0000");
    }

    #[test]
    fn every_spelling_reads_back_to_the_moment_it_was_written_from() {
        for spelling in [TIME_Format::Unix, TIME_Format::ISO8601, TIME_Format::RFC3339, TIME_Format::RFC2822] {
            let written = format(REFERENCE, spelling).expect("a real date");
            assert_eq!(parse(written.clone(), spelling).expect("what we just wrote"), REFERENCE, "{:?} did not round-trip through '{}'", spelling, written);
        }
    }

    #[test]
    fn milliseconds_round_trip_to_the_second() {
        let written = format(REFERENCE, TIME_Format::UnixMillis).expect("a real date");
        assert_eq!(parse(written, TIME_Format::UnixMillis).expect("what we just wrote"), REFERENCE);
    }

    #[test]
    fn text_that_is_not_a_date_is_an_error() {
        assert!(parse("not_a_number".to_string(), TIME_Format::Unix).unwrap_err().contains("could not read"));
        assert!(parse("13/02/2009".to_string(), TIME_Format::ISO8601).unwrap_err().contains("ISO 8601"));
        assert!(parse("2009-02-13T23:31:30Z".to_string(), TIME_Format::RFC2822).unwrap_err().contains("RFC 2822"));
    }

    #[test]
    fn a_custom_layout_writes_and_reads_the_same_date() {
        assert_eq!(format_custom(REFERENCE, "%Y-%m-%d".to_string()).expect("a real date"), "2009-02-13");
        assert_eq!(format_custom(REFERENCE, "%A %d %B %Y".to_string()).expect("a real date"), "Friday 13 February 2009");
        assert_eq!(parse_custom("2009-02-13 23:31:30".to_string(), "%Y-%m-%d %H:%M:%S".to_string()).expect("a matching layout"), REFERENCE);
        assert_eq!(parse_custom("2009-02-13".to_string(), "%Y-%m-%d".to_string()).expect("a matching layout"), 1_234_483_200);
    }

    #[test]
    fn a_layout_that_is_not_strftime_is_an_error_rather_than_a_panic() {
        assert!(format_custom(REFERENCE, "%Q".to_string()).unwrap_err().contains("not a valid layout"));
        assert!(parse_custom("2009".to_string(), "%Y-%m-%d".to_string()).unwrap_err().contains("could not read"));
    }

    #[test]
    fn the_parts_of_a_date_are_readable() {
        assert_eq!(year(REFERENCE).expect("a real date"), 2009);
        assert_eq!(month(REFERENCE).expect("a real date"), 2);
        assert_eq!(day(REFERENCE).expect("a real date"), 13);
        assert_eq!(hour(REFERENCE).expect("a real date"), 23);
        assert_eq!(minute(REFERENCE).expect("a real date"), 31);
        assert_eq!(second(REFERENCE).expect("a real date"), 30);
        assert_eq!(weekday(REFERENCE).expect("a real date"), "Friday");
        assert_eq!(day_of_year(REFERENCE).expect("a real date"), 44);
    }

    #[test]
    fn a_date_can_be_built_from_its_parts() {
        assert_eq!(from_parts(2009, 2, 13, 23, 31, 30).expect("a real date"), REFERENCE);
        assert_eq!(from_parts(1970, 1, 1, 0, 0, 0).expect("a real date"), 0);
    }

    #[test]
    fn a_day_that_is_not_on_the_calendar_is_an_error() {
        assert!(from_parts(2009, 2, 31, 0, 0, 0).unwrap_err().contains("not a date on the calendar"));
        assert!(from_parts(2009, 13, 1, 0, 0, 0).unwrap_err().contains("not a date on the calendar"));
        assert!(from_parts(2009, 2, 13, 25, 0, 0).unwrap_err().contains("not a date on the calendar"));
    }

    #[test]
    fn shifting_by_fixed_units_is_plain_arithmetic() {
        assert_eq!(add_seconds(REFERENCE, 10), REFERENCE + 10);
        assert_eq!(add_minutes(REFERENCE, 2), REFERENCE + 120);
        assert_eq!(add_hours(REFERENCE, 1), REFERENCE + 3600);
        assert_eq!(add_days(REFERENCE, 1), REFERENCE + 86_400);
        assert_eq!(add_days(REFERENCE, -1), REFERENCE - 86_400);
    }

    #[test]
    fn shifting_by_months_keeps_the_day_where_it_can() {
        let january_31 = from_parts(2009, 1, 31, 12, 0, 0).expect("a real date");
        let a_month_later = add_months(january_31, 1).expect("a real date");
        assert_eq!(format_custom(a_month_later, "%Y-%m-%d".to_string()).expect("a real date"), "2009-02-28");

        let a_year_later = add_months(january_31, 12).expect("a real date");
        assert_eq!(format_custom(a_year_later, "%Y-%m-%d".to_string()).expect("a real date"), "2010-01-31");

        let a_month_earlier = add_months(january_31, -1).expect("a real date");
        assert_eq!(format_custom(a_month_earlier, "%Y-%m-%d".to_string()).expect("a real date"), "2008-12-31");
    }

    #[test]
    fn the_start_of_the_day_is_midnight_utc() {
        let midnight = start_of_day(REFERENCE).expect("a real date");
        assert_eq!(format(midnight, TIME_Format::ISO8601).expect("a real date"), "2009-02-13T00:00:00Z");
        assert_eq!(start_of_day(midnight).expect("a real date"), midnight, "midnight is already the start of its day");
    }

    #[test]
    fn the_difference_between_two_moments_has_no_direction() {
        assert_eq!(diff(1000, 2000), 1000);
        assert_eq!(diff(2000, 1000), 1000);
        assert_eq!(diff(1000, 1000), 0);
    }

    #[test]
    fn a_length_of_time_reads_the_way_a_person_says_it() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(60), "1m");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(3900), "1h 5m");
        assert_eq!(format_duration(86_400), "1d");
        assert_eq!(format_duration(183_845), "2d 3h");
        assert_eq!(format_duration(-125), "-2m 5s");
    }

    #[test]
    fn a_timestamp_with_no_date_is_an_error_rather_than_a_wrong_date() {
        assert!(year(i64::MAX).unwrap_err().contains("too far from 1970"));
        assert!(format(i64::MAX, TIME_Format::ISO8601).unwrap_err().contains("too far from 1970"));
    }

    #[tokio::test]
    async fn sleeping_returns() {
        sleep(0.001).await;
    }
}

/// One field of a cron expression, expanded into the values it allows.
///
/// The five fields are minute, hour, day of month, month, and day of week, and
/// each takes `*`, a number, a `first-last` range, a `*/n` or `first-last/n`
/// step, or several of those separated by commas. That is the whole of the
/// syntax anybody writes by hand; the extensions (`@daily`, `L`, `#`) differ
/// between implementations, so they are not accepted rather than guessed at.
fn cron_field(field: &str, lowest: i64, highest: i64, field_name: &str) -> Result<Vec<i64>, String> {
    let mut allowed: Vec<i64> = Vec::new();
    for part in field.split(',') {
        let (range_text, step) = match part.split_once('/') {
            Some((range_text, step_text)) => {
                let step: i64 = step_text.parse().map_err(|_| format!("the {} field's step `{}` is not a number", field_name, step_text))?;
                if step < 1 {
                    return Err(format!("the {} field's step must be at least 1, got {}", field_name, step));
                }
                (range_text, step)
            }
            None => (part, 1),
        };

        let (from, to) = if range_text == "*" {
            (lowest, highest)
        } else if let Some((first, last)) = range_text.split_once('-') {
            let first: i64 = first.parse().map_err(|_| format!("the {} field's `{}` is not a number", field_name, first))?;
            let last: i64 = last.parse().map_err(|_| format!("the {} field's `{}` is not a number", field_name, last))?;
            (first, last)
        } else {
            let only: i64 = range_text.parse().map_err(|_| format!("the {} field's `{}` is not a number", field_name, range_text))?;
            (only, only)
        };

        if from < lowest || to > highest || from > to {
            return Err(format!("the {} field's `{}` is outside {} to {}", field_name, part, lowest, highest));
        }
        let mut value = from;
        while value <= to {
            if !allowed.contains(&value) {
                allowed.push(value);
            }
            value += step;
        }
    }
    if allowed.is_empty() {
        return Err(format!("the {} field allows no values at all", field_name));
    }
    allowed.sort();
    return Ok(allowed);
}

/// A cron expression, expanded into the values each field allows.
struct CronSchedule {
    minutes: Vec<i64>,
    hours: Vec<i64>,
    days_of_month: Vec<i64>,
    months: Vec<i64>,
    days_of_week: Vec<i64>,
    /// Whether the day fields were both given. Cron's own rule is that a day of
    /// month and a day of week together mean *either*, not both - which is why
    /// `0 0 13 * 5` is every Friday and every 13th, not only Friday the 13th.
    both_day_fields_given: bool,
}

fn parse_cron(expression: &str) -> Result<CronSchedule, String> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(format!("`{}` is not a cron expression: it needs five fields - minute, hour, day of month, month, day of week", expression));
    }
    return Ok(CronSchedule {
        minutes: cron_field(fields[0], 0, 59, "minute")?,
        hours: cron_field(fields[1], 0, 23, "hour")?,
        days_of_month: cron_field(fields[2], 1, 31, "day of month")?,
        months: cron_field(fields[3], 1, 12, "month")?,
        // Sunday is 0. Seven is also Sunday in every cron there has ever been,
        // so it is allowed and folded onto zero.
        days_of_week: cron_field(fields[4], 0, 7, "day of week")?.into_iter().map(|day| if day == 7 { 0 } else { day }).collect(),
        both_day_fields_given: fields[2] != "*" && fields[4] != "*",
    });
}

fn cron_matches_time(schedule: &CronSchedule, moment: DateTime<Utc>) -> bool {
    let day_of_month_matches = schedule.days_of_month.contains(&(moment.day() as i64));
    let day_of_week_matches = schedule.days_of_week.contains(&(moment.weekday().num_days_from_sunday() as i64));
    let day_matches = if schedule.both_day_fields_given { day_of_month_matches || day_of_week_matches } else { day_of_month_matches && day_of_week_matches };

    return schedule.minutes.contains(&(moment.minute() as i64)) && schedule.hours.contains(&(moment.hour() as i64)) && schedule.months.contains(&(moment.month() as i64)) && day_matches;
}

/// Whether a cron expression is one this understands, for checking a schedule
/// that came from a configuration file before the program relies on it.
pub fn cron_valid(expression: &String) -> bool {
    return parse_cron(expression).is_ok();
}

/// Whether a cron expression matches a moment, to the minute.
pub fn cron_matches(expression: String, timestamp: i64) -> Result<bool, String> {
    let schedule = parse_cron(&expression).map_err(|detail| format!("time_cron_matches: {}", detail))?;
    let moment = DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| format!("time_cron_matches: {} is not a time", timestamp))?;
    return Ok(cron_matches_time(&schedule, moment));
}

/// The next moment at or after the given time that a cron expression matches,
/// in seconds. This is the whole of what a scheduler needs from cron: a program
/// that wants to run something on a schedule asks when the next run is, sleeps
/// until then with `time_sleep`, does the work, and asks again.
///
/// Doing it that way rather than with a callback means the program keeps its own
/// control flow - the work is an ordinary call in an ordinary loop, and what
/// happens if it fails is written where anyone reading the loop can see it.
pub fn cron_next(expression: String, after_timestamp: i64) -> Result<i64, String> {
    let schedule = parse_cron(&expression).map_err(|detail| format!("time_cron_next: {}", detail))?;
    let start = DateTime::<Utc>::from_timestamp(after_timestamp, 0).ok_or_else(|| format!("time_cron_next: {} is not a time", after_timestamp))?;

    // Cron has a resolution of a minute, so the search starts at the next whole
    // minute after the moment given.
    let mut candidate = start.with_second(0).and_then(|moment| moment.with_nanosecond(0)).ok_or_else(|| "time_cron_next: could not round the time to a minute".to_string())? + Duration::minutes(1);

    // Four years of minutes is past any February 29th, which is the only date a
    // valid expression can wait years for.
    const MINUTES_IN_FOUR_YEARS: i64 = 4 * 366 * 24 * 60;
    for _ in 0..MINUTES_IN_FOUR_YEARS {
        if cron_matches_time(&schedule, candidate) {
            return Ok(candidate.timestamp());
        }
        candidate += Duration::minutes(1);
    }
    return Err(format!("time_cron_next: `{}` has no next run within four years, so nothing will ever match it", expression));
}

#[cfg(test)]
mod cron_tests {
    use super::*;

    /// A timestamp for a moment, so the tests read as dates rather than numbers.
    fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
        return from_parts(year, month, day, hour, minute, 0).expect("a real date");
    }

    #[test]
    fn an_expression_is_recognised_by_its_shape() {
        assert!(cron_valid(&"* * * * *".to_string()));
        assert!(cron_valid(&"0 3 * * *".to_string()));
        assert!(cron_valid(&"*/15 * * * *".to_string()));
        assert!(cron_valid(&"0 0 1,15 * *".to_string()));
        assert!(cron_valid(&"0 9-17 * * 1-5".to_string()));
        assert!(!cron_valid(&"0 3 * *".to_string()));
        assert!(!cron_valid(&"@daily".to_string()));
        assert!(!cron_valid(&"60 * * * *".to_string()));
        assert!(!cron_valid(&"0 24 * * *".to_string()));
        assert!(!cron_valid(&"0 0 0 * *".to_string()));
        assert!(!cron_valid(&"*/0 * * * *".to_string()));
    }

    #[test]
    fn every_minute_matches_every_minute() {
        let moment = at(2026, 8, 4, 13, 37);
        assert!(cron_matches("* * * * *".to_string(), moment).expect("a valid expression"));
        assert_eq!(cron_next("* * * * *".to_string(), moment).expect("a valid expression"), moment + 60);
    }

    #[test]
    fn a_daily_schedule_finds_tomorrow_when_today_has_passed() {
        // 04:00 on a Tuesday; the 03:00 run has been and gone.
        let after = at(2026, 8, 4, 4, 0);
        let next = cron_next("0 3 * * *".to_string(), after).expect("a valid expression");
        assert_eq!(next, at(2026, 8, 5, 3, 0));
    }

    #[test]
    fn a_daily_schedule_finds_today_when_it_is_still_to_come() {
        let after = at(2026, 8, 4, 1, 0);
        assert_eq!(cron_next("0 3 * * *".to_string(), after).expect("a valid expression"), at(2026, 8, 4, 3, 0));
    }

    #[test]
    fn a_step_runs_at_every_step_and_nowhere_between() {
        let after = at(2026, 8, 4, 13, 1);
        assert_eq!(cron_next("*/15 * * * *".to_string(), after).expect("a valid expression"), at(2026, 8, 4, 13, 15));
        assert!(cron_matches("*/15 * * * *".to_string(), at(2026, 8, 4, 13, 30)).expect("a valid expression"));
        assert!(!cron_matches("*/15 * * * *".to_string(), at(2026, 8, 4, 13, 31)).expect("a valid expression"));
    }

    #[test]
    fn a_weekday_schedule_skips_the_weekend() {
        // Friday 2026-08-07 at 18:00, after that day's run.
        let friday_evening = at(2026, 8, 7, 18, 0);
        let next = cron_next("0 9 * * 1-5".to_string(), friday_evening).expect("a valid expression");
        // The following Monday, not Saturday.
        assert_eq!(next, at(2026, 8, 10, 9, 0));
    }

    #[test]
    fn sunday_is_both_zero_and_seven() {
        let saturday = at(2026, 8, 8, 12, 0);
        let by_zero = cron_next("0 0 * * 0".to_string(), saturday).expect("a valid expression");
        let by_seven = cron_next("0 0 * * 7".to_string(), saturday).expect("a valid expression");
        assert_eq!(by_zero, by_seven);
        assert_eq!(by_zero, at(2026, 8, 9, 0, 0));
    }

    /// Cron's own rule, which surprises everyone: with both day fields given,
    /// either one matching is enough.
    #[test]
    fn two_day_fields_mean_either_rather_than_both() {
        // The 13th of August 2026 is a Thursday, and the 7th is a Friday.
        assert!(cron_matches("0 0 13 * 5".to_string(), at(2026, 8, 13, 0, 0)).expect("a valid expression"));
        assert!(cron_matches("0 0 13 * 5".to_string(), at(2026, 8, 7, 0, 0)).expect("a valid expression"));
        assert!(!cron_matches("0 0 13 * 5".to_string(), at(2026, 8, 12, 0, 0)).expect("a valid expression"));
    }

    #[test]
    fn a_yearly_schedule_is_found_across_the_year_boundary() {
        let after = at(2026, 8, 4, 0, 0);
        assert_eq!(cron_next("0 0 1 1 *".to_string(), after).expect("a valid expression"), at(2027, 1, 1, 0, 0));
    }

    #[test]
    fn a_date_that_only_exists_in_a_leap_year_is_still_found() {
        let after = at(2026, 8, 4, 0, 0);
        assert_eq!(cron_next("0 0 29 2 *".to_string(), after).expect("a valid expression"), at(2028, 2, 29, 0, 0));
    }

    #[test]
    fn a_broken_expression_says_which_field_is_wrong() {
        let failure = cron_next("0 25 * * *".to_string(), 0).unwrap_err();
        assert!(failure.contains("hour"), "got: {}", failure);
        let missing = cron_matches("0 3 * *".to_string(), 0).unwrap_err();
        assert!(missing.contains("five fields"), "got: {}", missing);
    }
}

/// The last second of the day the timestamp falls in - 23:59:59 UTC. The other
/// end of `time_start_of_day`, so a whole day is the range between them.
pub fn end_of_day(timestamp: i64) -> Result<i64, String> {
    let moment = to_datetime("time_end_of_day", timestamp)?;
    return Ok(moment.date_naive().and_hms_opt(23, 59, 59).expect("the last second is a valid time").and_utc().timestamp());
}

/// The timestamp moved by a number of whole weeks; negative goes back.
pub fn add_weeks(timestamp: i64, weeks: i64) -> i64 {
    return add_days(timestamp, weeks * 7);
}

/// Whether a year has a 29th of February, by the actual rule: every fourth
/// year, except centuries, except every fourth century.
pub fn is_leap_year(year: i64) -> bool {
    return (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
}

/// How many days a month has, which is the question behind every billing cycle
/// and calendar grid. Months are numbered 1 to 12.
pub fn days_in_month(year: i64, month: i64) -> Result<i64, String> {
    return match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Ok(31),
        4 | 6 | 9 | 11 => Ok(30),
        2 => Ok(if is_leap_year(year) { 29 } else { 28 }),
        other => Err(format!("time_days_in_month: {} is not a month between 1 and 12", other)),
    };
}

/// How long ago something was, written the way a page shows it: `just now`,
/// `5 minutes ago`, `3 days ago`, `in 2 hours` for something still to come.
/// Both moments are given, rather than one and a hidden clock, so the same
/// inputs always produce the same words.
pub fn ago(timestamp: i64, now: i64) -> String {
    let difference = now - timestamp;
    let elapsed = difference.abs();

    let (count, unit) = if elapsed < 45 {
        return "just now".to_string();
    } else if elapsed < 3600 {
        ((elapsed + 30) / 60, "minute")
    } else if elapsed < 86400 {
        ((elapsed + 1800) / 3600, "hour")
    } else if elapsed < 2592000 {
        ((elapsed + 43200) / 86400, "day")
    } else if elapsed < 31536000 {
        ((elapsed + 1296000) / 2592000, "month")
    } else {
        ((elapsed + 15768000) / 31536000, "year")
    };

    let plural = if count == 1 { "" } else { "s" };
    if difference < 0 {
        return format!("in {} {}{}", count, unit, plural);
    }
    return format!("{} {}{} ago", count, unit, plural);
}

#[cfg(test)]
mod calendar_tests {
    use super::*;

    // 2024-03-15 12:30:45 UTC
    const NOON_ISH: i64 = 1710505845;

    #[test]
    fn a_day_runs_from_its_first_second_to_its_last() {
        let start = start_of_day(NOON_ISH).expect("a valid timestamp");
        let end = end_of_day(NOON_ISH).expect("a valid timestamp");
        assert_eq!(end - start, 86399);
        assert_eq!(hour(end).expect("a valid timestamp"), 23);
        assert_eq!(minute(end).expect("a valid timestamp"), 59);
        assert_eq!(second(end).expect("a valid timestamp"), 59);
    }

    #[test]
    fn weeks_move_seven_days_at_a_time() {
        assert_eq!(add_weeks(NOON_ISH, 1), add_days(NOON_ISH, 7));
        assert_eq!(add_weeks(NOON_ISH, -2), add_days(NOON_ISH, -14));
    }

    #[test]
    fn the_leap_year_rule_includes_its_exceptions() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn february_knows_which_year_it_is_in() {
        assert_eq!(days_in_month(2024, 2).expect("a month"), 29);
        assert_eq!(days_in_month(2023, 2).expect("a month"), 28);
        assert_eq!(days_in_month(2023, 1).expect("a month"), 31);
        assert_eq!(days_in_month(2023, 4).expect("a month"), 30);
        assert!(days_in_month(2023, 13).unwrap_err().contains("not a month"));
    }

    #[test]
    fn how_long_ago_reads_like_a_page_says_it() {
        let now = NOON_ISH;
        assert_eq!(ago(now - 10, now), "just now");
        assert_eq!(ago(now - 60, now), "1 minute ago");
        assert_eq!(ago(now - 300, now), "5 minutes ago");
        assert_eq!(ago(now - 7200, now), "2 hours ago");
        assert_eq!(ago(now - 259200, now), "3 days ago");
        assert_eq!(ago(now - 5184000, now), "2 months ago");
        assert_eq!(ago(now - 63072000, now), "2 years ago");
    }

    #[test]
    fn something_still_to_come_reads_forwards() {
        let now = NOON_ISH;
        assert_eq!(ago(now + 7200, now), "in 2 hours");
        assert_eq!(ago(now + 60, now), "in 1 minute");
    }
}
