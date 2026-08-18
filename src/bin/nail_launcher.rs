//! `nail`: the thing you use to open Nail files.
//!
//! It does four things and nothing else. It reads which version of Nail a file
//! was written for, makes sure that exact version is on the machine, checks
//! that it really came from us, and hands the file to it. Everything else a
//! Nail toolchain does belongs to a release, not to The launcher.
//!
//! That restraint is the whole design. This is the one piece that can never
//! be replaced by a newer Nail, because it is what launches Nail in the first
//! place. So the things it promises have to hold forever:
//!
//!   * the version line grammar (see the `version_line` module, included by path below)
//!   * the URL shape it fetches from
//!   * the small set of subcommands it owns
//!
//! Everything else is forwarded. `nail fmt old.nail` runs the formatter that
//! shipped with `old.nail`'s own compiler, not today's, so commands invented in
//! ten years work through a The launcher built today without it ever being taught
//! about them. Only commands that are about the *set* of installed versions,
//! which no single version can answer, belong to The launcher itself.
//!
//! The version line parser is shared with the compiler by including its source
//! directly rather than by depending on the nail library, so The launcher stays a
//! small self-contained binary that links none of the language.

// The launcher only reads version lines. Writing them is the compiler's half of the same
// module, so those functions are dead code here and live code there.
#[allow(dead_code)]
#[path = "../version_line.rs"]
mod version_line;

use version_line::{Pin, Version};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{IsTerminal, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Frozen constants
// ---------------------------------------------------------------------------

/// Where installed versions live. One path, on every machine, and not
/// negotiable: cargo records absolute paths in its fingerprints, so the build
/// cache a release ships with is only valid at the path it was warmed at. A
/// store somewhere else means every dependency recompiles on first use, which
/// is the one thing a prebuilt toolchain exists to prevent. Installing costs
/// one sudo. Nothing after it does.
const STORE: &str = "/opt/nail";

/// The one URL The launcher knows, forever. Overridable for testing a release
/// before it is announced.
const DEFAULT_ORIGIN: &str = "https://nail.alex-wilkinson.ca";

/// The only target that exists today. It is in the URL anyway, so adding
/// others later does not change the shape of a request already in the wild.
const TARGET: &str = "x86_64-linux";

/// Where the source lives, for `nail github`.
const REPOSITORY: &str = "https://github.com/AlexTDWilkinson/Nail";

/// Size cap on what is read looking for a version line. The version line is in the first
/// two lines, and a file's body may not even be text.
const HEAD_BYTES: usize = version_line::HEAD_BYTES;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

type Fallible<T> = Result<T, String>;

fn fail<T>(message: impl Into<String>) -> Fallible<T> {
    return Err(message.into());
}

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().collect();
    match run(&arguments) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("nail: {}", message);
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Fallible<ExitCode> {
    let store = Store::new();

    let rest = &arguments[1..];

    // No arguments prints the help, the way every other command line tool
    // does. The editor with nothing in it is still reachable, as `nail open`.
    let command = match rest.first() {
        Some(first) => first.as_str(),
        None => {
            print!("{}", usage());
            return Ok(ExitCode::SUCCESS);
        }
    };
    let tail = &rest[1..];

    // The launcher owns exactly the commands that are about the set of installed
    // versions. That list is frozen: growing it later would shadow a
    // subcommand some future nailc wants for itself.
    match command {
        "install" => command_install(&store, tail),
        "remove" => command_remove(&store, tail),
        "list" => command_list(&store, tail),
        "gc" => command_gc(&store, tail),
        "which" => command_which(&store, tail),
        "fetch" => command_fetch(&store, tail),
        "update" => command_update(&store, tail),
        "export" => command_export(&store, tail),
        "import" => command_import(&store, tail),
        "doctor" => command_doctor(&store),
        "self-update" => command_self_update(&store),
        "config" => command_config(tail),
        "new" => command_new(&store, tail),
        "website" => command_website(&store, tail),
        // Both forms are questions about the standard library, and the answer
        // depends on which version is running, so both go to that compiler.
        // Bare `docs` is the whole library, which is the honest answer to
        // "what can this do".
        "docs" if tail.is_empty() => launch(&store, Binary::Compiler, &["--docs=".to_string()]),
        "docs" => launch(&store, Binary::Compiler, &[format!("--docs={}", tail[0])]),
        "test" => command_test(&store, tail),
        "github" | "source" => open_url(REPOSITORY),
        "run" => launch(&store, Binary::Compiler, &append(&resolve_names(tail), "--run")),
        "build" => launch(&store, Binary::Compiler, &append(&resolve_names(tail), "--build")),
        "check" => launch(&store, Binary::Compiler, &append(&resolve_names(tail), "--check-only")),
        "version" => launch(&store, Binary::Compiler, &["--version".to_string()]),
        "open" => launch(&store, Binary::Ide, &resolve_names(tail)),
        "help" | "--help" | "-h" => {
            print!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        "--version" => {
            println!("nail {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        // The escape hatch, for forwarding something that collides with a
        // reserved word above.
        "--" => launch(&store, Binary::Compiler, tail),
        // A bare file opens in the editor, spelled with the extension or
        // without it. The without-it case has to look at the disk, because
        // that is the only thing separating a file name from a subcommand this
        // launcher has never heard of.
        _ if looks_like_nail_file(command) => launch(&store, Binary::Ide, rest),
        _ if with_extension(command).is_file() => launch(&store, Binary::Ide, &resolve_names(rest)),
        // Anything else belongs to a release, and is forwarded to one.
        _ => launch(&store, Binary::Compiler, rest),
    }
}

fn usage() -> String {
    return concat!(
        "nail - the Nail language. Opens each file with the version that wrote it\n",
        "\n",
        "Writing code:\n",
        "  nail open <file>            open a file in the editor\n",
        "  nail open                   open the editor with nothing in it\n",
        "  nail new <file>             create a new file, ready to compile\n",
        "  nail run <file>             compile a file quickly and run it\n",
        "  nail build <file>           full release build, binary left beside the file\n",
        "  nail check <file>           type check a file without building it\n",
        "  nail fmt <file>             format a file to the one canonical style\n",
        "  nail agents                 write the primer into ./AGENTS.md, so coding\n",
        "                              agent tools are briefed on Nail automatically\n",
        "  nail test [pattern]         run every file in tests/, or those matching\n",
        "  nail docs                   every function in the standard library\n",
        "  nail docs <name>            what the library says about one of them\n",
        "  nail docs primer            the whole language on one page, for briefing\n",
        "                              a person or a coding agent meeting Nail cold\n",
        "\n",
        "The .nail extension is optional everywhere. `nail new hello` and\n",
        "`nail new hello.nail` do the same thing.\n",
        "\n",
        "`nail hello` opens a file too, but a name that is also a command is the\n",
        "command, so a file called list.nail needs `nail open list`.\n",
        "\n",
        "Versions of Nail:\n",
        "  nail list [--available]     which are installed, how big, last used\n",
        "  nail install <version>      download one, or `latest` for the newest\n",
        "  nail remove <version>       delete one\n",
        "  nail gc [--caches] [--yes]  reclaim disk, dry run unless --yes\n",
        "  nail which <file>           which version will run this, and why\n",
        "  nail fetch <path>           install every version a tree of files pins\n",
        "  nail update <path>          move files to a newer version, if they still compile\n",
        "  nail export <version> <to>  save a version for a machine with no network\n",
        "  nail import <file>          install one from that file\n",
        "  nail doctor                 check the install over\n",
        "  nail self-update            replace nail itself\n",
        "  nail config <key> [value]   warn, auto, auto-at, keep-days\n",
        "\n",
        "Elsewhere:\n",
        "  nail website [stdlib]       the website, or its standard library listing\n",
        "  nail github                 the source\n",
        "\n",
        "Anything nail does not recognise is passed to the compiler for that file,\n",
        "so a command added in a later release works through this one.\n"
    )
    .to_string();
}

/// Fills in a missing .nail extension on arguments that name a file, leaving
/// flags and anything that already exists on disk alone.
fn resolve_names(arguments: &[String]) -> Vec<String> {
    return arguments
        .iter()
        .map(|argument| {
            if argument.starts_with('-') || Path::new(argument).exists() {
                return argument.clone();
            }
            let guess = with_extension(argument);
            if guess.exists() {
                guess.display().to_string()
            } else {
                argument.clone()
            }
        })
        .collect();
}

fn append(rest: &[String], last: &str) -> Vec<String> {
    let mut all = rest.to_vec();
    all.push(last.to_string());
    return all;
}

/// `hello` and `hello.nail` name the same file. The extension is how the
/// desktop recognises the file, not something a person should have to type.
fn with_extension(name: &str) -> PathBuf {
    if name.ends_with(".nail") {
        return PathBuf::from(name);
    }
    return PathBuf::from(format!("{}.nail", name));
}

fn looks_like_nail_file(argument: &str) -> bool {
    let name = Path::new(argument).file_name().and_then(|name| name.to_str()).unwrap_or(argument);
    return name.ends_with(".nail") && !name.starts_with('.');
}

// ---------------------------------------------------------------------------
// The store: installed versions on disk
// ---------------------------------------------------------------------------

struct Store {
    root: PathBuf,
    origin: String,
}

/// Which binary inside a release to hand a file to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Binary {
    Ide,
    Compiler,
}

impl Binary {
    fn file_name(self) -> &'static str {
        match self {
            Binary::Ide => "nail",
            Binary::Compiler => "nailc",
        }
    }
}

/// The store, which is the same directory on every machine. `NAIL_STORE` moves
/// it, for the test suite and for trying a layout without touching a real
/// install, and a store anywhere else gives up the prebuilt cache: cargo
/// fingerprints hold absolute paths, so a release warmed at one path and used
/// from another rebuilds every dependency it has.
fn default_store() -> PathBuf {
    return PathBuf::from(STORE);
}

impl Store {
    fn new() -> Self {
        // NAIL_STORE is for tests and for trying a layout without touching a
        // real install. NAIL_ORIGIN is for validating a release before it is
        // announced.
        let root = std::env::var_os("NAIL_STORE").map(PathBuf::from).unwrap_or_else(default_store);
        let origin = std::env::var("NAIL_ORIGIN").unwrap_or_else(|_| DEFAULT_ORIGIN.to_string());
        return Store { root, origin };
    }

    fn versions_dir(&self) -> PathBuf {
        return self.root.join("versions");
    }

    fn version_dir(&self, version: &Version) -> PathBuf {
        return self.versions_dir().join(version.to_string());
    }

    fn is_installed(&self, version: &Version) -> bool {
        return self.version_dir(version).join("bin/nailc").is_file();
    }

    /// Every version on the machine, oldest first.
    fn installed(&self) -> Vec<Version> {
        let entries = match fs::read_dir(self.versions_dir()) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };
        let mut versions: Vec<Version> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().and_then(|name| name.parse::<Version>().ok()))
            .filter(|version| self.is_installed(version))
            .collect();
        versions.sort();
        return versions;
    }

    fn newest_installed(&self) -> Option<Version> {
        return self.installed().pop();
    }

    /// Records that a version was used, so `gc` can tell working versions from
    /// abandoned ones. An mtime is the whole mechanism: no database, no log of
    /// which files anyone opened.
    fn touch(&self, version: &Version) {
        let _ = fs::write(self.version_dir(version).join(".last-used"), b"");
    }

    fn last_used(&self, version: &Version) -> Option<SystemTime> {
        return fs::metadata(self.version_dir(version).join(".last-used")).and_then(|meta| meta.modified()).ok();
    }
}

// ---------------------------------------------------------------------------
// Resolution: which version gets the file
// ---------------------------------------------------------------------------

/// Why a version was chosen. Printed by `which`, and by the warnings that stop
/// a resolution from ever being a mystery.
enum Reason {
    Pinned(PathBuf),
    TracksLatest(PathBuf),
    Unpinned(PathBuf),
    Flag,
    NoFile,
}

struct Resolved {
    version: Version,
    reason: Reason,
}

impl Reason {
    fn describe(&self, version: &Version) -> String {
        match self {
            Reason::Pinned(path) => format!("{} pins {}", display_path(path), version),
            Reason::TracksLatest(path) => format!("{} says `nail latest`, newest installed is {}", display_path(path), version),
            Reason::Unpinned(path) => format!("{} pins no version, newest installed is {}", display_path(path), version),
            Reason::Flag => format!("--nail-version asked for {}", version),
            Reason::NoFile => format!("no file named, newest installed is {}", version),
        }
    }
}

fn display_path(path: &Path) -> String {
    return path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_else(|| path.display().to_string());
}

/// The first `.nail` path among the arguments. That file's version line decides the
/// version for the whole program: one compiler compiles every source it
/// reaches, and an imported file pinned to something older must not drag the
/// entry file's compiler backwards with it.
fn entry_file(arguments: &[String]) -> Option<PathBuf> {
    return arguments.iter().find(|argument| looks_like_nail_file(argument)).map(PathBuf::from);
}

fn read_pin(path: &Path) -> Fallible<Option<Pin>> {
    let mut file = fs::File::open(path).map_err(|error| format!("cannot read {}: {}", path.display(), error))?;
    let mut head = vec![0u8; HEAD_BYTES];
    let read = file.read(&mut head).map_err(|error| format!("cannot read {}: {}", path.display(), error))?;
    head.truncate(read);
    return Ok(version_line::read_version_line(&head));
}

/// Decides which version runs, in a fixed order: an explicit flag, then the
/// entry file's version line, then the newest version installed.
fn resolve(store: &Store, arguments: &[String]) -> Fallible<Resolved> {
    if let Some(text) = arguments.iter().find_map(|argument| argument.strip_prefix("--nail-version=")) {
        let version: Version = text.parse().map_err(|_| format!("`{}` is not a version like 0.3.1", text))?;
        ensure_installed(store, &version)?;
        return Ok(Resolved { version, reason: Reason::Flag });
    }

    let path = match entry_file(arguments) {
        Some(path) => path,
        None => {
            let version = newest_or_install(store)?;
            return Ok(Resolved { version, reason: Reason::NoFile });
        }
    };

    match read_pin(&path)? {
        Some(Pin::Exact(version)) => {
            ensure_installed(store, &version)?;
            Ok(Resolved { version, reason: Reason::Pinned(path) })
        }
        // `latest` is opt-in: the IDE writes a concrete version unless somebody
        // deliberately typed this, so typing it is the request to track. Every
        // open asks what the newest release is and fetches it if it is new.
        Some(Pin::Latest) => {
            let version = track_latest(store)?;
            Ok(Resolved { version, reason: Reason::TracksLatest(path) })
        }
        None => {
            let version = newest_or_install(store)?;
            Ok(Resolved { version, reason: Reason::Unpinned(path) })
        }
    }
}

/// What a file that says `nail latest` gets: the newest release there is,
/// fetched now if this machine does not have it.
///
/// The check is a few bytes over the wire and is deliberately given a short
/// deadline, because it sits in front of every open of such a file. When it
/// cannot be reached the newest installed version is used instead, so working
/// offline keeps working, on the reasoning that a stale compiler beats a file
/// that will not open.
fn track_latest(store: &Store) -> Fallible<Version> {
    match published_latest_within(store, Duration::from_secs(5)) {
        Ok(published) => {
            if !store.is_installed(&published) {
                eprintln!("nail: {} is out, fetching it (this file tracks latest)", published);
                install(store, &published)?;
            }
            Ok(published)
        }
        Err(_) => match store.newest_installed() {
            Some(installed) => {
                eprintln!("nail: cannot reach {} to check for a newer Nail, using {}", store.origin, installed);
                Ok(installed)
            }
            None => fail(format!("this file tracks latest, nothing is installed, and {} cannot be reached", store.origin)),
        },
    }
}

/// The newest installed version, installing the newest published one if the
/// machine has none at all. That first download is the only one resolution can
/// ever trigger on its own.
fn newest_or_install(store: &Store) -> Fallible<Version> {
    if let Some(version) = store.newest_installed() {
        return Ok(version);
    }
    eprintln!("nail: no version installed yet, fetching the newest");
    let version = published_latest(store)?;
    install(store, &version)?;
    return Ok(version);
}

fn ensure_installed(store: &Store, version: &Version) -> Fallible<()> {
    if store.is_installed(version) {
        return Ok(());
    }
    if version.is_prerelease() {
        return fail(format!(
            "this file pins {}, which is a local build that was never published\n\
             nail cannot fetch it. Build it, or restamp the file with `nail update`",
            version
        ));
    }
    eprintln!("nail: {} is not installed, fetching it", version);
    return install(store, version);
}

// ---------------------------------------------------------------------------
// Launching
// ---------------------------------------------------------------------------

fn launch(store: &Store, binary: Binary, arguments: &[String]) -> Fallible<ExitCode> {
    let resolved = resolve(store, arguments)?;
    store.touch(&resolved.version);

    if matches!(resolved.reason, Reason::Unpinned(_)) {
        // Silence here is how the no-rot promise quietly dies: the file will
        // compile with a different compiler next year and nobody was told.
        eprintln!("nail: {}", resolved.reason.describe(&resolved.version));
    }

    maybe_nag(store);

    let program = store.version_dir(&resolved.version).join("bin").join(binary.file_name());
    if !program.is_file() {
        return fail(format!("{} is installed but has no {}. Run `nail doctor`", resolved.version, binary.file_name()));
    }

    // Arguments meant for The launcher are not passed on.
    let forwarded: Vec<&String> = arguments.iter().filter(|argument| !argument.starts_with("--nail-version=")).collect();

    // exec rather than spawn, so signals, the exit code and the terminal all
    // belong to the compiler. The launcher is not in the picture once it has chosen.
    let error = Command::new(&program).args(forwarded).exec();
    return fail(format!("cannot run {}: {}", program.display(), error));
}

// ---------------------------------------------------------------------------
// Fetching a release
// ---------------------------------------------------------------------------

fn release_url(store: &Store, version: &Version) -> String {
    return format!("{}/versions/{}/{}", store.origin, version, TARGET);
}

fn get(url: &str) -> Fallible<reqwest::blocking::Response> {
    return get_within(url, Duration::from_secs(60 * 60));
}

fn get_within(url: &str, timeout: Duration) -> Fallible<reqwest::blocking::Response> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .user_agent(concat!("nail/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("cannot start a download: {}", error))?;
    let response = client.get(url).send().map_err(|error| format!("cannot reach {}: {}", url, error))?;
    return Ok(response);
}

fn published_latest(store: &Store) -> Fallible<Version> {
    return published_latest_within(store, Duration::from_secs(60));
}

fn published_latest_within(store: &Store, timeout: Duration) -> Fallible<Version> {
    let url = format!("{}/versions/latest", store.origin);
    let response = get_within(&url, timeout)?;
    if !response.status().is_success() {
        return fail(format!("{} answered {}", url, response.status()));
    }
    let body = response.text().map_err(|error| format!("cannot read {}: {}", url, error))?;
    let text = body.trim();
    return text.parse::<Version>().map_err(|_| format!("{} answered `{}`, which is not a version", url, text));
}

/// Downloads, verifies and unpacks one release.
///
/// Nothing is visible under `versions/` until it has been verified and
/// unpacked whole: the unpack happens beside the final location and is moved
/// into place with a rename, so an interrupted download can never leave
/// something that looks installed but is not.
fn install(store: &Store, version: &Version) -> Fallible<()> {
    if store.is_installed(version) {
        println!("{} is already installed", version);
        return Ok(());
    }
    require_unpack_tools()?;

    let versions = store.versions_dir();
    fs::create_dir_all(&versions).map_err(|error| format!("cannot create {}: {}\n{}", versions.display(), error, permission_hint(store)))?;

    let url = release_url(store, version);
    let staging = versions.join(format!(".incoming-{}", version));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|error| format!("cannot create {}: {}", staging.display(), error))?;
    let cleanup = Cleanup(staging.clone());

    let tarball = staging.join("release.tar.xz");
    let digest = download(&url, &tarball, &format!("Nail {}", version))?;

    println!("unpacking");
    unpack(&tarball, &staging)?;

    // The bundle packs as a single directory named for its version.
    let unpacked = staging.join(version.to_string());
    if !unpacked.join("bin/nailc").is_file() {
        return fail(format!("the {} release does not contain bin/nailc", version));
    }
    fs::write(unpacked.join(".installed"), format!("sha256 {}\n", hex(&digest))).map_err(|error| format!("cannot record the install: {}", error))?;

    let destination = store.version_dir(version);
    let _ = fs::remove_dir_all(&destination);
    fs::rename(&unpacked, &destination).map_err(|error| format!("cannot move {} into place: {}", version, error))?;
    drop(cleanup);

    store.touch(version);
    println!("Nail {} installed", version);

    return Ok(());
}

/// What to do about a store that cannot be written to. The installer hands the
/// store to whoever ran it, so this means somebody else installed Nail on this
/// machine, and saying so is the difference between a fixable problem and a
/// person reaching for sudo out of habit.
fn permission_hint(store: &Store) -> String {
    return format!(
        "That store belongs to whoever installed Nail on this machine.\n\
         Either ask them to install this version, or take the store over with\n  \
         sudo chown -R $USER {}",
        store.root.display()
    );
}

/// Whether a version could be installed into this directory. Reading the
/// permission bits gives the wrong answer for a directory somebody else owns,
/// so this asks the only question that matters by trying it. A store that has
/// not been created yet is not a fault, so what gets tested is the nearest
/// directory above it that does exist.
fn can_write(path: &Path) -> bool {
    let mut candidate = path;
    while !candidate.is_dir() {
        match candidate.parent() {
            Some(parent) => candidate = parent,
            None => return false,
        }
    }
    let probe = candidate.join(".nail-write-probe");
    match fs::File::create(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            return true;
        }
        Err(_) => return false,
    }
}

/// Removes a directory when it goes out of scope, so a failed install leaves
/// no half-downloaded gigabytes behind.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Streams a download to disk, hashing as it goes and drawing a progress bar
/// while it does. A release is most of a gigabyte, so the bar is not decoration:
/// without it the terminal sits silent for minutes and there is no way to tell
/// a slow download from a dead one. Streaming rather than buffering also keeps
/// a gigabyte out of memory, and the hash falls out of the same pass.
fn download(url: &str, destination: &Path, label: &str) -> Fallible<[u8; 32]> {
    use sha2::{Digest, Sha256};

    let mut response = get(url)?;
    match response.status().as_u16() {
        200 => {}
        404 => return fail(format!("there is no Nail {}", version_in(url))),
        status => return fail(format!("{} answered {}", url, status)),
    }

    let total = response.content_length();
    let mut file = fs::File::create(destination).map_err(|error| format!("cannot write {}: {}", destination.display(), error))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 128 * 1024];
    let mut done: u64 = 0;
    let started = std::time::Instant::now();
    let mut last_drawn = std::time::Instant::now();
    let show = std::io::stderr().is_terminal();

    loop {
        let read = response.read(&mut buffer).map_err(|error| format!("the download from {} was cut short: {}", url, error))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|error| format!("cannot write {}: {}", destination.display(), error))?;
        hasher.update(&buffer[..read]);
        done += read as u64;

        // Ten times a second is smooth to watch and cheap to draw. Every chunk
        // would be a thousand writes a second and no more informative.
        if show && last_drawn.elapsed() >= Duration::from_millis(100) {
            draw_progress(label, done, total, started.elapsed());
            last_drawn = std::time::Instant::now();
        }
    }

    if show {
        draw_progress(label, done, total, started.elapsed());
        eprintln!();
    }
    return Ok(hasher.finalize().into());
}

/// One line, rewritten in place. Falls back to a plain byte count when the
/// server did not say how big the thing is, because a bar with no end is a lie.
fn draw_progress(label: &str, done: u64, total: Option<u64>, elapsed: Duration) {
    let speed = if elapsed.as_secs_f64() > 0.0 { done as f64 / elapsed.as_secs_f64() } else { 0.0 };

    match total {
        Some(total) if total > 0 => {
            let share = (done as f64 / total as f64).min(1.0);
            let width = 24;
            let filled = (share * width as f64).round() as usize;
            let bar: String = "█".repeat(filled) + &"░".repeat(width - filled);
            eprint!("\r  {} [{}] {:>3}%  {} / {}  {}/s   ", label, bar, (share * 100.0) as u32, human(done), human(total), human(speed as u64));
        }
        _ => eprint!("\r  {} {}  {}/s   ", label, human(done), human(speed as u64)),
    }
    let _ = std::io::stderr().flush();
}

fn version_in(url: &str) -> String {
    return url.split('/').rev().nth(1).unwrap_or("that version").to_string();
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    return hasher.finalize().into();
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{:02x}", byte);
    }
    return text;
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    return text.as_bytes().chunks(2).map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()).collect();
}

/// Unpacking shells out to `tar`. A release is gigabytes of small files, and
/// the system tar unpacks it far faster than a pure Rust xz decoder would,
/// on a tool that is already documented as needing a mainstream distribution.
fn unpack(tarball: &Path, into: &Path) -> Fallible<()> {
    let status = Command::new("tar").arg("-xf").arg(tarball).arg("-C").arg(into).status().map_err(|error| format!("cannot run tar: {}", error))?;
    if !status.success() {
        return fail("tar could not unpack the release");
    }
    return Ok(());
}

fn have(program: &str) -> bool {
    return Command::new(program).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok();
}

/// A release is a .tar.xz, and tar does not decompress that itself: it runs
/// `xz`. So a machine with tar and no xz-utils used to download the whole
/// release and only then fail, in tar's words about a child process rather
/// than in ours about a missing program.
fn require_unpack_tools() -> Fallible<()> {
    if !have("tar") {
        return fail("`tar` is not installed, and nail needs it to unpack releases");
    }
    if !have("xz") {
        return fail("`xz` is not installed, and nail needs it to unpack releases. tar runs xz to read them, so tar alone is not enough");
    }
    return Ok(());
}

// ---------------------------------------------------------------------------
// Disk: sizes, and what can be reclaimed
// ---------------------------------------------------------------------------

/// A version's disk use, split by what it costs to get back.
struct Usage {
    /// The shared build cache. Deleting it costs minutes on the next build,
    /// and nothing else, which is why it is the first thing `gc` takes.
    cache: u64,
    /// Everything else: the toolchain, vendored sources, the binaries.
    /// Deleting these means a download to get them back.
    rest: u64,
}

impl Usage {
    fn total(&self) -> u64 {
        return self.cache + self.rest;
    }
}

fn measure(store: &Store, version: &Version) -> Usage {
    let root = store.version_dir(version);
    let cache = directory_size(&root.join("cache"));
    return Usage { cache, rest: directory_size(&root).saturating_sub(cache) };
}

fn directory_size(path: &Path) -> u64 {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    let mut total = 0;
    for entry in entries.flatten() {
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => total += directory_size(&entry.path()),
            // Symlinks are counted as nothing: their target is either inside
            // the tree already or is not ours to charge for.
            Ok(kind) if kind.is_file() => total += entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            _ => {}
        }
    }
    return total;
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return format!("{} B", bytes);
    }
    return format!("{:.1} {}", value, UNITS[unit]);
}

fn parse_size(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    let digits: String = trimmed.chars().take_while(|character| character.is_ascii_digit() || *character == '.').collect();
    if digits.is_empty() {
        return None;
    }
    let number: f64 = digits.parse().ok()?;
    let scale = match trimmed[digits.len()..].trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1u64,
        "K" | "KB" => 1024,
        "M" | "MB" => 1024 * 1024,
        "G" | "GB" => 1024 * 1024 * 1024,
        "T" | "TB" => 1024u64 * 1024 * 1024 * 1024,
        _ => return None,
    };
    return Some((number * scale as f64) as u64);
}

fn days_since(time: Option<SystemTime>) -> Option<u64> {
    let time = time?;
    let elapsed = SystemTime::now().duration_since(time).ok()?;
    return Some(elapsed.as_secs() / 86_400);
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// What `gc` does on its own, if anything. Written by `nail config` so
/// nobody hand-edits it, and kept under the user's config directory so that
/// wiping the store does not lose the preference.
struct Config {
    warn_at: u64,
    auto: Auto,
    auto_at: u64,
    keep_days: u64,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Auto {
    Off,
    Caches,
    Full,
}

impl Default for Config {
    fn default() -> Self {
        // Warning is on and cleaning is not. Deleting gigabytes that cost a
        // long download to restore should be something the user turned on.
        return Config { warn_at: 1024 * 1024 * 1024, auto: Auto::Off, auto_at: 10 * 1024 * 1024 * 1024, keep_days: 30 };
    }
}

fn config_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) => PathBuf::from(value),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    return Some(base.join("nail/config.toml"));
}

fn load_config() -> Config {
    let mut config = Config::default();
    let text = config_path().and_then(|path| fs::read_to_string(path).ok()).unwrap_or_default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let value = value.trim().trim_matches('"');
        match key.trim() {
            "warn" => config.warn_at = parse_size(value).unwrap_or(config.warn_at),
            "auto" => {
                config.auto = match value {
                    "caches" => Auto::Caches,
                    "full" => Auto::Full,
                    _ => Auto::Off,
                }
            }
            "auto-at" => config.auto_at = parse_size(value).unwrap_or(config.auto_at),
            "keep-days" => config.keep_days = value.parse().unwrap_or(config.keep_days),
            _ => {}
        }
    }
    return config;
}

fn command_config(arguments: &[String]) -> Fallible<ExitCode> {
    let path = config_path().ok_or("cannot find a config directory (no HOME)")?;
    let config = load_config();

    let key = match arguments.first() {
        Some(key) => key.as_str(),
        None => {
            println!("warn       {}", if config.warn_at == 0 { "off".to_string() } else { human(config.warn_at) });
            println!("auto       {}", match config.auto {
                Auto::Off => "off",
                Auto::Caches => "caches",
                Auto::Full => "full",
            });
            println!("auto-at    {}", human(config.auto_at));
            println!("keep-days  {}", config.keep_days);
            println!("\n{}", path.display());
            return Ok(ExitCode::SUCCESS);
        }
    };

    let value = match arguments.get(1) {
        Some(value) => value.clone(),
        None => return fail(format!("`nail config {}` needs a value", key)),
    };

    // Validate before writing, so a typo cannot silently turn a setting off.
    match key {
        "warn" | "auto-at" => {
            if parse_size(&value).is_none() && value != "0" {
                return fail(format!("`{}` is not a size like 2GB", value));
            }
        }
        "auto" => {
            if !matches!(value.as_str(), "off" | "caches" | "full") {
                return fail("auto is off, caches or full");
            }
        }
        "keep-days" => {
            if value.parse::<u64>().is_err() {
                return fail(format!("`{}` is not a number of days", value));
            }
        }
        _ => return fail(format!("unknown setting `{}`. Try warn, auto, auto-at or keep-days", key)),
    }

    let mut settings: BTreeMap<String, String> = BTreeMap::new();
    settings.insert("warn".to_string(), if config.warn_at == 0 { "0".to_string() } else { human(config.warn_at).replace(' ', "") });
    settings.insert(
        "auto".to_string(),
        match config.auto {
            Auto::Off => "off".to_string(),
            Auto::Caches => "caches".to_string(),
            Auto::Full => "full".to_string(),
        },
    );
    settings.insert("auto-at".to_string(), human(config.auto_at).replace(' ', ""));
    settings.insert("keep-days".to_string(), config.keep_days.to_string());
    settings.insert(key.to_string(), value.clone());

    let mut text = String::from("# Written by `nail config`.\n");
    for (name, setting) in &settings {
        let _ = writeln!(text, "{} = \"{}\"", name, setting);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("cannot create {}: {}", parent.display(), error))?;
    }
    fs::write(&path, text).map_err(|error| format!("cannot write {}: {}", path.display(), error))?;
    println!("{} = {}", key, value);
    return Ok(ExitCode::SUCCESS);
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn command_install(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let wanted = arguments.first().map(|text| text.as_str()).unwrap_or("latest");
    let version = if wanted == "latest" { published_latest(store)? } else { wanted.parse::<Version>().map_err(|_| format!("`{}` is not a version like 0.3.1", wanted))? };
    install(store, &version)?;
    return Ok(ExitCode::SUCCESS);
}

fn command_remove(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let wanted = arguments.first().ok_or("usage: nail remove <version>")?;
    let version: Version = wanted.parse().map_err(|_| format!("`{}` is not a version like 0.3.1", wanted))?;
    if !store.is_installed(&version) {
        return fail(format!("{} is not installed", version));
    }
    let usage = measure(store, &version);
    fs::remove_dir_all(store.version_dir(&version)).map_err(|error| format!("cannot remove {}: {}", version, error))?;
    println!("removed {}, freeing {}", version, human(usage.total()));
    return Ok(ExitCode::SUCCESS);
}

fn command_list(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    if arguments.iter().any(|argument| argument == "--available") {
        let latest = published_latest(store)?;
        println!("newest published: {}", latest);
        println!("(nail install <version> fetches any release, published ones are not listed)");
        return Ok(ExitCode::SUCCESS);
    }

    let installed = store.installed();
    if installed.is_empty() {
        println!("no versions installed. `nail install latest` fetches one");
        return Ok(ExitCode::SUCCESS);
    }

    let newest = store.newest_installed();
    println!("{:<14} {:>10} {:>10} {:>12}", "VERSION", "SIZE", "CACHE", "LAST USED");
    for version in &installed {
        let usage = measure(store, version);
        let used = match days_since(store.last_used(version)) {
            Some(0) => "today".to_string(),
            Some(1) => "yesterday".to_string(),
            Some(days) => format!("{} days ago", days),
            None => "never".to_string(),
        };
        let marker = if Some(version) == newest.as_ref() { " *" } else { "" };
        println!("{:<14} {:>10} {:>10} {:>12}{}", version.to_string(), human(usage.total()), human(usage.cache), used, marker);
    }
    println!("\n* newest installed, which is what `nail latest` and unpinned files use");
    return Ok(ExitCode::SUCCESS);
}

fn command_which(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let named = arguments.iter().find(|argument| !argument.starts_with("--")).ok_or("usage: nail which <file>")?;
    let path = with_extension(named);
    let pin = read_pin(&path)?;
    let installed = store.installed();

    let (version, reason) = match &pin {
        Some(Pin::Exact(version)) => (Some(version.clone()), Reason::Pinned(path.clone())),
        Some(Pin::Latest) => (installed.last().cloned(), Reason::TracksLatest(path.clone())),
        None => (installed.last().cloned(), Reason::Unpinned(path.clone())),
    };

    match version {
        Some(version) => {
            println!("{}", reason.describe(&version));
            if store.is_installed(&version) {
                println!("{}", store.version_dir(&version).join("bin/nail").display());
            } else {
                println!("not installed, opening the file would fetch it");
            }
        }
        None => println!("{} pins no version and none is installed", display_path(&path)),
    }
    return Ok(ExitCode::SUCCESS);
}

/// Walks a tree and installs every version its files pin. Run it once and the
/// machine can go offline with every file in the tree still openable.
fn command_fetch(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let root = arguments.first().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let files = nail_files(&root);
    let mut wanted: Vec<Version> = Vec::new();
    for file in &files {
        if let Ok(Some(Pin::Exact(version))) = read_pin(file) {
            if !version.is_prerelease() && !wanted.contains(&version) {
                wanted.push(version);
            }
        }
    }
    wanted.sort();

    let missing: Vec<Version> = wanted.iter().filter(|version| !store.is_installed(version)).cloned().collect();
    println!("{} Nail files, pinning {} version(s), {} missing", files.len(), wanted.len(), missing.len());
    for version in &missing {
        install(store, version)?;
    }
    if missing.is_empty() {
        println!("nothing to fetch");
    }
    return Ok(ExitCode::SUCCESS);
}

/// Migration. The launcher finds the files and makes sure the target compiler is
/// present, then that compiler decides file by file whether the move is safe.
/// A file that no longer compiles keeps its old version line and keeps working,
/// because migration is a choice and never a requirement.
fn command_update(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let paths: Vec<&String> = arguments.iter().filter(|argument| !argument.starts_with("--")).collect();
    let root = paths.first().map(|path| PathBuf::from(path.as_str())).unwrap_or_else(|| PathBuf::from("."));
    let apply = arguments.iter().any(|argument| argument == "--yes");

    let target = match arguments.iter().find_map(|argument| argument.strip_prefix("--to=")) {
        Some(text) => text.parse::<Version>().map_err(|_| format!("`{}` is not a version like 0.3.1", text))?,
        None => newest_or_install(store)?,
    };
    ensure_installed(store, &target)?;

    // Files that track `latest` are left alone: there is nothing to move.
    let files: Vec<PathBuf> = nail_files(&root)
        .into_iter()
        .filter(|file| match read_pin(file) {
            Ok(Some(Pin::Exact(version))) => version != target,
            Ok(Some(Pin::Latest)) => false,
            Ok(None) => true,
            Err(_) => false,
        })
        .collect();

    if files.is_empty() {
        println!("every file under {} is already at {} or tracks latest", root.display(), target);
        return Ok(ExitCode::SUCCESS);
    }

    println!("{} file(s) would move to {}", files.len(), target);
    if !apply {
        for file in files.iter().take(20) {
            println!("  {}", file.display());
        }
        if files.len() > 20 {
            println!("  ... and {} more", files.len() - 20);
        }
        println!("\nnothing changed. Add --yes to check each file and restamp the ones that pass");
        return Ok(ExitCode::SUCCESS);
    }

    let nailc = store.version_dir(&target).join("bin/nailc");
    let mut moved = 0;
    let mut refused = Vec::new();
    for file in &files {
        let mut command = Command::new(&nailc);
        command.arg(file).arg(format!("--stamp={}", target)).stdout(Stdio::null()).stderr(Stdio::null());
        match command.status() {
            Ok(status) if status.success() => moved += 1,
            _ => refused.push(file.clone()),
        }
    }

    println!("{} file(s) moved to {}", moved, target);
    if !refused.is_empty() {
        println!("{} file(s) do not compile under {} and were left alone:", refused.len(), target);
        for file in &refused {
            println!("  {}", file.display());
        }
        return Ok(ExitCode::FAILURE);
    }
    return Ok(ExitCode::SUCCESS);
}

/// Starts a file that already compiles. The compiler requires a version line,
/// so a file created by hand outside the IDE would otherwise be refused before
/// it ever ran. The launcher is the one thing that knows which versions exist, so
/// writing that line is its job.
fn command_new(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let path = with_extension(arguments.first().ok_or("usage: nail new <file>")?);
    if path.exists() {
        return fail(format!("{} already exists", path.display()));
    }

    // A concrete version, like everywhere else. `nail latest` is a preference
    // somebody states on purpose, never one a tool picks for them.
    let pin = if arguments.iter().any(|argument| argument == "--latest") { Pin::Latest } else { Pin::Exact(newest_or_install(store)?) };

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| format!("cannot create {}: {}", parent.display(), error))?;
        }
    }
    let body = format!("nail {}\nprint(`hello from nail`);\n", pin);
    fs::write(&path, body).map_err(|error| format!("cannot write {}: {}", path.display(), error))?;

    // Creating is not opening. mkdir does not enter the directory and touch
    // does not start an editor, and a command that launched a full screen one
    // would have to be quit before the shell came back, which is a trap when
    // the file was all that was wanted. The next step is printed instead.
    println!("{} (nail {})", path.display(), pin);
    if std::io::stdout().is_terminal() {
        let name = path.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_else(|| path.display().to_string());
        println!("  nail open {}   to edit it", name);
    }
    return Ok(ExitCode::SUCCESS);
}

/// Opens the website. The whole reference lives there, including the standard
/// library listing, which the compiler prints rather than anyone maintaining
/// it by hand.
fn command_website(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let url = match arguments.first().map(|argument| argument.as_str()) {
        Some("library") | Some("stdlib") => format!("{}/stdlib", store.origin),
        Some("downloads") => format!("{}/downloads", store.origin),
        _ => store.origin.clone(),
    };
    return open_url(&url);
}

/// Hands a URL to the desktop, and prints it either way. Over ssh or on a
/// server there is no browser to hand it to, and xdg-open can report success
/// while doing nothing at all, so printing is what makes this never a mystery.
///
/// The desktop is only asked when a person is watching. Piped or captured
/// output means something is reading the address rather than wanting a window,
/// and a window is not a thing to open behind somebody's back: the launcher's
/// own test suite runs every command with its output captured, and used to
/// open two browser tabs every time anyone ran it.
fn open_url(url: &str) -> Fallible<ExitCode> {
    println!("{}", url);
    if std::io::stdout().is_terminal() {
        let _ = Command::new("xdg-open").arg(url).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    return Ok(ExitCode::SUCCESS);
}

/// Runs every Nail file under `tests/`. A test is an ordinary program: assert
/// and the test_* functions panic, so a non-zero exit is a failure and there is
/// no convention to learn beyond where the file lives.
///
/// Output is quiet for a passing test and complete for a failing one, because
/// the output of eighteen passing programs is what buries the two that failed.
fn command_test(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let pattern = arguments.iter().find(|argument| !argument.starts_with("--"));
    let root = PathBuf::from("tests");
    if !root.is_dir() {
        return fail("no tests/ directory here. A test is any Nail file in it");
    }

    let files: Vec<PathBuf> = nail_files(&root)
        .into_iter()
        .filter(|file| match pattern {
            Some(pattern) => file.to_string_lossy().contains(pattern.as_str()),
            None => true,
        })
        .collect();

    if files.is_empty() {
        match pattern {
            Some(pattern) => return fail(format!("no test in tests/ matches '{}'", pattern)),
            None => return fail("tests/ has no Nail files in it"),
        }
    }

    let resolved = resolve(store, &[files[0].to_string_lossy().to_string()])?;
    let nailc = store.version_dir(&resolved.version).join("bin/nailc");

    println!("running {} test(s) with Nail {}", files.len(), resolved.version);
    let mut failed = Vec::new();
    for file in &files {
        let output = Command::new(&nailc).arg(file).arg("--run").output();
        let name = file.strip_prefix(&root).unwrap_or(file).display().to_string();
        match output {
            Ok(output) if output.status.success() => println!("  ok    {}", name),
            Ok(output) => {
                println!("  FAIL  {}", name);
                failed.push((name, String::from_utf8_lossy(&output.stderr).to_string()));
            }
            Err(error) => {
                println!("  FAIL  {}", name);
                failed.push((name, error.to_string()));
            }
        }
    }

    if failed.is_empty() {
        println!("\n{} passed", files.len());
        return Ok(ExitCode::SUCCESS);
    }
    for (name, message) in &failed {
        println!("\n--- {} ---", name);
        println!("{}", failure_reason(message));
    }
    println!("\n{} passed, {} failed", files.len() - failed.len(), failed.len());
    return Ok(ExitCode::FAILURE);
}

/// The part of a failed run worth reading. A test fails either because it did
/// not compile or because it panicked, and in both cases everything cargo said
/// about crates on the way there is noise.
fn failure_reason(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();

    if let Some(panic_at) = lines.iter().position(|line| line.contains("panicked at")) {
        // The message sits under the location, and the backtrace note under
        // that is advice nobody asked for.
        return lines[panic_at..].iter().filter(|line| !line.starts_with("note: run with")).take(4).cloned().collect::<Vec<_>>().join("\n");
    }

    if let Some(first_error) = lines.iter().position(|line| line.starts_with("error")) {
        return lines[first_error..].iter().take(10).cloned().collect::<Vec<_>>().join("\n");
    }

    return lines.iter().rev().take(5).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
}

fn command_export(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let wanted = arguments.first().ok_or("usage: nail export <version> <file.tar>")?;
    let version: Version = wanted.parse().map_err(|_| format!("`{}` is not a version like 0.3.1", wanted))?;
    if !store.is_installed(&version) {
        return fail(format!("{} is not installed", version));
    }
    let destination = arguments.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("nail-{}-{}.tar", version, TARGET)));
    require_unpack_tools()?;
    println!("packing {} (this takes a while, it is gigabytes)", version);
    let status = Command::new("tar")
        .arg("-cf")
        .arg(&destination)
        .arg("-C")
        .arg(store.versions_dir())
        .arg(version.to_string())
        .status()
        .map_err(|error| format!("cannot run tar: {}", error))?;
    if !status.success() {
        return fail("tar could not pack the release");
    }
    println!("{}", destination.display());
    return Ok(ExitCode::SUCCESS);
}

fn command_import(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let source = arguments.first().map(PathBuf::from).ok_or("usage: nail import <file.tar>")?;
    require_unpack_tools()?;
    let versions = store.versions_dir();
    fs::create_dir_all(&versions).map_err(|error| format!("cannot create {}: {}", versions.display(), error))?;

    // A release is built for one path and used at that path, so a release
    // exported from another machine unpacks and works as-is.
    unpack(&source, &versions)?;

    println!("imported. `nail list` to see what arrived");
    return Ok(ExitCode::SUCCESS);
}

fn command_doctor(store: &Store) -> Fallible<ExitCode> {
    let mut problems = 0;
    let mut report = |ok: bool, message: String| {
        println!("{} {}", if ok { "ok  " } else { "FAIL" }, message);
        if !ok {
            problems += 1;
        }
    };

    println!("store {}\n", store.root.display());

    report(have("tar"), "tar is available for unpacking releases".to_string());
    report(have("xz"), "xz is available, which is what tar unpacks a release with".to_string());

    let versions = store.versions_dir();
    let writable = can_write(&versions);
    report(writable, format!("{} can be written to", versions.display()));
    if !writable {
        println!("     {}", permission_hint(store));
    }

    let installed = store.installed();
    report(!installed.is_empty(), format!("{} version(s) installed", installed.len()));
    for version in &installed {
        let root = store.version_dir(version);
        for needed in ["bin/nail", "bin/nailc", "toolchain/bin/cargo", "nail/Cargo.toml"] {
            report(root.join(needed).exists(), format!("{} has {}", version, needed));
        }
    }

    match std::env::var_os("PATH") {
        Some(path) => {
            let shadowed = std::env::split_paths(&path).any(|directory| directory.starts_with(&versions));
            report(!shadowed, "no installed version is on PATH (they must not be)".to_string());
        }
        None => report(false, "PATH is not set".to_string()),
    }

    if problems == 0 {
        println!("\nnothing wrong");
        return Ok(ExitCode::SUCCESS);
    }
    println!("\n{} problem(s)", problems);
    return Ok(ExitCode::FAILURE);
}

fn command_self_update(store: &Store) -> Fallible<ExitCode> {
    let url = format!("{}/nail/{}", store.origin, TARGET);
    let current = std::env::current_exe().map_err(|error| format!("cannot find my own path: {}", error))?;

    let staging = current.with_extension("incoming");
    let _ = download(&url, &staging, "nail")?;
    let _guard = FileCleanup(staging.clone());

    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).map_err(|error| format!("cannot make it executable: {}", error))?;
    // A rename over a running binary is fine on Linux: the old inode stays
    // alive until this process exits.
    fs::rename(&staging, &current).map_err(|error| {
        format!(
            "cannot replace {}: {}\n\
             nail lives in the store so it can update itself without root. If it was \
             installed somewhere else, move it back or re-run the installer.",
            current.display(),
            error
        )
    })?;
    std::mem::forget(_guard);
    println!("nail updated");
    return Ok(ExitCode::SUCCESS);
}

struct FileCleanup(PathBuf);

impl Drop for FileCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Reclaiming disk
// ---------------------------------------------------------------------------

/// What a `gc` run would do, worked out before anything is deleted.
struct Plan {
    trim: Vec<(Version, u64)>,
    remove: Vec<(Version, u64)>,
}

impl Plan {
    fn reclaimable(&self) -> u64 {
        return self.trim.iter().map(|(_, size)| size).sum::<u64>() + self.remove.iter().map(|(_, size)| size).sum::<u64>();
    }

    fn is_empty(&self) -> bool {
        return self.trim.is_empty() && self.remove.is_empty();
    }
}

/// Works out what could go. Three things are never touched: the newest
/// installed version, anything used inside the keep window, and (when a
/// version is running) the one that is running.
fn plan_gc(store: &Store, config: &Config, caches_only: bool, in_use: Option<&Version>) -> Plan {
    let installed = store.installed();
    let newest = installed.last().cloned();
    let mut plan = Plan { trim: Vec::new(), remove: Vec::new() };

    for version in &installed {
        if Some(version) == newest.as_ref() || Some(version) == in_use {
            continue;
        }
        let idle = days_since(store.last_used(version)).unwrap_or(u64::MAX);
        if idle < config.keep_days {
            continue;
        }
        let usage = measure(store, version);
        // Six times the keep window before a version is uninstalled: a cache
        // costs minutes to rebuild, a version costs a long download.
        if !caches_only && idle >= config.keep_days.saturating_mul(6) {
            plan.remove.push((version.clone(), usage.total()));
        } else if usage.cache > 0 {
            plan.trim.push((version.clone(), usage.cache));
        }
    }
    return plan;
}

fn command_gc(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let mut config = load_config();
    if let Some(text) = arguments.iter().find_map(|argument| argument.strip_prefix("--keep-days=")) {
        config.keep_days = text.parse().map_err(|_| format!("`{}` is not a number of days", text))?;
    }
    let caches_only = arguments.iter().any(|argument| argument == "--caches");
    let apply = arguments.iter().any(|argument| argument == "--yes");

    let plan = plan_gc(store, &config, caches_only, None);
    if plan.is_empty() {
        println!("nothing to reclaim");
        return Ok(ExitCode::SUCCESS);
    }

    for (version, size) in &plan.trim {
        println!("trim   {:<14} {:>10}  build cache, rebuilt on next use", version.to_string(), human(*size));
    }
    for (version, size) in &plan.remove {
        println!("remove {:<14} {:>10}  unused for {} days", version.to_string(), human(*size), days_since(store.last_used(version)).unwrap_or(0));
    }
    println!("\n{} reclaimable", human(plan.reclaimable()));

    if !apply {
        println!("nothing changed. Add --yes to do it");
        return Ok(ExitCode::SUCCESS);
    }

    apply_gc(store, &plan);
    println!("reclaimed {}", human(plan.reclaimable()));
    return Ok(ExitCode::SUCCESS);
}

fn apply_gc(store: &Store, plan: &Plan) {
    for (version, _) in &plan.trim {
        let _ = fs::remove_dir_all(store.version_dir(version).join("cache"));
    }
    for (version, _) in &plan.remove {
        let _ = fs::remove_dir_all(store.version_dir(version));
    }
}

/// One line on the way past, when there is enough to reclaim to be worth
/// saying. Users will not run `gc` on their own and the disk will fill, but
/// deleting gigabytes without being asked is worse than nagging.
fn maybe_nag(store: &Store) {
    let config = load_config();
    if config.warn_at == 0 && config.auto == Auto::Off {
        return;
    }
    // Measuring means walking every version, so it happens once a day at most.
    let stamp = store.root.join(".last-measured");
    if days_since(fs::metadata(&stamp).and_then(|meta| meta.modified()).ok()) == Some(0) {
        return;
    }
    let _ = fs::write(&stamp, b"");

    let plan = plan_gc(store, &config, config.auto == Auto::Caches, store.newest_installed().as_ref());
    let reclaimable = plan.reclaimable();
    if reclaimable == 0 {
        return;
    }

    if config.auto != Auto::Off && reclaimable >= config.auto_at {
        // Detached, because The launcher is about to replace itself with the
        // compiler and cannot clean up afterwards. Nothing here touches the
        // version being launched or the newest one.
        let _ = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("nail"))).arg("gc").arg("--yes").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
        eprintln!("nail: reclaiming {} in the background (nail config auto off)", human(reclaimable));
        return;
    }

    if config.warn_at > 0 && reclaimable >= config.warn_at {
        let caches: u64 = plan.trim.iter().map(|(_, size)| size).sum();
        let versions: u64 = plan.remove.iter().map(|(_, size)| size).sum();
        eprintln!("nail: {} reclaimable ({} stale build caches, {} unused versions)", human(reclaimable), human(caches), human(versions));
        eprintln!("      run `nail gc`");
    }
}

// ---------------------------------------------------------------------------
// Walking a tree
// ---------------------------------------------------------------------------

/// Every `.nail` file under a path. Build output, version control and the
/// vendor folder are skipped: vendored source pinned by its author is not
/// ours to migrate.
fn nail_files(root: &Path) -> Vec<PathBuf> {
    const SKIP: [&str; 5] = [".git", "target", "vendor", "node_modules", ".nail"];
    let mut found = Vec::new();
    if root.is_file() {
        if looks_like_nail_file(&root.to_string_lossy()) {
            found.push(root.to_path_buf());
        }
        return found;
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return found,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                if !SKIP.contains(&name.as_str()) {
                    found.extend(nail_files(&path));
                }
            }
            // `.nail` itself is the editor's settings file, not source, and
            // restamping it would corrupt it.
            Ok(kind) if kind.is_file() && name.ends_with(".nail") && !name.starts_with('.') => found.push(path),
            _ => {}
        }
    }
    found.sort();
    return found;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_parse_and_print() {
        assert_eq!(parse_size("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("500MB"), Some(500 * 1024 * 1024));
        assert_eq!(parse_size("2.5GB"), Some(2684354560));
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("banana"), None);
        assert_eq!(human(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(human(0), "0 B");
    }

    #[test]
    fn hex_round_trips() {
        let bytes = vec![0x00, 0x5a, 0xff, 0x10];
        assert_eq!(unhex(&hex(&bytes)), Some(bytes));
        assert_eq!(unhex("zz"), None);
        assert_eq!(unhex("abc"), None);
    }

    #[test]
    fn the_entry_file_is_the_first_nail_path() {
        let arguments: Vec<String> = ["--check-only", "lib.nail", "main.nail"].iter().map(|text| text.to_string()).collect();
        assert_eq!(entry_file(&arguments), Some(PathBuf::from("lib.nail")));
        assert_eq!(entry_file(&["--help".to_string()]), None);
    }

    #[test]
    fn a_version_can_be_read_out_of_a_release_url() {
        assert_eq!(version_in("https://example.com/versions/0.3.1/x86_64-linux"), "0.3.1");
    }
}
