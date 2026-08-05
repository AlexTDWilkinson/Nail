use std::env as std_env;

pub fn get(key: String) -> Result<String, String> {
    std_env::var(&key).map_err(|_| format!("env_get: environment variable '{}' is not set", key))
}

pub fn set(key: String, value: String) -> Result<(), String> {
    std_env::set_var(key, value);
    Ok(())
}

pub fn args() -> Vec<String> {
    std_env::args().collect()
}

/// The directory the program is running in - what every relative path in the
/// program is relative to.
pub fn current_dir() -> Result<String, String> {
    let directory = std_env::current_dir().map_err(|e| format!("env_current_dir: could not read the current directory: {}", e))?;
    return Ok(directory.to_string_lossy().to_string());
}

/// Moves the program into another directory, so relative paths resolve from
/// there afterwards. Errors if the directory does not exist or cannot be
/// entered.
pub fn set_current_dir(path: String) -> Result<(), String> {
    std_env::set_current_dir(&path).map_err(|e| format!("env_set_current_dir: could not enter '{}': {}", path, e))?;
    return Ok(());
}

/// The home directory of the user running the program, from `HOME`. Errors
/// when it is not set, which happens in some service and container setups -
/// so a program that needs it finds out rather than writing to the wrong place.
pub fn home_dir() -> Result<String, String> {
    return std_env::var("HOME").map_err(|_| "env_home_dir: HOME is not set, so there is no home directory to use".to_string());
}

/// The name of the machine. Read from the kernel where that is possible, so it
/// is the live name rather than whatever a login shell was told.
pub fn hostname() -> Result<String, String> {
    for source in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(name) = std::fs::read_to_string(source) {
            let name = name.trim();
            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
    }
    return match std_env::var("HOSTNAME") {
        Ok(name) if !name.is_empty() => Ok(name),
        _ => Err("env_hostname: could not read the name of this machine".to_string()),
    };
}

/// The name of the user running the program, from `USER` or `LOGNAME`.
pub fn user() -> Result<String, String> {
    for key in ["USER", "LOGNAME"] {
        if let Ok(name) = std_env::var(key) {
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }
    return Err("env_user: neither USER nor LOGNAME is set, so there is no user name to report".to_string());
}

/// Which operating system this build is running on: `linux`, `macos`,
/// `windows`. The thing to branch on when a path or a command differs.
pub fn os() -> String {
    return std_env::consts::OS.to_string();
}

/// Which processor this build is for: `x86_64`, `aarch64`, and so on.
pub fn arch() -> String {
    return std_env::consts::ARCH.to_string();
}

/// The process id of the running program - what goes in a pid file, or in a
/// log line that has to be matched up with `ps` afterwards.
pub fn pid() -> i64 {
    return std::process::id() as i64;
}

/// How many processors the program may actually use - the count to size a
/// worker pool or a batch by. Falls back to 1 when the system will not say.
pub fn cpu_count() -> i64 {
    return match std::thread::available_parallelism() {
        Ok(count) => count.get() as i64,
        Err(_) => 1,
    };
}

/// Unsets an environment variable for this process. Removing one that was
/// never set is not an error.
pub fn remove(key: String) -> Result<(), String> {
    std_env::remove_var(key);
    return Ok(());
}

/// Every environment variable the process has, as a hashmap. Reading them all
/// is how a program logs its own configuration, or passes it on to a child.
pub fn all() -> dashmap::DashMap<String, String> {
    let variables = dashmap::DashMap::new();
    for (key, value) in std_env::vars() {
        variables.insert(key, value);
    }
    return variables;
}

/// Reads a `.env` file, sets every variable in it, and returns what it read.
///
/// The format is the one everyone already has a file in: one `KEY=value` per
/// line, blank lines and `#` comments ignored, an optional `export ` in front
/// of the key tolerated so the same file can be sourced by a shell. A value in
/// single quotes is taken literally; a value in double quotes has `\n`, `\t`,
/// `\"` and `\\` translated, which is how a multi-line private key fits on one
/// line.
///
/// Variables already set in the environment are left alone. That is the rule
/// that makes a `.env` file safe to keep around in production: what the
/// process was actually started with always wins over what the file says.
pub fn load_dotenv(path: String) -> Result<dashmap::DashMap<String, String>, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| format!("env_load_dotenv: could not read '{}': {}", path, e))?;
    let loaded = dashmap::DashMap::new();

    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let separator = match line.find('=') {
            Some(position) => position,
            None => return Err(format!("env_load_dotenv: line {} of '{}' has no '=' in it: {}", index + 1, path, raw_line)),
        };

        let key = line[..separator].trim().to_string();
        if key.is_empty() {
            return Err(format!("env_load_dotenv: line {} of '{}' has no name before the '='", index + 1, path));
        }

        let value = parse_value(line[separator + 1..].trim());
        if std_env::var(&key).is_err() {
            std_env::set_var(&key, &value);
        }
        loaded.insert(key, value);
    }

    return Ok(loaded);
}

/// The value half of a `.env` line, with its quoting resolved.
fn parse_value(raw: &str) -> String {
    if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        return raw[1..raw.len() - 1].to_string();
    }

    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        let inner = &raw[1..raw.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut characters = inner.chars();
        while let Some(character) = characters.next() {
            if character != '\\' {
                out.push(character);
                continue;
            }
            match characters.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        }
        return out;
    }

    // Unquoted: a `#` starts a comment, because that is what every other
    // reader of this format does.
    return match raw.find(" #") {
        Some(position) => raw[..position].trim_end().to_string(),
        None => raw.to_string(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp(name: &str, content: &str) -> String {
        let path = format!("{}/nail_env_{}.env", std_env::temp_dir().to_string_lossy(), name);
        std::fs::write(&path, content).expect("a writable temporary directory");
        return path;
    }

    #[test]
    fn values_keep_their_quoting_rules() {
        assert_eq!(parse_value("plain"), "plain");
        assert_eq!(parse_value("'raw \\n stays'"), "raw \\n stays");
        assert_eq!(parse_value("\"line\\nbreak\""), "line\nbreak");
        assert_eq!(parse_value("\"quoted \\\"inside\\\"\""), "quoted \"inside\"");
        assert_eq!(parse_value("value # a trailing comment"), "value");
        assert_eq!(parse_value("no#comment"), "no#comment");
    }

    #[test]
    fn a_file_of_settings_is_read_and_set() {
        let path = write_temp("basic", "# a comment\n\nGREETING=hello\nexport SHOUT=\"HI\\nTHERE\"\nEMPTY=\n");
        let loaded = load_dotenv(path.clone()).expect("a readable file");

        assert_eq!(loaded.get("GREETING").expect("the key").value().clone(), "hello");
        assert_eq!(loaded.get("SHOUT").expect("the key").value().clone(), "HI\nTHERE");
        assert_eq!(loaded.get("EMPTY").expect("the key").value().clone(), "");
        assert_eq!(std_env::var("GREETING").expect("the variable was set"), "hello");

        std::fs::remove_file(path).expect("a removable file");
    }

    #[test]
    fn what_the_process_was_started_with_wins() {
        std_env::set_var("NAIL_DOTENV_ALREADY_SET", "from the environment");
        let path = write_temp("precedence", "NAIL_DOTENV_ALREADY_SET=from the file\n");

        let loaded = load_dotenv(path.clone()).expect("a readable file");
        assert_eq!(loaded.get("NAIL_DOTENV_ALREADY_SET").expect("the key").value().clone(), "from the file");
        assert_eq!(std_env::var("NAIL_DOTENV_ALREADY_SET").expect("the variable"), "from the environment");

        std::fs::remove_file(path).expect("a removable file");
    }

    #[test]
    fn a_line_that_is_not_a_setting_is_an_error() {
        let path = write_temp("broken", "GOOD=1\nthis line has no equals sign\n");
        assert!(load_dotenv(path.clone()).unwrap_err().contains("line 2"));
        std::fs::remove_file(path).expect("a removable file");
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(load_dotenv("/nowhere/at/all/.env".to_string()).unwrap_err().contains("could not read"));
    }
}
#[cfg(test)]
mod machine_tests {
    use super::*;

    #[test]
    fn the_current_directory_is_absolute_and_can_be_changed_back() {
        let before = current_dir().expect("a readable current directory");
        assert!(before.starts_with('/'));
        set_current_dir("/".to_string()).expect("the root directory exists");
        assert_eq!(current_dir().expect("a readable current directory"), "/");
        set_current_dir(before.clone()).expect("the directory we came from");
        assert_eq!(current_dir().expect("a readable current directory"), before);
    }

    #[test]
    fn entering_a_directory_that_is_not_there_is_an_error() {
        assert!(set_current_dir("/nowhere/at/all".to_string()).unwrap_err().contains("could not enter"));
    }

    #[test]
    fn the_machine_describes_itself() {
        assert!(!os().is_empty());
        assert!(!arch().is_empty());
        assert!(pid() > 0);
        assert!(cpu_count() >= 1);
        assert!(!hostname().expect("a machine with a name").is_empty());
    }

    #[test]
    fn a_variable_can_be_set_read_listed_and_removed() {
        set("NAIL_ENV_ROUND_TRIP".to_string(), "here".to_string()).expect("setting works");
        assert_eq!(get("NAIL_ENV_ROUND_TRIP".to_string()).expect("just set"), "here");
        assert_eq!(all().get("NAIL_ENV_ROUND_TRIP").expect("just set").value().clone(), "here");

        remove("NAIL_ENV_ROUND_TRIP".to_string()).expect("removing works");
        assert!(get("NAIL_ENV_ROUND_TRIP".to_string()).is_err());
        // Removing what is not there is not an error.
        remove("NAIL_ENV_ROUND_TRIP".to_string()).expect("removing works");
    }
}

fn app_dir(base: Option<std::path::PathBuf>, app_name: &str, what: &str) -> Result<String, String> {
    let trimmed = app_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{}: the app needs a name to get its own directory", what));
    }
    let base = base.ok_or_else(|| format!("{}: this system does not say where such files live", what))?;
    return Ok(base.join(trimmed).to_string_lossy().to_string());
}

/// Where an app's configuration belongs on this system - ~/.config/<app> on
/// Linux, the platform's own convention elsewhere. Not created automatically.
pub fn config_dir(app_name: String) -> Result<String, String> {
    return app_dir(dirs::config_dir(), &app_name, "env_config_dir");
}

/// Where an app's own data belongs - state that is not configuration.
pub fn data_dir(app_name: String) -> Result<String, String> {
    return app_dir(dirs::data_dir(), &app_name, "env_data_dir");
}

/// Where an app's disposable cache belongs - what can be deleted without loss.
pub fn cache_dir(app_name: String) -> Result<String, String> {
    return app_dir(dirs::cache_dir(), &app_name, "env_cache_dir");
}
