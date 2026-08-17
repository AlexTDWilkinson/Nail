//! The fuzzer's driver.
//!
//! `nail-fuzz run` spreads a range of seeds across worker processes, watches
//! them, and collects what they find. Workers do the compiling: a worker
//! catches its own panics and reports them, so the only thing that kills one
//! is a failure no program can catch, which today means a stack overflow, an
//! abort, or a case that never finishes. The parent notices the death, reads
//! the seed the worker was on, and turns it back into the exact program that
//! killed it.
//!
//! Everything is reproducible from a seed:
//!
//!   nail-fuzz run --seconds=60          a minute of fuzzing on every core
//!   nail-fuzz case 12345                print the program seed 12345 makes
//!   nail-fuzz check some.nail           ask every question of one file
//!   nail-fuzz build                     hand the accepted programs to rustc,
//!                                       and run the ones that came with an
//!                                       answer to check they print it
//!   nail-fuzz predict 12345             print the program seed 12345 makes
//!                                       for the run tier, and what it owes
//!   nail-fuzz imports --cases=500       fuzz the sandbox import promises

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nail::fuzz::corpus::Corpus;
use nail::fuzz::imports::Kind as ImportKind;
use nail::fuzz::oracle::{Finding, Outcome, Property};
use nail::fuzz::{case, shrink, Engine};

/// Where the nail crate lives, for the Cargo.toml the build tier writes. The
/// fuzzer is built from the repository, so the repository is where it is.
const NAIL_CRATE_PATH: &str = env!("CARGO_MANIFEST_DIR");

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let command = arguments.get(1).map(String::as_str).unwrap_or("help");
    match command {
        "run" => run(&arguments),
        "worker" => worker(&arguments),
        "case" => print_case(&arguments),
        "predict" => print_predicted_case(&arguments),
        "check" => check_file(&arguments),
        "format" => format_file(&arguments),
        "build" => build_queue(&arguments),
        "imports" => fuzz_imports(&arguments),
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}

fn usage() {
    eprintln!("Usage: nail-fuzz <command> [options]");
    eprintln!();
    eprintln!("  run                Fuzz until the case count or the time runs out");
    eprintln!("    --seconds=N      Stop after N seconds (default: run the whole case count)");
    eprintln!("    --cases=N        How many programs to try (default 20000)");
    eprintln!("    --seed=N         First seed (default 1)");
    eprintln!("    --jobs=N         Worker processes (default: one per core, less two)");
    eprintln!("    --engine=E       mutate, generate, or both (default both)");
    eprintln!("    --dir=PATH       Where findings and progress go (default target/fuzz)");
    eprintln!("    --timeout=N      Seconds one case may take before it counts as hung (default 20)");
    eprintln!("    --queue=N        How many accepted programs to keep for `build` (default 200)");
    eprintln!();
    eprintln!("  case <seed>        Print the program a seed produces");
    eprintln!("    --engine=E       Which engine writes it (default both, so the seed decides)");
    eprintln!();
    eprintln!("  predict <seed>     Print the program a seed produces for the run tier, and");
    eprintln!("                     the standard output it is owed");
    eprintln!();
    eprintln!("  check <file>       Ask every in-process question of one file");
    eprintln!("                     Exit 0 when clean, 2 when something is wrong");
    eprintln!();
    eprintln!("  build              Compile every program in the queue with rustc, as one");
    eprintln!("                     shared cargo project, and report any the checker accepted");
    eprintln!("                     and rustc refused. A queued program that came with an");
    eprintln!("                     expected output is then run, in an empty directory of its");
    eprintln!("                     own, and what it prints is compared byte for byte");
    eprintln!("    --dir=PATH       Where the queue is (default target/fuzz)");
    eprintln!("    --timeout=N      Seconds one program may run before it counts as hung");
    eprintln!("                     (default 20)");
    eprintln!();
    eprintln!("  imports            Fuzz the sandbox import() promises: multi-file cases,");
    eprintln!("                     each one built knowing whether it must be accepted or refused");
    eprintln!("    --cases=N        How many cases to try (default 2000)");
    eprintln!("    --seed=N         First seed (default 1)");
    eprintln!("    --dir=PATH       Where findings and scratch files go (default target/fuzz)");
    eprintln!("  imports case <seed>  Print the files a seed produces and the answer they are owed");
}

// ---------------------------------------------------------------------------
// Arguments
// ---------------------------------------------------------------------------

fn flag<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    let prefix = format!("--{}=", name);
    arguments.iter().find_map(|argument| argument.strip_prefix(prefix.as_str()))
}

fn number(arguments: &[String], name: &str, fallback: u64) -> u64 {
    flag(arguments, name).and_then(|text| text.parse().ok()).unwrap_or(fallback)
}

fn engine_from(arguments: &[String]) -> Engine {
    flag(arguments, "engine").and_then(Engine::parse).unwrap_or(Engine::Both)
}

/// Where a run keeps its findings, its queue and its scratch programs.
/// Everything the fuzzer writes is generated, so it lives under target/ with
/// the rest of what a build produces: a scratch program left in the tree
/// would otherwise be swept up by the test scripts, which check every .nail
/// file in the repository and would fail on a case that is meant to be
/// broken.
fn directory_from(arguments: &[String]) -> PathBuf {
    match flag(arguments, "dir") {
        Some(given) => PathBuf::from(given),
        None => Path::new(NAIL_CRATE_PATH).join("target").join("fuzz"),
    }
}

/// The corpus every run and every reproduction draws from: every Nail program
/// in the repository.
fn load_corpus() -> Corpus {
    let root = Path::new(NAIL_CRATE_PATH);
    Corpus::load(&[&root.join("tests"), &root.join("examples")])
}

// ---------------------------------------------------------------------------
// run: the parent
// ---------------------------------------------------------------------------

struct Worker {
    child: std::process::Child,
    progress_file: PathBuf,
    /// The seed this worker would run next if it were restarted, kept so a
    /// death can be stepped over rather than repeated forever.
    stride: u64,
    last_seed: u64,
    last_done: u64,
    last_change: Instant,
    remaining: u64,
}

fn run(arguments: &[String]) {
    let engine = engine_from(arguments);
    let first_seed = number(arguments, "seed", 1);
    let total_cases = number(arguments, "cases", 20_000);
    let seconds = flag(arguments, "seconds").and_then(|text| text.parse::<u64>().ok());
    let stall_timeout = Duration::from_secs(number(arguments, "timeout", 20));
    let queue_cap = number(arguments, "queue", 200);
    let default_jobs = std::thread::available_parallelism().map(|count| count.get().saturating_sub(2).max(1)).unwrap_or(1) as u64;
    let jobs = number(arguments, "jobs", default_jobs).max(1);

    let directory = directory_from(arguments);
    for child in ["findings", "progress", "queue"] {
        fs::create_dir_all(directory.join(child)).expect("the fuzzer can write its own directory");
    }
    // Progress files are per run, so a stale one from a previous run cannot
    // be mistaken for a live worker.
    if let Ok(entries) = fs::read_dir(directory.join("progress")) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }

    let corpus = load_corpus();
    println!("corpus: {} programs, fingerprint {:016x}", corpus.len(), corpus.fingerprint());
    println!("engine: {}, seeds {}..{}, {} workers", engine.name(), first_seed, first_seed + total_cases, jobs);
    println!();

    let started = Instant::now();
    let mut workers: Vec<Worker> = Vec::new();
    for index in 0..jobs {
        let count = total_cases / jobs + if index < total_cases % jobs { 1 } else { 0 };
        workers.push(spawn_worker(&directory, engine, first_seed + index, jobs, count, index, queue_cap / jobs.max(1)));
    }

    let mut deaths: Vec<(u64, String)> = Vec::new();
    let mut last_report = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(150));

        let mut alive = 0;
        for index in 0..workers.len() {
            let status = workers[index].child.try_wait().expect("a spawned worker can be waited on");
            let progress = read_progress(&workers[index].progress_file);
            if let Some((seed, done)) = progress {
                if seed != workers[index].last_seed || done != workers[index].last_done {
                    workers[index].last_seed = seed;
                    workers[index].last_done = done;
                    workers[index].last_change = Instant::now();
                }
            }

            match status {
                Some(exit) if exit.success() => continue,
                Some(exit) => {
                    // A worker that did not exit cleanly was killed by the
                    // case it was on. That seed rebuilds the program exactly.
                    let seed = workers[index].last_seed;
                    let reason = format!("worker died ({}) on seed {}", exit, seed);
                    deaths.push((seed, reason.clone()));
                    println!("  ! {}", reason);
                    record_death(&directory, engine, seed, &corpus, "the compiler died on this program rather than reporting an error", stall_timeout);
                    let done = workers[index].last_done;
                    let remaining = workers[index].remaining.saturating_sub(done + 1);
                    if remaining > 0 {
                        let stride = workers[index].stride;
                        let next = seed + stride;
                        workers[index] = spawn_worker(&directory, engine, next, stride, remaining, index as u64, queue_cap / jobs.max(1));
                        alive += 1;
                    }
                    continue;
                }
                None => {}
            }

            if workers[index].last_change.elapsed() > stall_timeout {
                let seed = workers[index].last_seed;
                let reason = format!("worker stuck for {}s on seed {}", stall_timeout.as_secs(), seed);
                deaths.push((seed, reason.clone()));
                println!("  ! {}", reason);
                let _ = workers[index].child.kill();
                let _ = workers[index].child.wait();
                record_death(&directory, engine, seed, &corpus, "the compiler never finished with this program", stall_timeout);
                let done = workers[index].last_done;
                let remaining = workers[index].remaining.saturating_sub(done + 1);
                if remaining > 0 {
                    let stride = workers[index].stride;
                    let next = seed + stride;
                    workers[index] = spawn_worker(&directory, engine, next, stride, remaining, index as u64, queue_cap / jobs.max(1));
                    alive += 1;
                }
                continue;
            }
            alive += 1;
        }

        let done: u64 = workers.iter().map(|worker| worker.last_done).sum();
        if last_report.elapsed() > Duration::from_secs(3) {
            let rate = done as f64 / started.elapsed().as_secs_f64().max(0.001);
            println!("  {} cases, {:.0}/s, {} findings", done, rate, count_findings(&directory));
            last_report = Instant::now();
        }

        if alive == 0 {
            break;
        }
        if let Some(limit) = seconds {
            if started.elapsed() > Duration::from_secs(limit) {
                for worker in workers.iter_mut() {
                    let _ = worker.child.kill();
                    let _ = worker.child.wait();
                }
                break;
            }
        }
    }

    let done: u64 = workers.iter().map(|worker| worker.last_done).sum();
    let elapsed = started.elapsed().as_secs_f64();
    println!();
    println!("{} cases in {:.1}s, {:.0} per second", done, elapsed, done as f64 / elapsed.max(0.001));
    report_reach(&directory);
    report_findings(&directory);
    if !deaths.is_empty() {
        println!();
        println!("{} case(s) killed a worker outright:", deaths.len());
        for (seed, reason) in &deaths {
            println!("  seed {}: {}", seed, reason);
        }
    }
    let queued = fs::read_dir(directory.join("queue")).map(|entries| entries.flatten().count()).unwrap_or(0);
    if queued > 0 {
        println!();
        println!("{} accepted programs are queued for rustc. Run: nail-fuzz build --dir={}", queued, directory.display());
    }
}

fn spawn_worker(directory: &Path, engine: Engine, start: u64, stride: u64, count: u64, id: u64, queue_cap: u64) -> Worker {
    let progress_file = directory.join("progress").join(format!("worker_{}", id));
    let _ = fs::write(&progress_file, format!("{} 0", start));
    let child = Command::new(std::env::current_exe().expect("the fuzzer knows its own path"))
        .arg("worker")
        .arg(format!("--start={}", start))
        .arg(format!("--stride={}", stride))
        .arg(format!("--count={}", count))
        .arg(format!("--engine={}", engine.name()))
        .arg(format!("--dir={}", directory.display()))
        .arg(format!("--id={}", id))
        .arg(format!("--queue={}", queue_cap))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("a worker process can be started");
    Worker { child, progress_file, stride, last_seed: start, last_done: 0, last_change: Instant::now(), remaining: count }
}

/// The seed and the case count a worker last wrote. A half written file just
/// reads as no progress, and the next poll picks it up.
fn read_progress(path: &Path) -> Option<(u64, u64)> {
    let text = fs::read_to_string(path).ok()?;
    let mut parts = text.split_whitespace();
    let seed = parts.next()?.parse().ok()?;
    let done = parts.next()?.parse().ok()?;
    Some((seed, done))
}

/// How far the run's programs got, added up across the workers. A fuzzer that
/// never reaches the type checker is testing the lexer and nothing else, and
/// this is the line that says so.
fn report_reach(directory: &Path) {
    let mut totals: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    if let Ok(entries) = fs::read_dir(directory.join("progress")) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(true, |extension| extension != "reach") {
                continue;
            }
            let Ok(text) = fs::read_to_string(entry.path()) else { continue };
            for line in text.lines() {
                let mut parts = line.rsplitn(2, ' ');
                let Some(count) = parts.next().and_then(|number| number.parse::<u64>().ok()) else { continue };
                let Some(stage) = parts.next() else { continue };
                *totals.entry(stage.to_string()).or_default() += count;
            }
        }
    }
    let total: u64 = totals.values().sum();
    if total == 0 {
        return;
    }
    // In the order the stages run, so the line reads as a funnel.
    let order = ["lex", "parse", "check", "transpile", "built"];
    let parts: Vec<String> = order
        .iter()
        .filter_map(|stage| totals.get(*stage).map(|count| format!("{} {:.0}%", stage, 100.0 * *count as f64 / total as f64)))
        .collect();
    println!("refused at: {}", parts.join(", "));
}

fn count_findings(directory: &Path) -> usize {
    fs::read_dir(directory.join("findings")).map(|entries| entries.flatten().filter(|entry| entry.path().extension().map_or(false, |extension| extension == "nail")).count()).unwrap_or(0)
}

fn report_findings(directory: &Path) {
    let findings = directory.join("findings");
    let mut names: Vec<String> = fs::read_dir(&findings)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().map_or(false, |extension| extension == "txt"))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    if names.is_empty() {
        println!("no findings");
        return;
    }
    println!("{} finding(s) in {}:", names.len(), findings.display());
    for name in names {
        let path = findings.join(&name);
        let first_lines: String = fs::read_to_string(&path).unwrap_or_default().lines().take(2).collect::<Vec<_>>().join(" / ");
        println!("  {}", first_lines);
        println!("      {}", path.with_extension("nail").display());
    }
}

/// A case that killed or hung a worker. The parent rebuilds it from its seed,
/// shrinks it by asking a fresh process each time, and writes it out.
fn record_death(directory: &Path, engine: Engine, seed: u64, corpus: &Corpus, what: &str, timeout: Duration) {
    let scratch = directory.join("scratch");
    let _ = fs::create_dir_all(&scratch);
    let built = case(engine, seed, corpus);

    let probe_path = scratch.join(format!("probe_{}.nail", seed));
    let still_dies = |source: &str| -> bool {
        if fs::write(&probe_path, source).is_err() {
            return false;
        }
        matches!(run_check(&probe_path, timeout), CheckOutcome::Died | CheckOutcome::Hung)
    };

    if !still_dies(&built.source) {
        // The case does not reproduce on its own, which usually means the
        // worker died of something the case only contributed to. Keep it
        // anyway: an unreproducible death is still worth a look.
        write_finding_files(directory, &format!("process_died_seed_{}", seed), &built.source, &format!("{}\n\nseed {} ({}), which did not reproduce when run again on its own\n", what, seed, built.origin));
        return;
    }

    let minimal = shrink::shrink(&built.source, 400, still_dies);
    let key = format!("process_died_{:016x}", fingerprint_of(&minimal));
    write_finding_files(directory, &key, &minimal, &format!("{}\n\nseed {} ({}), shrunk from {} lines to {}\nreproduce: nail-fuzz check <this file>\n", what, seed, built.origin, built.source.lines().count(), minimal.lines().count()));
    let _ = fs::remove_file(&probe_path);
}

enum CheckOutcome {
    Clean,
    Finding,
    Died,
    Hung,
}

/// Ask a fresh process the questions, so a case that aborts the process is an
/// answer rather than the end of the run.
fn run_check(path: &Path, timeout: Duration) -> CheckOutcome {
    let Ok(mut child) = Command::new(std::env::current_exe().expect("the fuzzer knows its own path")).arg("check").arg(path).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else {
        return CheckOutcome::Clean;
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return match status.code() {
                    Some(0) => CheckOutcome::Clean,
                    Some(2) => CheckOutcome::Finding,
                    _ => CheckOutcome::Died,
                }
            }
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return CheckOutcome::Hung;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return CheckOutcome::Clean,
        }
    }
}

fn fingerprint_of(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Write a finding, unless one with the same name is already there. Workers
/// race each other for this, so the file is created exclusively and a loser
/// simply moves on.
fn write_finding_files(directory: &Path, key: &str, source: &str, description: &str) {
    let findings = directory.join("findings");
    let _ = fs::create_dir_all(&findings);
    let text_path = findings.join(format!("{}.txt", key));
    let Ok(mut file) = fs::OpenOptions::new().write(true).create_new(true).open(&text_path) else { return };
    let _ = file.write_all(description.as_bytes());
    let _ = fs::write(findings.join(format!("{}.nail", key)), source);
}

// ---------------------------------------------------------------------------
// worker: the process that actually compiles things
// ---------------------------------------------------------------------------

fn worker(arguments: &[String]) {
    nail::fuzz::oracle::install_panic_hook();
    // One compiler stack for the whole run of cases, rather than one per
    // case: examining a program asks for the compiler's stack, and asking
    // from here means it is already there.
    nail::common::with_compiler_stack(|| worker_loop(arguments));
}

fn worker_loop(arguments: &[String]) {
    let engine = engine_from(arguments);
    let start = number(arguments, "start", 1);
    let stride = number(arguments, "stride", 1).max(1);
    let count = number(arguments, "count", 1000);
    let id = number(arguments, "id", 0);
    let queue_cap = number(arguments, "queue", 50);
    let directory = directory_from(arguments);
    let corpus = load_corpus();

    let progress_file = directory.join("progress").join(format!("worker_{}", id));
    let scratch = directory.join("scratch");
    let _ = fs::create_dir_all(&scratch);
    // Cases are written to disk because `import` resolves against the file
    // that holds the program, so a case has to have a place it lives.
    let case_path = scratch.join(format!("worker_{}.nail", id));

    let mut queued = 0;
    let mut predicted = 0;
    // How far the cases got, so a run can say whether its engine is reaching
    // the stages it means to test.
    let mut reach: std::collections::BTreeMap<&'static str, u64> = std::collections::BTreeMap::new();
    for index in 0..count {
        let seed = start + index * stride;
        let _ = fs::write(&progress_file, format!("{} {}", seed, index));

        // The run tier's case for this seed: a program whose answer the
        // generator worked out as it wrote it. Only until the queue is full,
        // since the cost of one is not the writing but the rustc build and
        // the run that come later.
        if predicted < queue_cap {
            predicted += queue_predicted_case(&directory, seed, &case_path, engine, &corpus);
        }

        let built = case(engine, seed, &corpus);
        if fs::write(&case_path, &built.source).is_err() {
            continue;
        }
        let (finding, outcome) = nail::fuzz::examine(&built.source, &case_path);

        match &outcome {
            Outcome::Refused(stage) => *reach.entry(stage).or_default() += 1,
            Outcome::Built(_) => *reach.entry("built").or_default() += 1,
        }

        if let Some(finding) = finding {
            report(&directory, engine, seed, &built.origin, &built.source, &finding, &corpus, &case_path);
            continue;
        }

        // A program that got all the way through is a candidate for the
        // rustc tier, which is far too slow to run on everything.
        if let Outcome::Built(_) = outcome {
            if queued < queue_cap {
                let name = format!("seed_{}.nail", seed);
                if fs::write(directory.join("queue").join(name), &built.source).is_ok() {
                    queued += 1;
                }
            }
        }
    }
    let _ = fs::write(&progress_file, format!("{} {}", start + count * stride, count));
    let tally: Vec<String> = reach.iter().map(|(stage, count)| format!("{} {}", stage, count)).collect();
    let _ = fs::write(directory.join("progress").join(format!("worker_{}.reach", id)), tally.join("\n"));
}

/// Write one predicted program into the queue, with the standard output it
/// owes beside it, and say whether one was written. The in-process questions
/// are asked of it first: a predicted program is a program like any other,
/// and one that breaks an invariant is a finding here rather than a puzzle
/// later when its output does not match.
fn queue_predicted_case(directory: &Path, seed: u64, case_path: &Path, engine: Engine, corpus: &Corpus) -> u64 {
    let Some((source, origin, expected)) = nail::fuzz::generate::case_with_expected(seed) else { return 0 };
    if fs::write(case_path, &source).is_err() {
        return 0;
    }
    let (finding, outcome) = nail::fuzz::examine(&source, case_path);
    if let Some(finding) = finding {
        report(directory, engine, seed, &origin, &source, &finding, corpus, case_path);
        return 0;
    }
    if !matches!(outcome, Outcome::Built(_)) {
        return 0;
    }
    if fs::write(directory.join("queue").join(format!("predict_{}.nail", seed)), &source).is_err() {
        return 0;
    }
    // The answer goes beside the program, which is what tells `build` that
    // this one is to be run rather than only compiled.
    if fs::write(directory.join("queue").join(format!("predict_{}.stdout", seed)), &expected).is_err() {
        return 0;
    }
    1
}

/// Shrink a finding and write it out, if it is a kind not already recorded.
fn report(directory: &Path, engine: Engine, seed: u64, origin: &str, source: &str, finding: &Finding, _corpus: &Corpus, case_path: &Path) {
    let key = finding.fingerprint();
    let findings = directory.join("findings");
    let _ = fs::create_dir_all(&findings);
    if findings.join(format!("{}.txt", key)).exists() {
        return;
    }

    // Shrinking asks the same question of smaller and smaller programs, and
    // "the same question" means the same property broken in the same place.
    let target_property = finding.property;
    let target_site = finding.site.clone();
    let still_fails = |candidate: &str| -> bool {
        if fs::write(case_path, candidate).is_err() {
            return false;
        }
        let (found, _) = nail::fuzz::examine(candidate, case_path);
        match found {
            Some(other) => other.property == target_property && other.site == target_site,
            None => false,
        }
    };
    let minimal = shrink::shrink(source, 3000, still_fails);
    // Put the case file back the way the caller left it, since the shrink
    // just wrote a few hundred candidates over it.
    let _ = fs::write(case_path, source);

    let description = format!(
        "{}: {}\n{}\n\nstage: {}\nwhat broke: {}\nwhere: {}\nseed: {} ({}, engine {})\nshrunk: {} lines to {}\n",
        finding.property.name(),
        finding.detail,
        finding.property.promise(),
        finding.stage,
        finding.property.promise(),
        finding.site.clone().unwrap_or_else(|| "not a panic, so no place in the compiler".to_string()),
        seed,
        origin,
        engine.name(),
        source.lines().count(),
        minimal.lines().count(),
    );
    write_finding_files(directory, &key, &minimal, &description);
    println!("  ! {} in {}: {}", finding.property.name(), finding.stage, finding.detail);
}

// ---------------------------------------------------------------------------
// case and check: one program at a time
// ---------------------------------------------------------------------------

fn print_case(arguments: &[String]) {
    let Some(seed) = arguments.get(2).and_then(|text| text.parse::<u64>().ok()) else {
        eprintln!("Usage: nail-fuzz case <seed> [--engine=mutate|generate|both]");
        std::process::exit(1);
    };
    let corpus = load_corpus();
    let built = case(engine_from(arguments), seed, &corpus);
    eprintln!("// {}", built.origin);
    print!("{}", built.source);
}

/// The run tier's program for a seed, and the answer it owes. The program
/// goes to standard output so it can be piped into a file, and the answer to
/// standard error so it can be read beside it.
fn print_predicted_case(arguments: &[String]) {
    let Some(seed) = arguments.get(2).and_then(|text| text.parse::<u64>().ok()) else {
        eprintln!("Usage: nail-fuzz predict <seed>");
        std::process::exit(1);
    };
    let Some((source, origin, expected)) = nail::fuzz::generate::case_with_expected(seed) else {
        eprintln!("seed {} produced no predictable program", seed);
        std::process::exit(1);
    };
    eprintln!("// {}", origin);
    print!("{}", source);
    eprintln!();
    eprintln!("// it must print, exactly:");
    for line in expected.lines() {
        eprintln!("// {}", line);
    }
}

fn check_file(arguments: &[String]) {
    nail::fuzz::oracle::install_panic_hook();
    let Some(path) = arguments.get(2) else {
        eprintln!("Usage: nail-fuzz check <file.nail>");
        std::process::exit(1);
    };
    let path = Path::new(path);
    let Ok(source) = fs::read_to_string(path) else {
        eprintln!("cannot read {}", path.display());
        std::process::exit(1);
    };
    let (finding, outcome) = nail::fuzz::examine_thoroughly(&source, path);
    match finding {
        Some(finding) => {
            println!("{}: {}", finding.property.name(), finding.detail);
            println!("stage: {}", finding.stage);
            if let Some(site) = finding.site {
                println!("where: {}", site);
            }
            println!("promise: {}", finding.property.promise());
            std::process::exit(2);
        }
        None => {
            match outcome {
                Outcome::Built(_) => println!("clean, and it builds"),
                Outcome::Refused(stage) => println!("clean, and the compiler refused it at {}", stage),
            }
            std::process::exit(0);
        }
    }
}

/// The formatter's raw output, safety net and all removed. `nailc fmt`
/// refuses to write a file whose meaning formatting would change, which is
/// the right behavior and useless for finding out what it did wrong, so this
/// prints what the formatter actually produced.
fn format_file(arguments: &[String]) {
    let Some(path) = arguments.get(2) else {
        eprintln!("Usage: nail-fuzz format <file.nail>");
        std::process::exit(1);
    };
    let Ok(source) = fs::read_to_string(path) else {
        eprintln!("cannot read {}", path);
        std::process::exit(1);
    };
    let lines: Vec<String> = source.lines().map(String::from).collect();
    for line in nail::formatter::format_nail_code(&lines) {
        println!("{}", line);
    }
}

// ---------------------------------------------------------------------------
// imports: the sandbox tier
// ---------------------------------------------------------------------------

/// Fuzz what `import` promises. Every case here is several files, and unlike
/// the other engines it knows the answer before it asks: a helper that only
/// computes has to be accepted, and one that reaches the world has to be
/// refused. Cases are run in this process, one after another, because the
/// whole case is three small files and the compiler never gets near the
/// stack's edge on one.
fn fuzz_imports(arguments: &[String]) {
    nail::fuzz::oracle::install_panic_hook();
    if arguments.get(2).map(String::as_str) == Some("case") {
        print_import_case(arguments);
        return;
    }

    let first_seed = number(arguments, "seed", 1);
    let total = number(arguments, "cases", 2000).max(1);
    let directory = directory_from(arguments);
    // Everything the fuzzer writes lives under target/, because the test
    // scripts sweep every .nail file in the repository and a case built to be
    // refused would fail them.
    let scratch = directory.join("imports");
    if fs::create_dir_all(&scratch).is_err() {
        eprintln!("cannot write {}", scratch.display());
        std::process::exit(1);
    }

    println!("imports: seeds {}..{}, {} kinds of case", first_seed, first_seed + total, ImportKind::ALL.len());
    println!();

    let started = Instant::now();
    let mut answered_as_owed = 0u64;
    let mut findings = 0u64;
    // How many cases of each kind ran and how many were answered the way they
    // had to be, so a run can say what it covered rather than only what it
    // found.
    let mut per_kind: std::collections::BTreeMap<&'static str, (u64, u64)> = std::collections::BTreeMap::new();

    for index in 0..total {
        let seed = first_seed + index;
        let built = nail::fuzz::imports::case(seed);
        let (finding, answer) = nail::fuzz::imports::examine(&built, &scratch);
        let tally = per_kind.entry(built.kind.name()).or_insert((0, 0));
        tally.0 += 1;
        match finding {
            Some(finding) => {
                findings += 1;
                report_import_finding(&directory, &built, &finding, &answer);
            }
            None => {
                tally.1 += 1;
                answered_as_owed += 1;
            }
        }
        // The files are taken away again, so a run of thousands leaves the
        // scratch directory as it found it.
        nail::fuzz::imports::remove_files(&built, &scratch);
    }

    let elapsed = started.elapsed().as_secs_f64();
    println!("{} cases in {:.1}s, {:.0} per second", total, elapsed, total as f64 / elapsed.max(0.001));
    println!("{} of {} answered the way they had to be", answered_as_owed, total);
    for (kind, (ran, owed)) in &per_kind {
        println!("  {:<34} {} case(s), {} answered as owed", kind, ran, owed);
    }
    println!();
    if findings == 0 {
        println!("no findings");
    } else {
        report_findings(&directory);
    }
}

fn print_import_case(arguments: &[String]) {
    let Some(seed) = arguments.get(3).and_then(|text| text.parse::<u64>().ok()) else {
        eprintln!("Usage: nail-fuzz imports case <seed>");
        std::process::exit(1);
    };
    let built = nail::fuzz::imports::case(seed);
    eprintln!("// {}", built.origin());
    eprintln!("// {}", built.kind.reason());
    if !built.kind.marker().is_empty() {
        eprintln!("// the refusal has to say: {}", built.kind.marker());
    }
    print!("{}", built.written());
}

/// Write a sandbox finding out: the files as one readable text, a directory
/// holding them separately so the case can simply be compiled again, and what
/// the compiler said against what it owed.
fn report_import_finding(directory: &Path, case: &nail::fuzz::imports::ImportCase, finding: &Finding, answer: &nail::fuzz::imports::Answer) {
    let key = usable_as_a_file_name(&format!("{}_{}_{}", finding.property.name(), case.kind.name(), finding.class));
    // One bug is one finding, however many cases hit it. A hole in the
    // sandbox is hit by every case of its kind, and a run that printed and
    // wrote all of them would bury the first one under its own copies.
    if directory.join("findings").join(format!("{}.txt", key)).exists() {
        return;
    }
    println!("  ! {} in {}: {}", finding.property.name(), finding.stage, finding.detail);
    let description = format!(
        "{}: {}\n{}\n\nstage: {}\nkind: {} (must be {})\nwhy: {}\nwhat the compiler said: {}\nseed: {}\nreproduce: nail-fuzz imports case {}\nthe files, ready to compile: {}\n",
        finding.property.name(),
        finding.detail,
        finding.property.promise(),
        finding.stage,
        case.kind.name(),
        case.verdict().name(),
        case.kind.reason(),
        answer.describe(),
        case.seed,
        case.seed,
        directory.join("findings").join(&key).display(),
    );
    write_finding_files(directory, &key, &case.written(), &description);
    // The files on their own as well, because a sandbox case is only a case
    // when the imported file is a file.
    let case_directory = directory.join("findings").join(&key);
    if fs::create_dir_all(&case_directory).is_ok() {
        for file in &case.files {
            let _ = fs::write(case_directory.join(&file.name), &file.source);
        }
    }
}

/// A string reduced to something usable as a file name.
fn usable_as_a_file_name(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 90 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

// ---------------------------------------------------------------------------
// build: the rustc tier
// ---------------------------------------------------------------------------

/// Compile every queued program with rustc, as bins of one shared cargo
/// project so the dependencies build once instead of once per program. Any
/// error rustc reports is a program the type checker accepted and Rust
/// refused, which is the failure the checker exists to prevent.
fn build_queue(arguments: &[String]) {
    nail::fuzz::oracle::install_panic_hook();
    let directory = directory_from(arguments);
    let queue = directory.join("queue");
    let project = directory.join("build");
    let bins = project.join("src").join("bin");
    let _ = fs::create_dir_all(&bins);
    if let Ok(entries) = fs::read_dir(&bins) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }

    let Ok(entries) = fs::read_dir(&queue) else {
        println!("nothing queued in {}", queue.display());
        return;
    };
    let mut cases: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).filter(|path| path.extension().map_or(false, |extension| extension == "nail")).collect();
    cases.sort();
    if cases.is_empty() {
        println!("nothing queued in {}", queue.display());
        return;
    }

    let mut dependencies: BTreeSet<String> = BTreeSet::new();
    let mut features: BTreeSet<String> = BTreeSet::new();
    let mut sources: Vec<(String, PathBuf, String)> = Vec::new();
    for path in &cases {
        let Ok(source) = fs::read_to_string(path) else { continue };
        let name = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_else(|| "case".to_string());
        let Some((rust, manifest)) = transpile_for_build(&source, path) else { continue };
        merge_manifest(&manifest, &mut dependencies, &mut features);
        if fs::write(bins.join(format!("{}.rs", name)), &rust).is_ok() {
            sources.push((name, path.clone(), source));
        }
    }

    if sources.is_empty() {
        println!("nothing in the queue transpiles, so there is nothing to build");
        return;
    }

    let nail_dependency = if features.is_empty() {
        format!("nail = {{ path = \"{}\" }}", NAIL_CRATE_PATH)
    } else {
        format!("nail = {{ path = \"{}\", features = [{}] }}", NAIL_CRATE_PATH, features.iter().map(|feature| format!("\"{}\"", feature)).collect::<Vec<_>>().join(", "))
    };
    let manifest = format!(
        "[package]\nname = \"nail_fuzz_build\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{}\n{}\n\n# Hundreds of small binaries: linking is the whole cost, and debug info\n# doubles it for output nobody reads.\n[profile.dev]\ndebug = false\n",
        nail_dependency,
        dependencies.iter().cloned().collect::<Vec<_>>().join("\n")
    );
    let _ = fs::write(project.join("Cargo.toml"), manifest);

    println!("building {} program(s) with rustc, one shared project", sources.len());
    let output = Command::new("cargo")
        .arg("build")
        .arg("--message-format=json")
        .arg("--manifest-path")
        .arg(project.join("Cargo.toml"))
        // Named rather than left to cargo, so that a CARGO_TARGET_DIR set in
        // the environment cannot move the binaries out from under the run
        // tier.
        .arg("--target-dir")
        .arg(project.join("target"))
        // Captured rather than inherited, because cargo's own summary names
        // every binary that failed, and that list is what this tier is
        // checked against below. It is printed straight through, so nothing
        // is hidden from whoever is watching.
        .stderr(Stdio::piped())
        .output()
        .expect("cargo runs");

    let mut broken: Vec<(String, String)> = Vec::new();
    let mut unattributed: Vec<String> = Vec::new();
    // Where each program's binary ended up, taken from cargo's own report of
    // what it built rather than guessed from the layout of a target
    // directory.
    let mut executables: std::collections::BTreeMap<String, PathBuf> = std::collections::BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        if message.get("reason").and_then(|reason| reason.as_str()) == Some("compiler-artifact") {
            if let Some(executable) = message.get("executable").and_then(|executable| executable.as_str()) {
                let path = PathBuf::from(executable);
                if let Some(stem) = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()) {
                    executables.insert(stem, path);
                }
            }
            continue;
        }
        if message.get("reason").and_then(|reason| reason.as_str()) != Some("compiler-message") {
            continue;
        }
        let Some(diagnostic) = message.get("message") else { continue };
        if diagnostic.get("level").and_then(|level| level.as_str()) != Some("error") {
            continue;
        }
        let rendered = diagnostic.get("rendered").and_then(|rendered| rendered.as_str()).unwrap_or("").to_string();
        // Which program the error belongs to. The first span usually says,
        // but an error raised inside a macro points at the macro's own file,
        // so the whole rendered message is searched as well. An error that
        // could not be attributed was silently dropped, and the run then
        // reported that everything built while cargo had said otherwise.
        let named_files: Vec<String> = diagnostic
            .get("spans")
            .and_then(|spans| spans.as_array())
            .map(|spans| spans.iter().filter_map(|span| span.get("file_name").and_then(|name| name.as_str()).map(String::from)).collect())
            .unwrap_or_default();
        let from_spans = named_files
            .iter()
            .filter_map(|file| Path::new(file).file_stem().map(|stem| stem.to_string_lossy().into_owned()))
            .find(|stem| sources.iter().any(|(name, _, _)| name == stem));
        let stem = match from_spans {
            Some(stem) => Some(stem),
            None => sources.iter().map(|(name, _, _)| name.clone()).find(|name| rendered.contains(&format!("src/bin/{}.rs", name))),
        };
        let Some(stem) = stem else {
            unattributed.push(rendered);
            continue;
        };
        if !broken.iter().any(|(name, _)| *name == stem) {
            broken.push((stem, rendered));
        }
    }

    // Cargo's own summary names every binary it could not compile. Anything
    // in that list which produced no finding above is a failure this tier
    // would otherwise have reported as a pass, so it is named here. A fuzzer
    // that says "all clear" while the compiler underneath it said otherwise
    // is worse than no fuzzer.
    let cargo_said = String::from_utf8_lossy(&output.stderr);
    eprint!("{}", cargo_said);
    let mut missed: Vec<String> = Vec::new();
    for line in cargo_said.lines() {
        let Some(rest) = line.split("could not compile").nth(1) else { continue };
        let Some(named) = rest.split('"').nth(1) else { continue };
        if !broken.iter().any(|(name, _)| name == named) && !missed.iter().any(|name| name == named) {
            missed.push(named.to_string());
        }
    }
    if !missed.is_empty() {
        println!("{} program(s) cargo refused whose error this tier could not read:", missed.len());
        for name in &missed {
            println!("  {}: rebuild it alone with: cargo build --manifest-path {} --bin {}", name, project.join("Cargo.toml").display(), name);
        }
    }

    // An error nobody could be blamed for is still an error, and saying so is
    // the difference between a clean run and a run that looked clean.
    if !unattributed.is_empty() {
        println!("{} rustc error(s) that name no program of ours:", unattributed.len());
        for rendered in &unattributed {
            println!("  {}", rendered.lines().next().unwrap_or("rustc reported an error"));
        }
    }

    if broken.is_empty() && unattributed.is_empty() && missed.is_empty() {
        println!("every queued program built. {} passed", sources.len());
    } else {
        println!("{} program(s) the checker accepted and rustc refused:", broken.len());
        for (name, rendered) in &broken {
            let Some((_, _, source)) = sources.iter().find(|(candidate, _, _)| candidate == name) else { continue };
            let first_error = rendered.lines().next().unwrap_or("rustc reported an error").to_string();
            println!("  {}: {}", name, first_error);
            let key = format!("check_implies_build_{:016x}", fingerprint_of(&first_error));
            write_finding_files(
                &directory,
                &key,
                source,
                &format!("{}: {}\n{}\n\nstage: rustc\nrustc said:\n{}\n", Property::CheckImpliesBuild.name(), first_error, Property::CheckImpliesBuild.promise(), rendered),
            );
        }
        println!();
        println!("written to {}", directory.join("findings").display());
    }

    run_the_answered(&directory, &sources, &executables, Duration::from_secs(number(arguments, "timeout", 20)));
}

/// Run every queued program that came with an answer, and check that it gives
/// that answer. Building is only half of what a compiler owes: a program that
/// compiles and then prints the wrong number is worse than one that does not
/// compile at all, because nothing tells anybody it happened.
///
/// Each program runs in an empty directory of its own, so that one which
/// writes a file cannot be read by the next, and under a timeout, so that one
/// which never finishes does not hold up the rest.
fn run_the_answered(directory: &Path, sources: &[(String, PathBuf, String)], executables: &std::collections::BTreeMap<String, PathBuf>, timeout: Duration) {
    let scratch = directory.join("run");
    let _ = fs::create_dir_all(&scratch);
    let mut ran = 0;
    let mut wrong = 0;
    for (name, path, source) in sources {
        let Ok(expected) = fs::read(path.with_extension("stdout")) else { continue };
        let Some(executable) = executables.get(name) else { continue };

        let run_directory = scratch.join(name);
        let _ = fs::remove_dir_all(&run_directory);
        if fs::create_dir_all(&run_directory).is_err() {
            continue;
        }
        // Output goes to a file rather than a pipe, because a pipe nobody is
        // reading fills up and stops the program that is writing it, which
        // would look exactly like a program that hung.
        let output_path = scratch.join(format!("{}.stdout", name));
        let Ok(output_file) = fs::File::create(&output_path) else { continue };
        let Ok(mut child) = Command::new(executable).current_dir(&run_directory).stdin(Stdio::null()).stdout(Stdio::from(output_file)).stderr(Stdio::null()).spawn() else { continue };

        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if Instant::now() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break None,
            }
        };
        ran += 1;

        let actual = fs::read(&output_path).unwrap_or_default();
        let trouble = match status {
            None => Some(format!("it never finished, after {} seconds", timeout.as_secs())),
            Some(status) if !status.success() => Some(format!("it exited with {}", status)),
            Some(_) if actual != expected => Some(first_difference(&String::from_utf8_lossy(&expected), &String::from_utf8_lossy(&actual))),
            Some(_) => None,
        };
        let Some(trouble) = trouble else { continue };

        wrong += 1;
        println!("  ! {}: {}", name, trouble);
        // One finding file per program: a wrong answer is rare, and each one
        // is its own story, told by the program that produced it.
        let key = format!("runs_with_the_right_answer_{}", name);
        write_finding_files(
            directory,
            &key,
            source,
            &format!(
                "{}: {}\n{}\n\nstage: run\nreproduce: nail-fuzz predict {}\n\nit should have printed:\n{}\nit printed:\n{}\n",
                Property::RunsWithTheRightAnswer.name(),
                trouble,
                Property::RunsWithTheRightAnswer.promise(),
                name.trim_start_matches("predict_"),
                String::from_utf8_lossy(&expected),
                String::from_utf8_lossy(&actual),
            ),
        );
    }

    if ran == 0 {
        return;
    }
    println!();
    if wrong == 0 {
        println!("{} program(s) ran and printed exactly what the generator predicted", ran);
        return;
    }
    println!("{} of {} program(s) printed something other than their answer, written to {}", wrong, ran, directory.join("findings").display());
}

/// The first line two outputs disagree about, said in one line, because a
/// finding's first line is what a person reads before anything else.
fn first_difference(expected: &str, actual: &str) -> String {
    let mut wanted = expected.lines();
    let mut got = actual.lines();
    let mut line = 0;
    loop {
        line += 1;
        match (wanted.next(), got.next()) {
            (None, None) => return "the two outputs differ only in how the last line ends".to_string(),
            (Some(wanted), None) => return format!("line {} is missing, and should read '{}'", line, shorten(wanted)),
            (None, Some(got)) => return format!("line {} should not be there, and reads '{}'", line, shorten(got)),
            (Some(wanted), Some(got)) if wanted != got => return format!("line {} reads '{}' and should read '{}'", line, shorten(got), shorten(wanted)),
            _ => {}
        }
    }
}

/// A line cut to something that fits in a report.
fn shorten(text: &str) -> String {
    if text.chars().count() <= 90 {
        return text.to_string();
    }
    format!("{}...", text.chars().take(90).collect::<String>())
}

/// Transpile one accepted program, returning the Rust and the Cargo.toml it
/// asks for. Anything that does not get that far is not this tier's problem:
/// the in-process oracle already reported it.
fn transpile_for_build(source: &str, path: &Path) -> Option<(String, String)> {
    use nail::checker::checker;
    use nail::lexer::{collect_lexer_errors, lex_program};
    use nail::parser::parse;
    use nail::transpiler::Transpiler;

    let lexed = lex_program(source, Some(path));
    if !collect_lexer_errors(&lexed.tokens).is_empty() {
        return None;
    }
    let mut ast = parse(lexed.tokens).ok()?;
    checker(&mut ast).ok()?;
    let mut transpiler = Transpiler::new();
    transpiler.profile = false;
    let rust = transpiler.transpile(&ast).ok()?;
    let manifest = transpiler.generate_cargo_toml("nail_fuzz_build", NAIL_CRATE_PATH);
    Some((rust, manifest))
}

/// Fold one program's Cargo.toml into the shared one: its dependency lines,
/// and the nail features it asks for.
fn merge_manifest(manifest: &str, dependencies: &mut BTreeSet<String>, features: &mut BTreeSet<String>) {
    let mut in_dependencies = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line == "[dependencies]" {
            in_dependencies = true;
            continue;
        }
        if line.starts_with('[') {
            in_dependencies = false;
            continue;
        }
        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("nail ") || line.starts_with("nail=") {
            if let Some(start) = line.find("features = [") {
                let rest = &line[start + "features = [".len()..];
                if let Some(end) = rest.find(']') {
                    for feature in rest[..end].split(',') {
                        let feature = feature.trim().trim_matches('"').to_string();
                        if !feature.is_empty() {
                            features.insert(feature);
                        }
                    }
                }
            }
            continue;
        }
        dependencies.insert(line.to_string());
    }
}
