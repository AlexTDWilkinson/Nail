//! The command line a program was started with.
//!
//! Two ways in, for two sizes of program.
//!
//! A script that wants one setting reads it directly: `args_flag(`verbose`)`,
//! `args_value(`output`)`. No ceremony, no description, nothing to keep in
//! step.
//!
//! A real command-line tool describes what it accepts once, as an array of
//! `ARGS_Option`, and hands that to `args_parse`. One call reads the whole
//! command line, checks it, and returns it as data: the subcommand, the
//! positional arguments, the values, the flags that were present. The same
//! description generates `--help`, so the help page cannot describe a flag the
//! program does not accept, or miss one it does.
//!
//! The description is not optional ceremony - it is what makes the command
//! line readable at all. Given `mytool --output report.txt deploy`, nothing
//! can tell whether `report.txt` is the value of `--output` or the subcommand
//! without knowing that `--output` takes a value. Every parser needs this;
//! most hide it inside a builder. Here it is plain data you can print.

use serde::{Deserialize, Serialize};
use std::env;

/// One flag a program accepts.
///
/// `short` is a single letter without its dash, or empty for none.
/// `takes_value` separates `--output report.txt` from a bare `--verbose`.
/// `required` is checked by `args_parse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARGS_Option {
    pub name: String,
    pub short: String,
    pub description: String,
    pub takes_value: bool,
    pub required: bool,
}

/// A command line, read and checked.
///
/// `command` is the first positional argument, or empty - the subcommand, as
/// in `deploy` in `mytool deploy --force`. It is also `positional[0]`; skip it
/// with `array_skip(parsed.positional, 1)` when you want only what came after.
///
/// `values` is keyed by the long name however the flag was written, so `-o x`,
/// `--output x` and `--output=x` all arrive as `output`.
///
/// `flags` holds the long names of the options present that take no value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARGS_Parsed {
    pub command: String,
    pub positional: Vec<String>,
    pub values: dashmap::DashMap<String, String>,
    pub flags: Vec<String>,
}

/// Get command line argument by index (0 is the program name)
pub fn get(index: i64) -> Result<String, String> {
    let args: Vec<String> = env::args().collect();

    if index < 0 {
        return Err(format!("args_get: argument index cannot be negative, got {}", index));
    }

    let idx = index as usize;
    if idx >= args.len() {
        return Err(format!("args_get: index {} is out of bounds, the program received {} argument(s)", index, args.len()));
    }

    Ok(args[idx].clone())
}

/// Check if a flag exists (e.g., --flag or -f)
pub fn flag(name: String) -> bool {
    let args: Vec<String> = env::args().collect();

    // Check for both long form (--name) and short form (-n)
    let long_form = format!("--{}", name);
    let short_form = if name.len() == 1 {
        format!("-{}", name)
    } else {
        // For multi-character names, check if there's a common short form
        format!("-{}", name.chars().next().unwrap_or('?'))
    };

    args.iter().any(|arg| arg == &long_form || (name.len() == 1 && arg == &short_form))
}

/// Get value for a named argument (e.g., --name=value or --name value)
pub fn value(name: String) -> Result<String, String> {
    let args: Vec<String> = env::args().collect();

    let flag_with_equals = format!("--{}=", name);
    let flag_without_equals = format!("--{}", name);

    for (i, arg) in args.iter().enumerate() {
        // Check for --name=value format
        if arg.starts_with(&flag_with_equals) {
            let value = arg[flag_with_equals.len()..].to_string();
            if value.is_empty() {
                return Err(format!("args_value: argument --{} has an empty value", name));
            }
            return Ok(value);
        }

        // Check for --name value format (value is next argument)
        if arg == &flag_without_equals {
            if i + 1 < args.len() {
                let next_arg = &args[i + 1];
                // Make sure the next arg is not another flag
                if !next_arg.starts_with('-') {
                    return Ok(next_arg.clone());
                }
            }
            return Err(format!("args_value: no value provided for argument --{}", name));
        }
    }

    Err(format!("args_value: argument --{} was not passed to the program", name))
}

/// Get the number of command line arguments (including program name)
pub fn count() -> i64 {
    env::args().count() as i64
}

/// The value of a flag, or the fallback when it was not passed. The form with
/// no error, for the common case where a missing setting has a sensible
/// default.
pub fn value_or(name: String, fallback: String) -> String {
    return value(name).unwrap_or(fallback);
}

/// The value of a flag read as a whole number. A flag that is not there and a
/// flag whose value is not a number are different errors, because they need
/// different fixes.
pub fn value_int(name: String) -> Result<i64, String> {
    let raw = value(name.clone())?;
    return raw.trim().parse::<i64>().map_err(|_| format!("args_value_int: --{} was given '{}', which is not a whole number", name, raw));
}

/// The value of a flag read as a fraction.
pub fn value_float(name: String) -> Result<f64, String> {
    let raw = value(name.clone())?;
    return raw.trim().parse::<f64>().map_err(|_| format!("args_value_float: --{} was given '{}', which is not a number", name, raw));
}

/// Whether the program was asked for help, by `--help` or `-h`. Check this
/// before anything else and print `args_help_text`.
pub fn wants_help() -> bool {
    return arguments().iter().any(|word| word == "--help" || word == "-h");
}

/// Everything after the program name.
fn arguments() -> Vec<String> {
    return env::args().skip(1).collect();
}

/// Whether a word is a flag rather than a value: `--output`, `-v`, but not a
/// negative number, which is a value that happens to start with a dash.
fn is_flag(word: &str) -> bool {
    if !word.starts_with('-') || word == "-" {
        return false;
    }
    let rest = word.trim_start_matches('-');
    return !rest.chars().next().map(|first| first.is_ascii_digit()).unwrap_or(false);
}

/// The option a written flag refers to, by either of its names.
fn option_for<'a>(written: &str, options: &'a Vec<ARGS_Option>) -> Option<&'a ARGS_Option> {
    return options.iter().find(|option| option.name == written || (!option.short.is_empty() && option.short == written));
}

/// Reads and checks a whole command line against the program's description of
/// it. One pass, one place errors come from, and the answer is data.
///
/// The checks are: no flag the program does not accept, every option that
/// takes a value given one, no value handed to an option that takes none, and
/// every required option present. The first problem found is the error,
/// because a person fixes one thing and runs it again.
pub fn parse(options: &Vec<ARGS_Option>) -> Result<ARGS_Parsed, String> {
    return parse_words(&arguments(), options);
}

/// The whole of the reading, over a list of words rather than the real command
/// line - which is what makes every case below testable without starting a
/// process to test it.
fn parse_words(words: &[String], options: &Vec<ARGS_Option>) -> Result<ARGS_Parsed, String> {
    let values = dashmap::DashMap::new();
    let mut flags: Vec<String> = Vec::new();
    let mut positional: Vec<String> = Vec::new();

    let mut index = 0;
    while index < words.len() {
        let word = &words[index];

        if !is_flag(word) {
            positional.push(word.clone());
            index += 1;
            continue;
        }

        // `--help` is answered by args_wants_help, and a program that is being
        // asked for help has not necessarily been given anything else it
        // needs, so it is accepted here without being declared.
        if word == "--help" || word == "-h" {
            index += 1;
            continue;
        }

        let bare = word.trim_start_matches('-');
        let (written, inline_value) = match bare.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (bare, None),
        };

        let option = match option_for(written, options) {
            Some(option) => option,
            None => return Err(format!("args_parse: --{} is not an option this program accepts", written)),
        };

        if !option.takes_value {
            if inline_value.is_some() {
                return Err(format!("args_parse: --{} does not take a value", option.name));
            }
            if !flags.contains(&option.name) {
                flags.push(option.name.clone());
            }
            index += 1;
            continue;
        }

        let given = match inline_value {
            Some(value) if !value.is_empty() => value.to_string(),
            Some(_) => return Err(format!("args_parse: --{} needs a value", option.name)),
            None => match words.get(index + 1) {
                Some(next) if !is_flag(next) => {
                    index += 1;
                    next.clone()
                }
                _ => return Err(format!("args_parse: --{} needs a value", option.name)),
            },
        };

        values.insert(option.name.clone(), given);
        index += 1;
    }

    for option in options.iter() {
        if option.required && !values.contains_key(&option.name) && !flags.contains(&option.name) {
            return Err(format!("args_parse: --{} is required", option.name));
        }
    }

    let command = positional.first().cloned().unwrap_or_default();
    return Ok(ARGS_Parsed { command, positional, values, flags });
}

/// The `--help` page, built from the description of the command line so it
/// cannot drift from what the program actually accepts.
pub fn help_text(program: String, description: String, options: &Vec<ARGS_Option>) -> String {
    let mut out = String::new();
    if !description.is_empty() {
        out.push_str(&description);
        out.push_str("\n\n");
    }

    out.push_str(&format!("Usage: {} [options]", program));
    if options.iter().any(|option| option.required) {
        out.push_str("\n\nRequired options are marked.");
    }
    out.push_str("\n\nOptions:\n");

    // The left column is as wide as the widest flag, so the descriptions line
    // up however long the names are.
    let written: Vec<String> = options
        .iter()
        .map(|option| {
            let mut flag = String::new();
            if option.short.is_empty() {
                flag.push_str("    ");
            } else {
                flag.push_str(&format!("-{}, ", option.short));
            }
            flag.push_str(&format!("--{}", option.name));
            if option.takes_value {
                flag.push_str(" <value>");
            }
            flag
        })
        .collect();

    let widest = written.iter().map(|flag| flag.chars().count()).chain(std::iter::once("-h, --help".chars().count())).max().unwrap_or(0);

    for (index, option) in options.iter().enumerate() {
        let padding = " ".repeat(widest - written[index].chars().count());
        out.push_str(&format!("  {}{}  {}", written[index], padding, option.description));
        if option.required {
            out.push_str(" (required)");
        }
        out.push('\n');
    }

    let padding = " ".repeat(widest - "-h, --help".chars().count());
    out.push_str(&format!("  -h, --help{}  Show this help and exit\n", padding));
    return out;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(name: &str, short: &str, takes_value: bool, required: bool) -> ARGS_Option {
        return ARGS_Option { name: name.to_string(), short: short.to_string(), description: format!("the {}", name), takes_value, required };
    }

    fn example_options() -> Vec<ARGS_Option> {
        return vec![option("output", "o", true, false), option("verbose", "v", false, false), option("retries", "", true, false)];
    }

    fn words(line: &str) -> Vec<String> {
        return line.split_whitespace().map(|word| word.to_string()).collect();
    }

    fn parsed(line: &str) -> ARGS_Parsed {
        return parse_words(&words(line), &example_options()).expect("a valid command line");
    }

    fn failure(line: &str) -> String {
        return parse_words(&words(line), &example_options()).unwrap_err();
    }

    #[test]
    fn a_flag_is_told_apart_from_a_value() {
        assert!(is_flag("--output"));
        assert!(is_flag("-v"));
        assert!(!is_flag("deploy"));
        assert!(!is_flag("-"));
        assert!(!is_flag("-42"), "a negative number is a value, not a flag");
    }

    #[test]
    fn the_subcommand_is_found_past_a_flag_that_swallowed_its_value() {
        // The case the description exists for: without knowing that --output
        // takes a value, report.txt looks exactly like the subcommand.
        let result = parsed("--output report.txt deploy");
        assert_eq!(result.command, "deploy");
        assert_eq!(result.positional, vec!["deploy".to_string()]);
        assert_eq!(result.values.get("output").expect("the value").value().clone(), "report.txt");
    }

    #[test]
    fn a_flag_that_takes_no_value_does_not_swallow_the_next_word() {
        let result = parsed("--verbose deploy");
        assert_eq!(result.command, "deploy");
        assert_eq!(result.flags, vec!["verbose".to_string()]);
    }

    #[test]
    fn every_way_of_writing_a_value_arrives_under_the_long_name() {
        for line in ["--output report.txt", "--output=report.txt", "-o report.txt", "-o=report.txt"] {
            let result = parsed(line);
            assert_eq!(result.values.get("output").expect("the value").value().clone(), "report.txt", "failed for: {}", line);
        }
    }

    #[test]
    fn the_command_is_the_first_positional_and_the_rest_follow_it() {
        let result = parsed("deploy one two --verbose");
        assert_eq!(result.command, "deploy");
        assert_eq!(result.positional, vec!["deploy".to_string(), "one".to_string(), "two".to_string()]);
    }

    #[test]
    fn a_negative_number_is_read_as_a_value_not_a_flag() {
        let result = parsed("--retries -5");
        assert_eq!(result.values.get("retries").expect("the value").value().clone(), "-5");
    }

    #[test]
    fn an_empty_command_line_parses_to_nothing() {
        let result = parsed("");
        assert_eq!(result.command, "");
        assert!(result.positional.is_empty());
        assert!(result.flags.is_empty());
        assert_eq!(result.values.len(), 0);
    }

    #[test]
    fn asking_for_help_is_accepted_without_being_declared() {
        let result = parsed("--help");
        assert!(result.flags.is_empty(), "help is not one of the program's own flags");
        assert_eq!(result.command, "");
    }

    #[test]
    fn a_flag_the_program_does_not_accept_is_an_error() {
        assert!(failure("--nonsense").contains("--nonsense is not an option this program accepts"));
    }

    #[test]
    fn an_option_that_takes_a_value_must_be_given_one() {
        assert!(failure("--output").contains("--output needs a value"));
        assert!(failure("--output --verbose").contains("--output needs a value"));
        assert!(failure("--output=").contains("--output needs a value"));
    }

    #[test]
    fn an_option_that_takes_no_value_must_not_be_given_one() {
        assert!(failure("--verbose=yes").contains("--verbose does not take a value"));
    }

    #[test]
    fn a_required_option_that_is_missing_is_an_error() {
        let required = vec![option("output", "o", true, true)];
        assert!(parse_words(&words(""), &required).unwrap_err().contains("--output is required"));
        assert!(parse_words(&words("--output report.txt"), &required).is_ok());
        // A required flag that takes no value is satisfied by being present.
        let required_flag = vec![option("force", "f", false, true)];
        assert!(parse_words(&words(""), &required_flag).unwrap_err().contains("--force is required"));
        assert!(parse_words(&words("--force"), &required_flag).is_ok());
    }

    #[test]
    fn the_same_flag_twice_is_recorded_once() {
        let result = parsed("--verbose --verbose");
        assert_eq!(result.flags, vec!["verbose".to_string()]);
    }

    #[test]
    fn a_repeated_value_takes_the_last_one_given() {
        let result = parsed("--output first.txt --output second.txt");
        assert_eq!(result.values.get("output").expect("the value").value().clone(), "second.txt");
    }

    #[test]
    fn the_help_page_lists_every_option_and_help_itself() {
        let page = help_text("mytool".to_string(), "Does a thing.".to_string(), &example_options());
        assert!(page.contains("Does a thing."));
        assert!(page.contains("Usage: mytool [options]"));
        assert!(page.contains("-o, --output <value>"), "got:\n{}", page);
        assert!(page.contains("-v, --verbose"), "got:\n{}", page);
        assert!(page.contains("    --retries <value>"), "an option with no short form is still aligned:\n{}", page);
        assert!(page.contains("-h, --help"), "got:\n{}", page);
    }

    #[test]
    fn the_help_page_marks_what_is_required() {
        let page = help_text("mytool".to_string(), String::new(), &vec![option("output", "o", true, true)]);
        assert!(page.contains("(required)"), "got:\n{}", page);
    }

    #[test]
    fn the_help_page_columns_line_up() {
        let page = help_text("mytool".to_string(), String::new(), &example_options());
        let description_columns: Vec<usize> = page.lines().filter(|line| line.starts_with("  ") && line.contains("--")).map(|line| line.find("  the ").or_else(|| line.find("  Show")).unwrap_or(0)).collect();
        assert_eq!(description_columns.len(), 4, "expected a line per option and one for help, got:\n{}", page);
        assert!(description_columns.windows(2).all(|pair| pair[0] == pair[1]), "descriptions must start in the same column:\n{}", page);
    }

    /// The help page is generated from the same description the parsing is
    /// checked against, so this can be asserted rather than hoped for.
    #[test]
    fn every_option_the_program_accepts_appears_in_its_help_page() {
        let options = example_options();
        let page = help_text("mytool".to_string(), String::new(), &options);
        for option in options.iter() {
            assert!(page.contains(&format!("--{}", option.name)), "--{} is accepted but missing from the help page:\n{}", option.name, page);

            // What the page claims about each flag - whether it shows
            // `<value>` after it - has to be what the parser demands.
            let page_says_it_takes_a_value = page.lines().any(|line| line.contains(&format!("--{} <value>", option.name)));
            assert_eq!(page_says_it_takes_a_value, option.takes_value, "the help page and the description disagree about --{}", option.name);

            let alone = parse_words(&words(&format!("--{}", option.name)), &options);
            assert_eq!(alone.is_err(), option.takes_value, "the parser and the help page disagree about whether --{} needs a value", option.name);

            let given_a_value = parse_words(&words(&format!("--{}=x", option.name)), &options);
            assert_eq!(given_a_value.is_ok(), option.takes_value, "the parser and the help page disagree about whether --{} accepts a value", option.name);
        }
    }
}
