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

pub fn now_micros() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_micros() as i64,
        Err(e) => -(e.duration().as_micros() as i64),
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

/// Parse a human duration - `90s`, `2h30m`, `1.5h`, `2 days` - into whole
/// seconds. A bare number is already seconds.
pub fn parse_duration(text: String) -> Result<i64, String> {
    let cleaned = text.trim().to_lowercase();
    if cleaned.is_empty() {
        return Err("time_parse_duration: there is no duration in an empty string".to_string());
    }
    let mut total = 0.0f64;
    let mut rest = cleaned.as_str();
    let mut saw_any = false;
    while !rest.trim_start().is_empty() {
        rest = rest.trim_start();
        let number_len = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        if number_len == 0 {
            return Err(format!("time_parse_duration: expected a number where `{}` begins", rest));
        }
        let value: f64 = rest[..number_len].parse().map_err(|_| format!("time_parse_duration: `{}` is not a number", &rest[..number_len]))?;
        rest = rest[number_len..].trim_start();
        let unit_len = rest.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(rest.len());
        let unit = &rest[..unit_len];
        rest = &rest[unit_len..];
        let seconds_per = match unit {
            "" if !saw_any && rest.trim_start().is_empty() => 1.0,
            "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
            "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
            "d" | "day" | "days" => 86400.0,
            "w" | "week" | "weeks" => 604800.0,
            "" => return Err("time_parse_duration: a number in a longer duration needs a unit - s, m, h, d or w".to_string()),
            other => return Err(format!("time_parse_duration: `{}` is not a unit this reads - use s, m, h, d or w", other)),
        };
        total += value * seconds_per;
        saw_any = true;
    }
    return Ok(total.round() as i64);
}

#[cfg(test)]
mod duration_tests {
    use super::parse_duration;

    #[test]
    fn the_usual_shapes_all_read() {
        assert_eq!(parse_duration("90s".to_string()).unwrap(), 90);
        assert_eq!(parse_duration("2h30m".to_string()).unwrap(), 9000);
        assert_eq!(parse_duration("1.5h".to_string()).unwrap(), 5400);
        assert_eq!(parse_duration("1d".to_string()).unwrap(), 86400);
        assert_eq!(parse_duration("2 weeks".to_string()).unwrap(), 1209600);
        assert_eq!(parse_duration("1h 30min".to_string()).unwrap(), 5400);
    }

    #[test]
    fn a_bare_number_is_seconds() {
        assert_eq!(parse_duration("90".to_string()).unwrap(), 90);
        assert_eq!(parse_duration(" 45 ".to_string()).unwrap(), 45);
    }

    #[test]
    fn nonsense_is_refused_with_its_reason() {
        assert!(parse_duration("".to_string()).unwrap_err().contains("empty"));
        assert!(parse_duration("soon".to_string()).unwrap_err().contains("expected a number"));
        assert!(parse_duration("10x".to_string()).unwrap_err().contains("not a unit"));
        assert!(parse_duration("1h 30".to_string()).unwrap_err().contains("needs a unit"));
    }
}

/// Which quarter of the year a moment falls in, from 1 to 4, in UTC.
pub fn quarter(timestamp: i64) -> Result<i64, String> {
    let moment = to_datetime("time_quarter", timestamp)?;
    return Ok(((moment.month() - 1) / 3 + 1) as i64);
}

/// The ISO 8601 week number, from 1 to 53. ISO weeks start on Monday, and week
/// 1 is the week holding the year's first Thursday - so the days around New
/// Year can belong to the other year's numbering.
pub fn week_of_year(timestamp: i64) -> Result<i64, String> {
    return Ok(to_datetime("time_week_of_year", timestamp)?.iso_week().week() as i64);
}

/// Whether a moment falls on a Saturday or Sunday, in UTC.
pub fn is_weekend(timestamp: i64) -> Result<bool, String> {
    let day = to_datetime("time_is_weekend", timestamp)?.weekday();
    return Ok(day == chrono::Weekday::Sat || day == chrono::Weekday::Sun);
}

/// Midnight UTC on the first of the month the moment falls in.
pub fn start_of_month(timestamp: i64) -> Result<i64, String> {
    let date = to_datetime("time_start_of_month", timestamp)?.date_naive();
    let first = date.with_day(1).expect("the first is a day every month has");
    return Ok(first.and_hms_opt(0, 0, 0).expect("midnight is a valid time").and_utc().timestamp());
}

/// The last second of the month the moment falls in - 23:59:59 UTC on its last
/// day, the same convention as `time_end_of_day`. The other end of
/// `time_start_of_month`, so a whole month is the range between them.
pub fn end_of_month(timestamp: i64) -> Result<i64, String> {
    let date = to_datetime("time_end_of_month", timestamp)?.date_naive();
    let length = days_in_month(date.year() as i64, date.month() as i64).expect("a real date's month is between 1 and 12");
    let last = date.with_day(length as u32).expect("no month is shorter than its own length");
    return Ok(last.and_hms_opt(23, 59, 59).expect("the last second is a valid time").and_utc().timestamp());
}

/// Midnight UTC on the Monday of the week the moment falls in. Weeks start on
/// Monday here, as they do in ISO 8601 and on every calendar outside a wall.
pub fn start_of_week(timestamp: i64) -> Result<i64, String> {
    let moment = to_datetime("time_start_of_week", timestamp)?;
    let monday = moment.date_naive() - Duration::days(moment.weekday().num_days_from_monday() as i64);
    return Ok(monday.and_hms_opt(0, 0, 0).expect("midnight is a valid time").and_utc().timestamp());
}

/// The last second of the week the moment falls in - 23:59:59 UTC on its
/// Sunday. The other end of `time_start_of_week`.
pub fn end_of_week(timestamp: i64) -> Result<i64, String> {
    let moment = to_datetime("time_end_of_week", timestamp)?;
    let sunday = moment.date_naive() + Duration::days(6 - moment.weekday().num_days_from_monday() as i64);
    return Ok(sunday.and_hms_opt(23, 59, 59).expect("the last second is a valid time").and_utc().timestamp());
}

/// Midnight UTC on the first of January of the year the moment falls in.
pub fn start_of_year(timestamp: i64) -> Result<i64, String> {
    let year = to_datetime("time_start_of_year", timestamp)?.year();
    let first = NaiveDate::from_ymd_opt(year, 1, 1).expect("every year has a first of January");
    return Ok(first.and_hms_opt(0, 0, 0).expect("midnight is a valid time").and_utc().timestamp());
}

/// The last second of the year the moment falls in - 23:59:59 UTC on the 31st
/// of December. The other end of `time_start_of_year`.
pub fn end_of_year(timestamp: i64) -> Result<i64, String> {
    let year = to_datetime("time_end_of_year", timestamp)?.year();
    let last = NaiveDate::from_ymd_opt(year, 12, 31).expect("every year has a 31st of December");
    return Ok(last.and_hms_opt(23, 59, 59).expect("the last second is a valid time").and_utc().timestamp());
}

/// The moment moved by a number of working days - days that are not Saturday
/// or Sunday - keeping the time of day. Negative goes backwards. A weekend
/// start does not count itself: Saturday plus one workday is Monday, and
/// Saturday minus one is Friday.
pub fn add_workdays(timestamp: i64, workdays: i64) -> Result<i64, String> {
    let mut moment = to_datetime("time_add_workdays", timestamp)?;
    let step = if workdays >= 0 { 1 } else { -1 };
    let mut remaining = workdays.abs();
    while remaining > 0 {
        moment = moment.checked_add_signed(Duration::days(step)).ok_or_else(|| format!("time_add_workdays: {} workdays from {} is off the calendar", workdays, timestamp))?;
        let day = moment.weekday();
        if day != chrono::Weekday::Sat && day != chrono::Weekday::Sun {
            remaining -= 1;
        }
    }
    return Ok(moment.timestamp());
}

/// How many weekday dates lie after the start's date, up to and including the
/// end's date. Monday to the same week's Friday is 4, Friday to the following
/// Monday is 1, and two moments on the same date are 0 - the start's own date
/// is never counted, so chaining ranges never counts a day twice. An end
/// before the start is an error rather than a negative count.
pub fn workdays_between(start: i64, end: i64) -> Result<i64, String> {
    let start_date = to_datetime("time_workdays_between", start)?.date_naive();
    let end_date = to_datetime("time_workdays_between", end)?.date_naive();
    if end_date < start_date {
        return Err(format!("time_workdays_between: the end ({}) is before the start ({})", end, start));
    }
    let days = (end_date - start_date).num_days();
    // Any seven consecutive dates hold exactly five weekdays, so only the
    // leftover days after the whole weeks need their weekday checked.
    let mut count = (days / 7) * 5;
    let start_weekday = start_date.weekday().number_from_monday() as i64;
    for offset in 1..=(days % 7) {
        let weekday_number = (start_weekday - 1 + offset) % 7 + 1;
        if weekday_number <= 5 {
            count += 1;
        }
    }
    return Ok(count);
}

/// The whole days between the calendar dates of two moments, signed - negative
/// when the end is earlier. The clock is ignored: 23:00 to 01:00 the next
/// morning is one day, because the date changed once.
pub fn days_between(start: i64, end: i64) -> Result<i64, String> {
    let start_date = to_datetime("time_days_between", start)?.date_naive();
    let end_date = to_datetime("time_days_between", end)?.date_naive();
    return Ok((end_date - start_date).num_days());
}

/// The whole calendar months between two moments, signed. A month counts only
/// once the same day of the month has been reached: the 15th of January to the
/// 14th of March is one month, to the 15th is two. This is how a person counts
/// a subscription, and the same rule Java's LocalDate uses.
pub fn months_between(start: i64, end: i64) -> Result<i64, String> {
    let start_date = to_datetime("time_months_between", start)?.date_naive();
    let end_date = to_datetime("time_months_between", end)?.date_naive();
    let mut months = (end_date.year() as i64 - start_date.year() as i64) * 12 + (end_date.month() as i64 - start_date.month() as i64);
    if months > 0 && end_date.day() < start_date.day() {
        months -= 1;
    } else if months < 0 && end_date.day() > start_date.day() {
        months += 1;
    }
    return Ok(months);
}

/// Whether two moments fall on the same calendar date, in UTC - the same date,
/// not within twenty-four hours of each other.
pub fn same_day(first: i64, second: i64) -> Result<bool, String> {
    return Ok(to_datetime("time_same_day", first)?.date_naive() == to_datetime("time_same_day", second)?.date_naive());
}

/// Whether a moment falls on the first day of its month, in UTC - the day the
/// monthly jobs run.
pub fn is_first_of_month(timestamp: i64) -> Result<bool, String> {
    return Ok(to_datetime("time_is_first_of_month", timestamp)?.day() == 1);
}

/// The age in whole years at a moment, counted the way a person counts it: it
/// goes up on the birthday, not at New Year. A leap-day birthday turns over on
/// the first of March in ordinary years. A moment before the birth is an error
/// rather than a negative age.
pub fn age_years(born: i64, at: i64) -> Result<i64, String> {
    let born_date = to_datetime("time_age_years", born)?.date_naive();
    let at_date = to_datetime("time_age_years", at)?.date_naive();
    if at_date < born_date {
        return Err(format!("time_age_years: the moment ({}) is before the birth ({})", at, born));
    }
    let mut age = at_date.year() as i64 - born_date.year() as i64;
    if (at_date.month(), at_date.day()) < (born_date.month(), born_date.day()) {
        age -= 1;
    }
    return Ok(age);
}

/// A day of the week, for asking about the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TIME_Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl TIME_Weekday {
    /// Monday is 1 through Sunday, 7 - ISO 8601's numbering, the same one
    /// chrono's number_from_monday uses.
    fn number_from_monday(self) -> i64 {
        return match self {
            TIME_Weekday::Monday => 1,
            TIME_Weekday::Tuesday => 2,
            TIME_Weekday::Wednesday => 3,
            TIME_Weekday::Thursday => 4,
            TIME_Weekday::Friday => 5,
            TIME_Weekday::Saturday => 6,
            TIME_Weekday::Sunday => 7,
        };
    }
}

/// Which one of a weekday in a month a rule means. `Last` is its own answer
/// rather than a count, because "the last Friday" is what the rule says and how
/// many Fridays a month has depends on the month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TIME_Nth {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Last,
}

impl TIME_Nth {
    /// How many of that weekday have gone by including this one, or None for
    /// `Last`, which is counted from the end of the month instead.
    fn count(self) -> Option<i64> {
        return match self {
            TIME_Nth::First => Some(1),
            TIME_Nth::Second => Some(2),
            TIME_Nth::Third => Some(3),
            TIME_Nth::Fourth => Some(4),
            TIME_Nth::Fifth => Some(5),
            TIME_Nth::Last => None,
        };
    }

    /// What to call it in an error, so the message says "a fifth" rather than
    /// a number the caller never typed.
    fn name(self) -> &'static str {
        return match self {
            TIME_Nth::First => "a first",
            TIME_Nth::Second => "a second",
            TIME_Nth::Third => "a third",
            TIME_Nth::Fourth => "a fourth",
            TIME_Nth::Fifth => "a fifth",
            TIME_Nth::Last => "a last",
        };
    }
}

/// The next date strictly after the moment that falls on the given weekday,
/// keeping the time of day. Strictly after means a Monday asked for the next
/// Monday gets the one a week out, which is what "next Monday" means on any
/// day of the week.
pub fn next_weekday(timestamp: i64, weekday: TIME_Weekday) -> Result<i64, String> {
    let moment = to_datetime("time_next_weekday", timestamp)?;
    let today = moment.weekday().number_from_monday() as i64;
    let mut days_ahead = (weekday.number_from_monday() - today).rem_euclid(7);
    if days_ahead == 0 {
        days_ahead = 7;
    }
    return Ok(add_days(timestamp, days_ahead));
}

/// The date a rule like "the third Monday in January" or "the last Friday of
/// the month" names, at midnight UTC. This is how holidays, pay days, standing
/// meetings and billing dates are written down, and none of them can be worked
/// out by adding days to anything.
///
/// A month with only four Mondays asked for its fifth is an error rather than
/// the first Monday of the month after. `TIME_Nth::Last` never has that problem,
/// which is why it is a choice of its own rather than a count.
pub fn nth_weekday_of_month(year: i64, month: i64, weekday: TIME_Weekday, nth: TIME_Nth) -> Result<i64, String> {
    if !(1..=12).contains(&month) {
        return Err(format!("time_nth_weekday_of_month: {} is not a month", month));
    }

    let first_of_month = from_parts(year, month, 1, 0, 0, 0)?;
    let days_in_this_month = days_in_month(year, month)?;
    let first_weekday = to_datetime("time_nth_weekday_of_month", first_of_month)?.weekday().number_from_monday() as i64;
    let first_matching_day = 1 + (weekday.number_from_monday() - first_weekday).rem_euclid(7);

    let day = match nth.count() {
        Some(count) => first_matching_day + (count - 1) * 7,
        // Step back from the last one that fits inside the month.
        None => first_matching_day + ((days_in_this_month - first_matching_day) / 7) * 7,
    };
    if day > days_in_this_month {
        return Err(format!("time_nth_weekday_of_month: {}-{:02} does not have {} of that weekday in it", year, month, nth.name()));
    }
    return from_parts(year, month, day, 0, 0, 0);
}

#[cfg(test)]
mod workweek_and_boundary_tests {
    use super::*;

    /// A timestamp for a moment, so the tests read as dates rather than numbers.
    fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
        return from_parts(year, month, day, hour, minute, second).expect("a real date");
    }

    /// 2024-01-15 12:30:45 UTC, a Monday in the third ISO week of 2024.
    const MONDAY: i64 = 1_705_321_845;

    #[test]
    fn the_pinned_monday_is_the_date_its_comment_says() {
        assert_eq!(MONDAY, at(2024, 1, 15, 12, 30, 45));
        assert_eq!(weekday(MONDAY).expect("a real date"), "Monday");
    }

    #[test]
    fn the_year_splits_into_four_quarters() {
        assert_eq!(quarter(at(2024, 1, 1, 0, 0, 0)).unwrap(), 1);
        assert_eq!(quarter(at(2024, 3, 31, 23, 59, 59)).unwrap(), 1);
        assert_eq!(quarter(at(2024, 4, 1, 0, 0, 0)).unwrap(), 2);
        assert_eq!(quarter(at(2024, 9, 30, 0, 0, 0)).unwrap(), 3);
        assert_eq!(quarter(at(2024, 12, 31, 0, 0, 0)).unwrap(), 4);
    }

    #[test]
    fn iso_weeks_belong_to_the_year_of_their_thursday() {
        assert_eq!(week_of_year(MONDAY).unwrap(), 3);
        // 2023-01-01 is a Sunday, still in the last week of 2022's numbering.
        assert_eq!(week_of_year(at(2023, 1, 1, 12, 0, 0)).unwrap(), 52);
        // 2024-12-30 is a Monday whose Thursday is already in 2025.
        assert_eq!(week_of_year(at(2024, 12, 30, 0, 0, 0)).unwrap(), 1);
    }

    #[test]
    fn the_weekend_is_saturday_and_sunday() {
        assert!(!is_weekend(MONDAY).unwrap());
        // 2024-01-13 and 14 are the Saturday and Sunday before it.
        assert!(is_weekend(at(2024, 1, 13, 12, 0, 0)).unwrap());
        assert!(is_weekend(at(2024, 1, 14, 12, 0, 0)).unwrap());
        // 2024-01-19 is the Friday after it.
        assert!(!is_weekend(at(2024, 1, 19, 12, 0, 0)).unwrap());
    }

    #[test]
    fn a_month_runs_from_its_first_midnight_to_its_last_second() {
        let mid_february = at(2024, 2, 15, 12, 30, 45);
        assert_eq!(start_of_month(mid_february).unwrap(), at(2024, 2, 1, 0, 0, 0));
        // 2024 is a leap year, so its February runs to the 29th.
        assert_eq!(end_of_month(mid_february).unwrap(), at(2024, 2, 29, 23, 59, 59));
        assert_eq!(end_of_month(at(2023, 2, 10, 0, 0, 0)).unwrap(), at(2023, 2, 28, 23, 59, 59));
        let last_second = at(2024, 12, 31, 23, 59, 59);
        assert_eq!(end_of_month(last_second).unwrap(), last_second, "the last second is already the end of its month");
    }

    #[test]
    fn a_week_runs_from_monday_midnight_to_sunday_last_second() {
        // 2024-01-17 is the Wednesday after MONDAY.
        let wednesday = at(2024, 1, 17, 9, 15, 0);
        assert_eq!(start_of_week(wednesday).unwrap(), at(2024, 1, 15, 0, 0, 0));
        assert_eq!(end_of_week(wednesday).unwrap(), at(2024, 1, 21, 23, 59, 59));
        // A Monday is already in its own week, and a Sunday ends the same week.
        assert_eq!(start_of_week(MONDAY).unwrap(), at(2024, 1, 15, 0, 0, 0));
        assert_eq!(start_of_week(at(2024, 1, 21, 12, 0, 0)).unwrap(), at(2024, 1, 15, 0, 0, 0));
    }

    #[test]
    fn a_year_runs_from_january_first_to_december_thirty_first() {
        assert_eq!(start_of_year(MONDAY).unwrap(), at(2024, 1, 1, 0, 0, 0));
        assert_eq!(end_of_year(MONDAY).unwrap(), at(2024, 12, 31, 23, 59, 59));
    }

    #[test]
    fn workdays_skip_the_weekend_in_both_directions() {
        // Friday 2024-01-19 plus one workday is Monday the 22nd, same clock.
        let friday = at(2024, 1, 19, 14, 30, 0);
        let monday_after = at(2024, 1, 22, 14, 30, 0);
        assert_eq!(add_workdays(friday, 1).unwrap(), monday_after);
        assert_eq!(add_workdays(friday, 5).unwrap(), at(2024, 1, 26, 14, 30, 0));
        assert_eq!(add_workdays(monday_after, -1).unwrap(), friday);
        assert_eq!(add_workdays(friday, 0).unwrap(), friday);
    }

    #[test]
    fn a_weekend_start_does_not_count_itself() {
        // Saturday 2024-01-13: one workday on is Monday, one back is Friday.
        let saturday = at(2024, 1, 13, 9, 0, 0);
        assert_eq!(add_workdays(saturday, 1).unwrap(), at(2024, 1, 15, 9, 0, 0));
        assert_eq!(add_workdays(saturday, -1).unwrap(), at(2024, 1, 12, 9, 0, 0));
    }

    #[test]
    fn workdays_between_counts_weekday_dates_after_the_start() {
        let monday = at(2024, 1, 15, 9, 0, 0);
        assert_eq!(workdays_between(monday, at(2024, 1, 19, 17, 0, 0)).unwrap(), 4, "Monday to the same week's Friday");
        assert_eq!(workdays_between(at(2024, 1, 19, 9, 0, 0), at(2024, 1, 22, 9, 0, 0)).unwrap(), 1, "Friday to Monday crosses only the weekend");
        assert_eq!(workdays_between(monday, at(2024, 1, 22, 9, 0, 0)).unwrap(), 5, "one whole week on");
        assert_eq!(workdays_between(monday, at(2024, 1, 29, 9, 0, 0)).unwrap(), 10, "two whole weeks on");
        assert_eq!(workdays_between(monday, monday).unwrap(), 0, "the start's own date is never counted");
        // Saturday the 13th to Sunday the 14th holds no weekdays at all.
        assert_eq!(workdays_between(at(2024, 1, 13, 0, 0, 0), at(2024, 1, 14, 23, 0, 0)).unwrap(), 0);
        assert!(workdays_between(monday, at(2024, 1, 12, 0, 0, 0)).unwrap_err().contains("before the start"));
    }

    #[test]
    fn days_between_is_signed_and_ignores_the_clock() {
        assert_eq!(days_between(at(2024, 1, 15, 23, 0, 0), at(2024, 1, 16, 1, 0, 0)).unwrap(), 1);
        assert_eq!(days_between(at(2024, 1, 16, 1, 0, 0), at(2024, 1, 15, 23, 0, 0)).unwrap(), -1);
        assert_eq!(days_between(at(2024, 1, 15, 0, 0, 0), at(2024, 1, 15, 23, 59, 59)).unwrap(), 0);
        assert_eq!(days_between(at(2024, 1, 1, 0, 0, 0), at(2025, 1, 1, 0, 0, 0)).unwrap(), 366, "2024 is a leap year");
    }

    #[test]
    fn months_count_only_once_the_day_of_the_month_arrives() {
        assert_eq!(months_between(at(2024, 1, 15, 0, 0, 0), at(2024, 3, 15, 0, 0, 0)).unwrap(), 2);
        assert_eq!(months_between(at(2024, 1, 15, 0, 0, 0), at(2024, 3, 14, 0, 0, 0)).unwrap(), 1);
        assert_eq!(months_between(at(2024, 1, 31, 0, 0, 0), at(2024, 2, 29, 0, 0, 0)).unwrap(), 0, "the 31st never arrives in February");
        assert_eq!(months_between(at(2024, 3, 15, 0, 0, 0), at(2024, 1, 16, 0, 0, 0)).unwrap(), -1);
        assert_eq!(months_between(at(2023, 6, 1, 0, 0, 0), at(2024, 6, 1, 0, 0, 0)).unwrap(), 12);
    }

    #[test]
    fn the_same_day_is_the_same_date_not_within_twenty_four_hours() {
        assert!(same_day(at(2024, 1, 15, 0, 0, 0), at(2024, 1, 15, 23, 59, 59)).unwrap());
        assert!(!same_day(at(2024, 1, 15, 23, 59, 59), at(2024, 1, 16, 0, 0, 0)).unwrap());
    }

    #[test]
    fn the_first_of_the_month_is_recognised() {
        assert!(is_first_of_month(at(2024, 2, 1, 18, 0, 0)).unwrap());
        assert!(!is_first_of_month(at(2024, 2, 2, 0, 0, 0)).unwrap());
    }

    #[test]
    fn age_goes_up_on_the_birthday_and_not_before() {
        let born = at(1990, 6, 15, 8, 0, 0);
        assert_eq!(age_years(born, at(2024, 6, 14, 23, 0, 0)).unwrap(), 33);
        assert_eq!(age_years(born, at(2024, 6, 15, 0, 0, 0)).unwrap(), 34);
        // A leap-day birthday turns over on the 1st of March in ordinary years.
        let leapling = at(2000, 2, 29, 12, 0, 0);
        assert_eq!(age_years(leapling, at(2023, 2, 28, 0, 0, 0)).unwrap(), 22);
        assert_eq!(age_years(leapling, at(2023, 3, 1, 0, 0, 0)).unwrap(), 23);
        assert!(age_years(born, at(1989, 1, 1, 0, 0, 0)).unwrap_err().contains("before the birth"));
    }

    #[test]
    fn the_next_weekday_is_strictly_after_and_keeps_the_clock() {
        // From Monday 2024-01-15 at 12:30:45.
        assert_eq!(next_weekday(MONDAY, TIME_Weekday::Friday).unwrap(), at(2024, 1, 19, 12, 30, 45), "the coming Friday");
        assert_eq!(next_weekday(MONDAY, TIME_Weekday::Sunday).unwrap(), at(2024, 1, 21, 12, 30, 45), "the coming Sunday");
        assert_eq!(next_weekday(MONDAY, TIME_Weekday::Monday).unwrap(), at(2024, 1, 22, 12, 30, 45), "a Monday's next Monday is a week out");
    }

    #[test]
    fn a_timestamp_too_far_from_1970_is_refused_by_every_boundary() {
        assert!(quarter(i64::MAX).unwrap_err().contains("too far from 1970"));
        assert!(start_of_month(i64::MAX).unwrap_err().contains("too far from 1970"));
        assert!(add_workdays(i64::MAX, 1).unwrap_err().contains("too far from 1970"));
        assert!(workdays_between(0, i64::MAX).unwrap_err().contains("too far from 1970"));
    }
}

/// The weekday number Monday 1 through Sunday 7 for a lower-case name, if the
/// word is one.
fn weekday_number(name: &str) -> Option<TIME_Weekday> {
    return match name {
        "monday" => Some(TIME_Weekday::Monday),
        "tuesday" => Some(TIME_Weekday::Tuesday),
        "wednesday" => Some(TIME_Weekday::Wednesday),
        "thursday" => Some(TIME_Weekday::Thursday),
        "friday" => Some(TIME_Weekday::Friday),
        "saturday" => Some(TIME_Weekday::Saturday),
        "sunday" => Some(TIME_Weekday::Sunday),
        _ => None,
    };
}

/// Shifts the reference by a count of one named unit, or None when the word is
/// not a unit this reads. Months go through add_months because a month is not
/// a fixed number of seconds.
fn human_shift(reference: i64, count: i64, unit: &str) -> Option<Result<i64, String>> {
    let singular = unit.strip_suffix('s').unwrap_or(unit);
    let seconds_per = match singular {
        "second" => 1,
        "minute" => 60,
        "hour" => 3600,
        "day" => 86_400,
        "week" => 604_800,
        "month" => {
            return Some(add_months(reference, count).map_err(|_| format!("time_parse_human: {} months from {} is off the calendar", count, reference)));
        }
        _ => return None,
    };
    return Some(Ok(reference + count * seconds_per));
}

/// Reads a plain-English moment relative to a reference timestamp: `now`,
/// `today`, `tomorrow`, `yesterday`, `next monday` through `next sunday`,
/// `last monday` through `last sunday`, `in N seconds/minutes/hours/days/weeks/months`,
/// the same units with `N units ago`, and an absolute `YYYY-MM-DD` date.
/// Case and extra spaces are forgiven. `today`, `tomorrow`, `yesterday` and an
/// absolute date land on midnight UTC, while `next` and `last` weekdays keep
/// the reference's time of day, the way time_next_weekday does. Anything else
/// is an error naming the shapes this reads.
pub fn parse_human(text: String, reference: i64) -> Result<i64, String> {
    let cleaned = text.to_lowercase();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let unreadable = || {
        format!(
            "time_parse_human: could not read `{}` - this reads now, today, tomorrow, yesterday, next or last plus a weekday name, in N seconds/minutes/hours/days/weeks/months, N of those units ago, and YYYY-MM-DD",
            text.trim()
        )
    };
    let too_far = |stamp: i64| format!("time_parse_human: {} is too far from 1970 to be a date", stamp);

    match words.as_slice() {
        ["now"] => return Ok(reference),
        ["today"] => return start_of_day(reference).map_err(|_| too_far(reference)),
        ["tomorrow"] => return start_of_day(reference).map(|midnight| add_days(midnight, 1)).map_err(|_| too_far(reference)),
        ["yesterday"] => return start_of_day(reference).map(|midnight| add_days(midnight, -1)).map_err(|_| too_far(reference)),
        ["next", day] if weekday_number(day).is_some() => {
            let target = weekday_number(day).expect("the guard checked the name");
            return next_weekday(reference, target).map_err(|_| too_far(reference));
        }
        ["last", day] if weekday_number(day).is_some() => {
            let target = weekday_number(day).expect("the guard checked the name");
            let moment = to_datetime("time_parse_human", reference)?;
            let today_number = moment.weekday().number_from_monday() as i64;
            let mut back = (today_number - target.number_from_monday()).rem_euclid(7);
            if back == 0 {
                back = 7;
            }
            return Ok(add_days(reference, -back));
        }
        ["in", count_text, unit] => {
            let count: i64 = match count_text.parse() {
                Ok(number) => number,
                Err(_) => return Err(unreadable()),
            };
            return match human_shift(reference, count, unit) {
                Some(shifted) => shifted,
                None => Err(unreadable()),
            };
        }
        [count_text, unit, "ago"] => {
            let count: i64 = match count_text.parse() {
                Ok(number) => number,
                Err(_) => return Err(unreadable()),
            };
            return match human_shift(reference, -count, unit) {
                Some(shifted) => shifted,
                None => Err(unreadable()),
            };
        }
        [only] => {
            if let Ok(date) = NaiveDate::parse_from_str(only, "%Y-%m-%d") {
                return Ok(date.and_hms_opt(0, 0, 0).expect("midnight is a valid time").and_utc().timestamp());
            }
            return Err(unreadable());
        }
        _ => return Err(unreadable()),
    }
}

/// The name of a cron weekday number, with 0 and 7 both Sunday as they are in
/// every cron there has ever been.
fn cron_weekday_name(number: i64) -> Option<&'static str> {
    return match number {
        0 | 7 => Some("Sunday"),
        1 => Some("Monday"),
        2 => Some("Tuesday"),
        3 => Some("Wednesday"),
        4 => Some("Thursday"),
        5 => Some("Friday"),
        6 => Some("Saturday"),
        _ => None,
    };
}

/// A cron weekday field written in words, when it fits the vocabulary: a
/// single day, a range of days, or a comma list of days, with `1-5` reading as
/// weekdays. None sends the whole expression to the field-by-field fallback.
fn cron_weekday_phrase(field: &str) -> Option<String> {
    if field == "1-5" {
        return Some("weekdays".to_string());
    }
    if let Some((first, last)) = field.split_once('-') {
        let first_name = cron_weekday_name(first.parse().ok()?)?;
        let last_name = cron_weekday_name(last.parse().ok()?)?;
        return Some(format!("{} to {}", first_name, last_name));
    }
    if field.contains(',') {
        let names: Vec<&str> = field.split(',').map(|token| cron_weekday_name(token.parse().ok()?)).collect::<Option<Vec<&str>>>()?;
        let mut phrase = names[..names.len() - 1].join(", ");
        phrase.push_str(" and ");
        phrase.push_str(names[names.len() - 1]);
        return Some(phrase);
    }
    return Some(cron_weekday_name(field.parse().ok()?)?.to_string());
}

/// A day of the month written for a sentence: the first day, the 2nd day, the
/// 15th day.
fn cron_day_phrase(day: i64) -> String {
    if day == 1 {
        return "first".to_string();
    }
    let suffix = match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    return format!("{}{}", day, suffix);
}

/// A five-field cron expression written out in words. `0 3 * * *` reads
/// `every day at 03:00`, `*/15 * * * *` reads `every 15 minutes`,
/// `0 9 * * 1-5` reads `at 09:00 on weekdays`, and `0 0 1 * *` reads
/// `at 00:00 on the first day of the month`. An expression beyond that
/// vocabulary gets a faithful field-by-field reading such as
/// `minute 5,35, hour 3, any day, month 2, any weekday` rather than an error.
/// Only an expression whose five fields do not parse is an error.
pub fn cron_describe(expression: String) -> Result<String, String> {
    parse_cron(&expression).map_err(|detail| format!("time_cron_describe: {}", detail))?;
    let fields: Vec<&str> = expression.split_whitespace().collect();
    let (minute, hour, day_of_month, month, day_of_week) = (fields[0], fields[1], fields[2], fields[3], fields[4]);

    let fixed_minute: Option<i64> = minute.parse().ok();
    let fixed_hour: Option<i64> = hour.parse().ok();
    let rest_any = day_of_month == "*" && month == "*" && day_of_week == "*";

    if minute == "*" && hour == "*" && rest_any {
        return Ok("every minute".to_string());
    }
    if let Some(step) = minute.strip_prefix("*/") {
        if hour == "*" && rest_any {
            if step == "1" {
                return Ok("every minute".to_string());
            }
            return Ok(format!("every {} minutes", step));
        }
    }
    if let (Some(minute_number), Some(step)) = (fixed_minute, hour.strip_prefix("*/")) {
        if rest_any {
            let cadence = if step == "1" { "every hour".to_string() } else { format!("every {} hours", step) };
            if minute_number == 0 {
                return Ok(cadence);
            }
            return Ok(format!("{} at minute {}", cadence, minute_number));
        }
    }
    if let Some(minute_number) = fixed_minute {
        if hour == "*" && rest_any {
            if minute_number == 0 {
                return Ok("every hour".to_string());
            }
            return Ok(format!("every hour at minute {}", minute_number));
        }
    }
    if let (Some(minute_number), Some(hour_number)) = (fixed_minute, fixed_hour) {
        let clock = format!("{:02}:{:02}", hour_number, minute_number);
        if rest_any {
            return Ok(format!("every day at {}", clock));
        }
        if day_of_month == "*" && month == "*" {
            if let Some(days) = cron_weekday_phrase(day_of_week) {
                return Ok(format!("at {} on {}", clock, days));
            }
        }
        if month == "*" && day_of_week == "*" {
            if let Ok(day_number) = day_of_month.parse::<i64>() {
                return Ok(format!("at {} on the {} day of the month", clock, cron_day_phrase(day_number)));
            }
        }
    }

    // The faithful field-by-field fallback for anything beyond the vocabulary.
    let piece = |name: &str, any_name: &str, value: &str| if value == "*" { any_name.to_string() } else { format!("{} {}", name, value) };
    return Ok(format!(
        "{}, {}, {}, {}, {}",
        piece("minute", "any minute", minute),
        piece("hour", "any hour", hour),
        piece("day", "any day", day_of_month),
        piece("month", "any month", month),
        piece("weekday", "any weekday", day_of_week)
    ));
}

#[cfg(test)]
mod human_time_tests {
    use super::*;

    /// A timestamp for a moment, so the tests read as dates rather than numbers.
    fn at(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
        return from_parts(year, month, day, hour, minute, second).expect("a real date");
    }

    /// 2024-01-17 12:30:45 UTC, a Wednesday. Every relative reading in these
    /// tests is measured from here.
    fn wednesday() -> i64 {
        let reference = at(2024, 1, 17, 12, 30, 45);
        assert_eq!(weekday(reference).expect("a real date"), "Wednesday");
        return reference;
    }

    fn read(text: &str) -> i64 {
        return parse_human(text.to_string(), wednesday()).expect("a readable moment");
    }

    #[test]
    fn now_is_the_reference_itself() {
        assert_eq!(read("now"), wednesday());
    }

    #[test]
    fn the_named_days_land_on_their_midnights() {
        assert_eq!(read("today"), at(2024, 1, 17, 0, 0, 0));
        assert_eq!(read("tomorrow"), at(2024, 1, 18, 0, 0, 0));
        assert_eq!(read("yesterday"), at(2024, 1, 16, 0, 0, 0));
    }

    #[test]
    fn next_and_last_weekdays_keep_the_clock() {
        // From Wednesday the 17th, the next Monday is the 22nd and the last
        // Monday was the 15th.
        assert_eq!(read("next monday"), at(2024, 1, 22, 12, 30, 45));
        assert_eq!(read("last monday"), at(2024, 1, 15, 12, 30, 45));
        // A Wednesday's next Wednesday is a week out, and its last Wednesday
        // a week back, on the reading time_next_weekday uses.
        assert_eq!(read("next wednesday"), at(2024, 1, 24, 12, 30, 45));
        assert_eq!(read("last wednesday"), at(2024, 1, 10, 12, 30, 45));
        assert_eq!(read("next sunday"), at(2024, 1, 21, 12, 30, 45));
        assert_eq!(read("last sunday"), at(2024, 1, 14, 12, 30, 45));
    }

    #[test]
    fn fixed_units_shift_forwards_and_back() {
        assert_eq!(read("in 30 seconds"), wednesday() + 30);
        assert_eq!(read("in 2 hours"), wednesday() + 7200);
        assert_eq!(read("in 1 day"), wednesday() + 86_400);
        assert_eq!(read("in 2 weeks"), wednesday() + 14 * 86_400);
        assert_eq!(read("3 days ago"), wednesday() - 3 * 86_400);
        assert_eq!(read("45 minutes ago"), wednesday() - 2700);
    }

    #[test]
    fn month_steps_cross_the_year_end_in_both_directions() {
        // Two months back from January 2024 is November 2023, and twelve
        // months on is January 2025.
        assert_eq!(read("2 months ago"), at(2023, 11, 17, 12, 30, 45));
        assert_eq!(read("in 12 months"), at(2025, 1, 17, 12, 30, 45));
        assert_eq!(read("in 1 month"), at(2024, 2, 17, 12, 30, 45));
    }

    #[test]
    fn an_absolute_date_reads_at_its_midnight() {
        assert_eq!(read("2024-06-15"), at(2024, 6, 15, 0, 0, 0));
        // 2024 is a leap year, so its 29th of February exists.
        assert_eq!(read("2024-02-29"), at(2024, 2, 29, 0, 0, 0));
    }

    #[test]
    fn a_day_that_is_not_on_the_calendar_is_an_error() {
        assert!(parse_human("2023-02-29".to_string(), wednesday()).unwrap_err().contains("could not read"));
    }

    #[test]
    fn case_and_extra_spaces_are_forgiven() {
        assert_eq!(read("  NEXT   Monday  "), at(2024, 1, 22, 12, 30, 45));
        assert_eq!(read("In  2  HOURS"), wednesday() + 7200);
        assert_eq!(read(" Tomorrow "), at(2024, 1, 18, 0, 0, 0));
    }

    #[test]
    fn anything_else_names_the_shapes_it_reads() {
        let failure = parse_human("half past three".to_string(), wednesday()).unwrap_err();
        assert!(failure.contains("YYYY-MM-DD"), "got: {}", failure);
        assert!(failure.contains("next or last"), "got: {}", failure);
        assert!(parse_human("in five days".to_string(), wednesday()).unwrap_err().contains("could not read"));
        assert!(parse_human("in 2 fortnights".to_string(), wednesday()).unwrap_err().contains("could not read"));
        assert!(parse_human("".to_string(), wednesday()).unwrap_err().contains("could not read"));
    }
}

#[cfg(test)]
mod cron_describe_tests {
    use super::cron_describe;

    fn described(expression: &str) -> String {
        return cron_describe(expression.to_string()).expect("a valid expression");
    }

    #[test]
    fn the_common_schedules_read_as_sentences() {
        assert_eq!(described("0 3 * * *"), "every day at 03:00");
        assert_eq!(described("*/15 * * * *"), "every 15 minutes");
        assert_eq!(described("0 9 * * 1-5"), "at 09:00 on weekdays");
        assert_eq!(described("0 0 1 * *"), "at 00:00 on the first day of the month");
    }

    #[test]
    fn stars_and_steps_read_as_cadences() {
        assert_eq!(described("* * * * *"), "every minute");
        assert_eq!(described("*/1 * * * *"), "every minute");
        assert_eq!(described("0 */2 * * *"), "every 2 hours");
        assert_eq!(described("30 */6 * * *"), "every 6 hours at minute 30");
        assert_eq!(described("0 * * * *"), "every hour");
        assert_eq!(described("30 * * * *"), "every hour at minute 30");
    }

    #[test]
    fn weekdays_read_by_name_with_sunday_at_both_ends() {
        assert_eq!(described("0 12 * * 5"), "at 12:00 on Friday");
        assert_eq!(described("0 0 * * 0"), "at 00:00 on Sunday");
        assert_eq!(described("0 0 * * 7"), "at 00:00 on Sunday");
        assert_eq!(described("0 8 * * 1,3,5"), "at 08:00 on Monday, Wednesday and Friday");
        assert_eq!(described("0 8 * * 2-4"), "at 08:00 on Tuesday to Thursday");
    }

    #[test]
    fn fixed_days_of_the_month_read_as_ordinals() {
        assert_eq!(described("0 0 2 * *"), "at 00:00 on the 2nd day of the month");
        assert_eq!(described("30 6 15 * *"), "at 06:30 on the 15th day of the month");
    }

    #[test]
    fn anything_beyond_the_vocabulary_reads_field_by_field() {
        assert_eq!(described("5,35 3 * 2 *"), "minute 5,35, hour 3, any day, month 2, any weekday");
        assert_eq!(described("0 3 1 * 1"), "minute 0, hour 3, day 1, any month, weekday 1");
    }

    #[test]
    fn only_an_unparseable_expression_is_an_error() {
        assert!(cron_describe("0 3 * *".to_string()).unwrap_err().contains("five fields"));
        assert!(cron_describe("60 * * * *".to_string()).unwrap_err().contains("minute"));
        assert!(cron_describe("@daily".to_string()).unwrap_err().contains("five fields"));
    }
}

#[cfg(feature = "timezones")]
fn parse_zone(zone: &str, what: &str) -> Result<chrono_tz::Tz, String> {
    return zone.trim().parse::<chrono_tz::Tz>().map_err(|_| format!("{}: `{}` is not an IANA zone name - try the `America/Edmonton` form, or time_list_zones()", what, zone.trim()));
}

/// A moment shown on the wall clock of a place, in your strftime layout.
/// Zones are IANA names; daylight saving is the zone database's problem, not yours.
#[cfg(feature = "timezones")]
pub fn format_in_zone(timestamp: i64, zone: String, layout: String) -> Result<String, String> {
    let tz = parse_zone(&zone, "time_format_in_zone")?;
    let moment = chrono::DateTime::from_timestamp(timestamp, 0).ok_or_else(|| format!("time_format_in_zone: {} is not a moment", timestamp))?;
    if chrono::format::StrftimeItems::new(&layout).any(|item| matches!(item, chrono::format::Item::Error)) {
        return Err(format!("time_format_in_zone: '{}' is not a valid layout - see the strftime notation, such as %Y-%m-%d %H:%M", layout));
    }
    return Ok(moment.with_timezone(&tz).format(&layout).to_string());
}

/// Read a wall-clock time as seen in a place back into a timestamp. An
/// ambiguous time (the repeated hour when clocks fall back) takes the earlier
/// reading; a time that never happens (the skipped hour) is an error.
#[cfg(feature = "timezones")]
pub fn parse_in_zone(text: String, layout: String, zone: String) -> Result<i64, String> {
    use chrono::TimeZone;
    let tz = parse_zone(&zone, "time_parse_in_zone")?;
    let naive = chrono::NaiveDateTime::parse_from_str(text.trim(), &layout).map_err(|e| format!("time_parse_in_zone: `{}` does not read as `{}`: {}", text.trim(), layout, e))?;
    return match tz.from_local_datetime(&naive) {
        chrono::offset::LocalResult::Single(moment) => Ok(moment.timestamp()),
        chrono::offset::LocalResult::Ambiguous(earlier, _) => Ok(earlier.timestamp()),
        chrono::offset::LocalResult::None => Err(format!("time_parse_in_zone: `{}` never happens in {} - the clock jumps over it", text.trim(), zone.trim())),
    };
}

/// How far ahead of UTC a place is at a moment, in seconds. Negative is behind.
#[cfg(feature = "timezones")]
pub fn zone_offset(timestamp: i64, zone: String) -> Result<i64, String> {
    use chrono::Offset;
    let tz = parse_zone(&zone, "time_zone_offset")?;
    let moment = chrono::DateTime::from_timestamp(timestamp, 0).ok_or_else(|| format!("time_zone_offset: {} is not a moment", timestamp))?;
    return Ok(moment.with_timezone(&tz).offset().fix().local_minus_utc() as i64);
}

/// Whether a zone name is in the IANA database.
#[cfg(feature = "timezones")]
pub fn zone_valid(zone: String) -> bool {
    return zone.trim().parse::<chrono_tz::Tz>().is_ok();
}

/// Every zone name the database knows, for picking lists.
#[cfg(feature = "timezones")]
pub fn list_zones() -> Vec<String> {
    return chrono_tz::TZ_VARIANTS.iter().map(|tz| tz.name().to_string()).collect();
}

#[cfg(all(test, feature = "timezones"))]
mod zone_tests {
    use super::*;

    // 2024-01-15 12:00:00 UTC, deep in mountain standard time.
    const WINTER_NOON_UTC: i64 = 1705320000;

    #[test]
    fn edmonton_reads_seven_hours_behind_in_winter() {
        assert_eq!(format_in_zone(WINTER_NOON_UTC, "America/Edmonton".to_string(), "%H:%M".to_string()).unwrap(), "05:00");
        assert_eq!(zone_offset(WINTER_NOON_UTC, "America/Edmonton".to_string()).unwrap(), -7 * 3600);
        assert_eq!(zone_offset(WINTER_NOON_UTC, "UTC".to_string()).unwrap(), 0);
    }

    #[test]
    fn a_wall_clock_time_comes_back_as_the_same_moment() {
        let stamp = parse_in_zone("2024-01-15 05:00".to_string(), "%Y-%m-%d %H:%M".to_string(), "America/Edmonton".to_string()).unwrap();
        assert_eq!(stamp, WINTER_NOON_UTC);
    }

    #[test]
    fn zone_names_are_checked_and_listable() {
        assert!(zone_valid("Asia/Tokyo".to_string()));
        assert!(!zone_valid("Mars/Olympus_Mons".to_string()));
        assert!(format_in_zone(0, "Mars/Olympus_Mons".to_string(), "%H".to_string()).unwrap_err().contains("not an IANA zone name"));
        let zones = list_zones();
        assert!(zones.iter().any(|z| z == "America/Edmonton"));
        assert!(zones.len() > 400);
    }
}

#[cfg(test)]
mod nth_weekday_tests {
    use super::*;

    fn date(year: i64, month: i64, day: i64) -> i64 {
        return from_parts(year, month, day, 0, 0, 0).expect("a real date");
    }

    #[test]
    fn a_rule_like_the_third_monday_names_a_real_date() {
        // Family Day in Alberta, the third Monday in February 2026.
        assert_eq!(nth_weekday_of_month(2026, 2, TIME_Weekday::Monday, TIME_Nth::Third).expect("a real rule"), date(2026, 2, 16));
        // American Thanksgiving, the fourth Thursday in November 2026.
        assert_eq!(nth_weekday_of_month(2026, 11, TIME_Weekday::Thursday, TIME_Nth::Fourth).expect("a real rule"), date(2026, 11, 26));
        // The first of the month landing on the weekday asked for.
        assert_eq!(nth_weekday_of_month(2026, 5, TIME_Weekday::Friday, TIME_Nth::First).expect("a real rule"), date(2026, 5, 1));
    }

    #[test]
    fn the_last_one_is_found_without_knowing_the_length_of_the_month() {
        assert_eq!(nth_weekday_of_month(2026, 5, TIME_Weekday::Friday, TIME_Nth::Last).expect("a real rule"), date(2026, 5, 29));
        assert_eq!(nth_weekday_of_month(2024, 2, TIME_Weekday::Thursday, TIME_Nth::Last).expect("a real rule"), date(2024, 2, 29), "a leap day is still a Thursday");
        assert_eq!(nth_weekday_of_month(2026, 2, TIME_Weekday::Saturday, TIME_Nth::Last).expect("a real rule"), date(2026, 2, 28));
    }

    #[test]
    fn a_month_without_that_many_says_so_instead_of_spilling_over() {
        // February 2026 has four Mondays, not five.
        let missing = nth_weekday_of_month(2026, 2, TIME_Weekday::Monday, TIME_Nth::Fifth).unwrap_err();
        assert!(missing.contains("does not have a fifth"), "got: {}", missing);
        assert!(nth_weekday_of_month(2026, 13, TIME_Weekday::Monday, TIME_Nth::First).unwrap_err().contains("not a month"));
    }
}
