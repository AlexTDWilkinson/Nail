//! Hammer: the thing you use to open Nail files.
//!
//! It does four things and nothing else. It reads which version of Nail a file
//! was written for, makes sure that exact version is on the machine, checks
//! that it really came from us, and hands the file to it. Everything else a
//! Nail toolchain does belongs to a release, not to Hammer.
//!
//! That restraint is the whole design. Hammer is the one piece that can never
//! be replaced by a newer Nail, because it is what launches Nail in the first
//! place. So the things it promises have to hold forever:
//!
//!   * the version line grammar (see the `version_line` module, included by path below)
//!   * the URL shape it fetches from
//!   * the small set of subcommands it owns
//!
//! Everything else is forwarded. `hammer fmt old.nail` runs the formatter that
//! shipped with `old.nail`'s own compiler, not today's, so commands invented in
//! ten years work through a Hammer built today without it ever being taught
//! about them. Only commands that are about the *set* of installed versions,
//! which no single version can answer, belong to Hammer itself.
//!
//! The version line parser is shared with the compiler by including its source
//! directly rather than by depending on the nail library, so Hammer stays a
//! small self-contained binary that links none of the language.

// Hammer only reads version lines. Writing them is the compiler's half of the same
// module, so those functions are dead code here and live code there.
#[allow(dead_code)]
#[path = "../version_line.rs"]
mod version_line;

use version_line::{Pin, Version};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Frozen constants
// ---------------------------------------------------------------------------

/// Where installed versions live. Each is built at exactly the path it is
/// installed to, because cargo fingerprints embed absolute paths and a
/// bundle's pre-warmed cache is only valid at the path it was warmed at.
const STORE: &str = "/opt/nail";

/// The one URL Hammer knows, forever. Overridable for testing a release
/// before it is announced.
const DEFAULT_ORIGIN: &str = "https://nail.alex-wilkinson.ca";

/// The only target that exists today. It is in the URL anyway, so adding
/// others later does not change the shape of a request already in the wild.
const TARGET: &str = "x86_64-linux";

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
            eprintln!("hammer: {}", message);
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Fallible<ExitCode> {
    let store = Store::new();

    // Called as `nail` or `nailc` rather than as `hammer`: the same binary
    // under three names, so muscle memory, `#!/usr/bin/env nail` shebangs and
    // Makefiles that call `nailc` all keep working while only one thing is
    // ever on PATH.
    let called_as = Path::new(&arguments[0]).file_name().and_then(|name| name.to_str()).unwrap_or("hammer");
    let rest = &arguments[1..];
    match called_as {
        "nail" => return launch(&store, Binary::Ide, rest),
        "nailc" => return launch(&store, Binary::Compiler, rest),
        _ => {}
    }

    let command = match rest.first() {
        Some(first) => first.as_str(),
        None => return launch(&store, Binary::Ide, &[]),
    };
    let tail = &rest[1..];

    // Hammer owns exactly the commands that are about the set of installed
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
        "run" => launch(&store, Binary::Compiler, &append(tail, "--run")),
        "open" => launch(&store, Binary::Ide, tail),
        "help" | "--help" | "-h" => {
            print!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        "--version" => {
            println!("hammer {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        // The escape hatch, for forwarding something that collides with a
        // reserved word above.
        "--" => launch(&store, Binary::Compiler, tail),
        // A bare file opens in the IDE. Anything else is a subcommand Hammer
        // has never heard of, which is the normal case, not an error: it
        // belongs to a release and is forwarded to one.
        _ if looks_like_nail_file(command) => launch(&store, Binary::Ide, rest),
        _ => launch(&store, Binary::Compiler, rest),
    }
}

fn usage() -> String {
    return concat!(
        "hammer - opens Nail files with the version of Nail they were written for\n",
        "\n",
        "  hammer <file.nail>          open it in the IDE that wrote it\n",
        "  hammer run <file.nail>      compile and run it\n",
        "  hammer <anything else>      forwarded to the resolved version's nailc\n",
        "\n",
        "Managing installed versions:\n",
        "  install <version|latest>    download a release\n",
        "  remove <version>            delete one\n",
        "  list [--available]          what is installed, how big, last used\n",
        "  gc [--caches] [--yes]       reclaim disk, dry run unless --yes\n",
        "  which <file.nail>           print the resolved version and why\n",
        "  fetch <path>                install every version the tree pins\n",
        "  update <path> [--to <v>]    migrate files that still compile\n",
        "  export <version> <file>     save a release for an offline machine\n",
        "  import <file>               install one from that file\n",
        "  doctor                      check the install over\n",
        "  self-update                 replace hammer itself\n",
        "  config <key> [value]        warn, auto, auto-at, keep-days\n",
        "  new <file.nail> [--latest]  start a file, stamped and ready to compile\n"
    )
    .to_string();
}

/// nailc takes the file first and the mode after it, so a flag Hammer adds
/// goes on the end.
fn append(rest: &[String], last: &str) -> Vec<String> {
    let mut all = rest.to_vec();
    all.push(last.to_string());
    return all;
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

impl Store {
    fn new() -> Self {
        // NAIL_STORE is for tests and for trying a layout without touching a
        // real install. NAIL_ORIGIN is for validating a release before it is
        // announced.
        let root = std::env::var_os("NAIL_STORE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(STORE));
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
                eprintln!("hammer: Nail {} is out, fetching it (this file tracks latest)", published);
                install(store, &published)?;
            }
            Ok(published)
        }
        Err(_) => match store.newest_installed() {
            Some(installed) => {
                eprintln!("hammer: cannot reach {} to check for a newer Nail, using {}", store.origin, installed);
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
    eprintln!("hammer: no version of Nail installed yet, fetching the newest");
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
             hammer cannot fetch it. Build it, or restamp the file with `hammer update`",
            version
        ));
    }
    eprintln!("hammer: {} is not installed, fetching it", version);
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
        eprintln!("hammer: {}", resolved.reason.describe(&resolved.version));
    }

    maybe_nag(store);

    let program = store.version_dir(&resolved.version).join("bin").join(binary.file_name());
    if !program.is_file() {
        return fail(format!("{} is installed but has no {}. Run `hammer doctor`", resolved.version, binary.file_name()));
    }

    // Arguments meant for Hammer are not passed on.
    let forwarded: Vec<&String> = arguments.iter().filter(|argument| !argument.starts_with("--nail-version=")).collect();

    // exec rather than spawn, so signals, the exit code and the terminal all
    // belong to the compiler. Hammer is not in the picture once it has chosen.
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
        .user_agent(concat!("hammer/", env!("CARGO_PKG_VERSION")))
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
    require_tar()?;

    let versions = store.versions_dir();
    fs::create_dir_all(&versions).map_err(|error| {
        format!(
            "cannot create {}: {}\n\
             If this is a fresh machine, run the one-time setup:\n\
             sudo mkdir -p {} && sudo chown $USER {}",
            versions.display(),
            error,
            versions.display(),
            store.root.display()
        )
    })?;

    let url = release_url(store, version);
    let staging = versions.join(format!(".incoming-{}", version));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|error| format!("cannot create {}: {}", staging.display(), error))?;
    let cleanup = Cleanup(staging.clone());

    let tarball = staging.join("release.tar.xz");
    println!("downloading Nail {}", version);
    let downloaded = download(&url, &tarball)?;

    println!("unpacking");
    unpack(&tarball, &staging)?;

    // The bundle packs as a single directory named for its version.
    let unpacked = staging.join(version.to_string());
    if !unpacked.join("bin/nailc").is_file() {
        return fail(format!("the {} release does not contain bin/nailc", version));
    }
    fs::write(unpacked.join(".installed"), format!("sha256 {}\n", hex(&sha256(&downloaded)))).map_err(|error| format!("cannot record the install: {}", error))?;

    let destination = store.version_dir(version);
    let _ = fs::remove_dir_all(&destination);
    fs::rename(&unpacked, &destination).map_err(|error| format!("cannot move {} into place: {}", version, error))?;
    drop(cleanup);

    store.touch(version);
    println!("Nail {} installed", version);
    return Ok(());
}

/// Removes a directory when it goes out of scope, so a failed install leaves
/// no half-downloaded gigabytes behind.
struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn download(url: &str, destination: &Path) -> Fallible<Vec<u8>> {
    let mut response = get(url)?;
    match response.status().as_u16() {
        200 => {}
        404 => return fail(format!("there is no Nail {}", version_in(url))),
        status => return fail(format!("{} answered {}", url, status)),
    }

    let mut file = fs::File::create(destination).map_err(|error| format!("cannot write {}: {}", destination.display(), error))?;
    let mut bytes = Vec::new();
    response.read_to_end(&mut bytes).map_err(|error| format!("the download from {} was cut short: {}", url, error))?;
    file.write_all(&bytes).map_err(|error| format!("cannot write {}: {}", destination.display(), error))?;
    return Ok(bytes);
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

fn require_tar() -> Fallible<()> {
    let found = Command::new("tar").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok();
    if found {
        Ok(())
    } else {
        fail("`tar` is not installed, and hammer needs it to unpack releases")
    }
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

/// What `gc` does on its own, if anything. Written by `hammer config` so
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
    return Some(base.join("nail/hammer.toml"));
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
        None => return fail(format!("`hammer config {}` needs a value", key)),
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

    let mut text = String::from("# Written by `hammer config`.\n");
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
    let wanted = arguments.first().ok_or("usage: hammer remove <version>")?;
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
        println!("(hammer install <version> fetches any release, published ones are not listed)");
        return Ok(ExitCode::SUCCESS);
    }

    let installed = store.installed();
    if installed.is_empty() {
        println!("no versions installed. `hammer install latest` fetches one");
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
    let path = entry_file(arguments).ok_or("usage: hammer which <file.nail>")?;
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

/// Migration. Hammer finds the files and makes sure the target compiler is
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
/// it ever ran. Hammer is the one thing that knows which versions exist, so
/// writing that line is its job.
fn command_new(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let path = arguments.first().map(PathBuf::from).ok_or("usage: hammer new <file.nail>")?;
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
    println!("{} (nail {})", path.display(), pin);
    return Ok(ExitCode::SUCCESS);
}

fn command_export(store: &Store, arguments: &[String]) -> Fallible<ExitCode> {
    let wanted = arguments.first().ok_or("usage: hammer export <version> <file.tar>")?;
    let version: Version = wanted.parse().map_err(|_| format!("`{}` is not a version like 0.3.1", wanted))?;
    if !store.is_installed(&version) {
        return fail(format!("{} is not installed", version));
    }
    let destination = arguments.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("nail-{}-{}.tar", version, TARGET)));
    require_tar()?;
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
    let source = arguments.first().map(PathBuf::from).ok_or("usage: hammer import <file.tar>")?;
    require_tar()?;
    let versions = store.versions_dir();
    fs::create_dir_all(&versions).map_err(|error| format!("cannot create {}: {}", versions.display(), error))?;
    unpack(&source, &versions)?;
    println!("imported. `hammer list` to see what arrived");
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

    report(require_tar().is_ok(), "tar is available for unpacking releases".to_string());

    let versions = store.versions_dir();
    let writable = versions.is_dir() && fs::metadata(&versions).map(|meta| meta.permissions().mode() & 0o200 != 0).unwrap_or(false);
    report(writable, format!("{} exists and is writable", versions.display()));
    if !writable {
        println!("     one-time setup: sudo mkdir -p {} && sudo chown $USER {}", versions.display(), store.root.display());
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
    let url = format!("{}/hammer/{}", store.origin, TARGET);
    let current = std::env::current_exe().map_err(|error| format!("cannot find my own path: {}", error))?;

    println!("downloading a new hammer");
    let staging = current.with_extension("incoming");
    let bytes = download(&url, &staging)?;
    let _guard = FileCleanup(staging.clone());

    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).map_err(|error| format!("cannot make it executable: {}", error))?;
    // A rename over a running binary is fine on Linux: the old inode stays
    // alive until this process exits.
    fs::rename(&staging, &current).map_err(|error| {
        format!(
            "cannot replace {}: {}\n\
             hammer lives in the store so it can update itself without root. If it was \
             installed somewhere else, move it back or re-run the installer.",
            current.display(),
            error
        )
    })?;
    std::mem::forget(_guard);
    println!("hammer updated");
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
        // Detached, because Hammer is about to replace itself with the
        // compiler and cannot clean up afterwards. Nothing here touches the
        // version being launched or the newest one.
        let _ = Command::new(std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hammer"))).arg("gc").arg("--yes").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
        eprintln!("hammer: reclaiming {} in the background (hammer config auto off)", human(reclaimable));
        return;
    }

    if config.warn_at > 0 && reclaimable >= config.warn_at {
        let caches: u64 = plan.trim.iter().map(|(_, size)| size).sum();
        let versions: u64 = plan.remove.iter().map(|(_, size)| size).sum();
        eprintln!("nail: {} reclaimable ({} stale build caches, {} unused versions)", human(reclaimable), human(caches), human(versions));
        eprintln!("      run `hammer gc`");
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
