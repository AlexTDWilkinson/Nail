//! Time module stdlib registry entries.

use super::*;

/// One entry for a function that reads a part out of a timestamp: same shape,
/// same crate, same failure when the timestamp is too far from 1970 to be a
/// date.
fn part_of_date(rust_path: &str, description: &'static str, example: &'static str) -> StdlibFunction {
    return StdlibFunction {
        rust_path: rust_path.to_string(),
        crate_deps: vec![CrateDependency::Chrono],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Time,
        parameters: vec![nail_param!(timestamp: i)],
        return_type: nail_type!((i!e)),
        diverging: false,
        description,
        example,
    };
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Time:
        "time_now" => "std_lib::time::now", () -> i,
            "Returns the current Unix timestamp in seconds.",
            "now:i = time_now();";
        "time_sleep" [Tokio] => "std_lib::time::sleep", (seconds: f) -> v,
            "Pauses the current task for the given number of seconds.",
            "time_sleep(0.5);";
        "time_now_millis" => "std_lib::time::now_millis", () -> i,
            "Returns the current Unix timestamp in milliseconds.",
            "now_ms:i = time_now_millis();";
        "time_add_seconds" => "std_lib::time::add_seconds", (timestamp: i, seconds: i) -> i,
            "Returns the timestamp shifted by the given number of seconds (negative to subtract).",
            "later:i = time_add_seconds(now, 3600);";
        "time_add_minutes" => "std_lib::time::add_minutes", (timestamp: i, minutes: i) -> i,
            "Returns the timestamp shifted by the given number of minutes (negative to subtract).",
            "later:i = time_add_minutes(now, 90);";
        "time_add_hours" => "std_lib::time::add_hours", (timestamp: i, hours: i) -> i,
            "Returns the timestamp shifted by the given number of hours (negative to subtract).",
            "later:i = time_add_hours(now, 24);";
        "time_add_days" => "std_lib::time::add_days", (timestamp: i, days: i) -> i,
            "Returns the timestamp shifted by the given number of days (negative to subtract).",
            "tomorrow:i = time_add_days(now, 1);";
        "time_add_months" [Chrono] => "std_lib::time::add_months", (timestamp: i, months: i) -> (i!e),
            "Returns the timestamp a number of months away, keeping the day of the month where it can - the 31st moved into a shorter month lands on that month's last day.",
            "renewal:i = danger(time_add_months(signed_up, 1));";
        "time_diff" => "std_lib::time::diff", (timestamp1: i, timestamp2: i) -> i,
            "Returns the absolute difference between two timestamps in seconds.",
            "elapsed:i = time_diff(finish, start);";
        "time_add_weeks" => "std_lib::time::add_weeks", (timestamp: i, weeks: i) -> i,
            "Returns the timestamp shifted by the given number of weeks (negative to subtract).",
            "next_week:i = time_add_weeks(now, 1);";
        "time_end_of_day" [Chrono] => "std_lib::time::end_of_day", (timestamp: i) -> (i!e),
            "Returns 23:59:59 UTC on the day the timestamp falls in - the other end of time_start_of_day.",
            "last_second:i = danger(time_end_of_day(time_now()));";
        "time_is_leap_year" => "std_lib::time::is_leap_year", (year: i) -> b,
            "Whether the year has a 29th of February, by the actual rule including the century exceptions.",
            "leap:b = time_is_leap_year(2024);";
        "time_days_in_month" => "std_lib::time::days_in_month", (year: i, month: i) -> (i!e),
            "Returns how many days the month has, February included; errors on a month outside 1 to 12.",
            "days:i = danger(time_days_in_month(2024, 2));";
        "time_ago" => "std_lib::time::ago", (timestamp: i, now: i) -> s,
            "Writes how long ago a moment was the way a page shows it: just now, 5 minutes ago, 3 days ago, or in 2 hours for something still to come. Both moments are given so the same inputs always read the same.",
            "posted:s = time_ago(created_at, time_now());";
        "time_start_of_day" [Chrono] => "std_lib::time::start_of_day", (timestamp: i) -> (i!e),
            "Returns midnight UTC at the start of the day the timestamp falls in - the building block for everything that happened today.",
            "today:i = danger(time_start_of_day(time_now()));";
        "time_weekday" [Chrono] => "std_lib::time::weekday", (timestamp: i) -> (s!e),
            "Returns the day of the week written out, from Monday to Sunday.",
            "day:s = danger(time_weekday(time_now()));";
        "time_format_custom" [Chrono] => "std_lib::time::format_custom", (timestamp: i, layout: s) -> (s!e),
            "Writes a moment out in a layout of your own, in strftime notation: %Y-%m-%d, %H:%M, %A %d %B %Y.",
            "date:s = danger(time_format_custom(time_now(), `%Y-%m-%d`));";
        "time_parse_custom" [Chrono] => "std_lib::time::parse_custom", (time_str: s, layout: s) -> (i!e),
            "Reads a moment out of text laid out the way you say, in the same strftime notation. A layout with no time in it leaves the time at midnight.",
            "moment:i = danger(time_parse_custom(`2009-02-13`, `%Y-%m-%d`));";
        "time_format_duration" [Chrono] => "std_lib::time::format_duration", (seconds: i) -> s,
            "Writes a length of time the way a person says it: 2d 3h, 1h 5m, 45s.",
            "elapsed:s = time_format_duration(time_diff(finish, start));";
        "time_from_parts" [Chrono] => "std_lib::time::from_parts", (year: i, month: i, day: i, hour: i, minute: i, second: i) -> (i!e),
            "Builds a moment from the parts of a UTC date; a day that is not on the calendar is an error rather than the day it would spill into.",
            "moment:i = danger(time_from_parts(2009, 2, 13, 23, 31, 30));";
    }

    m.insert("time_year", part_of_date("std_lib::time::year", "Returns the year of a timestamp, in UTC.", "year:i = danger(time_year(time_now()));"));
    m.insert("time_month", part_of_date("std_lib::time::month", "Returns the month of a timestamp, from 1 to 12, in UTC.", "month:i = danger(time_month(time_now()));"));
    m.insert("time_day", part_of_date("std_lib::time::day", "Returns the day of the month of a timestamp, from 1 to 31, in UTC.", "day:i = danger(time_day(time_now()));"));
    m.insert("time_hour", part_of_date("std_lib::time::hour", "Returns the hour of a timestamp, from 0 to 23, in UTC.", "hour:i = danger(time_hour(time_now()));"));
    m.insert("time_minute", part_of_date("std_lib::time::minute", "Returns the minute of a timestamp, from 0 to 59, in UTC.", "minute:i = danger(time_minute(time_now()));"));
    m.insert("time_second", part_of_date("std_lib::time::second", "Returns the second of a timestamp, from 0 to 59, in UTC.", "second:i = danger(time_second(time_now()));"));
    m.insert("time_day_of_year", part_of_date("std_lib::time::day_of_year", "Returns which day of the year a timestamp falls on, from 1 to 366, in UTC.", "day:i = danger(time_day_of_year(time_now()));"));

    // time_format / time_parse take the TIME_Format enum, which needs a custom
    // type import, so they use the full struct form.
    m.insert("time_format", StdlibFunction {
        rust_path: "std_lib::time::format".to_string(),
        crate_deps: vec![CrateDependency::Chrono],
        struct_derives: vec![],
        custom_type_imports: vec![("TIME_Format", "nail::std_lib::time")],
        module: StdlibModule::Time,
        parameters: vec![
            nail_param!(timestamp: i),
            StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::Enum("TIME_Format".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((s!e)),
        diverging: false,
        description: "Writes a Unix timestamp out in one of the standard spellings named by TIME_Format.",
        example: "text:s = danger(time_format(now, TIME_Format::ISO8601));",
    });

    m.insert("time_parse", StdlibFunction {
        rust_path: "std_lib::time::parse".to_string(),
        crate_deps: vec![CrateDependency::Chrono],
        struct_derives: vec![],
        custom_type_imports: vec![("TIME_Format", "nail::std_lib::time")],
        module: StdlibModule::Time,
        parameters: vec![
            nail_param!(time_str: s),
            StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::Enum("TIME_Format".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Reads a Unix timestamp out of text written in the spelling named by TIME_Format; anything else is an error rather than a guess.",
        example: "moment:i = danger(time_parse(`2009-02-13T23:31:30Z`, TIME_Format::ISO8601));",
    });

    simple_fns! { m, Time:
        "time_cron_valid" [Chrono] => "std_lib::time::cron_valid", (expression: (&s)) -> b,
            "Whether the text is a five-field cron expression this understands, for checking a schedule from a configuration file before relying on it.",
            "usable:b = time_cron_valid(schedule);";
        "time_cron_matches" [Chrono] => "std_lib::time::cron_matches", (expression: s, timestamp: i) -> (b!e),
            "Whether a cron expression matches a moment, to the minute.",
            "due:b = danger(time_cron_matches(`*/15 * * * *`, time_now()));";
        "time_cron_next" [Chrono] => "std_lib::time::cron_next", (expression: s, after_timestamp: i) -> (i!e),
            "The next moment after the given time that a cron expression matches. A scheduler asks this, sleeps until then with time_sleep, does the work, and asks again.",
            "next_run:i = danger(time_cron_next(`0 3 * * *`, time_now()));";
    }
}
