use std::io::{self, BufRead, IsTerminal};
use crate::parser::std_lib::print::print_no_newline;

/// Read a line from stdin
pub async fn read_line() -> Result<String, String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => {
            // Remove trailing newline
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(line)
        }
        Err(e) => Err(format!("io_read_line: failed to read from stdin: {}", e))
    }
}

/// Read a line with a prompt
pub async fn read_line_prompt(prompt: String) -> Result<String, String> {
    print_no_newline(prompt);
    read_line().await
}

/// Read an integer from stdin
pub async fn read_int() -> Result<i64, String> {
    match read_line().await {
        Ok(line) => {
            match line.trim().parse::<i64>() {
                Ok(n) => Ok(n),
                Err(_) => Err(format!("io_read_int: could not parse '{}' as an integer", line.trim()))
            }
        }
        Err(e) => Err(e)
    }
}

/// Read an integer with a prompt
pub async fn read_int_prompt(prompt: String) -> Result<i64, String> {
    print_no_newline(prompt);
    read_int().await
}

/// Read a float from stdin
pub async fn read_float() -> Result<f64, String> {
    match read_line().await {
        Ok(line) => {
            match line.trim().parse::<f64>() {
                Ok(n) => Ok(n),
                Err(_) => Err(format!("io_read_float: could not parse '{}' as a float", line.trim()))
            }
        }
        Err(e) => Err(e)
    }
}

/// Read a float with a prompt
pub async fn read_float_prompt(prompt: String) -> Result<f64, String> {
    print_no_newline(prompt);
    read_float().await
}

/// A yes-or-no question, asked until it gets an answer it understands. The
/// default is what an empty line means, which is how a command-line tool lets
/// someone hold down return through a series of questions.
pub async fn confirm(question: String, default_answer: bool) -> Result<String, String> {
    let hint = if default_answer { "[Y/n]" } else { "[y/N]" };
    loop {
        print_no_newline(format!("{} {} ", question, hint));
        let answer = read_line().await.map_err(|detail| detail.replace("io_read_line", "io_confirm"))?;
        match answer.trim().to_lowercase().as_str() {
            "" => return Ok(if default_answer { "yes".to_string() } else { "no".to_string() }),
            "y" | "yes" => return Ok("yes".to_string()),
            "n" | "no" => return Ok("no".to_string()),
            _ => print_no_newline("Please answer yes or no.\n".to_string()),
        }
    }
}

/// A numbered list of choices, asked until one is picked. The answer is the
/// index of the chosen option, so the caller reads it back out of the same
/// array it passed in.
pub async fn select(question: String, options: Vec<String>) -> Result<i64, String> {
    if options.is_empty() {
        return Err("io_select: there were no options to choose from".to_string());
    }
    print_no_newline(format!("{}\n", question));
    for (position, option) in options.iter().enumerate() {
        print_no_newline(format!("  {}) {}\n", position + 1, option));
    }
    loop {
        print_no_newline(format!("Choose 1-{}: ", options.len()));
        let answer = read_line().await.map_err(|detail| detail.replace("io_read_line", "io_select"))?;
        match answer.trim().parse::<usize>() {
            Ok(chosen) if chosen >= 1 && chosen <= options.len() => return Ok(chosen as i64 - 1),
            _ => print_no_newline(format!("That is not one of the choices. Enter a number from 1 to {}.\n", options.len())),
        }
    }
}

/// A line read with nothing shown as it is typed, for a password or a token
/// being pasted into a terminal. The terminal's echo is turned off for the read
/// and turned back on afterwards, including when the read fails.
///
/// When input is not a terminal - a pipe, a script - there is no echo to turn
/// off, so this reads the line and says so by way of the empty prompt: a
/// password given this way was already visible to whatever produced it.
pub async fn read_secret(prompt: String) -> Result<String, String> {
    print_no_newline(prompt);
    if !std::io::stdin().is_terminal() {
        return read_line().await.map_err(|detail| detail.replace("io_read_line", "io_read_secret"));
    }

    crossterm::terminal::enable_raw_mode().map_err(|failure| format!("io_read_secret: could not turn off the terminal's echo: {}", failure))?;
    let typed = read_secret_from_terminal();
    // Raw mode is left however the read went, or the terminal is unusable after
    // an error - no echo and no line editing.
    crossterm::terminal::disable_raw_mode().map_err(|failure| format!("io_read_secret: could not turn the terminal's echo back on: {}", failure))?;
    let secret = typed?;
    print_no_newline("\n".to_string());
    return Ok(secret);
}

/// Reads keys until return, in raw mode, showing nothing. Backspace works
/// because someone typing a long password blind will need it.
fn read_secret_from_terminal() -> Result<String, String> {
    use crossterm::event::{Event, KeyCode, KeyModifiers};
    let mut secret = String::new();
    loop {
        let event = crossterm::event::read().map_err(|failure| format!("io_read_secret: could not read from the terminal: {}", failure))?;
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Enter => return Ok(secret),
                KeyCode::Backspace => {
                    secret.pop();
                }
                // Ctrl-C in raw mode does not reach the signal handler, so it is
                // honoured here rather than being typed into the password.
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Err("io_read_secret: cancelled".to_string());
                }
                KeyCode::Char(character) => secret.push(character),
                _ => {}
            }
        }
    }
}

/// A line read with a fallback for when nothing is typed, which is what makes a
/// setup script answerable by holding down return.
pub async fn read_line_or(prompt: String, default_answer: String) -> Result<String, String> {
    print_no_newline(format!("{} [{}] ", prompt, default_answer));
    let answer = read_line().await.map_err(|detail| detail.replace("io_read_line", "io_read_line_or"))?;
    if answer.trim().is_empty() {
        return Ok(default_answer);
    }
    return Ok(answer.trim().to_string());
}

/// Everything on standard input, read to the end - what `cat data | program`
/// hands over. Reading a line at a time is for a person at a terminal; this is
/// for the other half of a pipe, and it is what makes a program usable in one.
pub async fn read_all() -> Result<String, String> {
    let mut input = String::new();
    io::Read::read_to_string(&mut io::stdin(), &mut input).map_err(|e| format!("io_read_all: failed to read from stdin: {}", e))?;
    return Ok(input);
}

/// Whether standard input is a pipe or a file rather than a person typing.
/// A program that reads either way checks this to decide which it is doing.
pub async fn is_piped() -> bool {
    return !io::stdin().is_terminal();
}
