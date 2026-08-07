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
        "time_now_micros" => "std_lib::time::now_micros", () -> i,
            "Returns the current Unix timestamp in microseconds, for timing short work.",
            "now_us:i = time_now_micros();";
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
            "Returns how many days the month has, February included. Errors on a month outside 1 to 12.",
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
            "Builds a moment from the parts of a UTC date. A day that is not on the calendar is an error rather than the day it would spill into.",
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
        description: "Reads a Unix timestamp out of text written in the spelling named by TIME_Format. Anything else is an error rather than a guess.",
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
        "time_parse_human" [Chrono] => "std_lib::time::parse_human", (text: s, reference: i) -> (i!e),
            "Reads a plain-English moment relative to a reference timestamp: now, today, tomorrow, yesterday, next or last plus a weekday name, in N seconds/minutes/hours/days/weeks/months, N of those units ago, or an absolute YYYY-MM-DD date. Case and extra spaces are forgiven. Anything else is an error naming the shapes it reads.",
            "deadline:i = danger(time_parse_human(`in 2 days`, time_now()));";
        "time_cron_describe" [Chrono] => "std_lib::time::cron_describe", (expression: s) -> (s!e),
            "A five-field cron expression written out in words: 0 3 * * * reads every day at 03:00, and 0 9 * * 1-5 reads at 09:00 on weekdays. An expression beyond the vocabulary gets a faithful field-by-field reading rather than an error, and only an expression whose five fields do not parse is an error.",
            "when:s = danger(time_cron_describe(`0 3 * * *`));";
        "time_parse_duration" => "std_lib::time::parse_duration", (text: s) -> (i!e),
            "A human duration - `90s`, `2h30m`, `1.5h`, `2 days` - as whole seconds. A bare number is already seconds. The other direction is time_format_duration.",
            "ttl:i = danger(time_parse_duration(`2h30m`));";
        "time_format_in_zone" [ChronoTz, Chrono] => "std_lib::time::format_in_zone", (timestamp: i, zone: s, layout: s) -> (s!e),
            "A moment shown on the wall clock of a place, in your strftime layout. Zones are IANA names like `America/Edmonton`. Daylight saving is the zone database's problem, not yours.",
            "shown:s = danger(time_format_in_zone(time_now(), `America/Edmonton`, `%Y-%m-%d %H:%M`));";
        "time_parse_in_zone" [ChronoTz, Chrono] => "std_lib::time::parse_in_zone", (text: s, layout: s, zone: s) -> (i!e),
            "Reads a wall-clock time as seen in a place back into a timestamp. The repeated hour when clocks fall back takes the earlier reading. The skipped hour is an error.",
            "starts:i = danger(time_parse_in_zone(`2026-09-01 09:00`, `%Y-%m-%d %H:%M`, `America/Edmonton`));";
        "time_zone_offset" [ChronoTz, Chrono] => "std_lib::time::zone_offset", (timestamp: i, zone: s) -> (i!e),
            "How far ahead of UTC a place is at a moment, in seconds. Negative is behind. The answer changes with daylight saving, which is why a moment is asked for.",
            "offset:i = danger(time_zone_offset(time_now(), `Asia/Tokyo`));";
        "time_zone_valid" [ChronoTz] => "std_lib::time::zone_valid", (zone: s) -> b,
            "Whether a zone name is in the IANA database.",
            "known:b = time_zone_valid(user_zone);";
        "time_list_zones" [ChronoTz] => "std_lib::time::list_zones", () -> [s],
            "Every zone name the database knows, for picking lists.",
            "zones:a:s = time_list_zones();";
    }

    simple_fns! { m, Time:
        "time_quarter" [Chrono] => "std_lib::time::quarter", (timestamp: i) -> (i!e),
            "Returns which quarter of the year a moment falls in, from 1 to 4, in UTC.",
            "quarter:i = danger(time_quarter(time_now()));";
        "time_week_of_year" [Chrono] => "std_lib::time::week_of_year", (timestamp: i) -> (i!e),
            "Returns the ISO 8601 week number, from 1 to 53. ISO weeks start on Monday and week 1 holds the year's first Thursday, so days around New Year can belong to the other year's numbering.",
            "week:i = danger(time_week_of_year(time_now()));";
        "time_is_weekend" [Chrono] => "std_lib::time::is_weekend", (timestamp: i) -> (b!e),
            "Whether the moment falls on a Saturday or Sunday, in UTC.",
            "weekend:b = danger(time_is_weekend(time_now()));";
        "time_start_of_month" [Chrono] => "std_lib::time::start_of_month", (timestamp: i) -> (i!e),
            "Returns midnight UTC on the first of the month the timestamp falls in.",
            "month_start:i = danger(time_start_of_month(time_now()));";
        "time_end_of_month" [Chrono] => "std_lib::time::end_of_month", (timestamp: i) -> (i!e),
            "Returns 23:59:59 UTC on the last day of the month the timestamp falls in - the other end of time_start_of_month, so a whole month is the range between them.",
            "month_end:i = danger(time_end_of_month(time_now()));";
        "time_start_of_week" [Chrono] => "std_lib::time::start_of_week", (timestamp: i) -> (i!e),
            "Returns midnight UTC on the Monday of the week the timestamp falls in.",
            "week_start:i = danger(time_start_of_week(time_now()));";
        "time_end_of_week" [Chrono] => "std_lib::time::end_of_week", (timestamp: i) -> (i!e),
            "Returns 23:59:59 UTC on the Sunday of the week the timestamp falls in - the other end of time_start_of_week.",
            "week_end:i = danger(time_end_of_week(time_now()));";
        "time_start_of_year" [Chrono] => "std_lib::time::start_of_year", (timestamp: i) -> (i!e),
            "Returns midnight UTC on the first of January of the year the timestamp falls in.",
            "year_start:i = danger(time_start_of_year(time_now()));";
        "time_end_of_year" [Chrono] => "std_lib::time::end_of_year", (timestamp: i) -> (i!e),
            "Returns 23:59:59 UTC on the 31st of December of the year the timestamp falls in - the other end of time_start_of_year.",
            "year_end:i = danger(time_end_of_year(time_now()));";
        "time_add_workdays" [Chrono] => "std_lib::time::add_workdays", (timestamp: i, workdays: i) -> (i!e),
            "Returns the timestamp moved by a number of working days - skipping Saturdays and Sundays - keeping the time of day. Negative goes backwards. A weekend start does not count itself: Saturday plus one workday is Monday.",
            "due:i = danger(time_add_workdays(time_now(), 3));";
        "time_workdays_between" [Chrono] => "std_lib::time::workdays_between", (start: i, end: i) -> (i!e),
            "Counts the weekday dates after the start's date, up to and including the end's date: Monday to the same week's Friday is 4, Friday to the following Monday is 1, and a same-day pair is 0 - the start's own date is never counted. An end before the start is an error.",
            "billable:i = danger(time_workdays_between(kickoff, deadline));";
        "time_days_between" [Chrono] => "std_lib::time::days_between", (start: i, end: i) -> (i!e),
            "Returns the whole days between the calendar dates of two moments, signed - negative when the end is earlier. The clock is ignored: 23:00 to 01:00 the next morning is 1, because the date changed once.",
            "days_left:i = danger(time_days_between(time_now(), deadline));";
        "time_months_between" [Chrono] => "std_lib::time::months_between", (start: i, end: i) -> (i!e),
            "Returns the whole calendar months between two moments, signed. A month counts only once the same day of the month has been reached: the 15th of January to the 14th of March is 1, to the 15th is 2.",
            "paid_for:i = danger(time_months_between(signed_up, time_now()));";
        "time_same_day" [Chrono] => "std_lib::time::same_day", (first: i, second: i) -> (b!e),
            "Whether two moments fall on the same calendar date, in UTC - the same date, not within twenty-four hours of each other.",
            "today:b = danger(time_same_day(created_at, time_now()));";
        "time_is_first_of_month" [Chrono] => "std_lib::time::is_first_of_month", (timestamp: i) -> (b!e),
            "Whether the moment falls on the first day of its month, in UTC - the day the monthly jobs run.",
            "run_billing:b = danger(time_is_first_of_month(time_now()));";
        "time_age_years" [Chrono] => "std_lib::time::age_years", (born: i, at: i) -> (i!e),
            "Returns the age in whole years at a moment, counted the way a person counts it: it goes up on the birthday, not at New Year. A moment before the birth is an error.",
            "age:i = danger(time_age_years(birthday, time_now()));";
    }

    // time_next_weekday takes the TIME_Weekday enum, which needs a custom
    // type import, so it uses the full struct form.
    m.insert("time_next_weekday", StdlibFunction {
        rust_path: "std_lib::time::next_weekday".to_string(),
        crate_deps: vec![CrateDependency::Chrono],
        struct_derives: vec![],
        custom_type_imports: vec![("TIME_Weekday", "nail::std_lib::time")],
        module: StdlibModule::Time,
        parameters: vec![
            nail_param!(timestamp: i),
            StdlibParameter { name: "weekday".to_string(), param_type: NailDataTypeDescriptor::Enum("TIME_Weekday".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Returns the next date strictly after the timestamp that falls on the given weekday, keeping the time of day. A Monday asked for the next Monday gets the one a week out.",
        example: "next_monday:i = danger(time_next_weekday(time_now(), TIME_Weekday::Monday));",
    });

    m.insert("time_nth_weekday_of_month", StdlibFunction {
        rust_path: "std_lib::time::nth_weekday_of_month".to_string(),
        crate_deps: vec![CrateDependency::Chrono],
        struct_derives: vec![],
        custom_type_imports: vec![("TIME_Weekday", "nail::std_lib::time"), ("TIME_Nth", "nail::std_lib::time")],
        module: StdlibModule::Time,
        parameters: vec![
            nail_param!(year: i),
            nail_param!(month: i),
            StdlibParameter { name: "weekday".to_string(), param_type: NailDataTypeDescriptor::Enum("TIME_Weekday".to_string()), pass_by_reference: false },
            StdlibParameter { name: "nth".to_string(), param_type: NailDataTypeDescriptor::Enum("TIME_Nth".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Returns the date a rule like the third Monday in January names, at midnight UTC - how holidays, pay days and standing meetings are written down. TIME_Nth::Last is its own choice rather than a count, because how many of a weekday a month holds depends on the month. A month without that many of them is an error rather than a date in the month after.",
        example: "family_day:i = danger(time_nth_weekday_of_month(2026, 2, TIME_Weekday::Monday, TIME_Nth::Third));",
    });
}
