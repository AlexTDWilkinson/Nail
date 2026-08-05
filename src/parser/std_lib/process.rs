use tokio::process::Command as TokioCommand;
use std::future::Future;
use std::pin::Pin;

pub async fn exit(code: i64) -> ! {
    std::process::exit(code as i32)
}

pub async fn run(command: String, args: Vec<String>) -> Result<String, String> {
    let output = TokioCommand::new(&command)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("process_run: could not run '{}': {}", command, e))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| format!("process_run: output of '{}' is not valid UTF-8: {}", command, e))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub async fn spawn<F>(future: F) 
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}
use dashmap::DashMap;
use std::process::Stdio;

/// How to run a command, for the cases `process_run` does not cover: somewhere
/// else, with extra variables, with something on its standard input, or with a
/// limit on how long it may take.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PROCESS_Options {
    /// Directory to run in. Empty means the one the program is already in.
    pub directory: String,
    /// Variables added to the ones the program already has. Nothing is taken
    /// away, so a child still sees PATH and the rest.
    pub environment: DashMap<String, String>,
    /// Text written to the command's standard input, then closed so it sees the
    /// end. Empty means it gets no input at all.
    pub input: String,
    /// Seconds to wait before giving up and killing the command. 0 waits
    /// forever, which is the wrong answer for anything reached over a network.
    pub timeout_seconds: i64,
}

/// What a command did: everything it printed, on both streams, and the number
/// it exited with. Unlike `process_run`, a command that fails is not an error
/// here - the exit code is the answer, which is what a program checking for a
/// particular failure needs.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PROCESS_Result {
    pub stdout: String,
    pub stderr: String,
    /// The exit code, or -1 when a signal ended the command instead.
    pub exit_code: i64,
}

/// The defaults: run here, add nothing, no input, wait as long as it takes.
pub fn default_options() -> PROCESS_Options {
    return PROCESS_Options { directory: String::new(), environment: DashMap::new(), input: String::new(), timeout_seconds: 0 };
}

/// Runs a command and returns everything about how it went, instead of only
/// its output. Errors only when the command could not be started at all.
pub async fn run_result(command: String, args: Vec<String>) -> Result<PROCESS_Result, String> {
    return run_with(command, args, default_options()).await;
}

/// Runs a command the way the options describe it.
pub async fn run_with(command: String, args: Vec<String>, options: PROCESS_Options) -> Result<PROCESS_Result, String> {
    let mut builder = TokioCommand::new(&command);
    builder.args(&args);
    if !options.directory.is_empty() {
        builder.current_dir(&options.directory);
    }
    for entry in options.environment.iter() {
        builder.env(entry.key(), entry.value());
    }
    builder.stdin(if options.input.is_empty() { Stdio::null() } else { Stdio::piped() });
    builder.stdout(Stdio::piped());
    builder.stderr(Stdio::piped());
    // So that giving up on a slow command actually ends it.
    builder.kill_on_drop(true);

    let mut child = builder.spawn().map_err(|e| format!("process_run: could not run '{}': {}", command, e))?;

    if !options.input.is_empty() {
        let mut standard_input = child.stdin.take().ok_or_else(|| format!("process_run: could not write to the standard input of '{}'", command))?;
        tokio::io::AsyncWriteExt::write_all(&mut standard_input, options.input.as_bytes())
            .await
            .map_err(|e| format!("process_run: could not write to the standard input of '{}': {}", command, e))?;
        // Dropping it closes the pipe, which is how the command knows the
        // input has ended - without this a program reading to EOF hangs.
        drop(standard_input);
    }

    let finished = if options.timeout_seconds > 0 {
        let limit = std::time::Duration::from_secs(options.timeout_seconds as u64);
        match tokio::time::timeout(limit, child.wait_with_output()).await {
            Ok(result) => result,
            Err(_) => return Err(format!("process_run: '{}' did not finish within {} seconds", command, options.timeout_seconds)),
        }
    } else {
        child.wait_with_output().await
    };

    let output = finished.map_err(|e| format!("process_run: '{}' could not be waited for: {}", command, e))?;
    return Ok(PROCESS_Result {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        // A command killed by a signal has no exit code of its own.
        exit_code: output.status.code().map(|code| code as i64).unwrap_or(-1),
    });
}

/// Where a command would be found on PATH, the way `which` answers it. What a
/// program checks before offering a feature that shells out to something.
pub async fn which(name: String) -> Result<String, String> {
    // A name with a separator in it is a path already, not something to look up.
    if name.contains('/') {
        if crate::parser::std_lib::fs::is_executable(name.clone()).await {
            return Ok(name);
        }
        return Err(format!("process_which: '{}' is not a program that can be run", name));
    }

    let path = std::env::var("PATH").map_err(|_| "process_which: PATH is not set, so there is nowhere to look".to_string())?;
    for directory in path.split(':') {
        if directory.is_empty() {
            continue;
        }
        let candidate = std::path::Path::new(directory).join(&name).to_string_lossy().to_string();
        if crate::parser::std_lib::fs::is_executable(candidate.clone()).await {
            return Ok(candidate);
        }
    }
    return Err(format!("process_which: '{}' is not on PATH", name));
}

/// Waits until the program is asked to stop - Ctrl-C, or the TERM signal a
/// service manager sends on shutdown - and returns when it is. A server puts
/// this after starting everything and does its closing down afterwards,
/// instead of being killed halfway through a request.
pub async fn wait_for_interrupt() -> Result<(), String> {
    #[cfg(unix)]
    {
        let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).map_err(|e| format!("process_wait_for_interrupt: could not listen for the stop signal: {}", e))?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result.map_err(|e| format!("process_wait_for_interrupt: could not listen for Ctrl-C: {}", e))?,
            _ = terminate.recv() => {}
        }
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.map_err(|e| format!("process_wait_for_interrupt: could not listen for Ctrl-C: {}", e))?;
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_command_reports_its_output_and_its_exit_code() {
        let ran = run_result("sh".to_string(), vec!["-c".to_string(), "echo out; echo err 1>&2; exit 3".to_string()]).await.expect("a runnable shell");
        assert_eq!(ran.stdout, "out\n");
        assert_eq!(ran.stderr, "err\n");
        assert_eq!(ran.exit_code, 3);
    }

    #[tokio::test]
    async fn a_command_can_be_given_a_directory_variables_and_input() {
        let mut options = default_options();
        options.directory = "/tmp".to_string();
        options.environment.insert("NAIL_PROCESS_TEST".to_string(), "here".to_string());
        options.input = "one\ntwo\n".to_string();

        let ran = run_with("sh".to_string(), vec!["-c".to_string(), "pwd; echo $NAIL_PROCESS_TEST; cat".to_string()], options).await.expect("a runnable shell");
        assert_eq!(ran.stdout, "/tmp\nhere\none\ntwo\n");
        assert_eq!(ran.exit_code, 0);
    }

    #[tokio::test]
    async fn a_command_that_takes_too_long_is_given_up_on() {
        let mut options = default_options();
        options.timeout_seconds = 1;
        let error = run_with("sleep".to_string(), vec!["30".to_string()], options).await.unwrap_err();
        assert!(error.contains("did not finish within 1 seconds"), "got: {}", error);
    }

    #[tokio::test]
    async fn a_command_that_is_not_there_is_an_error_rather_than_an_exit_code() {
        let error = run_result("nail_no_such_command".to_string(), vec![]).await.unwrap_err();
        assert!(error.contains("could not run"));
    }

    #[tokio::test]
    async fn a_program_on_the_path_is_found_and_one_that_is_not_says_so() {
        let found = which("sh".to_string()).await.expect("a system with a shell");
        assert!(found.ends_with("/sh"));
        assert!(crate::parser::std_lib::fs::is_executable(found).await);

        assert!(which("nail_no_such_command".to_string()).await.unwrap_err().contains("not on PATH"));
        assert!(which("/bin/sh".to_string()).await.expect("an absolute path") == "/bin/sh");
        assert!(which("/etc/hostname".to_string()).await.unwrap_err().contains("not a program"));
    }
}
